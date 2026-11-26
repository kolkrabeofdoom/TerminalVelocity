use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Reject UNC (`\\server\share`) and Windows device (`\\.\...`) paths to prevent
/// unintended SMB/NTLM network probes or blocking I/O on the GUI thread.
pub fn is_unc_or_device_path(path: &Path) -> bool {
    use std::path::{Component, Prefix};
    let raw = path.as_os_str().to_string_lossy();
    raw.starts_with(r"\\") && !raw.starts_with(r"\\?\")
        || raw.starts_with("//")
        || matches!(
            path.components().next(),
            Some(Component::Prefix(prefix))
                if !matches!(prefix.kind(), Prefix::Disk(_) | Prefix::VerbatimDisk(_))
        )
}

pub fn is_local_absolute_path(path: &Path) -> bool {
    path.is_absolute() && !is_unc_or_device_path(path)
}

/// Write JSON to a unique staging file with `create_new(true)` and atomically replace `path`.
pub fn atomic_write_json(path: &Path, bytes: &[u8]) -> Result<(), String> {
    use std::io::Write;
    if let Ok(meta) = std::fs::symlink_metadata(path) {
        if meta.file_type().is_symlink() {
            return Err("Symbolische Links werden nicht überschrieben.".into());
        }
    }
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."));
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("config.json");
    let temporary = parent.join(format!(
        ".{name}.{}.{}.tmp",
        std::process::id(),
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    let mut staged = std::fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .map_err(|e| e.to_string())?;
    let result = (|| -> Result<(), String> {
        staged
            .write_all(bytes)
            .and_then(|_| staged.sync_all())
            .map_err(|e| e.to_string())?;
        drop(staged);
        #[cfg(windows)]
        {
            use std::os::windows::ffi::OsStrExt;
            use windows_sys::Win32::Storage::FileSystem::MoveFileExW;
            let from: Vec<u16> = temporary.as_os_str().encode_wide().chain(Some(0)).collect();
            let to: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
            if unsafe { MoveFileExW(from.as_ptr(), to.as_ptr(), 1 | 8) } == 0 {
                return Err(std::io::Error::last_os_error().to_string());
            }
        }
        #[cfg(not(windows))]
        std::fs::rename(&temporary, path).map_err(|e| e.to_string())?;
        Ok(())
    })();
    if result.is_err() {
        let _ = std::fs::remove_file(&temporary);
    }
    result
}

/// Copy an unreadable configuration file next to itself before anything may overwrite it.
/// Uses `create_new(true)`, so an earlier backup is never replaced.
pub fn backup_unloadable(path: &Path) -> Result<PathBuf, String> {
    let name = path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("workspace.json");
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let mut source = std::fs::File::open(path).map_err(|e| e.to_string())?;
    for attempt in 0..100u32 {
        let suffix = if attempt == 0 {
            String::new()
        } else {
            format!("-{attempt}")
        };
        let backup = path.with_file_name(format!("{name}.broken-{stamp}{suffix}"));
        let mut target = match std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&backup)
        {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
            Err(error) => return Err(error.to_string()),
        };
        let copied = std::io::copy(&mut source, &mut target).and_then(|_| target.sync_all());
        if let Err(error) = copied {
            drop(target);
            let _ = std::fs::remove_file(&backup);
            return Err(error.to_string());
        }
        return Ok(backup);
    }
    Err("Kein freier Name für die Sicherungskopie gefunden.".into())
}

/// Set when `workspace.json` existed but could not be loaded.
pub struct WorkspaceGuard {
    pub reason: String,
    pub backup: Option<PathBuf>,
}

impl WorkspaceGuard {
    /// Automatic saves never replace an unloaded file. An explicit save may, but only
    /// once a backup of the original exists.
    pub fn check(&self, explicit: bool) -> Result<(), String> {
        match (&self.backup, explicit) {
            (Some(_), true) => Ok(()),
            (Some(backup), false) => Err(format!(
                "Nicht gespeichert: workspace.json konnte nicht geladen werden ({}). Eine Kopie liegt unter {}. Mit „Workspace speichern“ wird die Datei bewusst ersetzt.",
                self.reason,
                backup.display()
            )),
            (None, _) => Err(format!(
                "Nicht gespeichert: workspace.json konnte nicht geladen und nicht gesichert werden ({}). Bitte die Datei prüfen oder beiseitelegen und TerminalVelocity neu starten.",
                self.reason
            )),
        }
    }
}

/// Prefix of the directory report that the shell integration sends as window title.
pub const CWD_MARKER: &str = "TERMINALVELOCITY_CWD:";
/// Environment variable that hands the session key to the Git Bash rc file.
pub const CWD_TOKEN_ENV: &str = "TERMINALVELOCITY_CWD_TOKEN";

/// Random per-session key for directory reports (128 bit, hex).
///
/// Output from programs, files or remote hosts cannot know it, so only the shell
/// integration started by TerminalVelocity can move a pane's working directory.
/// `RandomState` is seeded from the operating system's random source; SipHash with
/// that secret key is unpredictable without needing an extra dependency.
pub fn new_cwd_token() -> String {
    use std::hash::{BuildHasher, Hasher};
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let part = || {
        let mut hasher = std::collections::hash_map::RandomState::new().build_hasher();
        hasher.write_u64(NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed));
        hasher.write_u128(
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|d| d.as_nanos())
                .unwrap_or(0),
        );
        hasher.finish()
    };
    format!("{:016x}{:016x}", part(), part())
}

