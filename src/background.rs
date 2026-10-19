use eframe::egui;
use serde::{Deserialize, Serialize};
use std::{
    path::{Path, PathBuf},
    sync::mpsc,
};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub enabled: bool,
    pub path: String,
    pub darkness: f32,
    pub cover: bool,
}
impl Default for Settings {
    fn default() -> Self {
        Self {
            enabled: false,
            path: String::new(),
            darkness: 0.7,
            cover: true,
        }
    }
}
impl Settings {
    pub fn validate(&mut self) {
        self.darkness = if self.darkness.is_finite() {
            self.darkness.clamp(0.0, 1.0)
        } else {
            0.7
        };
    }
}

#[derive(Default)]
pub struct Cache {
    requested: Option<String>,
    pending: Option<(String, mpsc::Receiver<Result<egui::ColorImage, String>>)>,
    texture: Option<egui::TextureHandle>,
    error: Option<String>,
    draft: Option<String>,
}

impl Cache {
    pub fn update(&mut self, ctx: &egui::Context, settings: &Settings, base: &Path) {
        let desired =
            (settings.enabled && !settings.path.is_empty()).then(|| settings.path.clone());
        if self.requested != desired {
            self.requested = desired;
            self.texture = None;
            self.error = None;
        }
        if let Some((path, rx)) = &self.pending {
            let result = match rx.try_recv() {
                Ok(result) => Some(result),
                Err(mpsc::TryRecvError::Disconnected) => {
                    Some(Err("Bildladeprozess wurde beendet.".into()))
                }
                Err(mpsc::TryRecvError::Empty) => None,
            };
            if let Some(result) = result {
                if self.requested.as_ref() == Some(path) {
                    match result {
                        Ok(image) => {
                            self.texture = Some(ctx.load_texture(
                                "terminal-background",
                                image,
                                egui::TextureOptions::LINEAR,
                            ))
                        }
                        Err(error) => self.error = Some(error),
                    }
                }
                self.pending = None;
            }
        }
        if self.pending.is_none() && self.texture.is_none() && self.error.is_none() {
            if let Some(requested) = self.requested.clone() {
                let path = PathBuf::from(&requested);
                let path = if path.is_absolute() {
                    path
                } else {
                    base.join(path)
                };
                let (tx, rx) = mpsc::channel();
                let context = ctx.clone();
                std::thread::spawn(move || {
                    let _ = tx.send(load_image(&path));
                    context.request_repaint();
                });
                self.pending = Some((requested, rx));
            }
        }
        if self.pending.is_some() {
            ctx.request_repaint_after(std::time::Duration::from_millis(100));
        }
    }

    pub fn image(&self, settings: &Settings) -> Option<egui_term::BackgroundImage> {
        if !settings.enabled {
            return None;
        }
        self.texture.as_ref().map(|t| egui_term::BackgroundImage {
            texture: t.id(),
            size: t.size_vec2(),
            cover: settings.cover,
            darkness: settings.darkness,
        })
    }

    pub fn ui(&mut self, ui: &mut egui::Ui, settings: &mut Settings, owner: *mut std::ffi::c_void) {
        ui.label("Eigenes Hintergrundbild");
        ui.checkbox(&mut settings.enabled, "Hintergrundbild anzeigen");
        let draft = self.draft.get_or_insert_with(|| settings.path.clone());
        ui.add(
            egui::TextEdit::singleline(draft)
                .desired_width(450.0)
                .hint_text("Bildpfad – PNG, JPEG, WebP oder BMP"),
        );
        let mut chosen = None;
        ui.horizontal(|ui| {
            if ui.button("Datei auswählen …").clicked() {
                match choose_file(owner) {
                    Ok(path) => chosen = path,
                    Err(e) => self.error = Some(e),
                }
            }
            if ui.button("Laden / neu laden").clicked() {
                chosen = Some(PathBuf::from(draft.trim().trim_matches('"')));
            }
            if ui.button("Entfernen").clicked() {
                settings.enabled = false;
                settings.path.clear();
                draft.clear();
                self.texture = None;
                self.requested = None;
                self.error = None;
            }
        });
        if let Some(path) = chosen {
            settings.path = path.display().to_string();
            *draft = settings.path.clone();
            settings.enabled = !settings.path.is_empty();
            self.requested = None;
            self.texture = None;
            self.error = None;
        }
        ui.add(egui::Slider::new(&mut settings.darkness, 0.0..=1.0).text("Abdunklung"));
        ui.horizontal(|ui| {
            ui.selectable_value(&mut settings.cover, true, "Ausfüllen");
            ui.selectable_value(&mut settings.cover, false, "Einpassen");
        });
        if self.pending.is_some() {
            ui.label("Bild wird geladen …");
        }
        if let Some(error) = &self.error {
            ui.colored_label(egui::Color32::LIGHT_RED, error);
        }
        if settings.enabled && settings.path.is_empty() {
            ui.weak("Bitte zuerst ein Bild auswählen.");
        }
        ui.small("Gespeichert wird der Dateipfad, keine Kopie des Bildes. Relative Pfade gelten zum Programmordner. Die Grafikeffekt-Presets lassen sich unabhängig ein- und ausschalten.");
    }
}

