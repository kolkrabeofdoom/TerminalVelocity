use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Shell {
    #[default]
    PowerShell,
    Cmd,
    GitBash,
    Codex,
    Claude,
    Gemini,
    OpenAI,
}

impl Shell {
    pub const ALL: [Self; 7] = [
        Self::PowerShell,
        Self::Cmd,
        Self::GitBash,
        Self::Codex,
        Self::Claude,
        Self::Gemini,
        Self::OpenAI,
    ];
    pub fn is_ai(self) -> bool {
        matches!(
            self,
            Self::Codex | Self::Claude | Self::Gemini | Self::OpenAI
        )
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::PowerShell => "PowerShell",
            Self::Cmd => "CMD",
            Self::GitBash => "Git Bash",
            Self::Codex => "Codex CLI",
            Self::Claude => "Claude Code",
            Self::Gemini => "Gemini CLI",
            Self::OpenAI => "OpenAI API CLI",
        }
    }
    pub fn executable(self) -> Option<PathBuf> {
        self.resolve(None, &std::env::current_exe().ok()?.parent()?.to_path_buf())
    }
    pub fn resolve(self, custom: Option<&str>, app_dir: &std::path::Path) -> Option<PathBuf> {
        if let Some(value) = custom.filter(|value| !value.trim().is_empty()) {
            let path = PathBuf::from(value.trim().trim_matches('"'));
            let path = if path.is_absolute() {
                path
            } else {
                app_dir.join(path)
            };
            return path.is_file().then_some(path);
        }
        if let Some(path) = self.bundled(app_dir) {
            return Some(path);
        }
        if self == Self::Codex {
            if let Some(root) = std::env::var_os("APPDATA") {
                let path = PathBuf::from(root).join("npm/node_modules/@openai/codex/node_modules/@openai/codex-win32-x64/vendor/x86_64-pc-windows-msvc/bin/codex.exe");
                if path.is_file() {
                    return Some(path);
                }
            }
        }
        let names: &[&str] = match self {
            Self::PowerShell => &["pwsh.exe"],
            Self::Cmd => &["cmd.exe"],
            Self::GitBash => &["bash.exe"],
            Self::Codex => &["codex.exe", "codex.cmd"],
            Self::Claude => &["claude.exe", "claude.cmd"],
            Self::Gemini => &["gemini.exe", "gemini.cmd"],
            Self::OpenAI => &["openai.exe"],
        };
        // User installations also resolve when the desktop inherited an older PATH.
        if self.is_ai() {
            for directory in [
                std::env::var_os("APPDATA").map(|root| PathBuf::from(root).join("npm")),
                std::env::var_os("USERPROFILE").map(|root| PathBuf::from(root).join(".local/bin")),
                std::env::var_os("LOCALAPPDATA")
                    .map(|root| PathBuf::from(root).join("Programs/OpenAI/CLI")),
            ]
            .into_iter()
            .flatten()
            {
                for name in names {
                    let path = directory.join(name);
                    if path.is_file() {
                        return Some(path);
                    }
                }
            }
        }
        // Prefer the real Git for Windows shell over an unrelated bash on PATH.
        if self == Self::GitBash {
            for root in [
                std::env::var_os("ProgramFiles"),
                std::env::var_os("LOCALAPPDATA"),
            ]
            .into_iter()
            .flatten()
            {
                for relative in ["Git/bin/bash.exe", "Programs/Git/bin/bash.exe"] {
                    let path = PathBuf::from(&root).join(relative);
                    if path.is_file() {
                        return Some(path);
                    }
                }
            }
        }
        if let Some(paths) = std::env::var_os("PATH") {
            for directory in std::env::split_paths(&paths) {
                for name in names {
                    let path = directory.join(name);
                    if path.is_file()
                        && (self != Self::GitBash
                            || directory.join("../cmd/git.exe").is_file()
                            || directory.join("../../cmd/git.exe").is_file())
                    {
                        return Some(path);
                    }
                }
            }
        }
        let fallback = match self {
            Self::PowerShell => {
                PathBuf::from(std::env::var_os("ProgramFiles")?).join("PowerShell/7/pwsh.exe")
            }
            Self::Cmd => PathBuf::from(std::env::var_os("SystemRoot")?).join("System32/cmd.exe"),
            _ => return None,
        };
        fallback.is_file().then_some(fallback)
    }
    fn bundled(self, app_dir: &std::path::Path) -> Option<PathBuf> {
        let relative = match self {
            Self::PowerShell => "shells/powershell/pwsh.exe",
            Self::GitBash => "shells/git/bin/bash.exe",
            Self::Codex => "shells/codex/bin/codex.exe",
            _ => return None,
        };
        let path = app_dir.join(relative);
        path.is_file().then_some(path)
    }
    pub fn args(self) -> Vec<String> {
        match self {
            Self::PowerShell => vec!["-NoLogo".into()],
            Self::Cmd => vec!["/D".into()],
            Self::GitBash => vec!["--login".into(), "-i".into()],
            _ => vec![],
        }
    }
    pub fn launch(
        &self,
        executable: PathBuf,
        app_dir: &std::path::Path,
        version_only: bool,
    ) -> Result<(PathBuf, Vec<String>), String> {
        if *self == Self::PowerShell {
            let mut script = "$env:Path = [Environment]::GetEnvironmentVariable('Path','User') + ';' + $env:Path;".to_string();
            if let Some(codex) = Self::Codex.resolve(None, app_dir) {
                script.push_str(&format!(
                    " Set-Alias -Name codex -Value '{}' -Scope Global;",
                    codex.to_string_lossy().replace('\'', "''")
                ));
            }
            script.push_str(" $global:TerminalVelocitySavedPrompt = $function:prompt; function global:prompt { if ($PWD.Provider.Name -eq 'FileSystem') { [Console]::Write(([char]27).ToString() + ']0;TERMINALVELOCITY_CWD:' + $PWD.ProviderPath + [char]7) }; if ($global:TerminalVelocitySavedPrompt) { & $global:TerminalVelocitySavedPrompt } else { 'PS ' + $PWD.Path + '> ' } };");
            return Ok((
                executable,
                vec![
                    "-NoLogo".into(),
                    "-NoExit".into(),
                    "-Command".into(),
                    script,
                ],
            ));
        }
        if *self == Self::Cmd {
            return Ok((
                executable,
                vec![
                    "/D".into(),
                    "/K".into(),
                    "prompt $E]0;TERMINALVELOCITY_CWD:$P$E\\$P$G".into(),
                ],
            ));
        }
        if *self == Self::GitBash {
            return Ok((
                executable,
                vec![
                    "--rcfile".into(),
                    app_dir
                        .join("shells/terminalvelocity.bash")
                        .to_string_lossy()
                        .into_owned(),
                    "-i".into(),
                ],
            ));
        }
        if *self == Self::Codex
            && executable
                .extension()
                .is_some_and(|ext| ext.eq_ignore_ascii_case("exe"))
        {
            return Ok((
                executable,
                if version_only {
                    vec!["--version".into()]
                } else {
                    vec![]
                },
            ));
        }
        if !self.is_ai() {
            return Ok((executable, self.args()));
        }
        let host = Self::PowerShell
            .resolve(None, app_dir)
            .ok_or_else(|| "PowerShell 7 als Host für KI-CLIs nicht gefunden".to_string())?;
        let path = executable.to_string_lossy().replace('\'', "''");
        let option = if version_only {
            " --version"
        } else if *self == Self::OpenAI {
            " --help"
        } else {
            ""
        };
        let command = format!("$env:Path = [Environment]::GetEnvironmentVariable('Path','Machine') + ';' + [Environment]::GetEnvironmentVariable('Path','User') + ';' + $env:Path; & '{path}'{option}");
        let mut args = vec!["-NoLogo".into(), "-NoProfile".into()];
        if !version_only {
            args.push("-NoExit".into());
        }
        args.extend(["-Command".into(), command]);
        Ok((host, args))
    }
}

