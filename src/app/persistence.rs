//! Saving workspace.json, guarded while an unloadable file is protected.

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
            Ok(()) => "Arbeitsbereich gespeichert".into(),
            Err(e) => format!("Speichern fehlgeschlagen: {e}"),
        };
        saved
    }
}
