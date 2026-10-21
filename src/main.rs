#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod ai;
mod background;
mod editor;
mod effects;
mod model;
mod profiles;
mod remote;
mod remote_ui;

use eframe::egui::{self, Color32, FontId, Key, Modifiers, Rect, Sense, Stroke};
use egui_term::{
    BackendSettings, FontSettings, PtyEvent, TerminalBackend, TerminalFont, TerminalView,
};
use model::{Axis, Node, Pane, Shell, Tab, Workspace};
use std::{
    collections::HashMap,
    path::PathBuf,
    sync::mpsc::{self, Receiver, Sender},
};

struct Session {
    generation: u64,
    backend: Option<TerminalBackend>,
    error: Option<String>,
    exited: bool,
}

struct HistorySearch {
    pane_id: u64,
    query: String,
    focus_input: bool,
    error: Option<String>,
}

fn handle_session_event(
    workspace: &mut Workspace,
    sessions: &mut HashMap<u64, Session>,
    generation: u64,
    event: PtyEvent,
) {
    let Some((&pane_id, session)) = sessions
        .iter_mut()
        .find(|(_, session)| session.generation == generation)
    else {
        return;
    };
    if let PtyEvent::Title(ref title) = event {
        workspace.update_directory(pane_id, title);
    }
    if matches!(event, PtyEvent::Exit | PtyEvent::ChildExit(_)) {
        session.exited = true;
    }
}

fn set_window_opacity(
    window: &impl raw_window_handle::HasWindowHandle,
    alpha: Option<u8>,
) -> Result<(), String> {
    use raw_window_handle::RawWindowHandle;
    use windows_sys::Win32::UI::WindowsAndMessaging::{
        GetLayeredWindowAttributes, GetWindowLongPtrW, SetLayeredWindowAttributes,
        SetWindowLongPtrW, GWL_EXSTYLE, LWA_ALPHA, WS_EX_LAYERED,
    };
    let handle = window.window_handle().map_err(|error| error.to_string())?;
    let RawWindowHandle::Win32(handle) = handle.as_raw() else {
        return Err("Kein Windows-Fensterhandle".into());
    };
    let hwnd = handle.hwnd.get() as *mut std::ffi::c_void;
    // Layered-window alpha is composed by Windows, independently of OpenGL's framebuffer alpha.
    unsafe {
        let style = GetWindowLongPtrW(hwnd, GWL_EXSTYLE);
        if let Some(alpha) = alpha {
            if style & WS_EX_LAYERED as isize == 0 {
                SetWindowLongPtrW(hwnd, GWL_EXSTYLE, style | WS_EX_LAYERED as isize);
                if GetWindowLongPtrW(hwnd, GWL_EXSTYLE) & WS_EX_LAYERED as isize == 0 {
                    return Err(std::io::Error::last_os_error().to_string());
                }
            }
            if SetLayeredWindowAttributes(hwnd, 0, alpha, LWA_ALPHA) == 0 {
                return Err(std::io::Error::last_os_error().to_string());
            }
            let (mut actual_alpha, mut flags, mut color) = (0, 0, 0);
            if GetLayeredWindowAttributes(hwnd, &mut color, &mut actual_alpha, &mut flags) == 0 {
                return Err(std::io::Error::last_os_error().to_string());
            }
            if actual_alpha != alpha || flags & LWA_ALPHA == 0 {
                return Err("Windows hat die Fensterdeckkraft nicht übernommen".into());
            }
        } else if style & WS_EX_LAYERED as isize != 0 {
            SetLayeredWindowAttributes(hwnd, 0, 255, LWA_ALPHA);
            SetWindowLongPtrW(hwnd, GWL_EXSTYLE, style & !(WS_EX_LAYERED as isize));
            if GetWindowLongPtrW(hwnd, GWL_EXSTYLE) & WS_EX_LAYERED as isize != 0 {
                return Err(std::io::Error::last_os_error().to_string());
            }
        }
    }
    Ok(())
}

struct TerminalApp {
    editor: editor::Editor,
    background_cache: background::Cache,
    workspace: Workspace,
    sessions: HashMap<u64, Session>,
    sender: Sender<(u64, PtyEvent)>,
    receiver: Receiver<(u64, PtyEvent)>,
    focused: u64,
    next_id: u64,
    next_session_id: u64,
    settings: bool,
    directory: String,
    status: String,
    storage: PathBuf,
    close_target: Option<u64>,
    restart_target: Option<u64>,
    ai_draft: Option<ai::Draft>,
    profiles: profiles::Library,
    profile_dialog: Option<profiles::Dialog>,
    remote_dialog: Option<remote::Dialog>,
    profiles_error: Option<String>,
    search: Option<HistorySearch>,
    applied_acrylic: bool,
    applied_opacity: Option<Option<u8>>,
}

