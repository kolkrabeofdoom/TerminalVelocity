use eframe::egui::{self, Color32, FontId, Key, Modifiers};
use std::{
    fs,
    io::{Read, Write},
    path::{Path, PathBuf},
};

const LIMIT: usize = 256 * 1024;

#[derive(Default)]
pub struct Document {
    path: Option<PathBuf>,
    text: String,
    saved: String,
    disk: Option<Vec<u8>>,
    crlf: bool,
    bom: bool,
}

fn read_file(path: &Path) -> Result<Vec<u8>, String> {
    let file = fs::File::open(path).map_err(|e| e.to_string())?;
    if !file.metadata().map_err(|e| e.to_string())?.is_file() {
        return Err("Bitte eine normale Textdatei wählen.".into());
    }
    let mut bytes = Vec::new();
    file.take((LIMIT + 1) as u64)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > LIMIT {
        return Err("Der Editor unterstützt Dateien bis 256 KiB.".into());
    }
    Ok(bytes)
}

impl Document {
    fn open(path: PathBuf) -> Result<Self, String> {
        let bytes = read_file(&path)?;
        let bom = bytes.starts_with(&[0xef, 0xbb, 0xbf]);
        let raw = std::str::from_utf8(if bom { &bytes[3..] } else { &bytes })
            .map_err(|_| "Nur UTF-8 wird unterstützt; die Datei wurde nicht verändert.")?;
        if raw
            .chars()
            .any(|c| c.is_control() && !matches!(c, '\n' | '\r' | '\t'))
        {
            return Err("Die Datei enthält Binär- oder Steuerzeichen.".into());
        }
        let crlf = raw.contains("\r\n");
        let text = raw.replace("\r\n", "\n");
        if text.contains('\r') || (crlf && raw.matches('\n').count() != raw.matches("\r\n").count())
        {
            return Err("Gemischte oder reine CR-Zeilenenden werden nicht unterstützt; die Datei bleibt unverändert.".into());
        }
        Ok(Self {
            path: Some(path),
            saved: text.clone(),
            text,
            disk: Some(bytes),
            crlf,
            bom,
        })
    }
    pub fn dirty(&self) -> bool {
        self.text != self.saved
    }
    fn bytes(&self) -> Result<Vec<u8>, String> {
        let text = if self.crlf {
            self.text.replace('\n', "\r\n")
        } else {
            self.text.clone()
        };
        let mut bytes = if self.bom {
            vec![0xef, 0xbb, 0xbf]
        } else {
            Vec::new()
        };
        bytes.extend_from_slice(text.as_bytes());
        if bytes.len() > LIMIT {
            return Err("Mehr als 256 KiB: Bitte den Text vor dem Speichern verkleinern.".into());
        }
        Ok(bytes)
    }
    fn save_to(&mut self, path: PathBuf, expected: Option<Vec<u8>>) -> Result<(), String> {
        let bytes = self.bytes()?;
        atomic_save(&path, &bytes, expected.as_deref())?;
        self.path = Some(path);
        self.disk = Some(bytes);
        self.saved = self.text.clone();
        Ok(())
    }
    fn title(&self) -> String {
        let name = self
            .path
            .as_ref()
            .and_then(|p| p.file_name())
            .map(|p| p.to_string_lossy().into_owned())
            .unwrap_or_else(|| "Unbenannt".into());
        format!("{}{}", name, if self.dirty() { " *" } else { "" })
    }
}

