use super::*;

impl TerminalApp {
    fn open_remote(
        &mut self,
        connection: remote::Connection,
        placement: usize,
        ctx: &egui::Context,
    ) -> Result<(), String> {
        let command = connection.command(
            &self.workspace,
            self.storage.parent().unwrap_or(std::path::Path::new(".")),
        )?;
        let pane = Pane {
            id: self.next_id,
            shell: Shell::PowerShell,
            directory: self
                .workspace
                .inherited_directory(self.focused, std::path::Path::new(&self.directory)),
            remote: Some(connection.clone()),
        };
        if !pane.directory.is_dir() {
            return Err("Lokaler Startordner existiert nicht.".into());
        }
        let mut updated = self.workspace.clone();
        if placement == 0 {
            updated.tabs.push(Tab {
                name: connection.label(),
                root: Node::Pane(pane.clone()),
            });
            updated.active_tab = updated.tabs.len() - 1;
        } else {
            let tab = updated
                .tabs
                .get_mut(updated.active_tab)
                .ok_or("Kein aktiver Tab zum Teilen")?;
            if !tab.root.split(
                self.focused,
                if placement == 1 {
                    Axis::Columns
                } else {
                    Axis::Rows
                },
                pane.clone(),
            ) {
                return Err("Bereich konnte nicht geteilt werden (Layoutgrenze).".into());
            }
        }
        updated.validate()?;
        self.start_with_command(ctx, &pane, Some(command));
        if let Some(error) = self.sessions.get(&pane.id).and_then(|s| s.error.clone()) {
            self.sessions.remove(&pane.id);
            return Err(error);
        }
        self.workspace = updated;
        self.next_id += 1;
        self.focused = pane.id;
        self.status = format!(
            "Verbindungsaufbau: {} · Anmeldung im Terminal",
            connection.label()
        );
        Ok(())
    }

