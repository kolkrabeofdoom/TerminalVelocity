//! Application state: the `TerminalApp` struct and its construction.

mod frame;
mod layout;
mod persistence;
mod session;
#[cfg(test)]
mod tests;

pub(crate) use layout::*;
pub(crate) use persistence::*;
pub(crate) use session::*;

use crate::*;

pub(crate) struct TerminalApp {
    pub(crate) editor: editor::Editor,
    pub(crate) background_cache: background::Cache,
    pub(crate) image_loader: inline_image::Loader,
    pub(crate) workspace: Workspace,
    pub(crate) sessions: HashMap<u64, Session>,
    pub(crate) sender: Sender<(u64, PtyEvent)>,
    pub(crate) receiver: Receiver<(u64, PtyEvent)>,
    pub(crate) focused: u64,
    pub(crate) next_id: u64,
    pub(crate) next_session_id: u64,
    pub(crate) settings: bool,
    pub(crate) help: bool,
    pub(crate) about: bool,
    pub(crate) about_banner: Option<egui::TextureHandle>,
    pub(crate) directory: String,
    pub(crate) status: String,
    pub(crate) storage: PathBuf,
    pub(crate) close_target: Option<u64>,
    pub(crate) restart_target: Option<u64>,
    pub(crate) ai_draft: Option<ai::Draft>,
    pub(crate) profiles: profiles::Library,
    pub(crate) profile_dialog: Option<profiles::Dialog>,
    pub(crate) remote_dialog: Option<remote::Dialog>,
    pub(crate) profiles_error: Option<String>,
    pub(crate) workspace_guard: Option<model::WorkspaceGuard>,
    pub(crate) search: Option<HistorySearch>,
    pub(crate) palette: Option<CommandPalette>,
    pub(crate) applied_acrylic: bool,
    pub(crate) applied_opacity: Option<Option<u8>>,
}

impl TerminalApp {
    pub(crate) fn new(cc: &eframe::CreationContext<'_>) -> Self {
        cc.egui_ctx.set_visuals(egui::Visuals::dark());
        let mut fonts = egui::FontDefinitions::default();
        if let Some(bytes) = model::system_font_path("consola.ttf", |key| std::env::var(key).ok())
            .and_then(|path| std::fs::read(path).ok())
        {
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
        let (workspace, load_status, workspace_guard) = load_workspace(&storage);
        i18n::set_language(workspace.language);
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
            image_loader: inline_image::Loader::default(),
            workspace,
            sessions: HashMap::new(),
            sender,
            receiver,
            focused: 0,
            next_id,
            next_session_id: 1,
            settings: false,
            help: false,
            about: false,
            about_banner: None,
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
            workspace_guard,
            search: None,
            palette: None,
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

    pub(crate) fn app_dir(&self) -> &std::path::Path {
        self.storage.parent().unwrap_or(std::path::Path::new("."))
    }

    pub(crate) fn has_modal_dialog(&self) -> bool {
        self.editor.blocked()
            || self.settings
            || self.help
            || self.about
            || self.profile_dialog.is_some()
            || self.remote_dialog.is_some()
            || self.ai_draft.is_some()
            || self.close_target.is_some()
            || self.restart_target.is_some()
    }

    pub(crate) fn is_terminal_interactive(&self) -> bool {
        !self.editor.active
            && !self.has_modal_dialog()
            && self.palette.is_none()
            && self.search.is_none()
    }
}
