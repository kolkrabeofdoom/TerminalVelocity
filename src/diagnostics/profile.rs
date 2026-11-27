//! PTY test mode `--profile-test`, built only with the `test-tools` feature.

use crate::*;

pub(super) fn profile_test() -> Result<(), Box<dyn std::error::Error>> {
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
