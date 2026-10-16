use crate::model::{Node, Shell, Tab, Workspace};
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Profile {
    pub name: String,
    pub directory: PathBuf,
    pub default_shell: Shell,
    pub active_tab: usize,
    pub tabs: Vec<Tab>,
}

impl Profile {
    pub fn capture(name: &str, directory: PathBuf, workspace: &Workspace) -> Result<Self, String> {
        let profile = Self {
            name: name.trim().into(),
            directory,
            default_shell: workspace.default_shell,
            active_tab: workspace.active_tab,
            tabs: workspace.tabs.clone(),
        };
        profile.validate()?;
        Ok(profile)
    }

    fn validate(&self) -> Result<(), String> {
        if self.name.trim().is_empty()
            || self.name.chars().count() > 80
            || self.name.chars().any(char::is_control)
        {
            return Err("Bitte einen Projektnamen mit 1 bis 80 Zeichen eingeben.".into());
        }
        if !self.directory.is_absolute() || self.tabs.is_empty() {
            return Err(
                "Ein Profil benötigt einen absoluten Projektordner und mindestens einen Tab."
                    .into(),
            );
        }
        let mut workspace = Workspace {
            tabs: self.tabs.clone(),
            ..Workspace::default()
        };
        workspace.validate()?;
        if workspace
            .panes()
            .iter()
            .any(|pane| !pane.directory.is_absolute())
        {
            return Err("Alle Bereichsordner müssen absolute Pfade sein.".into());
        }
        Ok(())
    }

    /// Append a template to a clone first: invalid paths or limits leave the live workspace intact.
    pub fn prepare(
        &self,
        directory: &Path,
        current: &Workspace,
        next_id: u64,
    ) -> Result<(Workspace, u64), String> {
        self.validate()?;
        if !directory.is_absolute() || !directory.is_dir() {
            return Err("Der Projektordner existiert nicht oder ist kein absoluter Pfad.".into());
        }
        let mut tabs = self.tabs.clone();
        let mut id = next_id;
        fn remap(node: &mut Node, old: &Path, new: &Path, id: &mut u64) -> Result<(), String> {
            match node {
                Node::Pane(pane) => {
                    pane.id = *id;
                    *id = id.checked_add(1).ok_or("Bereichs-IDs ausgeschöpft")?;
                    if let Ok(relative) = pane.directory.strip_prefix(old) {
                        pane.directory = new.join(relative);
                    }
                    if !pane.directory.is_dir() {
                        return Err(format!(
                            "Bereichsordner fehlt: {}",
                            pane.directory.display()
                        ));
                    }
                }
                Node::Split { first, second, .. } => {
                    remap(first, old, new, id)?;
                    remap(second, old, new, id)?;
                }
            }
            Ok(())
        }
        for tab in &mut tabs {
            remap(&mut tab.root, &self.directory, directory, &mut id)?;
            tab.name = format!("{} · {}", self.name, tab.name);
        }
        let mut workspace = current.clone();
        workspace.active_tab = workspace.tabs.len() + self.active_tab.min(tabs.len() - 1);
        workspace.tabs.extend(tabs);
        workspace.default_directory = Some(directory.to_path_buf());
        workspace.default_shell = self.default_shell;
        workspace.validate()?;
        Ok((workspace, id))
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Library {
    pub version: u32,
    pub profiles: Vec<Profile>,
}

impl Default for Library {
    fn default() -> Self {
        Self {
            version: 1,
            profiles: vec![],
        }
    }
}

impl Library {
    pub fn validate(&self) -> Result<(), String> {
        if self.version != 1 || self.profiles.len() > 32 {
            return Err("Unbekannte Profildatei-Version oder mehr als 32 Profile.".into());
        }
        let mut names = std::collections::HashSet::new();
        for profile in &self.profiles {
            profile.validate()?;
            if !names.insert(profile.name.trim().to_lowercase()) {
                return Err("Ein Projektprofil mit diesem Namen existiert bereits.".into());
            }
        }
        Ok(())
    }

    pub fn load(path: &Path) -> Result<Self, String> {
        let file = match std::fs::File::open(path) {
            Ok(file) => file,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(Self::default())
            }
            Err(error) => return Err(error.to_string()),
        };
        use std::io::Read;
        let mut bytes = vec![];
        file.take(2_000_001)
            .read_to_end(&mut bytes)
            .map_err(|e| e.to_string())?;
        if bytes.len() > 2_000_000 {
            return Err("Profildatei ist zu groß.".into());
        }
        let library: Self = serde_json::from_slice(&bytes).map_err(|e| e.to_string())?;
        library.validate()?;
        Ok(library)
    }

    pub fn save(&self, path: &Path) -> Result<(), String> {
        self.validate()?;
        let bytes = serde_json::to_vec_pretty(self).map_err(|e| e.to_string())?;
        if bytes.len() > 2_000_000 {
            return Err("Profildatei ist zu groß.".into());
        }
        let temporary = path.with_extension("json.tmp");
        std::fs::write(&temporary, bytes).map_err(|e| e.to_string())?;
        #[cfg(windows)]
        {
            use std::os::windows::ffi::OsStrExt;
            unsafe extern "system" {
                fn MoveFileExW(existing: *const u16, new: *const u16, flags: u32) -> i32;
            }
            let from: Vec<u16> = temporary.as_os_str().encode_wide().chain(Some(0)).collect();
            let to: Vec<u16> = path.as_os_str().encode_wide().chain(Some(0)).collect();
            if unsafe { MoveFileExW(from.as_ptr(), to.as_ptr(), 1 | 8) } == 0 {
                return Err(std::io::Error::last_os_error().to_string());
            }
        }
        #[cfg(not(windows))]
        std::fs::rename(temporary, path).map_err(|e| e.to_string())?;
        Ok(())
    }
}

pub struct Dialog {
    pub saving: bool,
    pub name: String,
    pub directory: String,
    pub selected: Option<usize>,
    pub delete_target: Option<usize>,
    pub error: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Axis, Pane};

