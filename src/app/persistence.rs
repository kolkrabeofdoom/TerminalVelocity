//! Loading and saving workspace.json; an unloadable file is backed up and protected.

use crate::*;

impl TerminalApp {
    /// Implicit save (exit, settings, connections): blocked while an unloaded file is protected.
    pub(crate) fn save_workspace(&mut self) -> bool {
        self.write_workspace(false)
    }

    /// Save explicitly requested by the user; may replace a protected file once it is backed up.
    pub(crate) fn save_workspace_explicit(&mut self) -> bool {
        self.write_workspace(true)
    }

    pub(crate) fn write_workspace(&mut self, explicit: bool) -> bool {
        if let Some(guard) = &self.workspace_guard {
            if let Err(message) = guard.check(explicit) {
                self.status = message;
                return false;
            }
        }
        self.workspace.default_directory = Some(PathBuf::from(&self.directory));
        let result = (|| -> Result<(), String> {
            let encoded = serde_json::to_vec_pretty(&self.workspace).map_err(|e| e.to_string())?;
            model::atomic_write_json(&self.storage, &encoded)
        })();
        let saved = result.is_ok();
        if saved {
            // The file on disk is valid again; regular saving resumes.
            self.workspace_guard = None;
        }
        self.status = match result {
            Ok(()) => tr("Arbeitsbereich gespeichert").into(),
            Err(e) => trf!("Speichern fehlgeschlagen: {e}", e = e),
        };
        saved
    }
}

/// Load `workspace.json`. Returns the workspace, the first status line and, if the
/// file exists but cannot be used, a guard that protects it from being overwritten.
pub(crate) fn load_workspace(
    storage: &std::path::Path,
) -> (Workspace, String, Option<model::WorkspaceGuard>) {
    match std::fs::metadata(storage) {
        Ok(_) => match Workspace::load(storage) {
            Ok(state) => (
                state,
                tr("Arbeitsbereich wiederhergestellt").to_string(),
                None,
            ),
            Err(error) => {
                // Never let the defaults silently replace the user's file on exit.
                let backup = model::backup_unloadable(storage);
                let status = match &backup {
                    Ok(path) => trf!(
                        "Workspace nicht geladen: {error}. Kopie gesichert unter {path}; automatisches Speichern ist ausgesetzt.",
                        error = error,
                        path = path.display()
                    ),
                    Err(backup_error) => trf!(
                        "Workspace nicht geladen: {error}. Sicherung fehlgeschlagen ({backup_error}); Speichern ist gesperrt.",
                        error = error,
                        backup_error = backup_error
                    ),
                };
                (
                    Workspace::default(),
                    status,
                    Some(model::WorkspaceGuard {
                        reason: error,
                        backup: backup.ok(),
                    }),
                )
            }
        },
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            (Workspace::default(), tr("Bereit").into(), None)
        }
        Err(error) => (
            Workspace::default(),
            trf!(
                "Lesefehler: {error}; Speichern ist gesperrt.",
                error = error
            ),
            Some(model::WorkspaceGuard {
                reason: error.to_string(),
                backup: None,
            }),
        ),
    }
}
