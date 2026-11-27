//! PTY test mode `--smoke-test`, built only with the `test-tools` feature.

use crate::*;

pub(super) fn smoke_test() -> Result<(), Box<dyn std::error::Error>> {
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
            shell.launch(executable, &base, true, "")?
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
                env: Default::default(),
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