    fn workspace(root: &Path) -> Workspace {
        Workspace {
            tabs: vec![Tab {
                name: "Entwicklung".into(),
                root: Node::Split {
                    axis: Axis::Columns,
                    ratio: 0.35,
                    first: Box::new(Node::Pane(Pane {
                        id: 1,
                        shell: Shell::PowerShell,
                        directory: root.into(),
                    })),
                    second: Box::new(Node::Pane(Pane {
                        id: 2,
                        shell: Shell::Codex,
                        directory: root.join("src"),
                    })),
                },
            }],
            ..Workspace::default()
        }
    }

    #[test]
    fn profiles_relocate_subdirectories_append_tabs_and_preserve_settings() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let template = workspace(&root);
        let profile = Profile::capture("  Entwicklung  ", root.clone(), &template).unwrap();
        let destination = root.join("vendor/egui_term-0.1.0");
        let mut current = template.clone();
        current.opacity = 0.6;
        current.shell_paths[0] = "custom.exe".into();
        let (opened, next_id) = profile.prepare(&destination, &current, 10).unwrap();
        assert_eq!(profile.name, "Entwicklung");
        assert_eq!(opened.tabs.len(), 2);
        assert_eq!(opened.active_tab, 1);
        assert_eq!(opened.opacity, 0.6);
        assert_eq!(opened.shell_paths, current.shell_paths);
        let panes = opened.panes();
        assert_eq!(panes[0].id, 1);
        assert_eq!(panes[2].id, 10);
        assert_eq!(panes[3].id, 11);
        assert_eq!(panes[3].shell, Shell::Codex);
        assert_eq!(panes[3].directory, destination.join("src"));
        assert_eq!(next_id, 12);
        assert!(matches!(opened.tabs[1].root, Node::Split { ratio, .. } if ratio == 0.35));
        // The saved template remains unchanged when an instance is opened elsewhere.
        assert_eq!(profile.directory, root);
        assert_eq!(current.tabs.len(), 1);
    }

    #[test]
    fn opening_missing_directories_or_exceeding_limits_is_transactional() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let current = workspace(&root);
        let profile = Profile::capture("Test", root.clone(), &current).unwrap();
        let before = serde_json::to_string(&current).unwrap();
        assert!(profile
            .prepare(&root.join("does-not-exist-profile-test"), &current, 10)
            .is_err());
        // tools exists, but its src subdirectory does not.
        assert!(profile.prepare(&root.join("tools"), &current, 10).is_err());
        assert!(profile.prepare(&root, &current, 1).is_err());
        let mut full = current.clone();
        full.tabs = (0..32)
            .map(|id| Tab {
                name: "Tab".into(),
                root: Node::Pane(Pane {
                    id,
                    shell: Shell::Cmd,
                    directory: root.clone(),
                }),
            })
            .collect();
        assert!(profile.prepare(&root, &full, 100).is_err());
        assert_eq!(serde_json::to_string(&current).unwrap(), before);
    }

    #[test]
    fn external_directories_remain_absolute() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let mut template = workspace(&root);
        template.tabs[0].root.pane_mut(2).unwrap().directory = root.parent().unwrap().to_path_buf();
        let profile = Profile::capture("Test", root.clone(), &template).unwrap();
        let (opened, _) = profile
            .prepare(&root.join("tools"), &Workspace::default(), 1)
            .unwrap();
        assert_eq!(opened.panes()[1].directory, root.parent().unwrap());
    }

    #[test]
    fn library_roundtrip_replacement_and_validation() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        let profile = Profile::capture("Projekt ü", root.clone(), &workspace(&root)).unwrap();
        let mut library = Library {
            version: 1,
            profiles: vec![profile],
        };
        let path =
            std::env::temp_dir().join(format!("terminal-profiles-{}.json", std::process::id()));
        library.save(&path).unwrap();
        let loaded = Library::load(&path).unwrap();
        assert_eq!(
            serde_json::to_value(&loaded).unwrap(),
            serde_json::to_value(&library).unwrap()
        );
        let saved = std::fs::read(&path).unwrap();
        library.profiles.push(library.profiles[0].clone());
        library.profiles[1].name = "PROJEKT Ü".into();
        assert!(library.save(&path).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), saved);
        library.profiles.clear();
        library.save(&path).unwrap();
        assert!(Library::load(&path).unwrap().profiles.is_empty());
        std::fs::write(&path, b"invalid JSON").unwrap();
        assert!(Library::load(&path).is_err());
        assert_eq!(std::fs::read(&path).unwrap(), b"invalid JSON");
        std::fs::remove_file(&path).unwrap();
        assert!(Library::load(&path).unwrap().profiles.is_empty());
    }
}