/// Stage in the same directory and replace atomically, retaining Windows file metadata.
fn atomic_save(path: &Path, bytes: &[u8], expected: Option<&[u8]>) -> Result<(), String> {
    use std::os::windows::{ffi::OsStrExt, fs::OpenOptionsExt};
    use windows_sys::Win32::Storage::FileSystem::{MoveFileExW, ReplaceFileW};
    static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let parent = path.parent().ok_or("Kein Zielordner")?;
    let temp = parent.join(format!(
        ".terminalvelocity-{}-{}.tmp",
        std::process::id(),
        NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
    ));
    let mut staged = fs::OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temp)
        .map_err(|e| e.to_string())?;
    let result = (|| {
        staged
            .write_all(bytes)
            .and_then(|_| staged.sync_all())
            .map_err(|e| e.to_string())?;
        drop(staged);
        // Deny concurrent writes while checking and replacing an existing file.
        // Share deletion so ReplaceFileW can exchange the directory entry.
        let mut guard = None;
        if let Some(expected) = expected {
            let metadata = fs::symlink_metadata(path)
                .map_err(|e| format!("Datei nicht mehr verfügbar: {e}"))?;
            if metadata.file_type().is_symlink() || metadata.permissions().readonly() {
                return Err(
                    "Schreibgeschützte Dateien und symbolische Links werden nicht überschrieben."
                        .into(),
                );
            }
            let mut file = fs::OpenOptions::new()
                .read(true)
                .share_mode(5)
                .open(path)
                .map_err(|e| e.to_string())?;
            let mut actual = Vec::new();
            (&mut file)
                .take((LIMIT + 1) as u64)
                .read_to_end(&mut actual)
                .map_err(|e| e.to_string())?;
            if actual != expected {
                return Err("Die Datei wurde außerhalb des Editors geändert. Bitte neu öffnen oder unter anderem Namen speichern.".into());
            }
            guard = Some(file);
        }
        let wide = |p: &Path| {
            p.as_os_str()
                .encode_wide()
                .chain(Some(0))
                .collect::<Vec<_>>()
        };
        let destination = wide(path);
        let source = wide(&temp);
        let ok = unsafe {
            if expected.is_some() {
                ReplaceFileW(
                    destination.as_ptr(),
                    source.as_ptr(),
                    std::ptr::null(),
                    0,
                    std::ptr::null(),
                    std::ptr::null(),
                )
            } else {
                // No replace flag: a file created since the dialog must not be overwritten.
                MoveFileExW(source.as_ptr(), destination.as_ptr(), 0)
            }
        };
        let error = if ok == 0 {
            Some(std::io::Error::last_os_error().to_string())
        } else {
            None
        };
        drop(guard);
        error.map_or(Ok(()), Err)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temp);
    }
    result
}

enum Pending {
    Replace(Document),
    Close,
    Exit,
}

#[derive(Default)]
pub struct Editor {
    pub active: bool,
    pub visible: bool,
    document: Document,
    pending: Option<Pending>,
    allow_exit: bool,
    generation: u64,
    query: String,
    search: bool,
    focus_search: bool,
    next_match: usize,
    message: String,
}

