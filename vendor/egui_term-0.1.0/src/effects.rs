use egui::{Color32, Painter, Pos2, Rect, Stroke, Vec2};

#[derive(Clone, Copy, Debug)]
pub struct BackgroundImage {
    pub texture: egui::TextureId,
    pub size: Vec2,
    pub cover: bool,
    pub darkness: f32,
}
impl BackgroundImage {
    pub fn geometry(self, bounds: Rect) -> (Rect, Rect) {
        let mut uv = Rect::from_min_max(Pos2::ZERO, Pos2::new(1.0, 1.0));
        let source = self.size.x / self.size.y.max(1.0);
        let target = bounds.width() / bounds.height().max(1.0);
        if self.cover {
            if source > target {
                let half = target / source * 0.5;
                uv.min.x = 0.5 - half;
                uv.max.x = 0.5 + half;
            } else {
                let half = source / target * 0.5;
                uv.min.y = 0.5 - half;
                uv.max.y = 0.5 + half;
            }
            (bounds, uv)
        } else {
            let scale =
                (bounds.width() / self.size.x.max(1.0)).min(bounds.height() / self.size.y.max(1.0));
            (
                Rect::from_center_size(bounds.center(), self.size * scale),
                uv,
            )
        }
    }
    pub fn paint(self, painter: &Painter, bounds: Rect) {
        if !bounds.is_positive() {
            return;
        }
        let (rect, uv) = self.geometry(bounds);
        let painter = painter.with_clip_rect(bounds.intersect(painter.clip_rect()));
        painter.image(self.texture, rect, uv, Color32::WHITE);
        painter.rect_filled(
            rect,
            0.0,
            Color32::from_black_alpha((self.darkness.clamp(0.0, 1.0) * 255.0).round() as u8),
        );
    }
}

/// Optional decoration only: never modifies terminal cells or input handling.
#[derive(Clone, Copy, Debug)]
pub struct VisualEffects {
    /// 0 = off, 1 = Aurora, 2 = Phosphor, 3 = Deep Space.
    pub preset: u8,
    pub intensity: f32,
    pub animations: bool,
    pub cursor_trail: bool,
    pub amber: bool,
    pub text_afterglow: bool,
    pub afterglow_strength: f32,
    pub afterglow_duration: f32,
}

impl Default for VisualEffects {
    fn default() -> Self {
        Self {
            preset: 0,
            intensity: 0.4,
            animations: false,
            cursor_trail: false,
            amber: false,
            text_afterglow: false,
            afterglow_strength: 0.5,
            afterglow_duration: 0.6,
        }
    }
}

impl VisualEffects {
    pub fn strength(self) -> f32 {
        if self.preset == 0 || !self.intensity.is_finite() {
            0.0
        } else {
            self.intensity.clamp(0.0, 1.0)
        }
    }
    pub fn accent(self) -> Color32 {
        match self.preset {
            2 if self.amber => Color32::from_rgb(255, 190, 85),
            2 => Color32::from_rgb(115, 245, 155),
            3 => Color32::from_rgb(150, 170, 255),
            _ => Color32::from_rgb(85, 225, 210),
        }
    }
    pub fn animated(self) -> bool {
        self.strength() > 0.0 && self.animations
    }
    pub fn paint_background(self, painter: &Painter, rect: Rect, time: f64) {
        let strength = self.strength();
        if strength == 0.0 || !rect.is_positive() {
            return;
        }
        let t = if self.animations {
            (time % 3600.0) as f32
        } else {
            0.0
        };
        let painter = painter.with_clip_rect(rect.intersect(painter.clip_rect()));
        match self.preset {
            1 => {
                // Bounded mesh, no framebuffer copies or per-cell effects.
                for band in 0..3 {
                    let mut mesh = egui::Mesh::default();
                    for step in 0..=48 {
                        let u = step as f32 / 48.0;
                        let x = rect.left() + rect.width() * u;
                        let y = rect.top()
                            + rect.height()
                                * (0.22
                                    + band as f32 * 0.25
                                    + 0.12 * (u * 6.0 + t * 0.16 + band as f32).sin());
                        let color = if band == 1 {
                            Color32::from_rgba_unmultiplied(140, 85, 230, (32.0 * strength) as u8)
                        } else {
                            Color32::from_rgba_unmultiplied(45, 210, 185, (30.0 * strength) as u8)
                        };
                        for (offset, color) in [
                            (-rect.height() * 0.18, Color32::TRANSPARENT),
                            (0.0, color),
                            (rect.height() * 0.18, Color32::TRANSPARENT),
                        ] {
                            mesh.colored_vertex(Pos2::new(x, y + offset), color);
                        }
                        if step > 0 {
                            let a = (step - 1) * 3;
                            for row in 0..2 {
                                mesh.add_triangle(a + row, a + row + 3, a + row + 1);
                                mesh.add_triangle(a + row + 1, a + row + 3, a + row + 4);
                            }
                        }
                    }
                    painter.add(egui::Shape::mesh(mesh));
                }
            }
            2 => {
                // Scanlines stay behind glyphs and selection/search backgrounds.
                let color = self.accent().gamma_multiply(0.07 * strength);
                let count = (rect.height() / 4.0).ceil().min(600.0) as usize;
                for i in 0..count {
                    let y = rect.top() + i as f32 * 4.0;
                    painter.line_segment(
                        [Pos2::new(rect.left(), y), Pos2::new(rect.right(), y)],
                        Stroke::new(0.7, color),
                    );
                }
                painter.rect_filled(rect, 0.0, self.accent().gamma_multiply(0.025 * strength));
            }
            3 => {
                for i in 0..55 {
                    let u = ((i * 137 + 29) % 997) as f32 / 997.0;
                    let v = (((i * 263 + 71) % 991) as f32 / 991.0
                        + t * (0.001 + (i % 3) as f32 * 0.0003))
                        .fract();
                    let center = rect.min + Vec2::new(u * rect.width(), v * rect.height());
                    let radius = if i % 7 == 0 { 1.5 } else { 0.7 };
                    painter.circle_filled(
                        center,
                        radius * 3.0,
                        self.accent().gamma_multiply(0.04 * strength),
                    );
                    painter.circle_filled(
                        center,
                        radius,
                        self.accent().gamma_multiply(0.38 * strength),
                    );
                }
            }
            _ => {}
        }
    }
}

