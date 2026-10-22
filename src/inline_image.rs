use eframe::egui;
use std::{path::PathBuf, sync::mpsc};

pub enum Request {
    Choose(u64),
    Open(u64, PathBuf),
    Drop(u64, Vec<egui::DroppedFile>),
}

/// Resolve only a filename containing the clicked character, never execute text.
pub fn path_at(text: &str, offset: usize, base: &std::path::Path) -> Option<PathBuf> {
    if text.len() > 8192 || !text.is_char_boundary(offset) {
        return None;
    }
    let lower = text.to_ascii_lowercase();
    let mut fallback = None;
    let mut checked = 0;
    for extension in [".png", ".jpeg", ".jpg", ".webp", ".bmp"] {
        for (pos, _) in lower.match_indices(extension) {
            let end = pos + extension.len();
            if offset >= end
                || text[end..].chars().next().is_some_and(|c| {
                    !c.is_whitespace() && !matches!(c, '\'' | '"' | ')' | ']' | ',' | ';')
                })
            {
                continue;
            }
            let mut starts = vec![0];
            for (i, c) in text[..end].char_indices() {
                if c.is_whitespace() || matches!(c, '\'' | '"' | '(' | '[') {
                    starts.push(i + c.len_utf8());
                }
            }
            for start in starts.into_iter().filter(|start| *start <= offset) {
                let candidate = &text[start..end];
                if candidate.is_empty() || candidate.contains(['\n', '\r', '\t', '"']) {
                    continue;
                }
                let path = PathBuf::from(candidate);
                // Drive-relative paths and URI schemes are ambiguous; no expansion.
                if candidate.contains("://") || (candidate.contains(':') && !path.is_absolute()) {
                    continue;
                }
                let path = if path.is_absolute() {
                    path
                } else {
                    base.join(path)
                };
                checked += 1;
                if checked > 64 {
                    return fallback;
                }
                if path.is_file() {
                    return Some(path);
                }
                if fallback.is_none() && !candidate.contains(char::is_whitespace) {
                    fallback = Some(path);
                }
            }
        }
    }
    fallback
}

pub fn take_drop(ctx: &egui::Context, rect: egui::Rect) -> Vec<egui::DroppedFile> {
    ctx.input_mut(|i| {
        if i.pointer.latest_pos().is_some_and(|p| rect.contains(p)) {
            std::mem::take(&mut i.raw.dropped_files)
        } else {
            Vec::new()
        }
    })
}

#[derive(Default)]
pub struct Loader {
    pending: Option<(u64, usize, mpsc::Receiver<Result<image::RgbaImage, String>>)>,
}

pub fn fit(image: image::RgbaImage, available: usize) -> image::RgbaImage {
    let (w, h) = image.dimensions();
    let scale = (available.clamp(1, 4096) as f64 / w as f64)
        .min(4096.0 / h as f64)
        .min((2_097_152.0 / (w as f64 * h as f64)).sqrt())
        .min(1.0);
    if scale >= 1.0 {
        return image;
    }
    image::imageops::resize(
        &image,
        ((w as f64 * scale).floor() as u32).max(1),
        ((h as f64 * scale).floor() as u32).max(1),
        image::imageops::FilterType::Triangle,
    )
}

pub fn dropped_path(files: Vec<egui::DroppedFile>) -> Result<PathBuf, String> {
    if files.len() != 1 {
        return Err("Bitte jeweils genau eine Bilddatei ablegen.".into());
    }
    files
        .into_iter()
        .next()
        .unwrap()
        .path
        .ok_or_else(|| "Bitte eine lokale Bilddatei ablegen.".into())
}

impl Loader {
    pub fn start(
        &mut self,
        generation: u64,
        width: usize,
        path: PathBuf,
        ctx: &egui::Context,
    ) -> Result<(), String> {
        if self.pending.is_some() {
            return Err("Ein Bild wird bereits geladen. Bitte kurz warten.".into());
        }
        let (tx, rx) = mpsc::channel();
        let ctx = ctx.clone();
        std::thread::Builder::new()
            .name("inline-image".into())
            .spawn(move || {
                let result = crate::background::load_rgba(&path).map(|image| fit(image, width));
                let _ = tx.send(result);
                ctx.request_repaint();
            })
            .map_err(|e| e.to_string())?;
        self.pending = Some((generation, width, rx));
        Ok(())
    }

