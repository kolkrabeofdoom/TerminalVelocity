//! Terminal sessions: start, restart, close, PTY events, selection hand-off.

use crate::*;

pub(crate) struct Session {
    pub(crate) generation: u64,
    /// Key that a directory report from this session's shell must carry.
    pub(crate) cwd_token: String,
    pub(crate) backend: Option<TerminalBackend>,
    pub(crate) error: Option<String>,
    pub(crate) exited: bool,
    pub(crate) notified: bool,
}

pub(crate) fn handle_session_event(
    workspace: &mut Workspace,
    sessions: &mut HashMap<u64, Session>,
    generation: u64,
    event: PtyEvent,
    active_pane: Option<u64>,
) {
    let Some((&pane_id, session)) = sessions
        .iter_mut()
        .find(|(_, session)| session.generation == generation)
    else {
        return;
    };
    if let PtyEvent::Title(ref title) = event {
        workspace.update_directory(pane_id, title, &session.cwd_token);
    }
    if matches!(event, PtyEvent::Exit | PtyEvent::ChildExit(_)) {
        session.exited = true;
    }
    if matches!(
        event,
        PtyEvent::Bell | PtyEvent::Exit | PtyEvent::ChildExit(_)
    ) && active_pane != Some(pane_id)
    {
        session.notified = true;
    }
}

impl TerminalApp {
    pub(crate) fn clear_tab_notifications(&mut self, tab_index: usize) {
        if let Some(tab) = self.workspace.tabs.get(tab_index) {
            let mut panes = Vec::new();
            tab.root.panes(&mut panes);
            for pane in panes {
                if let Some(session) = self.sessions.get_mut(&pane.id) {
                    session.notified = false;
                }
            }
        }
    }

    pub(crate) fn start(&mut self, ctx: &egui::Context, pane: &Pane) {
        self.start_with_command(ctx, pane, None);
    }

    pub(crate) fn start_with_command(
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
        let cwd_token = model::new_cwd_token();
        let app_dir = self.app_dir().to_path_buf();
        let result = (|| {
            let mut env = Default::default();
            if !pane.directory.is_dir() {
                return Err(trf!(
                    "Startordner fehlt: {directory}",
                    directory = pane.directory.display()
                ));
            }
            let (executable, args) = if let Some(command) = command {
                command
            } else if let Some(remote) = &pane.remote {
                remote.command(&self.workspace, &app_dir)?
            } else {
                let executable = pane
                    .shell
                    .resolve(
                        Some(&self.workspace.shell_paths[pane.shell as usize]),
                        &app_dir,
                    )
                    .ok_or_else(|| {
                        trf!(
                            "{shell} wurde nicht gefunden. Unter Einstellungen → Shell-Programme einen Pfad angeben oder die portable Shell im Programmordner ablegen.",
                            shell = pane.shell.label()
                        )
                    })?;
                env = pane.shell.session_env(&cwd_token);
                pane.shell.launch(executable, &app_dir, false, &cwd_token)?
            };
            TerminalBackend::new(
                generation,
                ctx.clone(),
                self.sender.clone(),
                BackendSettings {
                    shell: executable.to_string_lossy().into_owned(),
                    args,
                    working_directory: Some(pane.directory.clone()),
                    env,
                },
            )
            .map_err(|e| e.to_string())
        })();
        let session = match result {
            Ok(backend) => Session {
                generation,
                cwd_token,
                backend: Some(backend),
                error: None,
                exited: false,
                notified: false,
            },
            Err(error) => Session {
                generation,
                cwd_token: String::new(),
                backend: None,
                error: Some(error),
                exited: false,
                notified: false,
            },
        };
        self.sessions.insert(pane.id, session);
    }

    pub(crate) fn send_selection(&mut self, ctx: &egui::Context) -> Result<(), String> {
        if self.workspace.tabs.len() >= 32 || self.sessions.len() >= 64 {
            return Err(tr("Limit: 32 Tabs / 64 Bereiche erreicht.").into());
        }
        let draft = self
            .ai_draft
            .as_ref()
            .ok_or(tr("Keine Auswahl vorhanden"))?;
        let base = self.app_dir();
        let executable = draft
            .provider
            .resolve(
                Some(&self.workspace.shell_paths[draft.provider as usize]),
                base,
            )
            .ok_or_else(|| {
                trf!(
                    "{provider} nicht gefunden. Bitte den Programmpfad in Einstellungen prüfen.",
                    provider = draft.provider.label()
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
            name: trf!("{shell} · Auswahl", shell = pane.shell.label()),
            root: Node::Pane(pane),
        });
        self.workspace.active_tab = self.workspace.tabs.len() - 1;
        self.ai_draft = None;
        self.status = tr("Auswahl an neue KI-Sitzung übergeben").into();
        Ok(())
    }

    pub(crate) fn restart_pane(&mut self, id: u64, ctx: &egui::Context) {
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
                trf!(
                    "{shell} neu gestartet · {directory}",
                    shell = pane.shell.label(),
                    directory = pane.directory.display()
                )
            } else {
                trf!(
                    "Neustart fehlgeschlagen: {shell}",
                    shell = pane.shell.label()
                )
            };
        }
    }

    pub(crate) fn close_pane(&mut self, id: u64, ctx: &egui::Context) {
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
