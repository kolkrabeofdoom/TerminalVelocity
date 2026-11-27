//! PTY test mode `--handoff-test`, built only with the `test-tools` feature.

use crate::*;

pub(super) fn handoff_test() -> Result<(), Box<dyn std::error::Error>> {
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
        let (program, args) = Shell::PowerShell.launch(executable, &base, false, "")?;
        let (sender, _receiver) = mpsc::channel();
        let mut source = TerminalBackend::new(
            1,
            egui::Context::default(),
            sender,
            BackendSettings {
                shell: program.display().to_string(),
                args,
                working_directory: Some(base.clone()),
                env: Default::default(),
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
        if let Some(agy) = Shell::Antigravity.resolve(None, &base) {
            let draft = ai::Draft::new(text.clone(), root.clone(), Shell::Antigravity);
            let (program, args) = draft.command(agy)?;
            use std::os::windows::process::CommandExt;
            let prefix = &args[..args.len() - 1];
            let output = std::process::Command::new(program)
                .args(prefix)
                .arg("--version")
                .creation_flags(0x08000000)
                .output()?;
            if !output.status.success() {
                return Err("Direkter Antigravity-Start fehlgeschlagen".into());
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
                    env: Default::default(),
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
