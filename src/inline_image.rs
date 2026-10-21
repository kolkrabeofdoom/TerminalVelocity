use eframe::egui;
use std::{path::PathBuf, sync::mpsc};

pub enum Request {
    Choose(u64),
    Drop(u64, Vec<egui::DroppedFile>),
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