    pub(super) fn remote_window(&mut self, ctx: &egui::Context) {
        let Some(mut dialog) = self.remote_dialog.take() else {
            return;
        };
        let (mut open, mut connect, mut save, mut remove) = (true, false, false, false);
        egui::Window::new("Remote-Verbindungen").open(&mut open).default_width(600.0).vscroll(true).show(ctx, |ui| {
            ui.label("Gespeicherte Verbindungen");
            let mut indices: Vec<_> = (0..self.workspace.connections.len()).collect();
            indices.sort_by_key(|&i| !self.workspace.connections[i].favorite);
            egui::ScrollArea::vertical().max_height(120.0).show(ui, |ui| {
                for index in indices {
                    let c = &self.workspace.connections[index];
                    if ui.selectable_label(dialog.selected == Some(index), format!("{}{} · {}", if c.favorite { "★ " } else { "" }, c.name, c.label())).clicked() {
                        dialog.connection = c.clone(); dialog.selected = Some(index);
                        dialog.port = c.port.map(|p| p.to_string()).unwrap_or_default();
                        dialog.error = None; dialog.delete = false;
                    }
                }
            });
            if ui.button("Neue Verbindung").clicked() { dialog = remote::Dialog::default(); }
            ui.separator();
            ui.horizontal(|ui| { ui.label("Name"); ui.text_edit_singleline(&mut dialog.connection.name); ui.checkbox(&mut dialog.connection.favorite, "Favorit"); });
            ui.horizontal(|ui| {
                ui.selectable_value(&mut dialog.connection.transport, remote::Transport::Ssh, "SSH");
                ui.selectable_value(&mut dialog.connection.transport, remote::Transport::WinRm, "PowerShell / WinRM");
            });
            ui.label("Host / IP-Adresse (bei SSH auch Hostalias)"); ui.text_edit_singleline(&mut dialog.connection.host);
            ui.label("Konto (leer = SSH-Konfiguration bzw. aktuelle Windows-Identität)"); ui.text_edit_singleline(&mut dialog.connection.user);
            ui.label("Port (leer = Konfiguration / Standard)"); ui.text_edit_singleline(&mut dialog.port);
            if dialog.connection.transport == remote::Transport::Ssh {
                ui.label("Schlüsseldatei (optional, absoluter Pfad)"); ui.text_edit_singleline(&mut dialog.connection.key_file);
                ui.small("OpenSSH nutzt deine SSH-Konfiguration, den Agent und known_hosts. Hostschlüssel und Anmeldung werden im Terminal abgefragt.");
            } else {
                ui.checkbox(&mut dialog.connection.https, "HTTPS verwenden");
                ui.label("PowerShell-Endpunkt (optional, z. B. PowerShell.7)"); ui.text_edit_singleline(&mut dialog.connection.endpoint);
                ui.small("Mit angegebenem Konto wird das Kennwort beim Verbinden abgefragt. Der Zielserver benötigt einen eingerichteten WinRM-Endpunkt.");
            }
            ui.small("Gespeichert werden nur Verbindungseinstellungen, keine Kennwörter oder Schlüssel-Inhalte. Änderungen gelten für neue Verbindungen; offene Bereiche behalten ihre Konfiguration.");
            ui.horizontal(|ui| {
                ui.selectable_value(&mut dialog.placement, 0, "Neuer Tab");
                ui.selectable_value(&mut dialog.placement, 1, "Nebeneinander");
                ui.selectable_value(&mut dialog.placement, 2, "Untereinander");
            });
            if let Some(error) = &dialog.error { ui.colored_label(Color32::LIGHT_RED, error); }
            ui.horizontal(|ui| {
                connect = ui.button("Verbinden").clicked();
                save = ui.button("Verbindung speichern").clicked();
                if ui.add_enabled(dialog.selected.is_some(), egui::Button::new("Löschen …")).clicked() { dialog.delete = true; }
            });
            if dialog.delete {
                ui.label("Gespeicherte Verbindung löschen? Offene Bereiche und Projektprofile behalten ihre Kopie.");
                ui.horizontal(|ui| {
                    remove = ui.button("Löschen bestätigen").clicked();
                    if ui.button("Behalten").clicked() { dialog.delete = false; }
                });
            }
        });
        if save || connect {
            let result = (|| -> Result<(), String> {
                dialog.connection.name = dialog.connection.name.trim().into();
                dialog.connection.host = dialog.connection.host.trim().into();
                dialog.connection.user = dialog.connection.user.trim().into();
                dialog.connection.port = if dialog.port.trim().is_empty() {
                    None
                } else {
                    Some(
                        dialog
                            .port
                            .trim()
                            .parse()
                            .map_err(|_| "Ungültiger Port (1–65535)")?,
                    )
                };
                if dialog.connection.transport == remote::Transport::WinRm {
                    dialog.connection.key_file.clear();
                }
                dialog.connection.validate()?;
                if save {
                    let previous = self.workspace.clone();
                    let mut updated = previous.clone();
                    let index = if let Some(i) = dialog.selected {
                        updated.connections[i] = dialog.connection.clone();
                        i
                    } else {
                        updated.connections.push(dialog.connection.clone());
                        updated.connections.len() - 1
                    };
                    updated.validate()?;
                    self.workspace = updated;
                    if !self.save_workspace() {
                        self.workspace = previous;
                        return Err(self.status.clone());
                    }
                    dialog.selected = Some(index);
                    self.status = "Verbindung gespeichert".into();
                }
                if connect {
                    self.open_remote(dialog.connection.clone(), dialog.placement, ctx)?;
                }
                Ok(())
            })();
            match result {
                Ok(()) => {
                    dialog.error = None;
                    if connect {
                        return;
                    }
                }
                Err(e) => dialog.error = Some(e),
            }
        }
        if remove {
            if let Some(index) = dialog.selected {
                let previous = self.workspace.clone();
                self.workspace.connections.remove(index);
                if self.save_workspace() {
                    dialog = remote::Dialog::default();
                } else {
                    self.workspace = previous;
                    dialog.error = Some(self.status.clone());
                }
            }
        }
        if open {
            self.remote_dialog = Some(dialog);
        }
    }
}
