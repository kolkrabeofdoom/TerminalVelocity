use crate::model::{Shell, Workspace};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Transport {
    #[default]
    Ssh,
    WinRm,
}

#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Connection {
    pub name: String,
    pub transport: Transport,
    pub host: String,
    pub user: String,
    // None preserves the SSH config or WinRM transport default.
    pub port: Option<u16>,
    pub key_file: String,
    pub https: bool,
    pub endpoint: String,
    pub favorite: bool,
}

impl Connection {
    pub fn label(&self) -> String {
        format!(
            "{} · {}{}",
            if self.transport == Transport::Ssh {
                "SSH"
            } else {
                "WinRM"
            },
            if self.user.is_empty() {
                String::new()
            } else {
                format!("{}@", self.user)
            },
            self.host
        )
    }

    pub fn validate(&self) -> Result<(), String> {
        if self.name.trim().is_empty()
            || self.name.chars().count() > 80
            || self.name.chars().any(char::is_control)
        {
            return Err("Verbindungsname: 1 bis 80 Zeichen erforderlich.".into());
        }
        if self.host.is_empty()
            || self.host.len() > 253
            || self.host.starts_with('-')
            || !self
                .host
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || ".-_:[]%".contains(c))
        {
            return Err("Bitte einen Hostnamen, eine IP-Adresse oder einen SSH-Hostalias eingeben (ohne Konto oder Optionen).".into());
        }
        for value in [&self.user, &self.key_file, &self.endpoint] {
            if value.len() > 4096 || value.chars().any(char::is_control) {
                return Err("Konto, Schlüsselpfad und Endpunkt dürfen keine Steuerzeichen enthalten und höchstens 4096 Bytes lang sein.".into());
            }
        }
        if self.port == Some(0) {
            return Err("Der Port muss zwischen 1 und 65535 liegen.".into());
        }
        if !self.key_file.is_empty() && !Path::new(&self.key_file).is_absolute() {
            return Err("Bitte einen absoluten Pfad zur Schlüsseldatei angeben.".into());
        }
        Ok(())
    }

    pub fn ssh_args(&self) -> Result<Vec<String>, String> {
        self.validate()?;
        let mut args = vec!["-tt".into()];
        if let Some(port) = self.port {
            args.extend(["-p".into(), port.to_string()]);
        }
        if !self.user.is_empty() {
            args.extend(["-l".into(), self.user.clone()]);
        }
        if !self.key_file.is_empty() {
            args.extend(["-i".into(), self.key_file.clone()]);
        }
        args.extend(["--".into(), self.host.clone()]);
        Ok(args)
    }

    pub fn winrm_script(&self) -> Result<String, String> {
        self.validate()?;
        fn literal(value: &str) -> String {
            format!("'{}'", value.replace('\'', "''"))
        }
        let mut command = format!(
            "Enter-PSSession -ComputerName {} -ErrorAction Stop",
            literal(&self.host)
        );
        if let Some(port) = self.port {
            command.push_str(&format!(" -Port {port}"));
        }
        if self.https {
            command.push_str(" -UseSSL");
        }
        if !self.endpoint.is_empty() {
            command.push_str(&format!(" -ConfigurationName {}", literal(&self.endpoint)));
        }
        let credential = if self.user.is_empty() {
            String::new()
        } else {
            command.push_str(" -Credential $credential");
            format!("$credential = Get-Credential -UserName {} -Message 'Remote-Anmeldung'; if ($null -eq $credential) {{ throw 'Anmeldung abgebrochen' }}; ", literal(&self.user))
        };
        // After Exit-PSSession the process exits: never leave a local prompt under a remote label.
        Ok(format!("function global:prompt {{ [Environment]::Exit(0) }}; try {{ {credential}{command} }} catch {{ Write-Error $_; exit 1 }}"))
    }

    pub fn command(
        &self,
        workspace: &Workspace,
        base: &Path,
    ) -> Result<(PathBuf, Vec<String>), String> {
        self.validate()?;
        match self.transport {
            Transport::Ssh => {
                if !self.key_file.is_empty() && !Path::new(&self.key_file).is_file() {
                    return Err("Die SSH-Schlüsseldatei wurde nicht gefunden.".into());
                }
                Ok((
                    ssh_executable(&workspace.ssh_path, base).ok_or(
                        "OpenSSH ssh.exe nicht gefunden. Bitte den Pfad in Einstellungen angeben.",
                    )?,
                    self.ssh_args()?,
                ))
            }
            Transport::WinRm => {
                let program = Shell::PowerShell
                    .resolve(workspace.shell_paths.first().map(String::as_str), base)
                    .ok_or(
                        "PowerShell 7 nicht gefunden. Bitte den Pfad in Einstellungen angeben.",
                    )?;
                Ok((
                    program,
                    vec![
                        "-NoLogo".into(),
                        "-NoProfile".into(),
                        "-NoExit".into(),
                        "-Command".into(),
                        self.winrm_script()?,
                    ],
                ))
            }
        }
    }
}