impl TerminalApp {
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        cc.egui_ctx.set_visuals(egui::Visuals::dark());
        let mut fonts = egui::FontDefinitions::default();
        if let Ok(bytes) = std::fs::read("C:/Windows/Fonts/consola.ttf") {
            fonts
                .font_data
                .insert("consolas".into(), egui::FontData::from_owned(bytes).into());
            fonts
                .families
                .entry(egui::FontFamily::Monospace)
                .or_default()
                .insert(0, "consolas".into());
            cc.egui_ctx.set_fonts(fonts);
        }
        let storage = std::env::current_exe()
            .unwrap_or_default()
            .with_file_name("workspace.json");
        let (workspace, load_status) = match std::fs::read(&storage) {
            Ok(bytes) => match serde_json::from_slice::<Workspace>(&bytes) {
                Ok(mut state) => match state.validate() {
                    Ok(()) => (state, "Arbeitsbereich wiederhergestellt".to_string()),
                    Err(error) => (
                        Workspace::default(),
                        format!("Workspace nicht geladen: {error}"),
                    ),
                },
                Err(error) => (
                    Workspace::default(),
                    format!("Workspace nicht geladen: {error}"),
                ),
            },
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                (Workspace::default(), "Bereit".into())
            }
            Err(error) => (Workspace::default(), format!("Lesefehler: {error}")),
        };
        let (profiles, profiles_error) =
            match profiles::Library::load(&storage.with_file_name("profiles.json")) {
                Ok(profiles) => (profiles, None),
                Err(error) => (profiles::Library::default(), Some(error)),
            };
        let (sender, receiver) = mpsc::channel();
        let panes = workspace.panes();
        let next_id = panes.iter().map(|p| p.id).max().unwrap_or(0) + 1;
        let directory = workspace
            .default_directory
            .clone()
            .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")))
            .display()
            .to_string();
        let mut app = Self {
            editor: editor::Editor::default(),
            background_cache: background::Cache::default(),
            workspace,
            sessions: HashMap::new(),
            sender,
            receiver,
            focused: 0,
            next_id,
            next_session_id: 1,
            settings: false,
            directory,
            status: load_status,
            storage,
            close_target: None,
            restart_target: None,
            ai_draft: None,
            profiles,
            profile_dialog: None,
            remote_dialog: None,
            profiles_error,
            search: None,
            applied_acrylic: false,
            applied_opacity: None,
        };
        if app.workspace.compatibility_transparency {
            app.workspace.acrylic = false;
        }
        if app.workspace.acrylic {
            match window_vibrancy::apply_acrylic(cc, Some((16, 20, 30, 1))) {
                Ok(()) => app.applied_acrylic = true,
                Err(error) => {
                    app.workspace.acrylic = false;
                    app.status = format!("Acryl-Effekt nicht verfügbar: {error}");
                }
            }
        }
        for pane in panes {
            app.start(&cc.egui_ctx, &pane);
        }
        if app.workspace.tabs.is_empty() {
            app.add_tab(&cc.egui_ctx);
        }
        app.focus_first();
        app
    }

    fn start(&mut self, ctx: &egui::Context, pane: &Pane) {
        self.start_with_command(ctx, pane, None);
    }

    fn start_with_command(
        &mut self,
        ctx: &egui::Context,
        pane: &Pane,
        command: Option<(PathBuf, Vec<String>)>,
    ) {
        // Drop the old PTY before starting its replacement. Its delayed events
        // carry a different generation and cannot terminate or mutate the new session.
        self.sessions.remove(&pane.id);
        let generation = self.next_session_id;
        self.next_session_id += 1;
        let result = (|| {
            if !pane.directory.is_dir() {
                return Err(format!("Startordner fehlt: {}", pane.directory.display()));
            }
            let (executable, args) = if let Some(command) = command {
                command
            } else if let Some(remote) = &pane.remote {
                remote.command(
                    &self.workspace,
                    self.storage.parent().unwrap_or(std::path::Path::new(".")),
                )?
            } else {
                let executable = pane.shell.resolve(Some(&self.workspace.shell_paths[pane.shell as usize]), self.storage.parent().unwrap_or(std::path::Path::new("."))).ok_or_else(|| {
                format!(
                    "{} wurde nicht gefunden. Unter Einstellungen → Shell-Programme einen Pfad angeben oder die portable Shell im Programmordner ablegen.",
                    pane.shell.label()
                )
            })?;
                let (executable, args) = pane.shell.launch(
                    executable,
                    self.storage.parent().unwrap_or(std::path::Path::new(".")),
                    false,
                )?;
                (executable, args)
            };
            TerminalBackend::new(
                generation,
                ctx.clone(),
                self.sender.clone(),
                BackendSettings {
                    shell: executable.to_string_lossy().into_owned(),
                    args,
                    working_directory: Some(pane.directory.clone()),
                },
            )
            .map_err(|e| e.to_string())
        })();
        let session = match result {
            Ok(backend) => Session {
                generation,
                backend: Some(backend),
                error: None,
                exited: false,
            },
            Err(error) => Session {
                generation,
                backend: None,
                error: Some(error),
                exited: false,
            },
        };
        self.sessions.insert(pane.id, session);
    }

    fn send_selection(&mut self, ctx: &egui::Context) -> Result<(), String> {
        if self.workspace.tabs.len() >= 32 || self.sessions.len() >= 64 {
            return Err("Limit: 32 Tabs / 64 Bereiche erreicht.".into());
        }
        let draft = self.ai_draft.as_ref().ok_or("Keine Auswahl vorhanden")?;
        let base = self.storage.parent().unwrap_or(std::path::Path::new("."));
        let executable = draft
            .provider
            .resolve(
                Some(&self.workspace.shell_paths[draft.provider as usize]),
                base,
            )
            .ok_or_else(|| {
                format!(
                    "{} nicht gefunden. Bitte den Programmpfad in Einstellungen prüfen.",
                    draft.provider.label()
                )
            })?;
        let command = draft.command(executable)?;
        let pane = Pane {
            remote: None,
            id: self.next_id,
            shell: draft.provider,
            directory: draft.directory.clone(),
        };
        self.next_id += 1;
        self.start_with_command(ctx, &pane, Some(command));
        if let Some(error) = self
            .sessions
            .get(&pane.id)
            .and_then(|session| session.error.clone())
        {
            self.sessions.remove(&pane.id);
            return Err(error);
        }
        self.focused = pane.id;
        self.workspace.tabs.push(Tab {
            name: format!("{} · Auswahl", pane.shell.label()),
            root: Node::Pane(pane),
        });
        self.workspace.active_tab = self.workspace.tabs.len() - 1;
        self.ai_draft = None;
        self.status = "Auswahl an neue KI-Sitzung übergeben".into();
        Ok(())
    }

    fn restart_pane(&mut self, id: u64, ctx: &egui::Context) {
        if let Some(pane) = self
            .workspace
            .panes()
            .into_iter()
            .find(|pane| pane.id == id)
        {
            self.start(ctx, &pane);
            self.focused = id;
            self.status = if self
                .sessions
                .get(&id)
                .is_some_and(|session| session.error.is_none())
            {
                format!(
                    "{} neu gestartet · {}",
                    pane.shell.label(),
                    pane.directory.display()
                )
            } else {
                format!("Neustart fehlgeschlagen: {}", pane.shell.label())
            };
        }
    }

    fn new_pane(&mut self) -> Pane {
        let id = self.next_id;
        self.next_id += 1;
        Pane {
            remote: None,
            id,
            shell: self.workspace.default_shell,
            directory: self
                .workspace
                .inherited_directory(self.focused, std::path::Path::new(&self.directory)),
        }
    }

    fn add_tab(&mut self, ctx: &egui::Context) {
        if self.workspace.tabs.len() >= 32 || self.sessions.len() >= 64 {
            self.status = "Limit: 32 Tabs / 64 Bereiche".into();
            return;
        }
        let pane = self.new_pane();
        self.editor.active = false;
        self.start(ctx, &pane);
        self.focused = pane.id;
        self.workspace.tabs.push(Tab {
            name: pane.shell.label().into(),
            root: Node::Pane(pane),
        });
        self.workspace.active_tab = self.workspace.tabs.len() - 1;
    }

    fn split(&mut self, ctx: &egui::Context, axis: Axis) {
        if self.editor.active {
            return;
        }
        if self.sessions.len() >= 64 {
            self.status = "Maximal 64 Bereiche".into();
            return;
        }
        let pane = self.new_pane();
        if let Some(tab) = self.workspace.tabs.get_mut(self.workspace.active_tab) {
            if tab.root.split(self.focused, axis, pane.clone()) {
                self.start(ctx, &pane);
                self.focused = pane.id;
            }
        }
    }

    fn focus_first(&mut self) {
        self.editor.active = false;
        let mut panes = vec![];
        if let Some(tab) = self.workspace.tabs.get(self.workspace.active_tab) {
            tab.root.panes(&mut panes);
        }
        self.focused = panes.first().map(|p| p.id).unwrap_or(0);
    }

    fn cycle_pane(&mut self) {
        let mut panes = vec![];
        if let Some(tab) = self.workspace.tabs.get(self.workspace.active_tab) {
            tab.root.panes(&mut panes);
        }
        if !panes.is_empty() {
            let index = panes.iter().position(|p| p.id == self.focused).unwrap_or(0);
            self.focused = panes[(index + 1) % panes.len()].id;
        }
    }

    fn save_workspace(&mut self) -> bool {
        self.workspace.default_directory = Some(PathBuf::from(&self.directory));
        let result = (|| -> Result<(), Box<dyn std::error::Error>> {
            let encoded = serde_json::to_vec_pretty(&self.workspace)?;
            let temporary = self.storage.with_extension("json.tmp");
            std::fs::write(&temporary, encoded)?;
            // Replace in one filesystem operation; preserve the previous file on write failure.
            #[cfg(windows)]
            {
                use std::os::windows::ffi::OsStrExt;
                unsafe extern "system" {
                    fn MoveFileExW(existing: *const u16, new: *const u16, flags: u32) -> i32;
                }
                let from: Vec<u16> = temporary.as_os_str().encode_wide().chain(Some(0)).collect();
                let to: Vec<u16> = self
                    .storage
                    .as_os_str()
                    .encode_wide()
                    .chain(Some(0))
                    .collect();
                if unsafe { MoveFileExW(from.as_ptr(), to.as_ptr(), 1 | 8) } == 0 {
                    return Err(std::io::Error::last_os_error().into());
                }
            }
            #[cfg(not(windows))]
            std::fs::rename(temporary, &self.storage)?;
            Ok(())
        })();
        let saved = result.is_ok();
        self.status = match result {
            Ok(()) => "Arbeitsbereich gespeichert".into(),
            Err(e) => format!("Speichern fehlgeschlagen: {e}"),
        };
        saved
    }

    fn show_profiles(&mut self, saving: bool) {
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

    fn close_search(&mut self) {
        if let Some(search) = self.search.take() {
            if let Some(backend) = self
                .sessions
                .get_mut(&search.pane_id)
                .and_then(|s| s.backend.as_mut())
            {
                let _ = backend.set_search("");
            }
        }
    }

    fn show_search(&mut self) {
        if self.editor.active {
            self.editor.start_search();
            return;
        }
        if let Some(search) = &mut self.search {
            search.focus_input = true;
        } else if self
            .sessions
            .get(&self.focused)
            .is_some_and(|s| s.backend.is_some())
        {
            self.search = Some(HistorySearch {
                pane_id: self.focused,
                query: String::new(),
                focus_input: true,
                error: None,
            });
        } else {
            self.status = "In diesem Bereich ist kein Terminalverlauf verfügbar.".into();
        }
    }

    fn search_bar(&mut self, ctx: &egui::Context, background: Color32) {
        if self
            .search
            .as_ref()
            .is_some_and(|s| s.pane_id != self.focused)
            || self.settings
            || (self.profile_dialog.is_some() || self.remote_dialog.is_some())
            || self.ai_draft.is_some()
            || self.restart_target.is_some()
            || self.close_target.is_some()
        {
            self.close_search();
        }
        let Some(search) = &mut self.search else {
            return;
        };
        let Some(backend) = self
            .sessions
            .get_mut(&search.pane_id)
            .and_then(|s| s.backend.as_mut())
        else {
            self.search = None;
            return;
        };
        let mut close = ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape));
        let mut previous = ctx.input_mut(|i| i.consume_key(Modifiers::SHIFT, Key::Enter));
        let mut next = ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Enter));
        egui::TopBottomPanel::bottom("history_search")
            .frame(egui::Frame::NONE.fill(background).inner_margin(6.0))
            .show(ctx, |ui| {
                ui.horizontal_wrapped(|ui| {
                    ui.label("Verlauf durchsuchen");
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut search.query)
                            .id_salt("history-query")
                            .desired_width(280.0)
                            .hint_text("Suchtext (Groß-/Kleinschreibung beachten)"),
                    );
                    if search.focus_input {
                        response.request_focus();
                        search.focus_input = false;
                    }
                    if response.changed() {
                        search.error = backend.set_search(&search.query).err();
                    }
                    let (active, count, truncated) = backend.search_status();
                    ui.label(if search.query.is_empty() {
                        "Suchtext eingeben".into()
                    } else if count == 0 {
                        "Keine Treffer".into()
                    } else {
                        format!("{active} / {count}{}", if truncated { "+" } else { "" })
                    });
                    previous |= ui
                        .add_enabled(count > 0, egui::Button::new("Zurück"))
                        .on_hover_text("Umschalt+Enter")
                        .clicked();
                    next |= ui
                        .add_enabled(count > 0, egui::Button::new("Weiter"))
                        .on_hover_text("Enter")
                        .clicked();
                    close |= ui.button("Schließen (Esc)").clicked();
                    if let Some(error) = &search.error {
                        ui.colored_label(Color32::LIGHT_RED, error);
                    }
                });
            });
        if previous || next {
            backend.next_search_match(previous);
            search.focus_input = true;
        }
        ctx.request_repaint_after(std::time::Duration::from_millis(300));
        if close {
            self.close_search();
        }
    }

    fn open_profile(
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
        let base = self.storage.parent().unwrap_or(std::path::Path::new("."));
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

    fn profile_window(&mut self, ctx: &egui::Context) {
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

    fn close_pane(&mut self, id: u64, ctx: &egui::Context) {
        if let Some(index) = self.workspace.tabs.iter().position(|tab| {
            let mut panes = vec![];
            tab.root.panes(&mut panes);
            panes.iter().any(|p| p.id == id)
        }) {
            let tab = self.workspace.tabs.remove(index);
            if let Some(root) = tab.root.remove(id) {
                self.workspace.tabs.insert(
                    index,
                    Tab {
                        name: tab.name,
                        root,
                    },
                );
            }
            self.sessions.remove(&id);
            self.workspace.active_tab = self
                .workspace
                .active_tab
                .min(self.workspace.tabs.len().saturating_sub(1));
            if self.workspace.tabs.is_empty() {
                self.add_tab(ctx);
            }
            self.focus_first();
        }
    }
}

fn draw_node(
    ui: &mut egui::Ui,
    rect: Rect,
    node: &mut Node,
    sessions: &mut HashMap<u64, Session>,
    focused: &mut u64,
    font_size: f32,
    effects: &effects::Settings,
    background_image: Option<egui_term::BackgroundImage>,
    enabled: bool,
    close: &mut Option<u64>,
    retry: &mut Option<u64>,
    ai_selection: &mut Option<ai::Draft>,
) {
    match node {
        Node::Split {
            axis,
            ratio,
            first,
            second,
        } => {
            let columns = matches!(axis, Axis::Columns);
            let extent = if columns { rect.width() } else { rect.height() };
            let offset = extent * *ratio;
            let (mut a, mut b, mut bar) = (rect, rect, rect);
            if columns {
                a.max.x = rect.min.x + offset - 3.0;
                b.min.x = a.max.x + 6.0;
                bar.min.x = a.max.x;
                bar.max.x = b.min.x;
            } else {
                a.max.y = rect.min.y + offset - 3.0;
                b.min.y = a.max.y + 6.0;
                bar.min.y = a.max.y;
                bar.max.y = b.min.y;
            }
            let id = ui.id().with(format!("split-{}", first_id(first)));
            let response = ui.interact(bar, id, Sense::drag());
            if response.dragged() {
                if let Some(pos) = response.interact_pointer_pos() {
                    *ratio = ((if columns {
                        pos.x - rect.min.x
                    } else {
                        pos.y - rect.min.y
                    }) / extent.max(1.0))
                    .clamp(0.15, 0.85);
                }
            }
            ui.painter().rect_filled(
                bar,
                0.0,
                if response.hovered() {
                    Color32::from_rgb(70, 180, 190)
                } else {
                    Color32::from_gray(50)
                },
            );
            draw_node(
                ui,
                a,
                first,
                sessions,
                focused,
                font_size,
                effects,
                background_image,
                enabled,
                close,
                retry,
                ai_selection,
            );
            draw_node(
                ui,
                b,
                second,
                sessions,
                focused,
                font_size,
                effects,
                background_image,
                enabled,
                close,
                retry,
                ai_selection,
            );
        }
        Node::Pane(pane) => {
            if rect.width() < 40.0 || rect.height() < 70.0 {
                return;
            }
            if enabled
                && ui.input(|i| {
                    i.pointer.any_pressed()
                        && i.pointer.interact_pos().is_some_and(|p| rect.contains(p))
                })
            {
                *focused = pane.id;
            }
            let active = pane.id == *focused;
            ui.painter().rect_stroke(
                rect.shrink(0.5),
                3.0,
                Stroke::new(
                    1.0_f32,
                    if active {
                        if effects.visual().strength() > 0.0 {
                            effects.visual().accent()
                        } else {
                            Color32::from_rgb(66, 195, 185)
                        }
                    } else {
                        Color32::from_gray(55)
                    },
                ),
                egui::StrokeKind::Inside,
            );
            ui.allocate_new_ui(egui::UiBuilder::new().max_rect(rect.shrink(5.0)), |ui| {
                ui.set_clip_rect(rect.shrink(2.0));
                ui.horizontal_wrapped(|ui| {
                    ui.colored_label(
                        if active {
                            Color32::from_rgb(100, 220, 200)
                        } else {
                            Color32::GRAY
                        },
                        pane.remote.as_ref().map(|r| r.label()).unwrap_or_else(|| pane.shell.label().into()),
                    );
                    ui.small(format!("#{}", pane.id));
                    if ui.add_enabled(enabled, egui::Button::new(if pane.remote.is_some() { "Neu verbinden" } else { "Sitzung neu starten" }).small()).on_hover_text("Gleicher Anbieter und aktueller Projektordner; bisherige Ausgabe wird geleert").clicked() {
                        *retry = Some(pane.id);
                    }
                    if ui
                        .add_enabled(enabled, egui::Button::new("×").small())
                        .on_hover_text("Bereich schließen · Strg+Umschalt+W")
                        .clicked()
                    {
                        *close = Some(pane.id);
                    }
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new(format!("{}{}", if pane.remote.is_some() { "Lokaler Startordner: " } else { "" }, pane.directory.display()))
                                .small()
                                .weak(),
                        )
                        .truncate(),
                    )
                    .on_hover_text(pane.directory.display().to_string());
                });
                ui.separator();
                if let Some(session) = sessions.get_mut(&pane.id) {
                    if let Some(error) = &session.error {
                        ui.colored_label(Color32::LIGHT_RED, error);
                        if ui.add_enabled(enabled, egui::Button::new("Sitzung neu starten")).clicked() {
                            *retry = Some(pane.id);
                        }
                    }
                    if session.exited {
                        ui.colored_label(
                            Color32::YELLOW,
                            if pane.remote.is_some() { "Verbindung beendet. Ausgabe bleibt erhalten; oben mit „Neu verbinden“ erneut öffnen." } else { "Sitzung beendet. Mit „Sitzung neu starten“ oben erneut öffnen." },
                        );
                    }
                    if let Some(backend) = &mut session.backend {
                        let view = TerminalView::new(ui, backend)
                            .set_background_image(background_image)
                            .set_effects(effects.visual())
                            .set_theme(effects.theme())
                            .set_focus(active && enabled && !session.exited)
                            .set_font(TerminalFont::new(FontSettings {
                                font_type: FontId::monospace(font_size),
                            }));
                        let response = ui.add(view);
                        if enabled {
                            response.context_menu(|ui| {
                                let text = backend.selectable_content();
                                let selected = !text.trim().is_empty();
                                if ui.add_enabled(selected, egui::Button::new("Kopieren")).clicked() { ui.ctx().copy_text(text.clone()); ui.close_menu(); }
                                if ui.add_enabled(selected, egui::Button::new("An KI übergeben …")).clicked() {
                                    *ai_selection = Some(ai::Draft::new(text.clone(), pane.directory.clone(), pane.shell));
                                    ui.close_menu();
                                }
                                ui.separator();
                                if pane.remote.is_some() { ui.small("KI-Übergabe öffnet eine lokale KI-Sitzung."); }
                                for action in ai::QuickAction::ALL {
                                    if ui.add_enabled(selected, egui::Button::new(action.label())).clicked() {
                                        *ai_selection = Some(ai::Draft::new(action.prompt(&text), pane.directory.clone(), pane.shell));
                                        ui.close_menu();
                                    }
                                }
                                if !selected { ui.weak("Zuerst Text im Terminal markieren."); }
                            });
                        }
                    }
                }
            });
        }
    }
}