impl Editor {
    pub fn blocked(&self) -> bool {
        self.pending.is_some()
    }
    pub fn title(&self) -> String {
        format!("Editor · {}", self.document.title())
    }
    pub fn activate(&mut self) {
        self.active = true;
        self.visible = true;
    }
    pub fn start_search(&mut self) {
        self.search = true;
        self.focus_search = true;
    }
    fn request(&mut self, action: Pending, ctx: &egui::Context) {
        if self.document.dirty() {
            self.pending = Some(action);
        } else {
            self.apply(action, ctx);
        }
    }
    fn apply(&mut self, action: Pending, ctx: &egui::Context) {
        if !matches!(action, Pending::Exit) {
            ctx.data_mut(|data| {
                data.remove::<egui::text_edit::TextEditState>(egui::Id::new((
                    "editor-text",
                    self.generation,
                )))
            });
        }
        match action {
            Pending::Replace(document) => {
                self.document = document;
                self.generation += 1;
                self.query.clear();
                self.next_match = 0;
                self.message.clear();
                self.activate();
            }
            Pending::Close => {
                self.document = Document::default();
                self.generation += 1;
                self.active = false;
                self.visible = false;
            }
            Pending::Exit => {
                self.allow_exit = true;
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
    }
    pub fn guard_exit(&mut self, ctx: &egui::Context) {
        if ctx.input(|i| i.viewport().close_requested())
            && self.document.dirty()
            && !self.allow_exit
        {
            ctx.send_viewport_cmd(egui::ViewportCommand::CancelClose);
            self.pending = Some(Pending::Exit);
        }
    }
    fn save(&mut self, owner: *mut std::ffi::c_void, base: &Path, save_as: bool) -> bool {
        let result = (|| {
            let path = if save_as || self.document.path.is_none() {
                let initial = self.document.path.as_deref().unwrap_or(base);
                let Some(path) = choose_file(owner, initial, true)? else {
                    return Ok(false);
                };
                path
            } else {
                self.document.path.clone().unwrap()
            };
            let expected = if self.document.path.as_ref() == Some(&path) {
                self.document.disk.clone()
            } else if path.try_exists().map_err(|e| e.to_string())? {
                Some(read_file(&path)?)
            } else {
                None
            };
            self.document.save_to(path, expected)?;
            Ok(true)
        })();
        match result {
            Ok(saved) => {
                if saved {
                    self.message = "Gespeichert".into();
                }
                saved
            }
            Err(error) => {
                self.message = error;
                false
            }
        }
    }
    pub fn confirmation(&mut self, ctx: &egui::Context, owner: *mut std::ffi::c_void, base: &Path) {
        if self.pending.is_none() {
            return;
        }
        let mut choice = 0;
        egui::Window::new("Ungespeicherte Änderungen")
            .collapsible(false)
            .resizable(false)
            .show(ctx, |ui| {
                ui.label(format!(
                    "Änderungen an {} speichern?",
                    self.document.title()
                ));
                ui.horizontal(|ui| {
                    if ui.button("Speichern und fortfahren").clicked() {
                        choice = 1;
                    }
                    if ui.button("Verwerfen").clicked() {
                        choice = 2;
                    }
                    if ui.button("Abbrechen").clicked() {
                        choice = 3;
                    }
                });
                ui.label(&self.message);
            });
        if choice == 3 {
            self.pending = None;
        }
        if choice == 2 || (choice == 1 && self.save(owner, base, false)) {
            if let Some(action) = self.pending.take() {
                self.apply(action, ctx);
            }
        }
    }
    pub fn ui(
        &mut self,
        ui: &mut egui::Ui,
        owner: *mut std::ffi::c_void,
        base: &Path,
        enabled: bool,
    ) {
        ui.add_enabled_ui(enabled && !self.blocked(), |ui| {
            let ctx = ui.ctx().clone();
            let mut save = false;
            let mut save_as = false;
            if ui.is_enabled() {
                save_as =
                    ctx.input_mut(|i| i.consume_key(Modifiers::CTRL | Modifiers::SHIFT, Key::S));
                save = ctx.input_mut(|i| i.consume_key(Modifiers::CTRL, Key::S));
                if ctx.input_mut(|i| i.consume_key(Modifiers::CTRL, Key::F)) {
                    self.search = true;
                    self.focus_search = true;
                }
            }
            ui.horizontal_wrapped(|ui| {
                if ui.button("Neu").clicked() {
                    self.request(Pending::Replace(Document::default()), &ctx);
                }
                if ui.button("Öffnen …").clicked() {
                    match choose_file(owner, base, false)
                        .and_then(|p| p.map(Document::open).transpose())
                    {
                        Ok(Some(document)) => self.request(Pending::Replace(document), &ctx),
                        Ok(None) => {}
                        Err(error) => self.message = error,
                    }
                }
                save |= ui.button("Speichern · Strg+S").clicked();
                save_as |= ui.button("Speichern unter …").clicked();
                if ui.button("Suchen · Strg+F").clicked() {
                    self.search = true;
                    self.focus_search = true;
                }
                if ui.button("Editor schließen").clicked() {
                    self.request(Pending::Close, &ctx);
                }
            });
            if save || save_as {
                self.save(owner, base, save_as);
            }
            ui.label(
                self.document
                    .path
                    .as_ref()
                    .map(|p| p.display().to_string())
                    .unwrap_or_else(|| format!("Neue Datei · Startordner: {}", base.display())),
            );
            let mut jump = false;
            if self.search {
                ui.horizontal(|ui| {
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut self.query)
                            .hint_text("Wörtliche Suche, Groß-/Kleinschreibung")
                            .char_limit(256),
                    );
                    if self.focus_search {
                        response.request_focus();
                        self.focus_search = false;
                    }
                    if response.changed() {
                        self.next_match = 0;
                    }
                    jump = ui.button("Nächster Treffer").clicked()
                        || (response.lost_focus() && ui.input(|i| i.key_pressed(Key::Enter)));
                    if ui.button("Schließen").clicked() {
                        self.search = false;
                    }
                });
            }
            let range = if jump && !self.query.is_empty() {
                let matches: Vec<_> = self
                    .document
                    .text
                    .match_indices(&self.query)
                    .map(|(start, matched)| start..start + matched.len())
                    .collect();
                if matches.is_empty() {
                    self.message = "Keine Treffer".into();
                    None
                } else {
                    let index = self.next_match % matches.len();
                    self.next_match = index + 1;
                    self.message = format!("Treffer {} / {}", index + 1, matches.len());
                    let found = matches[index].clone();
                    Some(
                        self.document.text[..found.start].chars().count()
                            ..self.document.text[..found.end].chars().count(),
                    )
                }
            } else {
                None
            };
            ui.small(format!(
                "UTF-8{} · {} · Tab: Einrückung · Strg+Z: Undo · Strg+Umschalt+Z: Redo · {}",
                if self.document.bom { " mit BOM" } else { "" },
                if self.document.crlf { "CRLF" } else { "LF" },
                self.document.title()
            ));
            if !self.message.is_empty() {
                ui.label(&self.message);
            }
            ui.separator();
            let extension = self
                .document
                .path
                .as_ref()
                .and_then(|p| p.extension())
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_ascii_lowercase();
            let font = FontId::monospace(14.0);
            let row_height = ui.fonts(|f| f.row_height(&font));
            let lines = (1..=self.document.text.bytes().filter(|b| *b == b'\n').count() + 1)
                .map(|n| n.to_string())
                .collect::<Vec<_>>()
                .join("\n");
            let id = egui::Id::new(("editor-text", self.generation));
            egui::ScrollArea::both()
                .id_salt(("editor-scroll", self.generation))
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.horizontal_top(|ui| {
                        ui.add(
                            egui::Label::new(
                                egui::RichText::new(lines)
                                    .font(font.clone())
                                    .line_height(Some(row_height))
                                    .color(Color32::GRAY),
                            )
                            .selectable(false),
                        );
                        let mut layouter = |ui: &egui::Ui, text: &str, _: f32| {
                            ui.fonts(|f| f.layout_job(highlight(text, &extension, row_height)))
                        };
                        let mut output = egui::TextEdit::multiline(&mut self.document.text)
                            .id(id)
                            .code_editor()
                            .font(font.clone())
                            .desired_width(ui.available_width().max(600.0))
                            .desired_rows(24)
                            .margin(egui::Vec2::ZERO)
                            .char_limit(LIMIT)
                            .layouter(&mut layouter)
                            .show(ui);
                        if let Some(range) = range {
                            let start = egui::text::CCursor::new(range.start);
                            let end = egui::text::CCursor::new(range.end);
                            output
                                .state
                                .cursor
                                .set_char_range(Some(egui::text::CCursorRange::two(start, end)));
                            output.state.store(ui.ctx(), id);
                            let cursor = output.galley.from_ccursor(start);
                            ui.scroll_to_rect(
                                output
                                    .galley
                                    .pos_from_cursor(&cursor)
                                    .translate(output.galley_pos.to_vec2()),
                                Some(egui::Align::Center),
                            );
                            output.response.request_focus();
                        }
                    });
                });
        });
    }
}