pub fn ssh_executable(custom: &str, base: &Path) -> Option<PathBuf> {
    if !custom.trim().is_empty() {
        let path = PathBuf::from(custom.trim().trim_matches('"'));
        let path = if path.is_absolute() {
            path
        } else {
            base.join(path)
        };
        return path.is_file().then_some(path);
    }
    if let Some(root) = std::env::var_os("SystemRoot") {
        let path = PathBuf::from(root).join("System32/OpenSSH/ssh.exe");
        if path.is_file() {
            return Some(path);
        }
    }
    std::env::var_os("PATH").and_then(|paths| {
        std::env::split_paths(&paths)
            .map(|p| p.join("ssh.exe"))
            .find(|p| p.is_file())
    })
}

#[derive(Default)]
pub struct Dialog {
    pub connection: Connection,
    pub selected: Option<usize>,
    pub port: String,
    pub placement: usize,
    pub error: Option<String>,
    pub delete: bool,
}

#[cfg(test)]
mod tests {
    use super::*;
    fn connection() -> Connection {
        Connection {
            name: "Test".into(),
            host: "server-alias".into(),
            ..Connection::default()
        }
    }
    #[test]
    fn ssh_preserves_config_defaults_and_separates_arguments() {
        let mut c = connection();
        assert_eq!(c.ssh_args().unwrap(), ["-tt", "--", "server-alias"]);
        c.user = "user with spaces".into();
        c.port = Some(2222);
        c.key_file = "C:/keys/O'Brien ü".into();
        assert_eq!(
            c.ssh_args().unwrap(),
            [
                "-tt",
                "-p",
                "2222",
                "-l",
                "user with spaces",
                "-i",
                "C:/keys/O'Brien ü",
                "--",
                "server-alias"
            ]
        );
        for host in [
            "-oProxyCommand=bad",
            "user@host",
            "host;bad",
            "host\nother",
            "",
        ] {
            c.host = host.into();
            assert!(c.ssh_args().is_err());
        }
    }
    #[test]
    fn winrm_quotes_values_and_never_leaves_local_prompt() {
        let mut c = connection();
        c.transport = Transport::WinRm;
        c.user = "DOMAIN\\O'Brien".into();
        c.endpoint = "name'; throw 'bad".into();
        c.https = true;
        let script = c.winrm_script().unwrap();
        assert!(script.contains("'DOMAIN\\O''Brien'"));
        assert!(script.contains("-ConfigurationName 'name''; throw ''bad'"));
        assert!(script.contains("-UseSSL"));
        let workspace = Workspace::default();
        if let Ok((_, args)) = c.command(&workspace, Path::new(".")) {
            assert!(args.iter().any(|a| a == "-NoExit"));
            assert!(script.contains("function global:prompt { [Environment]::Exit(0) }"));
        }
        c.user.clear();
        assert!(!c.winrm_script().unwrap().contains("Get-Credential"));
    }