/// Only current glyphs are retained. Erasure immediately removes their light.
#[derive(Clone, Default)]
pub(crate) struct TextAfterglow {
    previous: std::collections::BTreeMap<(i32, usize), char>,
    lit: std::collections::BTreeMap<(i32, usize), f64>,
    geometry: Option<(Rect, egui::FontId, usize, usize, bool)>,
    last_time: Option<f64>,
}

#[derive(Clone)]
pub(crate) struct GlowGlyph {
    pub key: (i32, usize),
    pub character: char,
    pub position: Pos2,
}

impl TextAfterglow {
    pub(crate) fn update(
        &mut self,
        glyphs: &[GlowGlyph],
        geometry: (Rect, egui::FontId, usize, usize, bool),
        time: f64,
        enabled: bool,
        duration: f32,
    ) {
        if !enabled || glyphs.len() > 8192 || !time.is_finite() {
            *self = Self::default();
            return;
        }
        let stable = self.geometry.as_ref() == Some(&geometry)
            && self.last_time.is_some_and(|last| time >= last);
        self.geometry = Some(geometry);
        self.last_time = Some(time);
        let current: std::collections::BTreeMap<_, _> = glyphs
            .iter()
            .map(|glyph| (glyph.key, glyph.character))
            .collect();
        if !stable {
            self.lit.clear();
        } else {
            self.lit
                .retain(|key, born| current.contains_key(key) && time - *born < duration as f64);
            for (key, character) in &current {
                if self.previous.get(key) != Some(character) {
                    // Bounded work even for full-screen output bursts.
                    if self.lit.len() < 512 || self.lit.contains_key(key) {
                        self.lit.insert(*key, time);
                    }
                }
            }
        }
        self.previous = current;
    }

    pub(crate) fn paint(
        &self,
        painter: &Painter,
        glyphs: &[GlowGlyph],
        time: f64,
        effects: VisualEffects,
    ) {
        let Some((bounds, font, ..)) = &self.geometry else {
            return;
        };
        let painter = painter.with_clip_rect(bounds.intersect(painter.clip_rect()));
        for glyph in glyphs {
            if let Some(born) = self.lit.get(&glyph.key) {
                let fade = (1.0 - ((time - born) / effects.afterglow_duration as f64) as f32)
                    .clamp(0.0, 1.0)
                    .powi(2);
                let color = effects
                    .accent()
                    .gamma_multiply(0.22 * effects.strength() * effects.afterglow_strength * fade);
                for offset in [
                    Vec2::new(-1.0, 0.0),
                    Vec2::new(1.0, 0.0),
                    Vec2::new(0.0, -1.0),
                    Vec2::new(0.0, 1.0),
                ] {
                    painter.text(
                        glyph.position + offset,
                        egui::Align2::CENTER_TOP,
                        glyph.character,
                        font.clone(),
                        color,
                    );
                }
            }
        }
        if !self.lit.is_empty() {
            painter
                .ctx()
                .request_repaint_after(std::time::Duration::from_millis(50));
        }
    }
}