/// Extract the reported directory if the title carries this session's key.
fn reported_directory<'a>(title: &'a str, token: &str) -> Option<&'a str> {
    if token.len() < 32 {
        return None;
    }
    title
        .strip_prefix(CWD_MARKER)?
        .strip_prefix(token)?
        .strip_prefix(':')
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[allow(clippy::enum_variant_names)]
pub enum Shell {
    #[default]
    PowerShell,
    Cmd,
    GitBash,
    Codex,
    Claude,
    #[serde(alias = "Gemini")]
    Antigravity,
    OpenAI,
}

impl Shell {
    pub const ALL: [Self; 7] = [
        Self::PowerShell,
        Self::Cmd,
        Self::GitBash,
        Self::Codex,
        Self::Claude,
        Self::Antigravity,
        Self::OpenAI,
    ];
    pub fn is_ai(self) -> bool {
        matches!(
            self,
            Self::Codex | Self::Claude | Self::Antigravity | Self::OpenAI
        )
    }
    pub fn label(self) -> &'static str {
        match self {
            Self::PowerShell => "PowerShell",
            Self::Cmd => "CMD",
            Self::GitBash => "Git Bash",
            Self::Codex => "Codex CLI",
            Self::Claude => "Claude Code",
            Self::Antigravity => "Antigravity CLI",
            Self::OpenAI => "OpenAI API CLI",
        }
    }
    pub fn executable(self) -> Option<PathBuf> {
        self.resolve(None, std::env::current_exe().ok()?.parent()?)
    }
    pub fn resolve(self, custom: Option<&str>, app_dir: &std::path::Path) -> Option<PathBuf> {
        if let Some(value) = custom.filter(|value| !value.trim().is_empty()) {
            let raw = value.trim().trim_matches('"');
            if raw.is_empty()
                || raw.len() > 4096
                || raw.contains('"')
                || raw.chars().any(char::is_control)
            {
                return None;
            }
            let path = PathBuf::from(raw);
            if is_unc_or_device_path(&path) {
                return None;
            }
            let path = if path.is_absolute() {
                path
            } else {
                app_dir.join(path)
            };
            return (!is_unc_or_device_path(&path)
                && path.is_file()
                && path
                    .canonicalize()
                    .map(|c| !is_unc_or_device_path(&c))
                    .unwrap_or(false))
            .then_some(path);
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
            Self::Antigravity => &["agy.exe", "agy.cmd"],
            Self::OpenAI => &["openai.exe"],
        };
        // User installations also resolve when the desktop inherited an older PATH.
        if self.is_ai() {
            for directory in [
                std::env::var_os("LOCALAPPDATA").map(|root| PathBuf::from(root).join("agy/bin")),
                std::env::var_os("USERPROFILE").map(|root| PathBuf::from(root).join(".gemini/bin")),
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
            Self::Antigravity => "shells/agy/bin/agy.exe",
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
    /// Extra environment for a local shell session. Only Git Bash needs the key this
    /// way; its rc file moves it into a shell variable and removes it from the environment.
    pub fn session_env(self, cwd_token: &str) -> std::collections::HashMap<String, String> {
        let mut env = std::collections::HashMap::new();
        if self == Self::GitBash {
            env.insert(CWD_TOKEN_ENV.to_string(), cwd_token.to_string());
        }
        env
    }
    pub fn launch(
        &self,
        executable: PathBuf,
        app_dir: &std::path::Path,
        version_only: bool,
        cwd_token: &str,
    ) -> Result<(PathBuf, Vec<String>), String> {
        // The key is hex only, so it is safe inside PowerShell and CMD strings.
        if !cwd_token.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err("Ungültiger Sitzungsschlüssel".into());
        }
        if *self == Self::PowerShell {
            let mut script = "$env:Path = [Environment]::GetEnvironmentVariable('Path','Machine') + ';' + [Environment]::GetEnvironmentVariable('Path','User') + ';' + $env:Path;".to_string();
            if let Some(codex) = Self::Codex.resolve(None, app_dir) {
                script.push_str(&format!(
                    " Set-Alias -Name codex -Value '{}' -Scope Global;",
                    codex.to_string_lossy().replace('\'', "''")
                ));
            }
            if let Some(agy) = Self::Antigravity.resolve(None, app_dir) {
                script.push_str(&format!(
                    " Set-Alias -Name agy -Value '{}' -Scope Global;",
                    agy.to_string_lossy().replace('\'', "''")
                ));
            }
            script.push_str(&format!(" $global:TerminalVelocitySavedPrompt = $function:prompt; function global:prompt {{ if ($PWD.Provider.Name -eq 'FileSystem') {{ [Console]::Write(([char]27).ToString() + ']0;{CWD_MARKER}{cwd_token}:' + $PWD.ProviderPath + [char]7) }}; if ($global:TerminalVelocitySavedPrompt) {{ & $global:TerminalVelocitySavedPrompt }} else {{ 'PS ' + $PWD.Path + '> ' }} }};"));
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
                    format!("prompt $E]0;{CWD_MARKER}{cwd_token}:$P$E\\$P$G"),
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
        if matches!(self, Self::Codex | Self::Antigravity)
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
    pub fn update_directory(&mut self, id: u64, title: &str, cwd_token: &str) -> bool {
        let Some(pane) = self.tabs.iter_mut().find_map(|tab| tab.root.pane_mut(id)) else {
            return false;
        };
        // Remote output must never cause local filesystem probes (including UNC paths).
        if pane.remote.is_some() {
            return false;
        }
        // Without the session key the report may come from any program output.
        let Some(path) = reported_directory(title, cwd_token) else {
            return false;
        };
        let path = PathBuf::from(path);
        if !is_local_absolute_path(&path) || !path.is_dir() {
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
        for value in std::iter::once(&self.ssh_path).chain(self.shell_paths.iter()) {
            let trimmed = value.trim().trim_matches('"');
            if value.len() > 4096
                || trimmed.contains('"')
                || value.chars().any(char::is_control)
                || (!trimmed.is_empty() && is_unc_or_device_path(Path::new(trimmed)))
            {
                return Err("Ungültiger Programmpfad".into());
            }
        }
        if let Some(dir) = &self.default_directory {
            if !is_local_absolute_path(dir) {
                return Err("Ungültiger Standardordner".into());
            }
        }
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
                    if !is_local_absolute_path(&p.directory) {
                        return Err("Alle Bereichsordner müssen lokale absolute Pfade sein.".into());
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
                    *ratio = if ratio.is_finite() {
                        ratio.clamp(0.15, 0.85)
                    } else {
                        0.5
                    };
                    walk(first, depth + 1, ids)?;
                    walk(second, depth + 1, ids)?;
                }
            }
            Ok(())
        }
        for tab in &mut self.tabs {
            if tab.name.trim().is_empty()
                || tab.name.chars().count() > 120
                || tab.name.chars().any(char::is_control)
            {
                return Err("Ungültiger Tab-Name".into());
            }
            walk(&mut tab.root, 0, &mut ids)?;
        }
        self.active_tab = self.active_tab.min(self.tabs.len().saturating_sub(1));
        self.opacity = if self.opacity.is_finite() {
            self.opacity.clamp(0.0, 1.0)
        } else {
            0.82
        };
        self.font_size = if self.font_size.is_finite() {
            self.font_size.clamp(10.0, 28.0)
        } else {
            16.0
        };
        Ok(())
    }
    pub fn load(path: &Path) -> Result<Self, String> {
        use std::io::Read;
        if is_unc_or_device_path(path) {
            return Err("Ungültiger Workspace-Pfad.".into());
        }
        let file = std::fs::File::open(path).map_err(|e| e.to_string())?;
        if !file.metadata().map_err(|e| e.to_string())?.is_file() {
            return Err("Workspace-Pfad ist keine reguläre Datei.".into());
        }
        let mut bytes = Vec::new();
        file.take(2_000_001)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        if bytes.len() > 2_000_000 {
            return Err("Workspace-Datei ist zu groß.".into());
        }
        let mut state: Self = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
        state.validate()?;
        Ok(state)
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
        let base = std::env::temp_dir().join(format!("resolve-test-{}", std::process::id()));
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
        let mut workspace = Workspace {
            default_shell: Shell::Antigravity,
            ..Workspace::default()
        };
        workspace.shell_paths[Shell::Antigravity as usize] = "tools/agy.exe".into();
        let encoded = serde_json::to_string(&workspace).unwrap();
        let mut restored: Workspace = serde_json::from_str(&encoded).unwrap();
        restored.validate().unwrap();
        assert_eq!(restored.default_shell, Shell::Antigravity);
        assert_eq!(
            restored.shell_paths[Shell::Antigravity as usize],
            "tools/agy.exe"
        );
        // Legacy "Gemini" entries transparently migrate to Antigravity.
        let legacy: Workspace =
            serde_json::from_str(r#"{"version":1,"default_shell":"Gemini","tabs":[]}"#).unwrap();
        assert_eq!(legacy.default_shell, Shell::Antigravity);
    }
    #[test]
    fn native_codex_launch_does_not_require_a_shell_or_node() {
        let executable = PathBuf::from("C:/tools/codex.exe");
        let (program, args) = Shell::Codex
            .launch(
                executable.clone(),
                std::path::Path::new("missing-host"),
                false,
                "",
            )
            .unwrap();
        assert_eq!(program, executable);
        assert!(args.is_empty());
        let (_, args) = Shell::Codex
            .launch(executable, std::path::Path::new("missing-host"), true, "")
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
        let token = new_cwd_token();
        let report = |path: &str| format!("{CWD_MARKER}{token}:{path}");
        assert!(workspace.update_directory(12, &report(&alternate.display().to_string()), &token));
        assert_eq!(
            workspace.inherited_directory(12, std::path::Path::new("fallback")),
            alternate
        );
        assert_eq!(
            workspace.inherited_directory(11, std::path::Path::new("fallback")),
            base
        );
        assert!(!workspace.update_directory(12, "normal window title", &token));
        assert!(!workspace.update_directory(12, &report("relative"), &token));
        assert!(!workspace.update_directory(12, &report(r"\\198.51.100.1\share"), &token));
        assert!(!workspace.update_directory(12, &report("//198.51.100.1/share"), &token));
        assert!(!workspace.update_directory(12, &report(r"\\?\UNC\198.51.100.1\share"), &token));
        assert_eq!(
            workspace.inherited_directory(99, std::path::Path::new("fallback")),
            PathBuf::from("fallback")
        );
    }
    #[test]
    fn security_hardening_rejects_unc_paths_unsafe_urls_and_preserves_machine_path_order() {
        let base = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        assert!(is_local_absolute_path(&base));
        for unc in [
            r"\\198.51.100.1\share",
            "//198.51.100.1/share",
            r"\\?\UNC\198.51.100.1\share",
            r"\\.\PhysicalDrive0",
        ] {
            let p = Path::new(unc);
            assert!(is_unc_or_device_path(p), "{unc}");
            assert!(!is_local_absolute_path(p), "{unc}");
            assert_eq!(Shell::PowerShell.resolve(Some(unc), &base), None);
        }
        let mut ws = Workspace::default();
        ws.tabs.push(Tab {
            name: "unc".into(),
            root: Node::Pane(Pane {
                remote: None,
                id: 1,
                shell: Shell::PowerShell,
                directory: PathBuf::from(r"\\198.51.100.1\share"),
            }),
        });
        assert!(ws.validate().is_err());
        let mut ws = Workspace {
            ssh_path: r"\\198.51.100.1\share\ssh.exe".into(),
            ..Workspace::default()
        };
        assert!(ws.validate().is_err());

        let mut nan_ws = Workspace {
            opacity: f32::NAN,
            font_size: f32::INFINITY,
            tabs: vec![Tab {
                name: "split".into(),
                root: Node::Split {
                    axis: Axis::Columns,
                    ratio: f32::NAN,
                    first: Box::new(Node::Pane(pane(1))),
                    second: Box::new(Node::Pane(pane(2))),
                },
            }],
            ..Workspace::default()
        };
        nan_ws.validate().unwrap();
        assert_eq!(nan_ws.opacity, 0.82);
        assert_eq!(nan_ws.font_size, 16.0);
        assert!(matches!(
            nan_ws.tabs[0].root,
            Node::Split { ratio, .. } if (ratio - 0.5).abs() < f32::EPSILON
        ));

        let (_, args) = Shell::PowerShell
            .launch(PathBuf::from("pwsh.exe"), &base, false, "")
            .unwrap();
        let script = &args[3];
        let machine_pos = script.find("('Path','Machine')").unwrap();
        let user_pos = script.find("('Path','User')").unwrap();
        assert!(machine_pos < user_pos);

        assert!(egui_term::is_safe_http_url("https://example.org/path?q=1"));
        assert!(egui_term::is_safe_http_url("http://localhost:8080"));
        for unsafe_url in [
            "file:///C:/Windows/System32/calc.exe",
            "file:////198.51.100.1/share/payload.exe",
            "ssh://user@example.org",
            "https://",
            "http:///path",
            "https://\\198.51.100.1/share",
            "https://example.org/a\nb",
        ] {
            assert!(!egui_term::is_safe_http_url(unsafe_url), "{unsafe_url}");
        }

        // TV-07: Bracketed paste neutralizes ESC/CSI/NUL and wraps in 200~/201~.
        assert_eq!(
            egui_term::encode_paste("echo ok\x1b[201~\nrm -rf\0", true),
            b"\x1b[200~echo ok[201~\nrm -rf\x1b[201~"
        );
        assert_eq!(egui_term::encode_paste("plain", false), b"plain");

        // TV-08 & TV-11: Reject internal quotes in executable paths and invalid tab names.
        assert_eq!(
            Shell::PowerShell.resolve(Some(r#""C:\path"\bad.exe""#), &base),
            None
        );
        let mut bad_quote = Workspace {
            ssh_path: r#"C:\bad"quote\ssh.exe"#.into(),
            ..Workspace::default()
        };
        assert!(bad_quote.validate().is_err());
        let mut bad_tab_name = Workspace {
            tabs: vec![Tab {
                name: "bad\x1bname".into(),
                root: Node::Pane(pane(1)),
            }],
            ..Workspace::default()
        };
        assert!(bad_tab_name.validate().is_err());
    }
    fn pane(id: u64) -> Pane {
        Pane {
            remote: None,
            id,
            shell: Shell::PowerShell,
            directory: PathBuf::from(env!("CARGO_MANIFEST_DIR")),
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

    #[test]
    fn unloadable_workspace_is_backed_up_and_never_silently_replaced() {
        let root = std::env::temp_dir().join(format!("tv-guard-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("workspace.json");
        let original = b"{ \"version\": 99, \"connections\": [] ".to_vec();
        std::fs::write(&path, &original).unwrap();
        assert!(Workspace::load(&path).is_err());

        let first = backup_unloadable(&path).unwrap();
        let second = backup_unloadable(&path).unwrap();
        assert_ne!(first, second, "an existing backup must not be replaced");
        assert_eq!(std::fs::read(&first).unwrap(), original);
        assert_eq!(std::fs::read(&second).unwrap(), original);
        assert_eq!(std::fs::read(&path).unwrap(), original);

        let guarded = WorkspaceGuard {
            reason: "kaputt".into(),
            backup: Some(first),
        };
        assert!(guarded.check(false).is_err());
        assert!(guarded.check(true).is_ok());
        let unprotected = WorkspaceGuard {
            reason: "kaputt".into(),
            backup: None,
        };
        assert!(unprotected.check(false).is_err());
        assert!(unprotected.check(true).is_err());

        assert!(backup_unloadable(&root.join("missing.json")).is_err());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn directory_reports_without_the_session_key_are_ignored() {
        let base = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let target = base.join("src");
        let mut workspace = Workspace::default();
        workspace.tabs.push(Tab {
            name: "local".into(),
            root: Node::Pane(Pane {
                remote: None,
                id: 1,
                shell: Shell::Cmd,
                directory: base.clone(),
            }),
        });
        let token = new_cwd_token();
        let other = new_cwd_token();
        assert_eq!(token.len(), 32);
        assert!(token.chars().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(token, other, "every session gets its own key");
        let path = target.display().to_string();
        // Output such as `type README.txt` or a remote host only knows the old format.
        for forged in [
            format!("{CWD_MARKER}{path}"),
            format!("{CWD_MARKER}{other}:{path}"),
            format!("{CWD_MARKER}{}:{path}", &token[..31]),
            format!("{CWD_MARKER}{token}{path}"),
            format!("{CWD_MARKER}:{path}"),
        ] {
            assert!(!workspace.update_directory(1, &forged, &token), "{forged}");
        }
        // An empty or short key must never match, e.g. the "--version" launches.
        assert!(!workspace.update_directory(1, &format!("{CWD_MARKER}:{path}"), ""));
        assert_eq!(workspace.inherited_directory(1, &base), base);
        assert!(workspace.update_directory(1, &format!("{CWD_MARKER}{token}:{path}"), &token));
        assert_eq!(workspace.inherited_directory(1, &base), target);
    }
    #[test]
    fn shell_integrations_carry_the_session_key() {
        let base = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let token = new_cwd_token();
        let marker = format!("{CWD_MARKER}{token}:");
        let (_, args) = Shell::PowerShell
            .launch(PathBuf::from("pwsh.exe"), &base, false, &token)
            .unwrap();
        assert!(args[3].contains(&marker));
        let (_, args) = Shell::Cmd
            .launch(PathBuf::from("cmd.exe"), &base, false, &token)
            .unwrap();
        assert_eq!(args[2], format!("prompt $E]0;{marker}$P$E\\$P$G"));
        assert_eq!(
            Shell::GitBash.session_env(&token).get(CWD_TOKEN_ENV),
            Some(&token)
        );
        assert!(Shell::PowerShell.session_env(&token).is_empty());
        assert!(Shell::Cmd
            .launch(PathBuf::from("cmd.exe"), &base, false, "x'; calc")
            .is_err());
    }
}
