//! PTY test mode `--restart-test`, built only with the `test-tools` feature.

use crate::*;

pub(super) fn restart_test() -> Result<(), Box<dyn std::error::Error>> {
    use std::time::{Duration, Instant};
    fn wait(app: &mut TerminalApp, predicate: impl Fn(&TerminalApp) -> bool) -> Result<(), String> {
        let deadline = Instant::now() + Duration::from_secs(20);
        while Instant::now() < deadline {
            while let Ok((generation, event)) = app.receiver.try_recv() {
                handle_session_event(
                    &mut app.workspace,
                    &mut app.sessions,
                    generation,
                    event,
                    Some(app.focused),
                );
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
                handle_session_event(
                    &mut app.workspace,
                    &mut app.sessions,
                    generation,
                    event,
                    Some(app.focused),
                );
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
        let workspace = Workspace {
            tabs: vec![Tab {
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
            }],
            ..Workspace::default()
        };
        let (sender, receiver) = mpsc::channel();
        let mut app = TerminalApp {
            editor: editor::Editor::default(),
            background_cache: background::Cache::default(),
            image_loader: inline_image::Loader::default(),
            workspace,
            sessions: HashMap::new(),
            sender,
            receiver,
            focused: 1,
            next_id: 3,
            next_session_id: 1,
            settings: false,
            help: false,
            about: false,
            about_banner: None,
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
            workspace_guard: None,
            search: None,
            palette: None,
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
                "Set-Location -LiteralPath {}\r",
                model::ps_literal(&root.to_string_lossy())
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
        let old_token = app.sessions[&1].cwd_token.clone();
        app.restart_pane(1, &ctx);
        if app.sessions[&1].generation == old_generation {
            return Err("Alte Sitzung wurde wiederverwendet".into());
        }
        if app.sessions[&1].cwd_token == old_token {
            return Err("Neue Sitzung hat den alten Ordnerschlüssel übernommen".into());
        }
        handle_session_event(
            &mut app.workspace,
            &mut app.sessions,
            old_generation,
            PtyEvent::Exit,
            Some(app.focused),
        );
        handle_session_event(
            &mut app.workspace,
            &mut app.sessions,
            old_generation,
            PtyEvent::Title(format!(
                "{}{old_token}:{}",
                model::CWD_MARKER,
                base.display()
            )),
            Some(app.focused),
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