#[derive(Clone, Default)]
pub(crate) struct CursorTrail {
    last: Option<Rect>,
    layout: Option<Rect>,
    scroll: usize,
    samples: Vec<(Rect, f64)>,
}

impl CursorTrail {
    pub(crate) fn paint(
        &mut self,
        painter: &Painter,
        cursor: Option<Rect>,
        bounds: Rect,
        scroll: usize,
        time: f64,
        effects: VisualEffects,
        active: bool,
    ) {
        let stable = self.layout == Some(bounds) && self.scroll == scroll;
        self.layout = Some(bounds);
        self.scroll = scroll;
        if !active || cursor.is_none() || !stable || !effects.animated() || !effects.cursor_trail {
            self.samples.clear();
            self.last = cursor;
        } else if self.last != cursor {
            if let Some(previous) = self.last {
                self.samples.push((previous, time));
                if self.samples.len() > 12 {
                    self.samples.remove(0);
                }
            }
            self.last = cursor;
        }
        self.samples
            .retain(|(_, born)| time >= *born && time - *born < 0.32);
        let painter = painter.with_clip_rect(bounds.intersect(painter.clip_rect()));
        for (rect, born) in &self.samples {
            let fade = (1.0 - ((time - born) / 0.32) as f32).powi(2);
            painter.rect_filled(
                *rect,
                2.0,
                effects
                    .accent()
                    .gamma_multiply(0.30 * effects.strength() * fade),
            );
            if let Some(current) = cursor {
                // Short trails only; no neon slash across an entire split pane.
                if current.center().distance(rect.center()) < 160.0 {
                    painter.line_segment(
                        [rect.center(), current.center()],
                        Stroke::new(
                            2.0,
                            effects
                                .accent()
                                .gamma_multiply(0.15 * effects.strength() * fade),
                        ),
                    );
                }
            }
        }
        if active && effects.strength() > 0.0 {
            if let Some(cursor) = cursor {
                for spread in [6.0, 3.0, 1.5] {
                    painter.rect_stroke(
                        cursor.expand(spread),
                        3.0,
                        Stroke::new(
                            1.5,
                            effects.accent().gamma_multiply(0.10 * effects.strength()),
                        ),
                        egui::StrokeKind::Inside,
                    );
                }
            }
        }
        if !self.samples.is_empty() {
            painter
                .ctx()
                .request_repaint_after(std::time::Duration::from_millis(33));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn text_afterglow_tracks_changes_erasure_expiry_and_resets() {
        let mut glow = TextAfterglow::default();
        let geometry = (
            Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0)),
            egui::FontId::monospace(14.0),
            0,
            0,
            false,
        );
        let glyph = |character| GlowGlyph {
            key: (0, 0),
            character,
            position: Pos2::new(8.0, 8.0),
        };
        glow.update(&[glyph('a')], geometry.clone(), 0.0, true, 0.6);
        assert!(glow.lit.is_empty()); // Opening an existing viewport does not flash.
        glow.update(&[glyph('b')], geometry.clone(), 10.0, true, 0.6);
        assert_eq!(glow.lit.len(), 1); // Also after an idle interval.
        glow.update(&[glyph('b')], geometry.clone(), 10.3, true, 0.6);
        assert_eq!(glow.lit[&(0, 0)], 10.0); // Stable text does not renew its light.
        glow.update(&[glyph('b')], geometry.clone(), 10.7, true, 0.6);
        assert!(glow.lit.is_empty());
        glow.update(&[glyph('c')], geometry.clone(), 10.8, true, 0.6);
        glow.update(&[], geometry.clone(), 10.9, true, 0.6);
        assert!(glow.lit.is_empty());
        for changed in [
            (geometry.0.shrink(1.0), geometry.1.clone(), 0, 0, false),
            (geometry.0, egui::FontId::monospace(16.0), 0, 0, false),
            (geometry.0, geometry.1.clone(), 1, 0, false),
            (geometry.0, geometry.1.clone(), 0, 1, false),
            (geometry.0, geometry.1.clone(), 0, 0, true),
        ] {
            glow.update(&[glyph('a')], geometry.clone(), 11.0, true, 0.6);
            glow.update(&[glyph('b')], geometry.clone(), 11.1, true, 0.6);
            glow.update(&[glyph('c')], changed, 11.2, true, 0.6);
            assert!(glow.lit.is_empty());
        }
        glow.update(&[glyph('d')], geometry.clone(), 12.0, false, 0.6);
        assert!(glow.previous.is_empty());
        assert!(glow.geometry.is_none());
        let many: Vec<_> = (0..1000)
            .map(|i| GlowGlyph {
                key: (0, i),
                ..glyph('x')
            })
            .collect();
        glow.update(&[], geometry.clone(), 13.0, true, 0.6);
        glow.update(&many, geometry.clone(), 13.1, true, 0.6);
        assert_eq!(glow.lit.len(), 512);
        let oversized: Vec<_> = (0..8193)
            .map(|i| GlowGlyph {
                key: (0, i),
                ..glyph('x')
            })
            .collect();
        glow.update(&oversized, geometry, 13.2, true, 0.6);
        assert!(glow.previous.is_empty());
    }