fn choose_file(
    owner: *mut std::ffi::c_void,
    initial: &Path,
    save: bool,
) -> Result<Option<PathBuf>, String> {
    use std::os::windows::ffi::{OsStrExt, OsStringExt};
    use windows_sys::Win32::UI::Controls::Dialogs::*;
    let mut buffer = vec![0u16; 32768];
    let folder = if initial.is_dir() {
        initial
    } else {
        initial.parent().unwrap_or(initial)
    };
    if save && initial.is_file() {
        let name: Vec<_> = initial.file_name().unwrap().encode_wide().collect();
        buffer[..name.len()].copy_from_slice(&name);
    }
    let directory: Vec<_> = folder.as_os_str().encode_wide().chain(Some(0)).collect();
    let filter: Vec<_> =
        "Text und Skripte\0*.txt;*.ps1;*.sh;*.json;*.toml;*.md;*.bash\0Alle Dateien\0*.*\0\0"
            .encode_utf16()
            .collect();
    unsafe {
        let mut dialog: OPENFILENAMEW = std::mem::zeroed();
        dialog.lStructSize = std::mem::size_of::<OPENFILENAMEW>() as u32;
        dialog.hwndOwner = owner;
        dialog.lpstrFile = buffer.as_mut_ptr();
        dialog.nMaxFile = buffer.len() as u32;
        dialog.lpstrInitialDir = directory.as_ptr();
        dialog.lpstrFilter = filter.as_ptr();
        dialog.Flags = OFN_PATHMUSTEXIST
            | OFN_NOCHANGEDIR
            | OFN_EXPLORER
            | if save {
                OFN_OVERWRITEPROMPT
            } else {
                OFN_FILEMUSTEXIST
            };
        let result = if save {
            GetSaveFileNameW(&mut dialog)
        } else {
            GetOpenFileNameW(&mut dialog)
        };
        if result != 0 {
            let end = buffer.iter().position(|c| *c == 0).unwrap_or(buffer.len());
            Ok(Some(std::ffi::OsString::from_wide(&buffer[..end]).into()))
        } else {
            let error = CommDlgExtendedError();
            if error == 0 {
                Ok(None)
            } else {
                Err(format!("Dateidialog fehlgeschlagen: {error}"))
            }
        }
    }
}