fn load_image(path: &Path) -> Result<egui::ColorImage, String> {
    use std::io::Read;
    let file = std::fs::File::open(path).map_err(|e| format!("Bild nicht geöffnet: {e}"))?;
    let mut bytes = Vec::new();
    file.take(32 * 1024 * 1024 + 1)
        .read_to_end(&mut bytes)
        .map_err(|e| e.to_string())?;
    if bytes.len() > 32 * 1024 * 1024 {
        return Err("Bilddatei ist größer als 32 MiB.".into());
    }
    decode_image(&bytes)
}

fn decode_image(bytes: &[u8]) -> Result<egui::ColorImage, String> {
    let format = image::guess_format(bytes).map_err(|_| "Kein unterstütztes Bild erkannt.")?;
    if !matches!(
        format,
        image::ImageFormat::Png
            | image::ImageFormat::Jpeg
            | image::ImageFormat::WebP
            | image::ImageFormat::Bmp
    ) {
        return Err("Bitte PNG, JPEG, WebP oder BMP verwenden.".into());
    }
    let mut reader = image::ImageReader::with_format(std::io::Cursor::new(bytes), format);
    let dimensions = image::ImageReader::with_format(std::io::Cursor::new(bytes), format)
        .into_dimensions()
        .map_err(|e| e.to_string())?;
    if dimensions.0 == 0
        || dimensions.1 == 0
        || dimensions.0 > 16384
        || dimensions.1 > 16384
        || dimensions.0 as u64 * dimensions.1 as u64 > 16_000_000
    {
        return Err("Das Bild darf höchstens 16 Megapixel und 16384 Pixel pro Seite haben.".into());
    }
    let mut limits = image::Limits::default();
    limits.max_alloc = Some(128 * 1024 * 1024);
    reader.limits(limits);
    let mut image = reader
        .decode()
        .map_err(|e| format!("Bild kann nicht gelesen werden: {e}"))?;
    if image.width() > 2048 || image.height() > 2048 {
        image = image.resize(2048, 2048, image::imageops::FilterType::Triangle);
    }
    let rgba = image.into_rgba8();
    Ok(egui::ColorImage::from_rgba_unmultiplied(
        [rgba.width() as usize, rgba.height() as usize],
        rgba.as_raw(),
    ))
}