    pub fn poll(&mut self) -> Option<(u64, usize, Result<image::RgbaImage, String>)> {
        let (generation, width, rx) = self.pending.as_ref()?;
        let result = match rx.try_recv() {
            Ok(result) => result,
            Err(mpsc::TryRecvError::Empty) => return None,
            Err(mpsc::TryRecvError::Disconnected) => Err("Bildladeprozess wurde beendet.".into()),
        };
        let result = (*generation, *width, result);
        self.pending = None;
        Some(result)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn clicked_image_paths_resolve_spaces_unicode_and_listing_columns() {
        let base = std::env::temp_dir().join(format!("tv-paths-{}", std::process::id()));
        std::fs::create_dir_all(&base).unwrap();
        for name in [
            "first.png",
            "zweites Bild ä.jpg",
            "Bild ä.jpg",
            "O'Brien.PNG",
        ] {
            std::fs::write(base.join(name), []).unwrap();
        }
        let listing = "-a--- 21.10.2026 1234 zweites Bild ä.jpg";
        assert_eq!(
            path_at(listing, listing.find("Bild").unwrap(), &base),
            Some(base.join("zweites Bild ä.jpg"))
        );
        let line = "first.png O'Brien.PNG";
        assert_eq!(path_at(line, 2, &base), Some(base.join("first.png")));
        assert_eq!(
            path_at(line, line.find("Brien").unwrap(), &base),
            Some(base.join("O'Brien.PNG"))
        );
        let absolute = format!("\"{}\"", base.join("zweites Bild ä.jpg").display());
        assert_eq!(
            path_at(&absolute, absolute.find("Bild").unwrap(), &base),
            Some(base.join("zweites Bild ä.jpg"))
        );
        assert!(path_at("readme.txt", 1, &base).is_none());
        assert!(path_at("first.png.exe", 1, &base).is_none());
        assert!(path_at("https://example.org/a.png", 2, &base).is_none());
        for name in [
            "first.png",
            "zweites Bild ä.jpg",
            "Bild ä.jpg",
            "O'Brien.PNG",
        ] {
            std::fs::remove_file(base.join(name)).unwrap();
        }
        std::fs::remove_dir(base).unwrap();
    }
    #[test]
    fn background_loading_preserves_target_and_recovers_after_failure() {
        let ctx = egui::Context::default();
        let mut loader = Loader::default();
        let path = std::env::temp_dir().join(format!("tv-inline-image-{}.png", std::process::id()));
        image::RgbaImage::from_pixel(80, 40, image::Rgba([20, 40, 80, 128]))
            .save(&path)
            .unwrap();
        loader.start(123, 40, path.clone(), &ctx).unwrap();
        assert!(loader.start(456, 40, path.clone(), &ctx).is_err());
        let deadline = std::time::Instant::now() + std::time::Duration::from_secs(5);
        let (target, width, image) = loop {
            if let Some(result) = loader.poll() {
                break result;
            }
            assert!(std::time::Instant::now() < deadline);
            std::thread::sleep(std::time::Duration::from_millis(10));
        };
        assert_eq!((target, width), (123, 40));
        assert_eq!(image.unwrap().dimensions(), (40, 20));
        std::fs::remove_file(&path).unwrap();
        loader.start(456, 40, path, &ctx).unwrap();
        loop {
            if let Some((target, _, result)) = loader.poll() {
                assert_eq!(target, 456);
                assert!(result.is_err());
                break;
            }
            assert!(std::time::Instant::now() < deadline);
            std::thread::sleep(std::time::Duration::from_millis(10));
        }
    }
    #[test]
    fn drop_targets_pointer_pane_and_is_consumed_once() {
        let ctx = egui::Context::default();
        let left = egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(100.0, 100.0));
        let right = left.translate(egui::vec2(110.0, 0.0));
        let _ = ctx.run(
            egui::RawInput {
                events: vec![egui::Event::PointerMoved(egui::pos2(150.0, 50.0))],
                dropped_files: vec![egui::DroppedFile {
                    path: Some("test.png".into()),
                    ..Default::default()
                }],
                ..Default::default()
            },
            |ctx| {
                assert!(take_drop(ctx, left).is_empty());
                assert_eq!(take_drop(ctx, right).len(), 1);
                assert!(take_drop(ctx, right).is_empty());
            },
        );
    }
    #[test]
    fn resize_preserves_aspect_alpha_and_bounds_without_upscaling() {
        let rgba = image::RgbaImage::from_pixel(1200, 600, image::Rgba([255, 20, 0, 128]));
        let fitted = fit(rgba, 300);
        assert_eq!(fitted.dimensions(), (300, 150));
        assert_eq!(fitted.get_pixel(10, 10).0, [255, 20, 0, 128]);
        assert_eq!(fit(fitted, 2000).dimensions(), (300, 150));
        let fitted = fit(image::RgbaImage::new(2048, 2048), 4096);
        assert!(fitted.width() as u64 * fitted.height() as u64 <= 2_097_152);
    }
    #[test]
    fn drops_require_one_local_file() {
        assert!(dropped_path(vec![]).is_err());
        assert!(dropped_path(vec![egui::DroppedFile::default()]).is_err());
        let file = egui::DroppedFile {
            path: Some("Bild mit Leerzeichen.png".into()),
            ..Default::default()
        };
        assert!(dropped_path(vec![file.clone(), file.clone()]).is_err());
        assert_eq!(
            dropped_path(vec![file]).unwrap(),
            PathBuf::from("Bild mit Leerzeichen.png")
        );
    }
}