/// Small lexical highlighter, deliberately without a parser or language server.
fn highlight(text: &str, extension: &str, row_height: f32) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    let supported = matches!(
        extension,
        "ps1" | "psm1" | "sh" | "bash" | "json" | "toml" | "md" | "markdown"
    );
    let mut remaining = text;
    while !remaining.is_empty() {
        let first = remaining.chars().next().unwrap();
        let mut color = Color32::from_rgb(215, 225, 235);
        let mut length = first.len_utf8();
        if first.is_whitespace() {
            length = remaining
                .find(|c: char| !c.is_whitespace())
                .unwrap_or(remaining.len());
        } else if supported && first == '#' && extension != "json" {
            length = remaining.find('\n').unwrap_or(remaining.len());
            color = Color32::from_rgb(120, 180, 140);
        } else if supported && matches!(first, '\'' | '"' | '`') {
            let mut escaped = false;
            for (index, c) in remaining.char_indices().skip(1) {
                if c == '\n' {
                    break;
                }
                length = index + c.len_utf8();
                if c == first && !escaped {
                    break;
                }
                escaped = c == '\\' && !escaped;
            }
            color = Color32::from_rgb(225, 185, 120);
        } else if first.is_alphanumeric() || first == '_' || first == '$' {
            length = remaining
                .find(|c: char| !c.is_alphanumeric() && !matches!(c, '_' | '$' | '-'))
                .unwrap_or(remaining.len());
            let token = &remaining[..length];
            if supported
                && (first.is_ascii_digit()
                    || matches!(
                        token.to_ascii_lowercase().as_str(),
                        "true" | "false" | "null" | "$true" | "$false" | "$null"
                    ))
            {
                color = Color32::from_rgb(190, 150, 230);
            } else if supported
                && (first == '$'
                    || matches!(
                        token.to_ascii_lowercase().as_str(),
                        "if" | "else"
                            | "elseif"
                            | "then"
                            | "fi"
                            | "for"
                            | "foreach"
                            | "while"
                            | "do"
                            | "done"
                            | "function"
                            | "return"
                            | "param"
                            | "try"
                            | "catch"
                            | "finally"
                            | "in"
                            | "case"
                            | "esac"
                    ))
            {
                color = Color32::from_rgb(100, 195, 240);
            }
        }
        job.append(
            &remaining[..length],
            0.0,
            egui::TextFormat {
                font_id: FontId::monospace(14.0),
                color,
                line_height: Some(row_height),
                ..Default::default()
            },
        );
        remaining = &remaining[length..];
    }
    job
}

