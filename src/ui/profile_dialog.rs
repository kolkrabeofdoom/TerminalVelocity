//! Project profile dialog: save and open profiles.

use crate::*;

impl TerminalApp {
    pub(crate) fn show_profiles(&mut self, saving: bool) {
        let directory = self
            .workspace
            .inherited_directory(self.focused, &PathBuf::from(&self.directory));
        let first = self.profiles.profiles.first();
        self.profile_dialog = Some(profiles::Dialog {
            saving,
            name: String::new(),
            directory: if saving {
                directory.display().to_string()
            } else {
                first
                    .map(|p| p.directory.display().to_string())
                    .unwrap_or_default()
            },
            selected: if saving || first.is_none() {
                None
            } else {
                Some(0)
            },
            delete_target: None,
            error: self.profiles_error.clone(),
        });
    }

    pub(crate) fn open_profile(
        &mut self,
        index: usize,
        directory: PathBuf,
        ctx: &egui::Context,
    ) -> Result<(), String> {
        let profile = self
            .profiles
            .profiles
            .get(index)
            .ok_or("Projektprofil fehlt")?;
        let name = profile.name.clone();
        let (workspace, next_id) = profile.prepare(&directory, &self.workspace, self.next_id)?;
        let panes: Vec<_> = workspace
            .panes()
            .into_iter()
            .filter(|p| p.id >= self.next_id)
            .collect();
        let base = self.app_dir();
        for pane in &panes {
            if let Some(remote) = &pane.remote {
                remote.command(&workspace, base)?;
            } else if pane
                .shell
                .resolve(Some(&workspace.shell_paths[pane.shell as usize]), base)
                .is_none()
            {
                return Err(format!(
                    "{} nicht gefunden. Bitte zuerst den Programmpfad unter Einstellungen prüfen.",
                    pane.shell.label()
                ));
            }
        }
        self.workspace = workspace;
        self.next_id = next_id;
        self.directory = directory.display().to_string();
        for pane in panes {
            self.start(ctx, &pane);
        }
        self.focus_first();
        self.status = if self.sessions.values().any(|s| s.error.is_some()) {
            format!("Projekt „{name}“ geöffnet. Startfehler im betroffenen Bereich prüfen.")
        } else {
            format!("Projekt „{name}“ in neuen Tabs geöffnet")
        };
        Ok(())
    }

    pub(crate) fn profile_window(&mut self, ctx: &egui::Context) {
        let Some(mut dialog) = self.profile_dialog.take() else {
            return;
        };
        let mut open = true;
        let mut submit = false;
        let mut cancel = false;
        let mut delete = None;
        let title = if dialog.saving {
            "Projektprofil speichern"
        } else {
            "Projekt öffnen"
        };
        egui::Window::new(title).open(&mut open).default_width(620.0).vscroll(true).show(ctx, |ui| {
            if dialog.saving {
                ui.label("Der aktuelle Aufbau wird als Vorlage gespeichert: Tabs, Teilungen, Anbieter und Bereichsordner.");
                ui.label("Projektname");
                ui.text_edit_singleline(&mut dialog.name);
                ui.label(format!("{} Tabs · {} Bereiche", self.workspace.tabs.len(), self.workspace.panes().len()));
            } else {
                ui.label("Das Projekt startet in neuen Tabs. Laufende Sitzungen bleiben erhalten.");
                if self.profiles.profiles.is_empty() { ui.weak("Noch keine Profile. Richte deine Bereiche ein und wähle oben „Projekt speichern …“."); }
                for (index, profile) in self.profiles.profiles.iter().enumerate() {
                    if ui.selectable_label(dialog.selected == Some(index), &profile.name).clicked() {
                        dialog.selected = Some(index);
                        dialog.directory = profile.directory.display().to_string();
                        dialog.delete_target = None;
                        dialog.error = None;
                    }
                }
            }
            ui.separator();
            ui.label("Projektordner (vollständiger Pfad)");
            ui.text_edit_singleline(&mut dialog.directory);
            ui.small("Bereichsordner innerhalb des gespeicherten Projektordners ziehen beim Öffnen mit um. Ordner außerhalb behalten ihren Pfad.");
            if let Some(profile) = dialog.selected.and_then(|i| self.profiles.profiles.get(i)) {
                ui.label(format!("{} Tabs · Standard: {}", profile.tabs.len(), profile.default_shell.label()));
                let mut panes = vec![];
                for tab in &profile.tabs { tab.root.panes(&mut panes); }
                for pane in panes {
                    let directory = pane.directory.strip_prefix(&profile.directory).map(|relative| PathBuf::from(&dialog.directory).join(relative)).unwrap_or(pane.directory);
                    ui.small(format!("{} · lokaler Ordner: {}", pane.remote.as_ref().map(|r| r.label()).unwrap_or_else(|| pane.shell.label().into()), directory.display()));
                }
                if ui.button("Profil löschen …").clicked() { dialog.delete_target = dialog.selected; }
            }
            if let Some(index) = dialog.delete_target {
                ui.colored_label(Color32::YELLOW, "Dieses gespeicherte Profil löschen? Laufende Sitzungen bleiben erhalten.");
                ui.horizontal(|ui| {
                    if ui.button("Profil löschen bestätigen").clicked() { delete = Some(index); }
                    if ui.button("Profil behalten").clicked() { dialog.delete_target = None; }
                });
            }
            if let Some(error) = &dialog.error { ui.colored_label(Color32::LIGHT_RED, error); }
            ui.small("Prozesse, Terminalausgaben und KI-Chat-Verläufe werden nicht gespeichert oder fortgesetzt.");
            ui.horizontal(|ui| {
                submit = ui.add_enabled(self.profiles_error.is_none() && (dialog.saving || dialog.selected.is_some()) && dialog.delete_target.is_none(), egui::Button::new(if dialog.saving { "Profil speichern" } else { "Projekt in neuen Tabs öffnen" })).clicked();
                cancel = ui.button("Abbrechen").clicked();
            });
        });
        if let Some(index) = delete {
            let mut updated = self.profiles.clone();
            updated.profiles.remove(index);
            match updated.save(&self.storage.with_file_name("profiles.json")) {
                Ok(()) => {
                    self.profiles = updated;
                    dialog.selected = None;
                    dialog.delete_target = None;
                    dialog.error = None;
                    dialog.directory.clear();
                    self.status = "Projektprofil gelöscht".into();
                }
                Err(error) => dialog.error = Some(error),
            }
        }
        if submit {
            let result = if dialog.saving {
                (|| -> Result<(), String> {
                    let directory = PathBuf::from(&dialog.directory);
                    if !directory.is_dir() {
                        return Err("Der Projektordner existiert nicht.".into());
                    }
                    let profile =
                        profiles::Profile::capture(&dialog.name, directory, &self.workspace)?;
                    let mut updated = self.profiles.clone();
                    updated.profiles.push(profile);
                    updated.save(&self.storage.with_file_name("profiles.json"))?;
                    self.profiles = updated;
                    self.status = format!("Projektprofil „{}“ gespeichert", dialog.name.trim());
                    Ok(())
                })()
            } else {
                self.open_profile(
                    dialog.selected.unwrap(),
                    PathBuf::from(&dialog.directory),
                    ctx,
                )
            };
            match result {
                Ok(()) => return,
                Err(error) => dialog.error = Some(error),
            }
        }
        if open && !cancel {
            self.profile_dialog = Some(dialog);
        }
    }
}
