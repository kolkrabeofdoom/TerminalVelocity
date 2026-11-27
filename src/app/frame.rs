//! Per-frame drawing and input handling (`eframe::App`).

use crate::*;

impl eframe::App for TerminalApp {
    fn clear_color(&self, _: &egui::Visuals) -> [f32; 4] {
        [0.0; 4]
    }

    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        let app_dir = self.app_dir().to_path_buf();
        self.background_cache
            .update(ctx, &self.workspace.background, &app_dir);
        let dialog_owner = raw_window_handle::HasWindowHandle::window_handle(frame)
            .ok()
            .and_then(|h| match h.as_raw() {
                raw_window_handle::RawWindowHandle::Win32(h) => {
                    Some(h.hwnd.get() as *mut std::ffi::c_void)
                }
                _ => None,
            })
            .unwrap_or(std::ptr::null_mut());
        let active_pane = (!self.editor.active).then_some(self.focused);
        while let Ok((id, event)) = self.receiver.try_recv() {
            handle_session_event(
                &mut self.workspace,
                &mut self.sessions,
                id,
                event,
                active_pane,
            );
        }
        if let Some(id) = active_pane {
            if let Some(session) = self.sessions.get_mut(&id) {
                session.notified = false;
            }
        }
        for session in self.sessions.values_mut() {
            if let Some(backend) = &mut session.backend {
                backend.set_sixel_enabled(self.workspace.sixel_enabled);
            }
        }
        self.editor.guard_exit(ctx);
        let editor_base = self
            .workspace
            .inherited_directory(self.focused, std::path::Path::new(&self.directory));
        let shortcuts = Modifiers {
            ctrl: true,
            shift: true,
            ..Modifiers::NONE
        };
        if !self.has_modal_dialog() && ctx.input_mut(|i| i.consume_key(shortcuts, Key::P)) {
            if self.palette.is_some() {
                self.palette = None;
            } else {
                self.show_palette();
            }
        }
        if !self.editor.blocked()
            && self.palette.is_none()
            && ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::F1))
        {
            self.help = !self.help;
        }
        if !self.editor.active
            && !self.has_modal_dialog()
            && self.palette.is_none()
            && ctx.input_mut(|i| i.consume_key(Modifiers::CTRL, Key::F))
        {
            self.show_search();
        }
        if self.is_terminal_interactive() {
            if ctx.input_mut(|i| i.consume_key(shortcuts, Key::T)) {
                self.add_tab(ctx);
            }
            if ctx.input_mut(|i| i.consume_key(shortcuts, Key::D)) {
                self.split(ctx, Axis::Columns);
            }
            if ctx.input_mut(|i| i.consume_key(shortcuts, Key::E)) {
                self.split(ctx, Axis::Rows);
            }
            if ctx.input_mut(|i| i.consume_key(shortcuts, Key::W)) {
                self.close_target = Some(self.focused);
            }
            if ctx.input_mut(|i| i.consume_key(shortcuts, Key::S)) {
                self.save_workspace_explicit();
            }
            if ctx.input_mut(|i| i.consume_key(Modifiers::CTRL, Key::Tab)) {
                self.cycle_pane();
            }
            if ctx.input_mut(|i| i.consume_key(shortcuts, Key::ArrowRight))
                && !self.workspace.tabs.is_empty()
            {
                self.workspace.active_tab =
                    (self.workspace.active_tab + 1) % self.workspace.tabs.len();
                self.focus_first();
            }
        }
        let background = Color32::from_rgba_unmultiplied(
            13,
            18,
            28,
            if self.workspace.compatibility_transparency {
                255
            } else {
                (self.workspace.opacity * 255.0).round() as u8
            },
        );
        egui::TopBottomPanel::top("toolbar")
            .frame(egui::Frame::NONE.fill(background).inner_margin(6.0))
            .show(ctx, |ui| {
                if self.editor.blocked()
                    || self.profile_dialog.is_some()
                    || self.remote_dialog.is_some()
                    || self.palette.is_some()
                {
                    ui.disable();
                }
                let dialog_free = !self.has_modal_dialog();
                ui.add_space(5.0);
                ui.horizontal_wrapped(|ui| {
                    ui.strong("TERMINALVELOCITY");
                    if ui
                        .add_enabled(dialog_free, egui::Button::new("Befehle …"))
                        .on_hover_text("Befehlspalette · Strg+Umschalt+P")
                        .clicked()
                    {
                        self.show_palette();
                    }
                    if ui.button("Editor").clicked() {
                        self.close_search();
                        self.editor.activate();
                    }
                    if ui
                        .add_enabled(dialog_free, egui::Button::new("Verbinden …"))
                        .clicked()
                    {
                        self.close_search();
                        self.remote_dialog = Some(remote::Dialog::default());
                    }
                    ui.label(
                        egui::RichText::new(env!("CARGO_PKG_VERSION"))
                            .small()
                            .color(Color32::from_rgb(100, 210, 190)),
                    );
                    ui.separator();
                    egui::ComboBox::from_id_salt("shell")
                        .selected_text(self.workspace.default_shell.label())
                        .show_ui(ui, |ui| {
                            for shell in Shell::ALL {
                                ui.selectable_value(
                                    &mut self.workspace.default_shell,
                                    shell,
                                    shell.label(),
                                );
                            }
                        });
                    if ui
                        .button("+ Tab")
                        .on_hover_text("Strg+Umschalt+T")
                        .clicked()
                    {
                        self.add_tab(ctx);
                    }
                    if ui
                        .add_enabled(!self.editor.active, egui::Button::new("Nebeneinander"))
                        .on_hover_text("Vertikale Trennlinie · Strg+Umschalt+D")
                        .clicked()
                    {
                        self.split(ctx, Axis::Columns);
                    }
                    if ui
                        .add_enabled(!self.editor.active, egui::Button::new("Untereinander"))
                        .on_hover_text("Horizontale Trennlinie · Strg+Umschalt+E")
                        .clicked()
                    {
                        self.split(ctx, Axis::Rows);
                    }
                    if ui
                        .button("Workspace speichern")
                        .on_hover_text("Strg+Umschalt+S")
                        .clicked()
                    {
                        self.save_workspace_explicit();
                    }
                    if ui.button("Einstellungen").clicked() {
                        self.settings = !self.settings;
                    }
                    if ui
                        .add_enabled(dialog_free, egui::Button::new("Suchen"))
                        .on_hover_text("Strg+F · Aktiven Terminalverlauf durchsuchen")
                        .clicked()
                    {
                        self.show_search();
                    }
                    if ui
                        .add_enabled(dialog_free, egui::Button::new("Projekt öffnen"))
                        .clicked()
                    {
                        self.show_profiles(false);
                    }
                    if ui
                        .add_enabled(dialog_free, egui::Button::new("Projekt speichern …"))
                        .clicked()
                    {
                        self.show_profiles(true);
                    }
                    if ui
                        .button("Hilfe")
                        .on_hover_text("Bedienung & Tastenkürzel · F1")
                        .clicked()
                    {
                        self.help = !self.help;
                    }
                    if ui
                        .button("Über")
                        .on_hover_text("GitHub-Repository, Version & Release Notes")
                        .clicked()
                    {
                        self.about = !self.about;
                    }
                });
                ui.horizontal_wrapped(|ui| {
                    let mut selected = None;
                    for (index, tab) in self.workspace.tabs.iter().enumerate() {
                        let mut panes = Vec::new();
                        tab.root.panes(&mut panes);
                        let notified = panes
                            .iter()
                            .any(|p| self.sessions.get(&p.id).is_some_and(|s| s.notified));
                        let label = if notified {
                            egui::RichText::new(format!("{}  {} ●", index + 1, tab.name))
                                .color(self.workspace.effects.visual().accent())
                        } else {
                            egui::RichText::new(format!("{}  {}", index + 1, tab.name))
                        };
                        if ui
                            .selectable_label(
                                !self.editor.active && index == self.workspace.active_tab,
                                label,
                            )
                            .clicked()
                        {
                            selected = Some(index);
                        }
                    }
                    if self.editor.visible
                        && ui
                            .selectable_label(self.editor.active, self.editor.title())
                            .clicked()
                    {
                        self.close_search();
                        self.editor.active = true;
                    }
                    if let Some(index) = selected {
                        self.editor.active = false;
                        self.workspace.active_tab = index;
                        self.clear_tab_notifications(index);
                        self.focus_first();
                    }
                });
            });
        egui::TopBottomPanel::bottom("status")
            .frame(egui::Frame::NONE.fill(background).inner_margin(6.0))
            .show(ctx, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.small(&self.status);
                    ui.separator();
                    ui.small("KI-CLI auswählen → + Tab · Anmeldung direkt beim Anbieter");
                });
            });
        let mut retry_target = None;
        self.search_bar(ctx, background);
        let mut ai_selection = None;
        let mut image_request = None;
        let mut editor_request = None;
        let editor_enabled = !self.has_modal_dialog() && self.palette.is_none();
        let terminal_enabled = self.is_terminal_interactive();
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(background).inner_margin(8.0))
            .show(ctx, |ui| {
                if self.editor.active {
                    self.editor
                        .ui(ui, dialog_owner, &editor_base, editor_enabled);
                    return;
                }
                let rect = ui.available_rect_before_wrap();
                let background_image = self.background_cache.image(&self.workspace.background);
                if let Some(tab) = self.workspace.tabs.get_mut(self.workspace.active_tab) {
                    let mut render_ctx = PaneRenderContext {
                        sessions: &mut self.sessions,
                        focused: &mut self.focused,
                        font_size: self.workspace.font_size,
                        effects: &self.workspace.effects,
                        background_image,
                        enabled: terminal_enabled,
                        close: &mut self.close_target,
                        retry: &mut retry_target,
                        ai_selection: &mut ai_selection,
                        image_request: &mut image_request,
                        editor_request: &mut editor_request,
                    };
                    draw_node(ui, rect, &mut tab.root, &mut render_ctx);
                }
            });
        if let Some(request) = editor_request {
            self.close_search();
            let result = match request {
                editor::Request::OpenFile(path, line) => {
                    let display = match line {
                        Some(l) => format!("{} (Zeile {l})", path.display()),
                        None => path.display().to_string(),
                    };
                    self.editor
                        .open_file(path, line, ctx)
                        .map(|()| format!("Im Editor geöffnet: {display}"))
                }
                editor::Request::OpenSelection(text) => self
                    .editor
                    .open_selection(text, ctx)
                    .map(|()| "Auswahl als neues Dokument im Editor geöffnet".into()),
            };
            self.status = match result {
                Ok(msg) => msg,
                Err(error) => error,
            };
        }
        if let Some(request) = image_request {
            let (id, path) = match request {
                inline_image::Request::Open(id, path) => (id, Ok(Some(path))),
                inline_image::Request::Choose(id) => (
                    id,
                    background::choose_image_file(dialog_owner, "Bild im Terminal anzeigen"),
                ),
                inline_image::Request::Drop(id, files) => {
                    (id, inline_image::dropped_path(files).map(Some))
                }
            };
            match path {
                Ok(Some(path)) => {
                    let result = if !self.workspace.sixel_enabled {
                        Err("Bitte zuerst die Bildanzeige in den Einstellungen einschalten.".into())
                    } else if let Some(session) = self.sessions.get(&id) {
                        if let Some(backend) = &session.backend {
                            self.image_loader.start(
                                session.generation,
                                backend.image_width(),
                                path,
                                ctx,
                            )
                        } else {
                            Err("Der Terminalbereich ist nicht verfügbar.".into())
                        }
                    } else {
                        Err("Der Terminalbereich wurde geschlossen.".into())
                    };
                    self.status = match result {
                        Ok(()) => "Bild wird geladen …".into(),
                        Err(error) => error,
                    };
                }
                Ok(None) => {}
                Err(error) => self.status = error,
            }
        }
        if let Some((generation, width, result)) = self.image_loader.poll() {
            let result = result.and_then(|image| {
                if !self.workspace.sixel_enabled {
                    return Err("Bildanzeige wurde abgeschaltet.".into());
                }
                let backend = self
                    .sessions
                    .values_mut()
                    .find(|s| s.generation == generation)
                    .and_then(|s| s.backend.as_mut())
                    .ok_or("Zielsitzung wurde geschlossen oder neu gestartet.")?;
                if backend.image_width() != width {
                    return Err(
                        "Terminalgröße wurde geändert. Bitte das Bild erneut öffnen.".into(),
                    );
                }
                backend
                    .display_rgba(
                        image.width() as usize,
                        image.height() as usize,
                        image.into_raw(),
                    )
                    .map_err(|e| e.to_string())
            });
            self.status = match result {
                Ok(()) => "Bild lokal im Terminal angezeigt.".into(),
                Err(error) => format!("Bild nicht angezeigt: {error}"),
            };
            ctx.request_repaint();
        }
        if let Some(mut draft) = ai_selection {
            if self.workspace.default_shell.is_ai() {
                draft.provider = self.workspace.default_shell;
            }
            self.ai_draft = Some(draft);
        }
        self.editor.confirmation(ctx, dialog_owner, &editor_base);
        self.palette_window(ctx);
        self.profile_window(ctx);
        self.remote_window(ctx);
        self.help_window(ctx);
        self.about_window(ctx);
        let mut submit = false;
        let mut cancel = false;
        let mut open = true;
        if let Some(draft) = self.ai_draft.as_mut() {
            egui::Window::new("Markierten Text an KI übergeben").open(&mut open).default_width(650.0).vscroll(true).show(ctx, |ui| {
                ui.label("Prüfe und bearbeite den Text. Gesendet wird erst nach Klick auf „Senden“.");
                ui.horizontal(|ui| {
                    ui.label("Anbieter");
                    egui::ComboBox::from_id_salt("ai-selection-provider").selected_text(draft.provider.label()).show_ui(ui, |ui| {
                        for provider in ai::PROVIDERS { ui.selectable_value(&mut draft.provider, provider, provider.label()); }
                    });
                });
                ui.label("Lokaler Projektordner (KI läuft auf diesem Computer)");
                let mut path = draft.directory.display().to_string();
                if ui.text_edit_singleline(&mut path).changed() { draft.directory = PathBuf::from(path); }
                if draft.provider == Shell::OpenAI {
                    ui.horizontal(|ui| { ui.label("Modell-ID"); ui.text_edit_singleline(&mut draft.model); });
                    ui.small("Die OpenAI API benötigt einen eingerichteten API-Schlüssel. Die Antwort erscheint im neuen Tab.");
                }
                ui.add(egui::TextEdit::multiline(&mut draft.text).desired_rows(12).desired_width(f32::INFINITY).code_editor());
                ui.small(format!("{} / {} Zeichen", draft.text.encode_utf16().count(), ai::MAX_PROMPT_UNITS));
                ui.small("Der Text wird an den gewählten KI-Anbieter übergeben. Es öffnet sich eine neue Sitzung im angegebenen Projektordner.");
                if let Some(error) = &draft.error { ui.colored_label(Color32::LIGHT_RED, error); }
                ui.horizontal(|ui| { submit = ui.button("Senden und KI-Tab öffnen").clicked(); cancel = ui.button("Abbrechen").clicked(); });
            });
        }
        if cancel || !open {
            self.ai_draft = None;
        } else if submit {
            if let Err(error) = self.send_selection(ctx) {
                if let Some(draft) = self.ai_draft.as_mut() {
                    draft.error = Some(error);
                }
            }
        }
        if let Some(id) = retry_target {
            if self
                .sessions
                .get(&id)
                .is_some_and(|session| session.backend.is_some() && !session.exited)
            {
                self.restart_target = Some(id);
            } else {
                self.restart_pane(id, ctx);
            }
        }
        let mut apply_paths = false;
        if self.settings {
            egui::Window::new("Einstellungen").open(&mut self.settings).resizable(false).vscroll(true).show(ctx, |ui| {
                ui.checkbox(&mut self.workspace.compatibility_transparency, "Kompatible Fenstertransparenz");
                let minimum = if self.workspace.compatibility_transparency { 0.2 } else { 0.0 };
                ui.add(egui::Slider::new(&mut self.workspace.opacity, minimum..=1.0).text("Deckkraft"));
                if self.workspace.compatibility_transparency {
                    self.workspace.acrylic = false;
                    ui.small("Windows mischt das gesamte Fenster mit dem Desktop. Auch Schrift und Bedienelemente werden durchsichtig. 1 = undurchsichtig.");
                } else {
                    ui.small("Nur die Hintergrundflächen werden durchsichtig; benötigt Unterstützung durch den Grafiktreiber.");
                }
                ui.add_enabled(!self.workspace.compatibility_transparency, egui::Checkbox::new(&mut self.workspace.acrylic, "Hintergrund weichzeichnen (Windows-Acryl)"));
                ui.add(egui::Slider::new(&mut self.workspace.font_size, 10.0..=28.0).text("Schriftgröße"));
                ui.separator();
                ui.checkbox(&mut self.workspace.sixel_enabled, "Bilder im Terminal anzeigen (Sixel und lokale Dateien)");
                ui.small("Bis 4096 Pixel pro Seite, 2 Megapixel je Bild; 16 MiB Bilddaten je Bildschirm. Größenänderungen löschen Bilder. Abschalten entfernt gespeicherte Grafiken.");
                if let Some(backend) = self.sessions.get(&self.focused).and_then(|s| s.backend.as_ref()) { let (images, rejected) = backend.sixel_status(); ui.small(format!("Aktiver Bereich: {images} Bilder · {rejected} verworfene Bildsequenzen")); }
                ui.separator();
                self.workspace.effects.ui(ui);
                ui.separator();
                self.background_cache.ui(ui, &mut self.workspace.background, dialog_owner);
                ui.separator();
                ui.label("Startordner ohne aktiven Bereich");
                ui.small("Neue Tabs und Teilungen übernehmen den aktuellen Ordner des aktiven Bereichs. KI-Sitzungen starten im selben Projektordner.");
                ui.text_edit_singleline(&mut self.directory);
                if !PathBuf::from(&self.directory).is_dir() { ui.colored_label(Color32::LIGHT_RED, "Dieser Ordner existiert nicht."); }
                ui.separator();
                ui.label("OpenSSH (leer = automatisch erkennen)");
                ui.text_edit_singleline(&mut self.workspace.ssh_path);
                ui.label("Shells und KI-CLIs (leer = automatisch erkennen)");
                egui::ScrollArea::vertical().max_height(330.0).show(ui, |ui| {
                for shell in Shell::ALL {
                    ui.label(shell.label());
                    ui.add(egui::TextEdit::singleline(&mut self.workspace.shell_paths[shell as usize]).desired_width(480.0).hint_text("Vollständiger EXE-Pfad oder relativ zum Programmordner"));
                    if let Some(path) = shell.resolve(Some(&self.workspace.shell_paths[shell as usize]), &app_dir) {
                        ui.label(egui::RichText::new(path.display().to_string()).small().color(Color32::LIGHT_GREEN));
                    } else { ui.colored_label(Color32::YELLOW, "Nicht gefunden"); }
                }
                });
                apply_paths = ui.button("Speichern und fehlgeschlagene Sitzungen neu starten").clicked();
                ui.small("Laufende Sitzungen bleiben aktiv. Geänderte Pfade gelten für neue Sitzungen.");
                ui.separator();
                ui.label("Strg+Umschalt+P: Befehlspalette");
                ui.label("Strg+Tab: Nächster Bereich");
                ui.label("Strg+Umschalt+Rechts: Nächster Tab");
                ui.label("Strg+Umschalt+C / V: Kopieren / Einfügen");
                ui.small("Layouts werden beim Beenden gespeichert. Prozesse und Ausgaben werden nicht wiederhergestellt.");
            });
        }
        if self.applied_acrylic != self.workspace.acrylic {
            let result = if self.workspace.acrylic {
                window_vibrancy::apply_acrylic(&*frame, Some((16, 20, 30, 1)))
            } else {
                window_vibrancy::clear_acrylic(&*frame)
            };
            match result {
                Ok(()) => self.applied_acrylic = self.workspace.acrylic,
                Err(error) => {
                    self.workspace.acrylic = self.applied_acrylic;
                    self.status = format!("Acryl konnte nicht geändert werden: {error}");
                }
            }
            ctx.request_repaint();
        }
        let desired_opacity = if self.workspace.compatibility_transparency {
            Some((self.workspace.opacity.clamp(0.2, 1.0) * 255.0).round() as u8)
        } else {
            None
        };
        if self.applied_opacity != Some(desired_opacity) {
            match set_window_opacity(&*frame, desired_opacity) {
                Ok(()) => {
                    self.applied_opacity = Some(desired_opacity);
                }
                Err(error) => self.status = format!("Fenstertransparenz fehlgeschlagen: {error}"),
            }
        }
        if apply_paths {
            self.save_workspace();
            let failed: Vec<_> = self
                .workspace
                .panes()
                .into_iter()
                .filter(|p| self.sessions.get(&p.id).is_some_and(|s| s.error.is_some()))
                .collect();
            for pane in failed {
                self.start(ctx, &pane);
            }
        }
        if let Some(id) = self.restart_target {
            let mut confirm = false;
            let mut cancel = false;
            egui::Window::new("Sitzung neu starten?").collapsible(false).resizable(false).show(ctx, |ui| {
                if let Some(pane) = self.workspace.panes().into_iter().find(|pane| pane.id == id) {
                    ui.label(format!("{} · Bereich #{id}", pane.remote.as_ref().map(|r| r.label()).unwrap_or_else(|| pane.shell.label().into())));
                    ui.label(format!("Projektordner: {}", pane.directory.display()));
                }
                ui.label("Laufende Prozesse in diesem Bereich werden beendet. Die Ausgabe wird geleert.");
                ui.horizontal(|ui| {
                    confirm = ui.button("Jetzt neu starten").clicked();
                    cancel = ui.button("Abbrechen").clicked();
                });
            });
            if confirm {
                self.restart_target = None;
                self.restart_pane(id, ctx);
            }
            if cancel {
                self.restart_target = None;
            }
        }
        if let Some(id) = self.close_target {
            let mut confirm = false;
            let mut cancel = false;
            egui::Window::new("Sitzung schließen?")
                .collapsible(false)
                .resizable(false)
                .show(ctx, |ui| {
                    ui.label(format!(
                        "Bereich #{id} und die zugehörige Shell werden beendet."
                    ));
                    ui.horizontal(|ui| {
                        confirm = ui.button("Sitzung beenden").clicked();
                        cancel = ui.button("Abbrechen").clicked();
                    });
                });
            if confirm {
                self.close_target = None;
                self.close_pane(id, ctx);
            }
            if cancel {
                self.close_target = None;
            }
        }
    }

    fn on_exit(&mut self, _: Option<&eframe::glow::Context>) {
        self.save_workspace();
    }
}
