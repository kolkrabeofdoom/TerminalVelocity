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
    if crate::model::is_unc_or_device_path(path) {
        return Err("Netzwerk- und Gerätepfade werden nicht unterstützt.".into());
    }
    let file = fs::File::open(path).map_err(|e| e.to_string())?;
    if !file.metadata().map_err(|e| e.to_string())?.is_file() {
        return Err("Bitte eine normale Textdatei wählen.".into());
    }
    if path
        .canonicalize()
        .map(|c| crate::model::is_unc_or_device_path(&c))
        .unwrap_or(true)
    {
        return Err("Netzwerk- und Gerätepfade werden nicht unterstützt.".into());
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
    if crate::model::is_unc_or_device_path(path) {
        return Err("Netzwerk- und Gerätepfade werden nicht unterstützt.".into());
    }
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

pub enum Request {
    OpenFile(PathBuf, Option<usize>),
    OpenSelection(String),
}

enum Pending {
    Replace(Document, Option<usize>),
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
    jump_line: Option<usize>,
    message: String,
}

pub fn line_char_range(text: &str, line: usize) -> std::ops::Range<usize> {
    let target = line.saturating_sub(1);
    let mut char_offset = 0;
    for (index, part) in text.split('\n').enumerate() {
        let len = part.chars().count();
        if index == target {
            return char_offset..char_offset + len;
        }
        char_offset += len + 1;
    }
    let total = text.chars().count();
    total..total
}

/// Resolve a clicked local text file path and optional 1-based `:line` / `:line:col` suffix.
pub fn path_at(text: &str, offset: usize, base: &Path) -> Option<(PathBuf, Option<usize>)> {
    if text.len() > 8192 || offset >= text.len() || !text.is_char_boundary(offset) {
        return None;
    }
    let mut checked = 0usize;
    let mut check = |raw: &str| -> Option<(PathBuf, Option<usize>)> {
        checked += 1;
        if checked > 64 {
            return None;
        }
        parse_path_and_line(raw, base)
    };

    // 1. Check quoted spans containing `offset` (plus any immediately trailing `:line(:col)`).
    for quote in ['"', '\''] {
        let mut search_from = 0;
        while let Some(rel_open) = text[search_from..].find(quote) {
            let open = search_from + rel_open;
            let content_start = open + 1;
            let Some(rel_close) = text[content_start..].find(quote) else {
                break;
            };
            let close = content_start + rel_close;
            let mut end = close + 1;
            for (i, c) in text[end..].char_indices() {
                if c.is_ascii_digit() || c == ':' {
                    end = close + 1 + i + c.len_utf8();
                } else {
                    break;
                }
            }
            if offset >= open && offset < end {
                if let Some(found) = check(&text[open..end]) {
                    return Some(found);
                }
            }
            search_from = close + 1;
        }
    }

    // 2. Check the whitespace/bracket-delimited token around `offset`.
    let is_delim = |c: char| {
        c.is_whitespace() || matches!(c, '(' | ')' | '[' | ']' | '<' | '>' | '"' | ',' | ';')
    };
    let start = text[..offset]
        .char_indices()
        .rev()
        .find(|(_, c)| is_delim(*c))
        .map(|(i, c)| i + c.len_utf8())
        .unwrap_or(0);
    let end = text[offset..]
        .char_indices()
        .find(|(_, c)| is_delim(*c))
        .map(|(i, _)| offset + i)
        .unwrap_or(text.len());
    if start < end {
        if let Some(found) = check(&text[start..end]) {
            return Some(found);
        }
    }

    // 3. Check unquoted multi-word paths in directory listings or compiler lines
    //    (e.g. `-a--- ... Skript O'Brien ü.ps1` or `  --> Skript O'Brien ü.ps1:2:5: Fehler`).
    let line_end = text.trim_end().len();
    if offset < line_end {
        let mut starts = vec![0];
        let mut ends = Vec::new();
        for (i, c) in text[..line_end].char_indices() {
            let next = i + c.len_utf8();
            if next <= offset && (c.is_whitespace() || matches!(c, '(' | '[' | '"' | '\'')) {
                starts.push(next);
            }
            if i > offset && (c.is_whitespace() || matches!(c, ')' | ']' | '"' | '\'' | ',' | ';'))
            {
                ends.push(i);
            }
        }
        ends.push(line_end);
        for s in starts {
            for &e in ends.iter().rev() {
                if s < e {
                    if let Some(found) = check(&text[s..e]) {
                        return Some(found);
                    }
                }
            }
        }
    }
    None
}

fn parse_path_and_line(raw: &str, base: &Path) -> Option<(PathBuf, Option<usize>)> {
    let trimmed = raw
        .trim()
        .trim_matches(|c| matches!(c, '(' | ')' | '[' | ']' | '<' | '>' | ',' | ';'))
        .trim_end_matches(':');
    if trimmed.is_empty() || trimmed.contains(['\n', '\r', '\t']) || trimmed.contains("://") {
        return None;
    }
    let (mut path_part, mut line_num) = (trimmed, None);
    // Strip up to two trailing `:number` segments (`path:line` or `path:line:col`),
    // keeping Windows drive prefixes (`C:...`) intact.
    if let Some((before_first, first_num)) = split_trailing_number(path_part) {
        if let Some((before_second, second_num)) = split_trailing_number(before_first) {
            path_part = before_second;
            line_num = Some(second_num);
        } else {
            path_part = before_first;
            line_num = Some(first_num);
        }
    }
    let path_part = path_part.trim_matches(|c| matches!(c, '"' | '\''));
    if path_part.is_empty() || path_part.contains('"') {
        return None;
    }
    let candidate = PathBuf::from(path_part);
    if (path_part.contains(':') && !candidate.is_absolute())
        || crate::model::is_unc_or_device_path(&candidate)
    {
        return None;
    }
    let resolved = if candidate.is_absolute() {
        candidate
    } else {
        base.join(candidate)
    };
    if crate::model::is_unc_or_device_path(&resolved) {
        return None;
    }
    if let Some(ext) = resolved.extension().and_then(|e| e.to_str()) {
        if matches!(
            ext.to_ascii_lowercase().as_str(),
            "png"
                | "jpg"
                | "jpeg"
                | "webp"
                | "bmp"
                | "exe"
                | "dll"
                | "pdb"
                | "zip"
                | "nupkg"
                | "ttf"
                | "ico"
                | "obj"
                | "lib"
                | "rlib"
                | "rmeta"
        ) {
            return None;
        }
    }
    let meta = fs::metadata(&resolved).ok()?;
    if !meta.is_file()
        || meta.len() > LIMIT as u64
        || resolved
            .canonicalize()
            .map(|c| crate::model::is_unc_or_device_path(&c))
            .unwrap_or(true)
    {
        return None;
    }
    Some((resolved, line_num))
}

fn split_trailing_number(input: &str) -> Option<(&str, usize)> {
    let colon = input.rfind(':')?;
    if colon <= 2 {
        return None;
    }
    let number: usize = input[colon + 1..].parse().ok()?;
    if number == 0 || number > 999_999 {
        return None;
    }
    Some((&input[..colon], number))
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
    pub fn open_file(
        &mut self,
        path: PathBuf,
        line: Option<usize>,
        ctx: &egui::Context,
    ) -> Result<(), String> {
        let document = Document::open(path)?;
        self.request(Pending::Replace(document, line), ctx);
        self.activate();
        Ok(())
    }
    pub fn open_selection(&mut self, selection: String, ctx: &egui::Context) -> Result<(), String> {
        let text = selection.replace("\r\n", "\n").replace('\r', "\n");
        if text.trim().is_empty() {
            return Err("Bitte zuerst Text im Terminal markieren.".into());
        }
        if text.len() > LIMIT {
            return Err("Die Auswahl ist größer als 256 KiB.".into());
        }
        if text
            .chars()
            .any(|c| c.is_control() && !matches!(c, '\n' | '\t'))
        {
            return Err("Die Auswahl enthält nicht unterstützte Steuerzeichen.".into());
        }
        let document = Document {
            path: None,
            text,
            saved: String::new(),
            disk: None,
            crlf: false,
            bom: false,
        };
        self.request(Pending::Replace(document, Some(1)), ctx);
        self.activate();
        Ok(())
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
            Pending::Replace(document, line) => {
                self.document = document;
                self.generation += 1;
                self.query.clear();
                self.next_match = 0;
                self.jump_line = line;
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
                    self.request(Pending::Replace(Document::default(), None), &ctx);
                }
                if ui.button("Öffnen …").clicked() {
                    match choose_file(owner, base, false)
                        .and_then(|p| p.map(Document::open).transpose())
                    {
                        Ok(Some(document)) => self.request(Pending::Replace(document, None), &ctx),
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
            let mut jump_forward = false;
            let mut jump_backward = false;
            if self.search {
                if ui.is_enabled() && ctx.input_mut(|i| i.consume_key(Modifiers::NONE, Key::Escape))
                {
                    self.search = false;
                } else {
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
                        let enter_shift = response.lost_focus()
                            && ui.input(|i| i.key_pressed(Key::Enter) && i.modifiers.shift);
                        let enter_plain = response.lost_focus()
                            && ui.input(|i| i.key_pressed(Key::Enter) && !i.modifiers.shift);
                        jump_backward = ui
                            .button("Zurück")
                            .on_hover_text("Umschalt+Enter")
                            .clicked()
                            || enter_shift;
                        jump_forward = ui
                            .button("Nächster Treffer")
                            .on_hover_text("Enter")
                            .clicked()
                            || enter_plain;
                        if ui.button("Schließen").on_hover_text("Esc").clicked() {
                            self.search = false;
                        }
                    });
                }
            }
            let range = if let Some(line) = self.jump_line.take() {
                let total_lines = self.document.text.bytes().filter(|b| *b == b'\n').count() + 1;
                let target = line.clamp(1, total_lines);
                if line > 1 {
                    self.message = format!("Zeile {target} / {total_lines}");
                }
                Some(line_char_range(&self.document.text, target))
            } else if (jump_forward || jump_backward) && !self.query.is_empty() {
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
                    let len = matches.len();
                    let index = if jump_backward {
                        (self.next_match + len.saturating_sub(2)) % len
                    } else {
                        self.next_match % len
                    };
                    self.next_match = (index + 1) % len;
                    self.message = format!("Treffer {} / {}", index + 1, len);
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
        "Text und Code\0*.txt;*.ps1;*.sh;*.json;*.toml;*.md;*.bash;*.rs;*.py;*.js;*.ts;*.yaml;*.yml;*.cmd;*.bat\0Alle Dateien\0*.*\0\0"
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
        "ps1"
            | "psm1"
            | "sh"
            | "bash"
            | "json"
            | "toml"
            | "md"
            | "markdown"
            | "rs"
            | "py"
            | "js"
            | "ts"
            | "tsx"
            | "jsx"
            | "go"
            | "c"
            | "cpp"
            | "h"
            | "yaml"
            | "yml"
    );
    let slash_comments = matches!(
        extension,
        "rs" | "js" | "ts" | "tsx" | "jsx" | "go" | "c" | "cpp" | "h"
    );
    let hash_comments =
        supported && !matches!(extension, "json" | "js" | "ts" | "tsx" | "jsx" | "go");
    let mut remaining = text;
    while !remaining.is_empty() {
        let first = remaining.chars().next().unwrap();
        let mut color = Color32::from_rgb(215, 225, 235);
        let mut length = first.len_utf8();
        if first.is_whitespace() {
            length = remaining
                .find(|c: char| !c.is_whitespace())
                .unwrap_or(remaining.len());
        } else if (hash_comments && first == '#') || (slash_comments && remaining.starts_with("//"))
        {
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
                        "true" | "false" | "null" | "none" | "$true" | "$false" | "$null"
                    ))
            {
                color = Color32::from_rgb(190, 150, 230);
            } else if supported
                && (first == '$'
                    || matches!(
                        token.to_ascii_lowercase().as_str(),
                        "if" | "else"
                            | "elseif"
                            | "elif"
                            | "then"
                            | "fi"
                            | "for"
                            | "foreach"
                            | "while"
                            | "loop"
                            | "do"
                            | "done"
                            | "function"
                            | "fn"
                            | "def"
                            | "class"
                            | "struct"
                            | "enum"
                            | "impl"
                            | "trait"
                            | "pub"
                            | "let"
                            | "mut"
                            | "const"
                            | "use"
                            | "import"
                            | "from"
                            | "match"
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
        let doc = Document {
            text: "🦀".repeat(LIMIT),
            ..Document::default()
        };
        assert!(doc.bytes().is_err());
    }

    #[test]
    fn new_file_and_save_as_preserve_original_document() {
        let fixture = Fixture::new();
        let mut doc = Document {
            text: "new\n".into(),
            ..Document::default()
        };
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
        editor.request(Pending::Replace(Document::default(), None), &ctx);
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
        let text =
            "# Grüße 🦀\n// Kommentar ü\nfn main() { if ($true) { echo \"a\\\"b\" } }\n[section]\nkey = 'value'\n";
        for extension in ["ps1", "sh", "json", "toml", "md", "rs", "py", "ts", "txt"] {
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

    #[test]
    fn clicked_text_paths_and_stacktraces_resolve_with_line_numbers_and_selection_opens() {
        let fixture = Fixture::new();
        let file = fixture.path();
        fs::write(&file, "Erste Zeile\nZweite Zeile ü\nDritte Zeile\n").unwrap();
        let relative_line = "  --> Skript O'Brien ü.ps1:2:5: Fehler";
        assert_eq!(
            path_at(
                relative_line,
                relative_line.find("Brien").unwrap(),
                &fixture.0
            ),
            Some((file.clone(), Some(2)))
        );
        let quoted = format!("\"{}\":3", file.display());
        assert_eq!(
            path_at(&quoted, quoted.find("Brien").unwrap(), &fixture.0),
            Some((file.clone(), Some(3)))
        );
        assert!(path_at(r"\\198.51.100.1\share\a.txt:2", 4, &fixture.0).is_none());

        let ctx = egui::Context::default();
        let mut editor = Editor::default();
        editor.open_file(file, Some(2), &ctx).unwrap();
        assert!(editor.active);
        assert_eq!(editor.jump_line, Some(2));
        assert_eq!(&editor.document.text[12..27], "Zweite Zeile ü");
        assert_eq!(line_char_range(&editor.document.text, 2), 12..26);

        editor
            .open_selection("Zeile 1\r\nZeile 2".into(), &ctx)
            .unwrap();
        assert!(editor.document.dirty());
        assert!(editor.document.path.is_none());
        assert_eq!(editor.document.text, "Zeile 1\nZeile 2");
    }
}