    #[test]
    fn text_afterglow_paints_clipped_and_stops_after_expiry() {
        let ctx = egui::Context::default();
        let bounds = Rect::from_min_size(Pos2::ZERO, Vec2::splat(100.0));
        let font = egui::FontId::monospace(14.0);
        let geometry = (bounds, font, 0, 0, false);
        let glyphs = [GlowGlyph {
            key: (0, 0),
            character: 'X',
            position: Pos2::new(10.0, 10.0),
        }];
        let mut glow = TextAfterglow::default();
        glow.update(&[], geometry.clone(), 0.0, true, 0.6);
        glow.update(&glyphs, geometry.clone(), 0.1, true, 0.6);
        let effects = VisualEffects {
            preset: 2,
            ..VisualEffects::default()
        };
        let output = ctx.run(egui::RawInput::default(), |ctx| {
            glow.paint(
                &ctx.layer_painter(egui::LayerId::background()),
                &glyphs,
                0.1,
                effects,
            );
        });
        assert_eq!(output.shapes.len(), 4);
        assert!(output.shapes.iter().all(|shape| shape.clip_rect == bounds));
        glow.update(&glyphs, geometry, 1.0, true, 0.6);
        let output = ctx.run(egui::RawInput::default(), |ctx| {
            glow.paint(
                &ctx.layer_painter(egui::LayerId::background()),
                &glyphs,
                1.0,
                effects,
            );
        });
        assert!(output.shapes.is_empty());
    }
    #[test]
    fn trail_expires_is_bounded_and_resets_on_scroll_focus_and_layout() {
        let ctx = egui::Context::default();
        let mut trail = CursorTrail::default();
        let bounds = Rect::from_min_size(Pos2::ZERO, Vec2::new(800.0, 600.0));
        let effects = VisualEffects {
            preset: 1,
            animations: true,
            cursor_trail: true,
            ..VisualEffects::default()
        };
        let cursor = |x| {
            Some(Rect::from_min_size(
                Pos2::new(x, 20.0),
                Vec2::new(9.0, 18.0),
            ))
        };
        let _ = ctx.run(egui::RawInput::default(), |ctx| {
            let painter = ctx.layer_painter(egui::LayerId::background());
            trail.paint(&painter, cursor(10.0), bounds, 0, 0.0, effects, true);
            for i in 1..30 {
                trail.paint(
                    &painter,
                    cursor(10.0 + i as f32),
                    bounds,
                    0,
                    i as f64 * 0.001,
                    effects,
                    true,
                );
            }
            assert_eq!(trail.samples.len(), 12);
            trail.paint(&painter, cursor(39.0), bounds, 0, 1.0, effects, true);
            assert!(trail.samples.is_empty());
            trail.paint(&painter, cursor(40.0), bounds, 0, 1.01, effects, true);
            assert_eq!(trail.samples.len(), 1);
            trail.paint(&painter, cursor(41.0), bounds, 1, 1.02, effects, true);
            assert!(trail.samples.is_empty());
            trail.paint(&painter, cursor(42.0), bounds, 1, 1.03, effects, true);
            trail.paint(&painter, cursor(43.0), bounds, 1, 1.04, effects, false);
            assert!(trail.samples.is_empty());
            trail.paint(&painter, cursor(44.0), bounds, 1, 1.05, effects, true);
            trail.paint(
                &painter,
                cursor(45.0),
                bounds.shrink(10.0),
                1,
                1.06,
                effects,
                true,
            );
            assert!(trail.samples.is_empty());
            trail.paint(&painter, None, bounds, 1, 1.07, effects, true);
            assert!(trail.last.is_none());
        });
    }
}
