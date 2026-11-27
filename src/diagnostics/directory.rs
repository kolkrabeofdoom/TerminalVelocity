//! PTY test mode `--directory-test`, built only with the `test-tools` feature.

use crate::*;

pub(super) fn directory_test() -> Result<(), Box<dyn std::error::Error>> {
    use std::time::{Duration, Instant};
    fn wait_for_directory(
        backend: &mut TerminalBackend,
        receiver: &Receiver<(u64, PtyEvent)>,
        expected: &std::path::Path,
        token: &str,
    ) -> Result<String, String> {
        let deadline = Instant::now() + Duration::from_secs(20);
        let mut last_title = String::new();
        while Instant::now() < deadline {
            backend.sync();
            if let Ok((_, PtyEvent::Title(title))) =
                receiver.recv_timeout(Duration::from_millis(50))
            {
                if model::reported_directory(&title, token).is_some_and(|path| {
                    PathBuf::from(path)
                        .canonicalize()
                        .ok()
                        .zip(expected.canonicalize().ok())
                        .is_some_and(|(actual, expected)| actual == expected)
                }) {
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
        "TerminalVelocity cwd O'Brien D\u{2019}Angelo ł ü {}",
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
            let token = model::new_cwd_token();
            let (program, args) = shell.launch(program, &base, false, &token)?;
            let (sender, receiver) = mpsc::channel();
            let mut backend = TerminalBackend::new(
                1,
                egui::Context::default(),
                sender,
                BackendSettings {
                    shell: program.display().to_string(),
                    args,
                    working_directory: Some(base.clone()),
                    env: shell.session_env(&token),
                },
            )?;
            wait_for_directory(&mut backend, &receiver, &base, &token)?;
            let command = match shell {
                Shell::PowerShell => format!(
                    "Set-Location -LiteralPath {}\r",
                    model::ps_literal(&test_root.to_string_lossy())
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
            let title = wait_for_directory(&mut backend, &receiver, &test_root, &token)?;
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
            if workspace.update_directory(1, &title, &model::new_cwd_token()) {
                return Err("Ordnerereignis mit fremdem Schlüssel wurde übernommen".into());
            }
            if !workspace.update_directory(1, &title, &token) {
                return Err("Ordnerereignis wurde nicht übernommen".into());
            }
            let inherited = workspace.inherited_directory(1, &base);
            let next_program = Shell::PowerShell
                .resolve(None, &base)
                .ok_or("PowerShell fehlt")?;
            let next_token = model::new_cwd_token();
            let (next_program, next_args) =
                Shell::PowerShell.launch(next_program, &base, false, &next_token)?;
            let (next_sender, next_receiver) = mpsc::channel();
            let mut next = TerminalBackend::new(
                2,
                egui::Context::default(),
                next_sender,
                BackendSettings {
                    shell: next_program.display().to_string(),
                    args: next_args,
                    working_directory: Some(inherited),
                    env: Default::default(),
                },
            )?;
            wait_for_directory(&mut next, &next_receiver, &test_root, &next_token)?;
            report.push(format!("{}: cd erkannt; neuer Bereich im übernommenen Ordner gestartet (Leerzeichen, Apostroph, typografischer Apostroph, Zeichen außerhalb der Codepage, Unicode)", shell.label()));
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