#[derive(Clone, Copy, Debug, Serialize, Deserialize)]
pub enum Axis {
    Columns,
    Rows,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Pane {
    #[serde(default)]
    pub remote: Option<crate::remote::Connection>,
    pub id: u64,
    pub shell: Shell,
    pub directory: PathBuf,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub enum Node {
    Pane(Pane),
    Split {
        axis: Axis,
        ratio: f32,
        first: Box<Node>,
        second: Box<Node>,
    },
}

impl Node {
    pub fn pane_mut(&mut self, id: u64) -> Option<&mut Pane> {
        match self {
            Self::Pane(pane) if pane.id == id => Some(pane),
            Self::Split { first, second, .. } => first.pane_mut(id).or_else(|| second.pane_mut(id)),
            _ => None,
        }
    }
    pub fn panes(&self, output: &mut Vec<Pane>) {
        match self {
            Self::Pane(pane) => output.push(pane.clone()),
            Self::Split { first, second, .. } => {
                first.panes(output);
                second.panes(output);
            }
        }
    }
    pub fn split(&mut self, target: u64, axis: Axis, pane: Pane) -> bool {
        self.split_at_depth(target, axis, pane, 0)
    }
    fn split_at_depth(&mut self, target: u64, axis: Axis, pane: Pane, depth: usize) -> bool {
        match self {
            Self::Pane(current) if current.id == target && depth < 10 => {
                *self = Self::Split {
                    axis,
                    ratio: 0.5,
                    first: Box::new(self.clone()),
                    second: Box::new(Self::Pane(pane)),
                };
                true
            }
            Self::Split { first, second, .. } => {
                first.split_at_depth(target, axis, pane.clone(), depth + 1)
                    || second.split_at_depth(target, axis, pane, depth + 1)
            }
            _ => false,
        }
    }
    pub fn remove(self, target: u64) -> Option<Self> {
        match self {
            Self::Pane(ref pane) if pane.id == target => None,
            Self::Pane(_) => Some(self),
            Self::Split {
                axis,
                ratio,
                first,
                second,
            } => match (first.remove(target), second.remove(target)) {
                (Some(first), Some(second)) => Some(Self::Split {
                    axis,
                    ratio,
                    first: Box::new(first),
                    second: Box::new(second),
                }),
                (first, second) => first.or(second),
            },
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Tab {
    pub name: String,
    pub root: Node,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Workspace {
    pub sixel_enabled: bool,
    pub background: crate::background::Settings,
    pub effects: crate::effects::Settings,
    pub connections: Vec<crate::remote::Connection>,
    pub ssh_path: String,
    pub version: u32,
    pub opacity: f32,
    pub acrylic: bool,
    pub compatibility_transparency: bool,
    pub font_size: f32,
    pub default_shell: Shell,
    pub default_directory: Option<PathBuf>,
    pub shell_paths: Vec<String>,
    pub active_tab: usize,
    pub tabs: Vec<Tab>,
}

impl Default for Workspace {
    fn default() -> Self {
        Self {
            version: 1,
            background: crate::background::Settings::default(),
            sixel_enabled: true,
            effects: crate::effects::Settings::default(),
            connections: vec![],
            ssh_path: String::new(),
            opacity: 0.82,
            acrylic: false,
            compatibility_transparency: true,
            font_size: 16.0,
            default_shell: Shell::PowerShell,
            default_directory: None,
            shell_paths: vec![String::new(); Shell::ALL.len()],
            active_tab: 0,
            tabs: vec![],
        }
    }
}

impl Workspace {
    pub fn update_directory(&mut self, id: u64, title: &str) -> bool {
        let Some(pane) = self.tabs.iter_mut().find_map(|tab| tab.root.pane_mut(id)) else {
            return false;
        };
        // Remote output must never cause local filesystem probes (including UNC paths).
        if pane.remote.is_some() {
            return false;
        }
        let Some(path) = title.strip_prefix("TERMINALVELOCITY_CWD:") else {
            return false;
        };
        let path = PathBuf::from(path);
        if !path.is_absolute() || !path.is_dir() {
            return false;
        }
        pane.directory = path;
        true
    }
    pub fn inherited_directory(&self, focused: u64, fallback: &std::path::Path) -> PathBuf {
        self.panes()
            .into_iter()
            .find(|pane| pane.id == focused)
            .map(|pane| pane.directory)
            .unwrap_or_else(|| fallback.to_path_buf())
    }
    pub fn validate(&mut self) -> Result<(), String> {
        self.effects.validate();
        self.background.validate();
        if self.connections.len() > 128 {
            return Err("Maximal 128 gespeicherte Verbindungen.".into());
        }
        let mut names = std::collections::HashSet::new();
        for connection in &self.connections {
            connection.validate()?;
            if !names.insert(connection.name.trim().to_lowercase()) {
                return Err("Verbindungsname bereits vorhanden.".into());
            }
        }
        self.shell_paths.resize(Shell::ALL.len(), String::new());
        if self.version != 1 {
            return Err("Unbekannte Workspace-Version".into());
        }
        if self.tabs.len() > 32 {
            return Err("Maximal 32 Tabs pro Workspace".into());
        }
        let mut ids = std::collections::HashSet::new();
        fn walk(
            node: &mut Node,
            depth: usize,
            ids: &mut std::collections::HashSet<u64>,
        ) -> Result<(), String> {
            if depth > 10 {
                return Err("Zu tief verschachteltes Layout".into());
            }
            match node {
                Node::Pane(p) => {
                    if let Some(remote) = &p.remote {
                        remote.validate()?;
                    }
                    if !ids.insert(p.id) || p.id > 1_000_000 || ids.len() > 64 {
                        return Err("Ungültige oder zu viele Bereiche".into());
                    }
                }
                Node::Split {
                    ratio,
                    first,
                    second,
                    ..
                } => {
                    *ratio = ratio.clamp(0.15, 0.85);
                    walk(first, depth + 1, ids)?;
                    walk(second, depth + 1, ids)?;
                }
            }
            Ok(())
        }
        for tab in &mut self.tabs {
            walk(&mut tab.root, 0, &mut ids)?;
        }
        self.active_tab = self.active_tab.min(self.tabs.len().saturating_sub(1));
        self.opacity = self.opacity.clamp(0.0, 1.0);
        self.font_size = self.font_size.clamp(10.0, 28.0);
        Ok(())
    }
    pub fn panes(&self) -> Vec<Pane> {
        let mut result = vec![];
        for tab in &self.tabs {
            tab.root.panes(&mut result);
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn portable_shell_resolves_without_path_and_custom_path_takes_priority() {
        let base = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../work")
            .join(format!("resolve-test-{}", std::process::id()));
        let shell_dir = base.join("shells/powershell");
        std::fs::create_dir_all(&shell_dir).unwrap();
        let exe = shell_dir.join("pwsh.exe");
        std::fs::write(&exe, b"resolution fixture; never executed").unwrap();
        assert_eq!(Shell::PowerShell.bundled(&base), Some(exe.clone()));
        assert_eq!(Shell::PowerShell.resolve(None, &base), Some(exe.clone()));
        assert_eq!(
            Shell::PowerShell.resolve(Some("shells/powershell/pwsh.exe"), &base),
            Some(exe.clone())
        );
        assert_eq!(Shell::PowerShell.resolve(Some("missing.exe"), &base), None);
        std::fs::remove_file(exe).unwrap();
        std::fs::remove_dir(shell_dir).unwrap();
        std::fs::remove_dir(base.join("shells")).unwrap();
        std::fs::remove_dir(base).unwrap();
    }
    #[test]
    fn old_workspace_loads_with_automatic_shell_paths() {
        let mut workspace: Workspace = serde_json::from_str(
            r#"{"version":1,"tabs":[],"shell_paths":["custom-pwsh.exe","",""]}"#,
        )
        .unwrap();
        workspace.validate().unwrap();
        assert_eq!(
            workspace.shell_paths,
            ["custom-pwsh.exe", "", "", "", "", "", ""]
        );
    }
    #[test]
    fn ai_workspace_roundtrip_keeps_provider_and_custom_path() {
        let mut workspace = Workspace::default();
        workspace.default_shell = Shell::Gemini;
        workspace.shell_paths[Shell::Gemini as usize] = "tools/gemini.cmd".into();
        let encoded = serde_json::to_string(&workspace).unwrap();
        let mut restored: Workspace = serde_json::from_str(&encoded).unwrap();
        restored.validate().unwrap();
        assert_eq!(restored.default_shell, Shell::Gemini);
        assert_eq!(
            restored.shell_paths[Shell::Gemini as usize],
            "tools/gemini.cmd"
        );
    }
    #[test]
    fn native_codex_launch_does_not_require_a_shell_or_node() {
        let executable = PathBuf::from("C:/tools/codex.exe");
        let (program, args) = Shell::Codex
            .launch(
                executable.clone(),
                std::path::Path::new("missing-host"),
                false,
            )
            .unwrap();
        assert_eq!(program, executable);
        assert!(args.is_empty());
        let (_, args) = Shell::Codex
            .launch(executable, std::path::Path::new("missing-host"), true)
            .unwrap();
        assert_eq!(args, ["--version"]);
    }
    #[test]
    fn directory_updates_stay_with_the_source_pane_and_follow_focus() {
        let base = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let alternate = base.join("src");
        let mut workspace = Workspace::default();
        workspace.tabs.push(Tab {
            name: "source".into(),
            root: Node::Split {
                axis: Axis::Columns,
                ratio: 0.5,
                first: Box::new(Node::Pane(Pane {
                    remote: None,
                    id: 11,
                    shell: Shell::PowerShell,
                    directory: base.clone(),
                })),
                second: Box::new(Node::Pane(Pane {
                    remote: None,
                    id: 12,
                    shell: Shell::Cmd,
                    directory: base.clone(),
                })),
            },
        });
        assert!(workspace
            .update_directory(12, &format!("TERMINALVELOCITY_CWD:{}", alternate.display())));
        assert_eq!(
            workspace.inherited_directory(12, std::path::Path::new("fallback")),
            alternate
        );
        assert_eq!(
            workspace.inherited_directory(11, std::path::Path::new("fallback")),
            base
        );
        assert!(!workspace.update_directory(12, "normal window title"));
        assert!(!workspace.update_directory(12, "TERMINALVELOCITY_CWD:relative"));
        assert_eq!(
            workspace.inherited_directory(99, std::path::Path::new("fallback")),
            PathBuf::from("fallback")
        );
    }
    fn pane(id: u64) -> Pane {
        Pane {
            remote: None,
            id,
            shell: Shell::PowerShell,
            directory: PathBuf::from("."),
        }
    }
    #[test]
    fn nested_split_close_preserves_sibling_and_roundtrip() {
        let mut tree = Node::Pane(pane(1));
        assert!(tree.split(1, Axis::Columns, pane(2)));
        assert!(tree.split(2, Axis::Rows, pane(3)));
        let encoded = serde_json::to_string(&tree).unwrap();
        let restored: Node = serde_json::from_str(&encoded).unwrap();
        let remaining = restored.remove(2).unwrap();
        let mut panes = vec![];
        remaining.panes(&mut panes);
        assert_eq!(panes.iter().map(|p| p.id).collect::<Vec<_>>(), vec![1, 3]);
    }
    #[test]
    fn rejects_duplicate_ids_before_starting_shells() {
        let mut state = Workspace::default();
        state.tabs.push(Tab {
            name: "test".into(),
            root: Node::Split {
                axis: Axis::Rows,
                ratio: 0.5,
                first: Box::new(Node::Pane(pane(1))),
                second: Box::new(Node::Pane(pane(1))),
            },
        });
        assert!(state.validate().is_err());
    }
}