fn first_id(node: &Node) -> u64 {
    match node {
        Node::Pane(p) => p.id,
        Node::Split { first, .. } => first_id(first),
    }
}

impl eframe::App for TerminalApp {
    fn clear_color(&self, _: &egui::Visuals) -> [f32; 4] {
        [0.0; 4]
    }

    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        self.background_cache.update(
            ctx,
            &self.workspace.background,
            self.storage.parent().unwrap_or(std::path::Path::new(".")),
        );
        let dialog_owner = raw_window_handle::HasWindowHandle::window_handle(frame)
            .ok()
            .and_then(|h| match h.as_raw() {
                raw_window_handle::RawWindowHandle::Win32(h) => {
                    Some(h.hwnd.get() as *mut std::ffi::c_void)
                }
                _ => None,
            })
            .unwrap_or(std::ptr::null_mut());
        while let Ok((id, event)) = self.receiver.try_recv() {
            handle_session_event(&mut self.workspace, &mut self.sessions, id, event);
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
        if !self.editor.active
            && !self.editor.blocked()
            && !self.settings
            && self.profile_dialog.is_none()
            && self.remote_dialog.is_none()
            && self.ai_draft.is_none()
            && self.close_target.is_none()
            && self.restart_target.is_none()
            && ctx.input_mut(|i| i.consume_key(Modifiers::CTRL, Key::F))
        {
            self.show_search();
        }
        if !self.editor.active
            && !self.editor.blocked()
            && !self.settings
            && self.close_target.is_none()
            && self.restart_target.is_none()
            && self.ai_draft.is_none()
            && self.profile_dialog.is_none()
            && self.remote_dialog.is_none()
            && self.search.is_none()
        {
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
                self.save_workspace();
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
                {
                    ui.disable();
                }
                ui.add_space(5.0);
                ui.horizontal_wrapped(|ui| {
                    ui.strong("TERMINALVELOCITY");
                    if ui.button("Editor").clicked() {
                        self.close_search();
                        self.editor.activate();
                    }
                    if ui
                        .add_enabled(
                            !self.settings
                                && self.ai_draft.is_none()
                                && self.close_target.is_none()
                                && self.restart_target.is_none(),
                            egui::Button::new("Verbinden …"),
                        )
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
                        self.save_workspace();
                    }
                    if ui.button("Einstellungen").clicked() {
                        self.settings = !self.settings;
                    }
                    if ui
                        .add_enabled(
                            !self.settings
                                && self.ai_draft.is_none()
                                && self.close_target.is_none()
                                && self.restart_target.is_none(),
                            egui::Button::new("Suchen"),
                        )
                        .on_hover_text("Strg+F · Aktiven Terminalverlauf durchsuchen")
                        .clicked()
                    {
                        self.show_search();
                    }
                    if ui
                        .add_enabled(
                            !self.settings
                                && self.ai_draft.is_none()
                                && self.close_target.is_none()
                                && self.restart_target.is_none(),
                            egui::Button::new("Projekt öffnen"),
                        )
                        .clicked()
                    {
                        self.show_profiles(false);
                    }
                    if ui
                        .add_enabled(
                            !self.settings
                                && self.ai_draft.is_none()
                                && self.close_target.is_none()
                                && self.restart_target.is_none(),
                            egui::Button::new("Projekt speichern …"),
                        )
                        .clicked()
                    {
                        self.show_profiles(true);
                    }
                });
                ui.horizontal_wrapped(|ui| {
                    let mut selected = None;
                    for (index, tab) in self.workspace.tabs.iter().enumerate() {
                        if ui
                            .selectable_label(
                                !self.editor.active && index == self.workspace.active_tab,
                                format!("{}  {}", index + 1, tab.name),
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
        egui::CentralPanel::default()
            .frame(egui::Frame::NONE.fill(background).inner_margin(8.0))
            .show(ctx, |ui| {
                if self.editor.active {
                    let enabled = !self.settings
                        && self.ai_draft.is_none()
                        && self.profile_dialog.is_none()
                        && self.remote_dialog.is_none()
                        && self.close_target.is_none()
                        && self.restart_target.is_none();
                    self.editor.ui(ui, dialog_owner, &editor_base, enabled);
                    return;
                }
                let rect = ui.available_rect_before_wrap();
                if let Some(tab) = self.workspace.tabs.get_mut(self.workspace.active_tab) {
                    let enabled = !self.settings
                        && self.close_target.is_none()
                        && self.restart_target.is_none()
                        && self.ai_draft.is_none()
                        && self.profile_dialog.is_none()
                        && self.remote_dialog.is_none()
                        && self.search.is_none()
                        && !self.editor.blocked();
                    draw_node(
                        ui,
                        rect,
                        &mut tab.root,
                        &mut self.sessions,
                        &mut self.focused,
                        self.workspace.font_size,
                        &self.workspace.effects,
                        self.background_cache.image(&self.workspace.background),
                        enabled,
                        &mut self.close_target,
                        &mut retry_target,
                        &mut ai_selection,
                    );
                }
            });
        if let Some(mut draft) = ai_selection {
            if self.workspace.default_shell.is_ai() {
                draft.provider = self.workspace.default_shell;
            }
            self.ai_draft = Some(draft);
        }
        self.editor.confirmation(ctx, dialog_owner, &editor_base);
        self.profile_window(ctx);
        self.remote_window(ctx);
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
                ui.checkbox(&mut self.workspace.sixel_enabled, "Sixel-Grafiken im Terminal anzeigen");
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
                    if let Some(path) = shell.resolve(Some(&self.workspace.shell_paths[shell as usize]), self.storage.parent().unwrap_or(std::path::Path::new("."))) {
                        ui.label(egui::RichText::new(path.display().to_string()).small().color(Color32::LIGHT_GREEN));
                    } else { ui.colored_label(Color32::YELLOW, "Nicht gefunden"); }
                }
                });
                apply_paths = ui.button("Speichern und fehlgeschlagene Sitzungen neu starten").clicked();
                ui.small("Laufende Sitzungen bleiben aktiv. Geänderte Pfade gelten für neue Sitzungen.");
                ui.separator();
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
                    let _ = std::fs::write(
                        self.storage.with_file_name("transparency-status.txt"),
                        format!("Windows window alpha applied: {desired_opacity:?}"),
                    );
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

fn smoke_test() -> Result<(), Box<dyn std::error::Error>> {
    use std::time::{Duration, Instant};
    let ctx = egui::Context::default();
    let mut report = vec![];
    for (index, shell) in Shell::ALL.into_iter().enumerate() {
        let Some(executable) = shell.executable() else {
            report.push(format!("{}: NICHT INSTALLIERT", shell.label()));
            continue;
        };
        let (sender, receiver) = mpsc::channel();
        let args: Vec<String> = match shell {
            Shell::PowerShell => vec!["-NoProfile", "-NoLogo"],
            Shell::Cmd => vec!["/D", "/Q"],
            Shell::GitBash => vec!["--noprofile", "--norc", "-i"],
            _ => vec![],
        }
        .into_iter()
        .map(String::from)
        .collect();
        let (executable, args) = if shell.is_ai() || shell == Shell::PowerShell {
            let base = std::env::current_exe()?.parent().unwrap().to_path_buf();
            shell.launch(executable, &base, true)?
        } else {
            (executable, args)
        };
        let expected_version = if shell.is_ai() {
            use std::os::windows::process::CommandExt;
            let output = std::process::Command::new(&executable)
                .args(&args)
                .creation_flags(0x08000000)
                .output()?;
            if !output.status.success() {
                return Err(format!(
                    "{} --version fehlgeschlagen: {}",
                    shell.label(),
                    String::from_utf8_lossy(&output.stderr)
                )
                .into());
            }
            let version = String::from_utf8_lossy(&output.stdout).trim().to_string();
            if version.is_empty() {
                return Err("Leere CLI-Versionsausgabe".into());
            }
            Some(version)
        } else {
            None
        };
        let mut backend = TerminalBackend::new(
            index as u64,
            ctx.clone(),
            sender,
            BackendSettings {
                shell: executable.display().to_string(),
                args,
                working_directory: Some(std::env::current_dir()?),
            },
        )?;
        let deadline = Instant::now() + Duration::from_secs(25);
        let command = match shell {
            Shell::PowerShell => "codex --version; Write-Output ('TERMINALVELOCITY_' + 'OK')\r",
            Shell::Cmd => "set TV=TERMINALVELOCITY_\r\necho %TV%OK\r\n",
            Shell::GitBash => "printf 'TERMINALVELOCITY_%s\\n' OK\r",
            _ => "",
        };
        backend.process_command(egui_term::BackendCommand::Write(
            command.as_bytes().to_vec(),
        ));
        let mut found = false;
        while Instant::now() < deadline {
            let content = backend.sync();
            let text: String = content.grid.display_iter().map(|c| c.c).collect();
            if (text.contains("TERMINALVELOCITY_OK")
                && (shell != Shell::PowerShell || text.contains("codex-cli ")))
                || expected_version
                    .as_ref()
                    .is_some_and(|version| text.contains(version))
            {
                found = true;
                break;
            }
            let _ = receiver.recv_timeout(Duration::from_millis(50));
        }
        report.push(format!(
            "{}: {} ({})",
            shell.label(),
            if found { "OK" } else { "FEHLER" },
            executable.display()
        ));
        if !found {
            return Err(format!("PTY-Test fehlgeschlagen: {}", shell.label()).into());
        }
    }
    let path = std::env::current_exe()?.with_file_name("shell-test.txt");
    std::fs::write(path, report.join("\n"))?;
    Ok(())
}

fn directory_test() -> Result<(), Box<dyn std::error::Error>> {
    use std::time::{Duration, Instant};
    fn wait_for_directory(
        backend: &mut TerminalBackend,
        receiver: &Receiver<(u64, PtyEvent)>,
        expected: &std::path::Path,
    ) -> Result<String, String> {
        let deadline = Instant::now() + Duration::from_secs(20);
        let mut last_title = String::new();
        while Instant::now() < deadline {
            backend.sync();
            if let Ok((_, PtyEvent::Title(title))) =
                receiver.recv_timeout(Duration::from_millis(50))
            {
                if title
                    .strip_prefix("TERMINALVELOCITY_CWD:")
                    .is_some_and(|path| {
                        PathBuf::from(path)
                            .canonicalize()
                            .ok()
                            .zip(expected.canonicalize().ok())
                            .is_some_and(|(actual, expected)| actual == expected)
                    })
                {
                    return Ok(title);
                }
                last_title = title;
            }
        }
        Err(format!(
            "Ordner nicht gemeldet: {} (letzter Titel: {last_title:?})",
            expected.display()
        ))
    }
    let base = std::env::current_exe()?.parent().unwrap().to_path_buf();
    let test_root = std::env::temp_dir().join(format!(
        "TerminalVelocity cwd O'Brien ü {}",
        std::process::id()
    ));
    std::fs::create_dir(&test_root)?;
    let result = (|| -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let mut report = vec![];
        for shell in [Shell::PowerShell, Shell::Cmd, Shell::GitBash] {
            let Some(program) = shell.resolve(None, &base) else {
                report.push(format!("{}: nicht installiert", shell.label()));
                continue;
            };
            let (program, args) = shell.launch(program, &base, false)?;
            let (sender, receiver) = mpsc::channel();
            let mut backend = TerminalBackend::new(
                1,
                egui::Context::default(),
                sender,
                BackendSettings {
                    shell: program.display().to_string(),
                    args,
                    working_directory: Some(base.clone()),
                },
            )?;
            wait_for_directory(&mut backend, &receiver, &base)?;
            let command = match shell {
                Shell::PowerShell => format!(
                    "Set-Location -LiteralPath '{}'\r",
                    test_root.to_string_lossy().replace('\'', "''")
                ),
                Shell::Cmd => format!("cd /d \"{}\"\r", test_root.display()),
                _ => format!(
                    "cd '{}'\r",
                    test_root
                        .to_string_lossy()
                        .replace('\\', "/")
                        .replace('\'', "'\\''")
                ),
            };
            backend.process_command(egui_term::BackendCommand::Write(command.into_bytes()));
            let title = wait_for_directory(&mut backend, &receiver, &test_root)?;
            let mut workspace = Workspace::default();
            workspace.tabs.push(Tab {
                name: "source".into(),
                root: Node::Pane(Pane {
                    remote: None,
                    id: 1,
                    shell,
                    directory: base.clone(),
                }),
            });
            if !workspace.update_directory(1, &title) {
                return Err("Ordnerereignis wurde nicht übernommen".into());
            }
            let inherited = workspace.inherited_directory(1, &base);
            let next_program = Shell::PowerShell
                .resolve(None, &base)
                .ok_or("PowerShell fehlt")?;
            let (next_program, next_args) = Shell::PowerShell.launch(next_program, &base, false)?;
            let (next_sender, next_receiver) = mpsc::channel();
            let mut next = TerminalBackend::new(
                2,
                egui::Context::default(),
                next_sender,
                BackendSettings {
                    shell: next_program.display().to_string(),
                    args: next_args,
                    working_directory: Some(inherited),
                },
            )?;
            wait_for_directory(&mut next, &next_receiver, &test_root)?;
            report.push(format!("{}: cd erkannt; neuer Bereich im übernommenen Ordner gestartet (Leerzeichen, Apostroph, Unicode)", shell.label()));
        }
        Ok(report)
    })();
    // ConPTY releases its current-directory handle asynchronously on shutdown.
    let mut cleanup = std::fs::remove_dir(&test_root);
    for _ in 0..30 {
        if cleanup.is_ok() {
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
        cleanup = std::fs::remove_dir(&test_root);
    }
    let report = result?;
    cleanup?;
    std::fs::write(base.join("directory-test.txt"), report.join("\n"))?;
    Ok(())
}

fn restart_test() -> Result<(), Box<dyn std::error::Error>> {
    use std::time::{Duration, Instant};
    fn wait(app: &mut TerminalApp, predicate: impl Fn(&TerminalApp) -> bool) -> Result<(), String> {
        let deadline = Instant::now() + Duration::from_secs(20);
        while Instant::now() < deadline {
            while let Ok((generation, event)) = app.receiver.try_recv() {
                handle_session_event(&mut app.workspace, &mut app.sessions, generation, event);
            }
            if predicate(app) {
                return Ok(());
            }
            std::thread::sleep(Duration::from_millis(30));
        }
        Err("Zeitüberschreitung beim Sitzungsneustart-Test".into())
    }
    fn wait_output(app: &mut TerminalApp, marker: &str) -> Result<(), String> {
        let deadline = Instant::now() + Duration::from_secs(20);
        while Instant::now() < deadline {
            let session = app.sessions.get_mut(&1).ok_or("Testbereich fehlt")?;
            let backend = session.backend.as_mut().ok_or("Test-PTY fehlt")?;
            let content = backend.sync();
            let text: String = content.grid.display_iter().map(|cell| cell.c).collect();
            if text.contains(marker) {
                return Ok(());
            }
            while let Ok((generation, event)) = app.receiver.try_recv() {
                handle_session_event(&mut app.workspace, &mut app.sessions, generation, event);
            }
            std::thread::sleep(Duration::from_millis(30));
        }
        Err(format!("Testausgabe fehlt: {marker}"))
    }
    fn send(app: &mut TerminalApp, text: &str) {
        app.sessions
            .get_mut(&1)
            .unwrap()
            .backend
            .as_mut()
            .unwrap()
            .process_command(egui_term::BackendCommand::Write(text.as_bytes().to_vec()));
    }
    let base = std::env::current_exe()?.parent().unwrap().to_path_buf();
    let root =
        std::env::temp_dir().join(format!("TerminalVelocity restart ü {}", std::process::id()));
    std::fs::create_dir(&root)?;
    let result = (|| -> Result<(), Box<dyn std::error::Error>> {
        let mut workspace = Workspace::default();
        workspace.tabs = vec![Tab {
            name: "test".into(),
            root: Node::Split {
                axis: Axis::Columns,
                ratio: 0.5,
                first: Box::new(Node::Pane(Pane {
                    remote: None,
                    id: 1,
                    shell: Shell::PowerShell,
                    directory: base.clone(),
                })),
                second: Box::new(Node::Pane(Pane {
                    remote: None,
                    id: 2,
                    shell: Shell::Cmd,
                    directory: base.clone(),
                })),
            },
        }];
        let (sender, receiver) = mpsc::channel();
        let mut app = TerminalApp {
            editor: editor::Editor::default(),
            background_cache: background::Cache::default(),
            workspace,
            sessions: HashMap::new(),
            sender,
            receiver,
            focused: 1,
            next_id: 3,
            next_session_id: 1,
            settings: false,
            directory: base.display().to_string(),
            status: String::new(),
            storage: base.join("unused-restart-test.json"),
            close_target: None,
            restart_target: None,
            ai_draft: None,
            profiles: profiles::Library::default(),
            profile_dialog: None,
            remote_dialog: None,
            profiles_error: None,
            search: None,
            applied_acrylic: false,
            applied_opacity: None,
        };
        let ctx = egui::Context::default();
        for pane in app.workspace.panes() {
            app.start(&ctx, &pane);
        }
        if app.sessions.values().any(|session| session.error.is_some()) {
            return Err("Test-Shell konnte nicht gestartet werden".into());
        }
        let sibling_generation = app.sessions[&2].generation;
        send(
            &mut app,
            &format!(
                "Set-Location -LiteralPath '{}'\r",
                root.to_string_lossy().replace('\'', "''")
            ),
        );
        wait(&mut app, |app| {
            app.workspace
                .inherited_directory(1, &base)
                .canonicalize()
                .ok()
                == root.canonicalize().ok()
        })?;
        send(
            &mut app,
            "$global:RestartProbe = 'old'; Write-Output ('RESTART_' + 'OLD')\r",
        );
        wait_output(&mut app, "RESTART_OLD")?;
        let old_generation = app.sessions[&1].generation;
        app.restart_pane(1, &ctx);
        if app.sessions[&1].generation == old_generation {
            return Err("Alte Sitzung wurde wiederverwendet".into());
        }
        handle_session_event(
            &mut app.workspace,
            &mut app.sessions,
            old_generation,
            PtyEvent::Exit,
        );
        handle_session_event(
            &mut app.workspace,
            &mut app.sessions,
            old_generation,
            PtyEvent::Title(format!("TERMINALVELOCITY_CWD:{}", base.display())),
        );
        if app.sessions[&1].exited
            || app
                .workspace
                .inherited_directory(1, &base)
                .canonicalize()
                .ok()
                != root.canonicalize().ok()
        {
            return Err("Alte Ereignisse haben neue Sitzung verändert".into());
        }
        send(&mut app, "if (Test-Path Variable:global:RestartProbe) { Write-Output ('RESTART_' + 'STALE') } else { Write-Output ('RESTART_' + 'FRESH') }; Write-Output ('DIR:' + $PWD.ProviderPath)\r");
        wait_output(&mut app, "RESTART_FRESH")?;
        wait_output(
            &mut app,
            &format!("DIR:{}", root.canonicalize()?.display()).replace("\\\\?\\", ""),
        )?;
        send(&mut app, "exit\r");
        wait(&mut app, |app| app.sessions[&1].exited)?;
        app.restart_pane(1, &ctx);
        send(&mut app, "Write-Output ('RESTART_' + 'AFTER_EXIT')\r");
        wait_output(&mut app, "RESTART_AFTER_EXIT")?;
        app.workspace.tabs[0].root.pane_mut(1).unwrap().directory = root.join("missing-directory");
        app.restart_pane(1, &ctx);
        if app.sessions[&1].error.is_none() || app.sessions[&1].backend.is_some() {
            return Err("Fehlender Projektordner wurde nicht als Fehler gemeldet".into());
        }
        app.workspace.tabs[0].root.pane_mut(1).unwrap().directory = root.clone();
        app.restart_pane(1, &ctx);
        send(&mut app, "Write-Output ('RESTART_' + 'AFTER_ERROR')\r");
        wait_output(&mut app, "RESTART_AFTER_ERROR")?;
        if app.sessions[&2].generation != sibling_generation
            || app.sessions[&2].exited
            || app.workspace.panes().len() != 2
            || app.workspace.panes()[0].shell != Shell::PowerShell
        {
            return Err("Anderer Bereich oder Anbieter wurde verändert".into());
        }
        Ok(())
    })();
    let mut cleanup = std::fs::remove_dir(&root);
    for _ in 0..30 {
        if cleanup.is_ok() {
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
        cleanup = std::fs::remove_dir(&root);
    }
    result?;
    cleanup?;
    std::fs::write(base.join("restart-test.txt"), "OK: laufende Sitzung ersetzt; Prozesszustand und Ausgabe frisch; aktueller Projektordner erhalten; Neustart nach Exit und Startfehler; alte Ereignisse ignoriert; Nachbarbereich und Anbieter unverändert.")?;
    Ok(())
}

fn handoff_test() -> Result<(), Box<dyn std::error::Error>> {
    use std::time::{Duration, Instant};
    fn wait_text(backend: &mut TerminalBackend, marker: &str) -> Result<(), String> {
        let deadline = Instant::now() + Duration::from_secs(20);
        while Instant::now() < deadline {
            let text: String = backend
                .sync()
                .grid
                .display_iter()
                .map(|cell| cell.c)
                .collect();
            if text.contains(marker) {
                return Ok(());
            }
            std::thread::sleep(Duration::from_millis(30));
        }
        Err(format!(
            "Testausgabe fehlt: {marker}; tatsächlich: {:?}",
            backend
                .sync()
                .grid
                .display_iter()
                .map(|cell| cell.c)
                .collect::<String>()
        ))
    }
    let base = std::env::current_exe()?.parent().unwrap().to_path_buf();
    let root = std::env::temp_dir().join(format!("TerminalVelocity KI ü {}", std::process::id()));
    std::fs::create_dir(&root)?;
    let probe = root.join("prompt-probe.exe");
    std::fs::copy(base.join("prompt-probe.exe"), &probe)?;
    let result = (|| -> Result<(), Box<dyn std::error::Error>> {
        let executable = Shell::PowerShell
            .resolve(None, &base)
            .ok_or("PowerShell fehlt")?;
        let (program, args) = Shell::PowerShell.launch(executable, &base, false)?;
        let (sender, _receiver) = mpsc::channel();
        let mut source = TerminalBackend::new(
            1,
            egui::Context::default(),
            sender,
            BackendSettings {
                shell: program.display().to_string(),
                args,
                working_directory: Some(base.clone()),
            },
        )?;
        source.process_command(egui_term::BackendCommand::Write(
            b"Write-Output ('FIRST_' + 'LINE'); Write-Output ('SECOND_' + 'LINE')\r".to_vec(),
        ));
        wait_text(&mut source, "SECOND_LINE")?;
        let content = source.sync();
        let first = content
            .grid
            .display_iter()
            .find(|cell| cell.c == 'F' && cell.point.column.0 == 0)
            .ok_or("Erste Ausgabe fehlt")?
            .point;
        let second = content
            .grid
            .display_iter()
            .find(|cell| cell.c == 'S' && cell.point.column.0 == 0)
            .ok_or("Zweite Ausgabe fehlt")?
            .point;
        source.process_command(egui_term::BackendCommand::SelectStart(
            egui_term::SelectionType::Simple,
            0.0,
            first.line.0 as f32,
        ));
        source.process_command(egui_term::BackendCommand::SelectUpdate(
            11.0,
            second.line.0 as f32,
        ));
        let selected = source.selectable_content();
        if selected != "FIRST_LINE\nSECOND_LINE" {
            return Err(format!("Mehrzeilige Auswahl falsch: {selected:?}").into());
        }
        drop(source);
        let text = format!("{selected}\n--help\n'ü' & %PATH% $(whoami) ; \"text\" C:\\end\\");
        if let Some(gemini) = Shell::Gemini.resolve(None, &base) {
            let draft = ai::Draft::new(text.clone(), root.clone(), Shell::Gemini);
            let (program, args) = draft.command(gemini)?;
            use std::os::windows::process::CommandExt;
            let prefix = &args[..args.len() - 1];
            let output = std::process::Command::new(program)
                .args(prefix)
                .arg("--version")
                .creation_flags(0x08000000)
                .output()?;
            if !output.status.success() {
                return Err("Direkter Gemini-/Node-Start fehlgeschlagen".into());
            }
        }
        for (provider, text) in ai::PROVIDERS.into_iter().flat_map(|provider| {
            [
                text.clone(),
                "a\nb".into(),
                "--help".into(),
                "@private-file".into(),
            ]
            .into_iter()
            .chain(
                ai::QuickAction::ALL
                    .into_iter()
                    .map(|action| action.prompt(&text)),
            )
            .map(move |text| (provider, text))
        }) {
            let mut draft = ai::Draft::new(text.clone(), root.clone(), provider);
            draft.model = "test-model".into();
            let (program, args) = draft.command(probe.clone())?;
            let expected: Vec<String> = args
                .iter()
                .map(|arg| {
                    format!(
                        "AI_ARG:{}",
                        arg.as_bytes()
                            .iter()
                            .map(|byte| format!("{byte:02X}"))
                            .collect::<String>()
                    )
                })
                .collect();
            let (sender, _receiver) = mpsc::channel();
            let mut backend = TerminalBackend::new(
                2,
                egui::Context::default(),
                sender,
                BackendSettings {
                    shell: program.display().to_string(),
                    args,
                    working_directory: Some(root.clone()),
                },
            )?;
            for marker in expected {
                wait_text(&mut backend, &marker)?;
            }
            wait_text(&mut backend, &format!("AI_CWD:{}", root.display()))?;
        }
        Ok(())
    })();
    let mut removed = std::fs::remove_file(&probe);
    for _ in 0..30 {
        if removed.is_ok() {
            break;
        }
        std::thread::sleep(Duration::from_millis(100));
        removed = std::fs::remove_file(&probe);
    }
    result?;
    removed?;
    std::fs::remove_dir(&root)?;
    std::fs::write(base.join("handoff-test.txt"), "OK: echte mehrzeilige Terminalauswahl; alle vier Anbieter-Argumente über echte Windows-PTYs mit Testprogramm geprüft (Unicode, Anführungszeichen, Zeilenumbrüche, Shell-Sonderzeichen); richtiger Projektordner. Keine KI-Anfragen gesendet.")?;
    Ok(())
}

fn profile_test() -> Result<(), Box<dyn std::error::Error>> {
    use std::time::{Duration, Instant};
    fn wait_output(backend: &mut TerminalBackend, marker: &str) -> Result<(), String> {
        let deadline = Instant::now() + Duration::from_secs(20);
        loop {
            let text: String = backend
                .sync()
                .grid
                .display_iter()
                .map(|cell| cell.c)
                .collect();
            if text.contains(marker) {
                return Ok(());
            }
            if Instant::now() >= deadline {
                return Err(format!("Profil-PTY: {marker} fehlt: {text}"));
            }
            std::thread::sleep(Duration::from_millis(30));
        }
    }
    let base = std::env::current_exe()?.parent().unwrap().to_path_buf();
    let root =
        std::env::temp_dir().join(format!("TerminalVelocity Profile ü {}", std::process::id()));
    std::fs::create_dir(&root)?;
    let result = (|| -> Result<(), Box<dyn std::error::Error>> {
        let old = root.join("original");
        let new = root.join("relocated");
        std::fs::create_dir_all(old.join("src"))?;
        std::fs::create_dir_all(new.join("src"))?;
        let workspace = Workspace {
            tabs: vec![Tab {
                name: "Projekt".into(),
                root: Node::Split {
                    axis: Axis::Columns,
                    ratio: 0.4,
                    first: Box::new(Node::Pane(Pane {
                        remote: None,
                        id: 1,
                        shell: Shell::Cmd,
                        directory: old.clone(),
                    })),
                    second: Box::new(Node::Pane(Pane {
                        remote: None,
                        id: 2,
                        shell: Shell::Cmd,
                        directory: old.join("src"),
                    })),
                },
            }],
            default_shell: Shell::Cmd,
            ..Workspace::default()
        };
        let profile = profiles::Profile::capture("Testprojekt", old, &workspace)?;
        let (sender, receiver) = mpsc::channel();
        let mut app = TerminalApp {
            editor: editor::Editor::default(),
            background_cache: background::Cache::default(),
            workspace,
            sessions: HashMap::new(),
            sender,
            receiver,
            focused: 1,
            next_id: 3,
            next_session_id: 1,
            settings: false,
            directory: base.display().to_string(),
            status: String::new(),
            storage: base.join("unused-profile-test.json"),
            close_target: None,
            restart_target: None,
            ai_draft: None,
            profiles: profiles::Library {
                version: 1,
                profiles: vec![profile],
            },
            profile_dialog: None,
            remote_dialog: None,
            profiles_error: None,
            search: None,
            applied_acrylic: false,
            applied_opacity: None,
        };
        let ctx = egui::Context::default();
        for pane in app.workspace.panes() {
            app.start(&ctx, &pane);
        }
        let generations = [app.sessions[&1].generation, app.sessions[&2].generation];
        app.sessions
            .get_mut(&1)
            .unwrap()
            .backend
            .as_mut()
            .ok_or("Alte PTY fehlt")?
            .process_command(egui_term::BackendCommand::Write(
                b"set TV_PROFILE_STATE=OLD\r".to_vec(),
            ));
        if app.open_profile(0, root.join("missing"), &ctx).is_ok()
            || app.sessions.len() != 2
            || app.workspace.tabs.len() != 1
        {
            return Err("Fehlerhafter Profilstart hat den Arbeitsbereich verändert".into());
        }
        app.open_profile(0, new.clone(), &ctx)?;
        if app.workspace.tabs.len() != 2
            || app.sessions.len() != 4
            || app.workspace.active_tab != 1
            || app.sessions[&1].generation != generations[0]
            || app.sessions[&2].generation != generations[1]
        {
            return Err("Profilstart hat laufende Sitzungen oder das Layout verändert".into());
        }
        for (id, directory) in [(3, new.clone()), (4, new.join("src"))] {
            let backend = app
                .sessions
                .get_mut(&id)
                .unwrap()
                .backend
                .as_mut()
                .ok_or("Neue PTY fehlt")?;
            backend.process_command(egui_term::BackendCommand::Write(b"@echo off\rif defined TV_PROFILE_STATE (echo STALE_STATE) else (echo FRESH_STATE)\r".to_vec()));
            backend.process_command(egui_term::BackendCommand::Write(
                b"echo PROFILE_CWD:%CD%\r".to_vec(),
            ));
            wait_output(backend, "FRESH_STATE")?;
            wait_output(backend, &format!("PROFILE_CWD:{}", directory.display()))?;
        }
        let original = app.sessions.get_mut(&1).unwrap().backend.as_mut().unwrap();
        original.process_command(egui_term::BackendCommand::Write(
            b"@echo off\recho PRESERVED_STATE:%TV_PROFILE_STATE%\r".to_vec(),
        ));
        wait_output(original, "PRESERVED_STATE:OLD")?;
        drop(app);
        Ok(())
    })();
    // All folders below this uniquely created test root are owned by this test.
    if root.parent() != Some(std::env::temp_dir().as_path())
        || !root
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with("TerminalVelocity Profile ü ")
    {
        return Err("Unerwarteter Testordner; Aufräumen abgebrochen".into());
    }
    let mut cleanup = std::fs::remove_dir_all(&root);
    for _ in 0..30 {
        if cleanup.is_ok()
            || cleanup
                .as_ref()
                .is_err_and(|error| error.kind() == std::io::ErrorKind::NotFound)
        {
            cleanup = Ok(());
            break;
        }
        // PTY shutdown is asynchronous; allow Windows to release child working directories.
        std::thread::sleep(Duration::from_millis(100));
        cleanup = std::fs::remove_dir_all(&root);
    }
    result?;
    cleanup?;
    std::fs::write(base.join("profile-test.txt"), "OK: Projektprofil mit Teilung in neuen Tabs geöffnet; alte PTY-Sitzungen erhalten; frische CMD-Prozesse und umgezogene Projekt-/Unterordner geprüft; fehlender Ordner ohne Änderung abgewiesen. Keine KI-Anfragen gesendet.")?;
    Ok(())
}

fn search_test() -> Result<(), Box<dyn std::error::Error>> {
    use std::time::{Duration, Instant};
    fn wait(backend: &mut TerminalBackend, marker: &str) -> Result<(), String> {
        let deadline = Instant::now() + Duration::from_secs(20);
        loop {
            let text: String = backend
                .sync()
                .grid
                .display_iter()
                .map(|cell| cell.c)
                .collect();
            if text.contains(marker) {
                return Ok(());
            }
            if Instant::now() >= deadline {
                return Err(format!("Suchtest: Ausgabe fehlt: {marker}"));
            }
            std::thread::sleep(Duration::from_millis(30));
        }
    }
    let base = std::env::current_exe()?.parent().unwrap().to_path_buf();
    let (sender, _receiver) = mpsc::channel();
    let executable = Shell::Cmd.resolve(None, &base).ok_or("CMD fehlt")?;
    let (program, _) = Shell::Cmd.launch(executable, &base, false)?;
    let commands = format!("(for /L %i in (1,1,140) do @echo TV_SEARCH_%i literal.[x] Grüße) & echo {}WRAP_NEEDLE & echo TV_SEARCH_DONE", "X".repeat(75));
    let args = vec!["/D".into(), "/Q".into(), "/K".into(), commands];
    let mut backend = TerminalBackend::new(
        1,
        egui::Context::default(),
        sender,
        BackendSettings {
            shell: program.display().to_string(),
            args,
            working_directory: Some(base),
        },
    )?;
    wait(&mut backend, "TV_SEARCH_DONE")?;
    backend.set_search("literal.[x]")?;
    let (start, count, _) = backend.search_status();
    if count != 140 {
        return Err(format!("Wörtliche Suche: erwartet 140 Treffer, erhalten {count}").into());
    }
    backend.next_search_match(false);
    if backend.search_status().0 != start % count + 1 {
        return Err("Vorwärtssuche fehlerhaft".into());
    }
    backend.next_search_match(true);
    if backend.search_status().0 != start {
        return Err("Rückwärtssuche fehlerhaft".into());
    }
    for _ in 0..count {
        backend.next_search_match(false);
    }
    if backend.search_status().0 != start {
        return Err("Umlauf der Suche fehlerhaft".into());
    }
    backend.set_search("TV_SEARCH_1 ")?;
    backend.sync();
    if backend.search_status().1 != 1 || backend.last_content().grid.display_offset() == 0 {
        return Err("Treffer im zurückliegenden Verlauf nicht angesprungen".into());
    }
    if !backend
        .last_content()
        .grid
        .display_iter()
        .any(|cell| backend.search_highlight(cell.point) == Some(true))
    {
        return Err("Aktiver Treffer wird nicht hervorgehoben".into());
    }
    backend.set_search("WRAP_NEEDLE")?;
    backend.sync();
    let lines: std::collections::HashSet<_> = backend
        .last_content()
        .grid
        .display_iter()
        .filter(|cell| backend.search_highlight(cell.point).is_some())
        .map(|cell| cell.point.line.0)
        .collect();
    if backend.search_status().1 != 1 || lines.len() != 2 {
        return Err("Suche über weichen Zeilenumbruch fehlerhaft".into());
    }
    backend.set_search("Grüße")?;
    if backend.search_status().1 != 140 {
        return Err("Unicode-Suche fehlerhaft".into());
    }
    backend.set_search("LITERAL.[x]")?;
    if backend.search_status().1 != 0 {
        return Err("Groß-/Kleinschreibung wird nicht beachtet".into());
    }
    backend.set_search("TV_SEARCH_LIVE")?;
    backend.process_command(egui_term::BackendCommand::Write(
        b"echo TV_SEARCH_LIVE\r".to_vec(),
    ));
    let deadline = Instant::now() + Duration::from_secs(20);
    while backend.search_status().1 == 0 {
        backend.sync();
        if Instant::now() >= deadline {
            return Err("Neue Ausgabe wurde nicht in die Suche aufgenommen".into());
        }
        std::thread::sleep(Duration::from_millis(30));
    }
    if backend.set_search(&"a".repeat(257)).is_ok() {
        return Err("Zu langer Suchtext akzeptiert".into());
    }
    backend.set_search("")?;
    if backend.search_status() != (0, 0, false) {
        return Err("Suche nicht zurückgesetzt".into());
    }
    std::fs::write(std::env::current_exe()?.with_file_name("search-test.txt"), "OK: echte CMD-PTY; 140 wörtliche Treffer im Verlauf; Vor-/Rückwärtssuche und Umlauf; Scrollen zu älteren Treffern und Markierung; weiche Zeilenumbrüche; Unicode; Groß-/Kleinschreibung; neue Ausgabe; Eingabelimit und Leeren der Suche. Keine KI-Anfragen gesendet.")?;
    Ok(())
}

fn main() -> eframe::Result {
    if std::env::args().any(|arg| arg == "--search-test") {
        if let Err(error) = search_test() {
            let _ = std::fs::write(
                std::env::current_exe()
                    .unwrap_or_default()
                    .with_file_name("search-test.txt"),
                format!("FEHLER: {error}"),
            );
            std::process::exit(1);
        }
        return Ok(());
    }
    if std::env::args().any(|arg| arg == "--profile-test") {
        if let Err(error) = profile_test() {
            let _ = std::fs::write(
                std::env::current_exe()
                    .unwrap_or_default()
                    .with_file_name("profile-test.txt"),
                format!("FEHLER: {error}"),
            );
            std::process::exit(1);
        }
        return Ok(());
    }
    if std::env::args().any(|arg| arg == "--handoff-test") {
        if let Err(error) = handoff_test() {
            let _ = std::fs::write(
                std::env::current_exe()
                    .unwrap_or_default()
                    .with_file_name("handoff-test.txt"),
                format!("FEHLER: {error}"),
            );
            std::process::exit(1);
        }
        return Ok(());
    }
    if std::env::args().any(|arg| arg == "--restart-test") {
        if let Err(error) = restart_test() {
            let _ = std::fs::write(
                std::env::current_exe()
                    .unwrap_or_default()
                    .with_file_name("restart-test.txt"),
                format!("FEHLER: {error}"),
            );
            std::process::exit(1);
        }
        return Ok(());
    }
    if std::env::args().any(|arg| arg == "--directory-test") {
        if let Err(error) = directory_test() {
            let path = std::env::current_exe()
                .unwrap_or_default()
                .with_file_name("directory-test.txt");
            let _ = std::fs::write(path, format!("FEHLER: {error}"));
            std::process::exit(1);
        }
        return Ok(());
    }
    if std::env::args().any(|arg| arg == "--smoke-test") {
        if let Err(error) = smoke_test() {
            let path = std::env::current_exe()
                .unwrap_or_default()
                .with_file_name("shell-test.txt");
            let _ = std::fs::write(path, format!("FEHLER: {error}"));
            std::process::exit(1);
        }
        return Ok(());
    }
    eframe::run_native(
        concat!("TerminalVelocity · ", env!("CARGO_PKG_VERSION")),
        eframe::NativeOptions {
            viewport: egui::ViewportBuilder::default()
                .with_inner_size([1200.0, 760.0])
                .with_min_inner_size([800.0, 480.0])
                .with_transparent(true),
            ..Default::default()
        },
        Box::new(|cc| Ok(Box::new(TerminalApp::new(cc)))),
    )
}