fn choose_file(owner: *mut std::ffi::c_void) -> Result<Option<PathBuf>, String> {
    use std::os::windows::ffi::OsStringExt;
    use windows_sys::Win32::UI::Controls::Dialogs::*;
    let mut buffer = vec![0u16; 32768];
    let filter: Vec<u16> = "Bilder (PNG, JPEG, WebP, BMP)\0*.png;*.jpg;*.jpeg;*.webp;*.bmp\0\0"
        .encode_utf16()
        .collect();
    let title: Vec<u16> = "Hintergrundbild auswählen\0".encode_utf16().collect();
    // The buffers outlive the modal dialog; no hooks or callbacks are installed.
    unsafe {
        let mut dialog: OPENFILENAMEW = std::mem::zeroed();
        dialog.lStructSize = std::mem::size_of::<OPENFILENAMEW>() as u32;
        dialog.hwndOwner = owner;
        dialog.lpstrFile = buffer.as_mut_ptr();
        dialog.nMaxFile = buffer.len() as u32;
        dialog.lpstrFilter = filter.as_ptr();
        dialog.lpstrTitle = title.as_ptr();
        dialog.Flags = OFN_FILEMUSTEXIST | OFN_PATHMUSTEXIST | OFN_NOCHANGEDIR | OFN_EXPLORER;
        if GetOpenFileNameW(&mut dialog) != 0 {
            let length = buffer.iter().position(|c| *c == 0).unwrap_or(buffer.len());
            Ok(Some(
                std::ffi::OsString::from_wide(&buffer[..length]).into(),
            ))
        } else {
            let code = CommDlgExtendedError();
            if code == 0 {
                Ok(None)
            } else {
                Err(format!("Dateiauswahl fehlgeschlagen: {code}"))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn decoding_preserves_alpha_and_rejects_invalid_images() {
        let rgba = image::RgbaImage::from_pixel(2, 1, image::Rgba([255, 0, 0, 128]));
        let mut png = std::io::Cursor::new(Vec::new());
        rgba.write_to(&mut png, image::ImageFormat::Png).unwrap();
        let decoded = decode_image(png.get_ref()).unwrap();
        assert_eq!(decoded.size, [2, 1]);
        assert_eq!(decoded.pixels[0].a(), 128);
        assert!(decode_image(b"not an image").is_err());
        assert!(load_image(Path::new("missing-background-fixture.png")).is_err());
        let settings: Settings = serde_json::from_str("{}").unwrap();
        assert!(!settings.enabled);
        assert_eq!(settings.darkness, 0.7);
    }

    #[test]
    fn stale_load_cannot_replace_a_newer_image_or_restart_on_error() {
        let ctx = egui::Context::default();
        let (tx, rx) = mpsc::channel();
        let mut cache = Cache {
            requested: Some("new.png".into()),
            pending: Some(("old.png".into(), rx)),
            error: Some("new image failed".into()),
            ..Cache::default()
        };
        tx.send(Ok(egui::ColorImage::new([1, 1], egui::Color32::RED)))
            .unwrap();
        let settings = Settings {
            enabled: true,
            path: "new.png".into(),
            ..Settings::default()
        };
        cache.update(&ctx, &settings, Path::new("."));
        assert!(cache.texture.is_none());
        assert!(cache.pending.is_none());
        assert_eq!(cache.error.as_deref(), Some("new image failed"));
        cache.update(&ctx, &Settings::default(), Path::new("."));
        assert!(cache.error.is_none());
        assert!(cache.requested.is_none());
    }

    #[test]
    fn image_placement_and_darkening_are_clipped_to_each_pane() {
        use egui::{Pos2, Rect, Vec2};
        let wide = egui_term::BackgroundImage {
            texture: egui::TextureId::Managed(9),
            size: Vec2::new(1600.0, 800.0),
            cover: true,
            darkness: 0.7,
        };
        let bounds = Rect::from_min_size(Pos2::new(20.0, 30.0), Vec2::new(300.0, 300.0));
        let (rect, uv) = wide.geometry(bounds);
        assert_eq!(rect, bounds);
        assert_eq!(uv.min.x, 0.25);
        assert_eq!(uv.max.x, 0.75);
        let (rect, uv) = egui_term::BackgroundImage {
            cover: false,
            ..wide
        }
        .geometry(bounds);
        assert_eq!(rect.size(), Vec2::new(300.0, 150.0));
        assert_eq!(rect.center(), bounds.center());
        assert_eq!(uv, Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0)));
        let ctx = egui::Context::default();
        let output = ctx.run(egui::RawInput::default(), |ctx| {
            wide.paint(&ctx.layer_painter(egui::LayerId::background()), bounds);
        });
        assert!(output.shapes.iter().all(|shape| shape.clip_rect == bounds));
        let tall = egui_term::BackgroundImage {
            size: Vec2::new(800.0, 1600.0),
            ..wide
        };
        assert_eq!(tall.geometry(bounds).1.min.y, 0.25);
    }

    #[test]
    fn all_supported_formats_decode_and_large_textures_are_reduced() {
        for format in [
            image::ImageFormat::Png,
            image::ImageFormat::Jpeg,
            image::ImageFormat::Bmp,
            image::ImageFormat::WebP,
        ] {
            let image = image::RgbImage::from_pixel(32, 16, image::Rgb([30, 100, 200]));
            let mut bytes = std::io::Cursor::new(Vec::new());
            image.write_to(&mut bytes, format).unwrap();
            assert_eq!(decode_image(bytes.get_ref()).unwrap().size, [32, 16]);
        }
        let image = image::RgbImage::new(3000, 1000);
        let mut bytes = std::io::Cursor::new(Vec::new());
        image.write_to(&mut bytes, image::ImageFormat::Png).unwrap();
        let decoded = decode_image(bytes.get_ref()).unwrap();
        assert_eq!(decoded.size[0], 2048);
        assert!(decoded.size[1] <= 684);
        let image = image::RgbImage::new(16385, 1);
        let mut bytes = std::io::Cursor::new(Vec::new());
        image.write_to(&mut bytes, image::ImageFormat::Png).unwrap();
        assert!(decode_image(bytes.get_ref()).is_err());
        let mut settings = Settings {
            darkness: f32::NAN,
            ..Settings::default()
        };
        settings.validate();
        assert_eq!(settings.darkness, 0.7);
    }
}