    #[test]
    fn remote_layout_roundtrip_and_directory_isolation() {
        use crate::model::{Node, Pane, Tab};
        let base = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let mut workspace = Workspace::default();
        let c = connection();
        workspace.connections.push(c.clone());
        workspace.tabs.push(Tab {
            name: "Remote".into(),
            root: Node::Pane(Pane {
                id: 1,
                shell: Shell::PowerShell,
                directory: base.clone(),
                remote: Some(c.clone()),
            }),
        });
        assert!(!workspace.update_directory(
            1,
            &format!("RUST_TERMINAL_CWD:{}", base.join("src").display())
        ));
        assert_eq!(workspace.inherited_directory(1, Path::new("missing")), base);
        let encoded = serde_json::to_vec(&workspace).unwrap();
        let mut restored: Workspace = serde_json::from_slice(&encoded).unwrap();
        restored.validate().unwrap();
        assert_eq!(restored.panes()[0].remote.as_ref(), Some(&c));
        let profile = crate::profiles::Profile::capture("Mixed", base.clone(), &restored).unwrap();
        let (opened, _) = profile.prepare(&base.join("src"), &restored, 10).unwrap();
        assert_eq!(opened.panes()[1].remote.as_ref(), Some(&c));
        assert_eq!(opened.panes()[1].directory, base.join("src"));
        // Removing a favorite does not destroy the connection embedded in a layout.
        restored.connections.clear();
        assert_eq!(restored.panes()[0].remote.as_ref(), Some(&c));
        restored.connections.extend([c.clone(), c]);
        assert!(restored.validate().is_err());
        let old: crate::model::Pane =
            serde_json::from_str(r#"{"id":1,"shell":"Cmd","directory":"C:/"}"#).unwrap();
        assert!(old.remote.is_none());
    }

    #[test]
    #[ignore = "requires locally installed PowerShell 7"]
    fn powershell_receives_literal_values_without_executing_them() {
        use std::os::windows::process::CommandExt;
        let pwsh = Shell::PowerShell
            .resolve(None, Path::new("."))
            .expect("PowerShell 7 required for offline integration test");
        let mut c = connection();
        c.transport = Transport::WinRm;
        c.user = "DOMAIN\\O'Brien".into();
        c.endpoint = "end'; throw 'INJECTION".into();
        c.https = true;
        c.port = Some(5986);
        // Replace only the networking/credential cmdlets, retaining the actual PowerShell parser.
        let fixture = "function Get-Credential { param($UserName, $Message) return $UserName }; function Enter-PSSession { [CmdletBinding()] param($ComputerName, $Port, [switch]$UseSSL, $ConfigurationName, $Credential) @{host=$ComputerName;port=$Port;https=$UseSSL.IsPresent;endpoint=$ConfigurationName;user=$Credential} | ConvertTo-Json -Compress }; ";
        let output = std::process::Command::new(pwsh)
            .args([
                "-NoLogo",
                "-NoProfile",
                "-NonInteractive",
                "-Command",
                &format!("{fixture}{}", c.winrm_script().unwrap()),
            ])
            .creation_flags(0x08000000)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let received: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(received["endpoint"], c.endpoint);
        assert_eq!(received["user"], c.user);
        assert_eq!(received["host"], c.host);
        assert_eq!(received["port"], 5986);
        assert_eq!(received["https"], true);
    }

    #[test]
    #[ignore = "requires locally installed OpenSSH"]
    fn openssh_resolves_alias_and_jump_host_offline() {
        use std::os::windows::process::CommandExt;
        let ssh = ssh_executable("", Path::new("."))
            .expect("OpenSSH required for offline integration test");
        let config = std::env::temp_dir().join(format!("tv-ssh-{}.config", std::process::id()));
        std::fs::write(&config, "Host test-alias\n HostName server.example.invalid\n User fixture\n Port 2222\n ProxyJump jump.example.invalid\n").unwrap();
        let mut c = connection();
        c.host = "test-alias".into();
        let output = std::process::Command::new(ssh)
            .arg("-G")
            .arg("-F")
            .arg(&config)
            .args(c.ssh_args().unwrap())
            .creation_flags(0x08000000)
            .output()
            .unwrap();
        std::fs::remove_file(config).unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let text = String::from_utf8_lossy(&output.stdout);
        for expected in [
            "hostname server.example.invalid",
            "user fixture",
            "port 2222",
            "proxyjump jump.example.invalid",
        ] {
            assert!(text.contains(expected), "Missing {expected}");
        }
    }

    #[test]
    #[ignore = "requires PowerShell 7 and Windows ConPTY"]
    fn winrm_host_exits_instead_of_exposing_local_prompt() {
        use egui_term::{BackendSettings, PtyEvent, TerminalBackend};
        let c = Connection {
            transport: Transport::WinRm,
            ..connection()
        };
        let (program, mut args) = c.command(&Workspace::default(), Path::new(".")).unwrap();
        let script = args.last_mut().unwrap();
        // Simulate the return from a remote session without contacting a server.
        *script = format!("function Enter-PSSession {{ [CmdletBinding()] param($ComputerName) Write-Output 'REMOTE_MOCK_END' }}; {script}");
        let (tx, rx) = std::sync::mpsc::channel();
        let mut backend = TerminalBackend::new(
            1,
            eframe::egui::Context::default(),
            tx,
            BackendSettings {
                shell: program.display().to_string(),
                args,
                working_directory: Some(std::env::current_dir().unwrap()),
            },
        )
        .unwrap();
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(10);
        let mut exited = false;
        while std::time::Instant::now() < deadline {
            backend.sync();
            if let Ok((_, PtyEvent::Exit | PtyEvent::ChildExit(_))) =
                rx.recv_timeout(std::time::Duration::from_millis(50))
            {
                exited = true;
                break;
            }
        }
        assert!(
            exited,
            "Local host remained active: {}",
            backend
                .sync()
                .grid
                .display_iter()
                .map(|cell| cell.c)
                .collect::<String>()
        );
    }
}