#[cfg(test)]
mod tests {
    use super::*;
    struct Fixture(PathBuf);
    impl Fixture {
        fn new() -> Self {
            static NEXT: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
            let path = std::env::temp_dir().join(format!(
                "TerminalVelocity-editor-{}-{}",
                std::process::id(),
                NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed)
            ));
            fs::create_dir(&path).unwrap();
            Self(path)
        }
        fn path(&self) -> PathBuf {
            self.0.join("Skript O'Brien ü.ps1")
        }
    }
    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.0);
        }
    }

    #[test]
    fn encoding_and_line_endings_survive_edit_and_atomic_save() {
        let fixture = Fixture::new();
        for (bom, crlf) in [(false, false), (false, true), (true, false), (true, true)] {
            let raw = format!(
                "{}Grüße{}Welt{}",
                if bom { "\u{feff}" } else { "" },
                if crlf { "\r\n" } else { "\n" },
                if crlf { "\r\n" } else { "\n" }
            );
            fs::write(fixture.path(), raw.as_bytes()).unwrap();
            let mut doc = Document::open(fixture.path()).unwrap();
            assert_eq!(doc.bytes().unwrap(), raw.as_bytes());
            assert!(!doc.dirty());
            doc.text.push_str("Neu 🦀\n");
            assert!(doc.dirty());
            let expected = doc.disk.clone();
            doc.save_to(fixture.path(), expected).unwrap();
            assert!(!doc.dirty());
            let reopened = Document::open(fixture.path()).unwrap();
            assert_eq!(reopened.text, doc.text);
            assert_eq!(reopened.bom, bom);
            assert_eq!(reopened.crlf, crlf);
        }
        assert_eq!(fs::read_dir(&fixture.0).unwrap().count(), 1);
    }

    #[test]
    fn external_changes_deletion_and_new_file_collision_are_not_overwritten() {
        let fixture = Fixture::new();
        fs::write(fixture.path(), "original").unwrap();
        let mut doc = Document::open(fixture.path()).unwrap();
        doc.text = "editor".into();
        fs::write(fixture.path(), "external").unwrap();
        assert!(doc.save_to(fixture.path(), doc.disk.clone()).is_err());
        assert_eq!(fs::read(fixture.path()).unwrap(), b"external");
        assert!(doc.dirty());
        fs::remove_file(fixture.path()).unwrap();
        assert!(doc.save_to(fixture.path(), doc.disk.clone()).is_err());
        assert!(!fixture.path().exists());
        fs::write(fixture.path(), "collision").unwrap();
        assert!(atomic_save(&fixture.path(), b"new", None).is_err());
        assert_eq!(fs::read(fixture.path()).unwrap(), b"collision");
        assert_eq!(fs::read_dir(&fixture.0).unwrap().count(), 1);
    }

    #[test]
    fn unsupported_files_are_rejected_without_modification() {
        let fixture = Fixture::new();
        for bytes in [
            vec![0xff, 0xfe, 0],
            b"one\r\ntwo\n".to_vec(),
            b"a\rb".to_vec(),
            b"a\0b".to_vec(),
            vec![b'x'; LIMIT + 1],
        ] {
            fs::write(fixture.path(), &bytes).unwrap();
            assert!(Document::open(fixture.path()).is_err());
            assert_eq!(fs::read(fixture.path()).unwrap(), bytes);
        }
        assert!(read_file(&fixture.0).is_err());
        let mut doc = Document::default();
        doc.text = "🦀".repeat(LIMIT);
        assert!(doc.bytes().is_err());
    }

    #[test]
    fn new_file_and_save_as_preserve_original_document() {
        let fixture = Fixture::new();
        let mut doc = Document::default();
        doc.text = "new\n".into();
        doc.save_to(fixture.path(), None).unwrap();
        doc.text = "changed\n".into();
        let other = fixture.0.join("copy.txt");
        doc.save_to(other.clone(), None).unwrap();
        assert_eq!(fs::read(fixture.path()).unwrap(), b"new\n");
        assert_eq!(fs::read(other).unwrap(), b"changed\n");
    }

    #[test]
    fn dirty_document_requires_confirmation_for_replace_close_and_exit() {
        let ctx = egui::Context::default();
        let mut editor = Editor::default();
        editor.activate();
        editor.document.text = "keep me".into();
        editor.request(Pending::Replace(Document::default()), &ctx);
        assert!(editor.blocked());
        assert_eq!(editor.document.text, "keep me");
        editor.pending = None; // Cancel leaves the document intact.
        editor.request(Pending::Close, &ctx);
        assert!(editor.visible);
        assert!(editor.blocked());
        editor.request(Pending::Exit, &ctx);
        assert!(!editor.allow_exit);
        assert!(matches!(editor.pending, Some(Pending::Exit)));
        editor.apply(Pending::Close, &ctx);
        assert!(!editor.visible);
        assert!(!editor.document.dirty());
    }

    #[test]
    fn syntax_layout_preserves_unicode_and_all_source_bytes() {
        let text = "# Grüße 🦀\nif ($true) { echo \"a\\\"b\" }\n[section]\nkey = 'value'\n";
        for extension in ["ps1", "sh", "json", "toml", "md", "txt"] {
            let job = highlight(text, extension, 17.0);
            assert_eq!(job.text, text);
            let recovered: String = job
                .sections
                .iter()
                .map(|s| &job.text[s.byte_range.clone()])
                .collect();
            assert_eq!(recovered, text);
            if extension != "txt" {
                assert!(job
                    .sections
                    .iter()
                    .any(|s| s.format.color != job.sections[0].format.color));
            }
        }
    }

    #[test]
    fn editor_widget_accepts_unicode_and_renders_without_terminal_focus() {
        let ctx = egui::Context::default();
        let mut editor = Editor::default();
        editor.activate();
        let frame = |editor: &mut Editor, input| {
            ctx.run(input, |ctx| {
                egui::CentralPanel::default().show(ctx, |ui| {
                    editor.ui(ui, std::ptr::null_mut(), Path::new("C:/"), true)
                });
            })
        };
        let _ = frame(&mut editor, egui::RawInput::default());
        ctx.memory_mut(|memory| memory.request_focus(egui::Id::new(("editor-text", 0u64))));
        let output = frame(
            &mut editor,
            egui::RawInput {
                events: vec![egui::Event::Text("Grüße 🦀\nzweite Zeile".into())],
                ..Default::default()
            },
        );
        assert_eq!(editor.document.text, "Grüße 🦀\nzweite Zeile");
        assert!(editor.document.dirty());
        assert!(!output.shapes.is_empty());
        let key_input = |key, modifiers, time| egui::RawInput {
            time: Some(time),
            modifiers,
            events: vec![egui::Event::Key {
                key,
                physical_key: None,
                pressed: true,
                repeat: false,
                modifiers,
            }],
            ..Default::default()
        };
        let command = Modifiers {
            ctrl: true,
            command: true,
            ..Modifiers::NONE
        };
        let _ = frame(&mut editor, key_input(Key::Z, command, 2.0));
        assert!(editor.document.text.is_empty());
        let _ = frame(
            &mut editor,
            key_input(Key::Z, command | Modifiers::SHIFT, 3.0),
        );
        assert_eq!(editor.document.text, "Grüße 🦀\nzweite Zeile");
        let _ = frame(&mut editor, key_input(Key::Tab, Modifiers::NONE, 4.0));
        assert!(editor.document.text.ends_with('\t'));
        editor.query = "🦀".into();
        editor.start_search();
        let _ = frame(&mut editor, egui::RawInput::default());
        assert!(editor.search);
        let _ = frame(&mut editor, key_input(Key::Enter, Modifiers::NONE, 5.0));
        assert_eq!(editor.message, "Treffer 1 / 1");
        let state =
            egui::text_edit::TextEditState::load(&ctx, egui::Id::new(("editor-text", 0u64)))
                .unwrap();
        assert_eq!(
            state.cursor.char_range().unwrap().sorted().map(|c| c.index),
            [6, 7]
        );
    }

    #[test]
    fn window_close_is_cancelled_until_dirty_document_is_resolved() {
        let ctx = egui::Context::default();
        let mut editor = Editor::default();
        editor.document.text = "unsaved".into();
        let mut input = egui::RawInput::default();
        input
            .viewports
            .get_mut(&egui::ViewportId::ROOT)
            .unwrap()
            .events
            .push(egui::ViewportEvent::Close);
        let output = ctx.run(input, |ctx| editor.guard_exit(ctx));
        assert!(matches!(editor.pending, Some(Pending::Exit)));
        assert!(output.viewport_output[&egui::ViewportId::ROOT]
            .commands
            .iter()
            .any(|c| matches!(c, egui::ViewportCommand::CancelClose)));
    }

    #[test]
    fn failed_save_keeps_buffer_and_readonly_file() {
        let fixture = Fixture::new();
        fs::write(fixture.path(), "original").unwrap();
        let mut doc = Document::open(fixture.path()).unwrap();
        doc.text = "new".into();
        let original_permissions = fs::metadata(fixture.path()).unwrap().permissions();
        let mut readonly = original_permissions.clone();
        readonly.set_readonly(true);
        fs::set_permissions(fixture.path(), readonly).unwrap();
        let result = doc.save_to(fixture.path(), doc.disk.clone());
        fs::set_permissions(fixture.path(), original_permissions).unwrap();
        assert!(result.is_err());
        assert!(doc.dirty());
        assert_eq!(fs::read(fixture.path()).unwrap(), b"original");
        assert_eq!(fs::read_dir(&fixture.0).unwrap().count(), 1);
    }
}
