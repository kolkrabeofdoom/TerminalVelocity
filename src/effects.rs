use eframe::egui;
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Preset {
    Off,
    #[default]
    Aurora,
    Phosphor,
    DeepSpace,
}

impl Preset {
    pub fn label(self) -> &'static str {
        match self {
            Self::Off => "Aus",
            Self::Aurora => "Aurora",
            Self::Phosphor => "Phosphor",
            Self::DeepSpace => "Deep Space",
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub preset: Preset,
    pub intensity: f32,
    pub animations: bool,
    pub cursor_trail: bool,
    pub amber: bool,
    pub text_afterglow: bool,
    pub afterglow_strength: f32,
    pub afterglow_duration: f32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            preset: Preset::Aurora,
            intensity: 0.45,
            animations: true,
            cursor_trail: true,
            amber: false,
            text_afterglow: true,
            afterglow_strength: 0.5,
            afterglow_duration: 0.6,
        }
    }
}

impl Settings {
    pub fn validate(&mut self) {
        self.afterglow_strength = if self.afterglow_strength.is_finite() {
            self.afterglow_strength.clamp(0.0, 1.0)
        } else {
            0.5
        };
        self.afterglow_duration = if self.afterglow_duration.is_finite() {
            self.afterglow_duration.clamp(0.1, 1.5)
        } else {
            0.6
        };
        self.intensity = if self.intensity.is_finite() {
            self.intensity.clamp(0.0, 1.0)
        } else {
            0.45
        };
    }
    pub fn visual(&self) -> egui_term::VisualEffects {
        egui_term::VisualEffects {
            preset: match self.preset {
                Preset::Off => 0,
                Preset::Aurora => 1,
                Preset::Phosphor => 2,
                Preset::DeepSpace => 3,
            },
            intensity: self.intensity,
            animations: self.animations,
            cursor_trail: self.cursor_trail,
            amber: self.amber,
            text_afterglow: self.text_afterglow,
            afterglow_strength: self.afterglow_strength,
            afterglow_duration: self.afterglow_duration,
        }
    }
    pub fn theme(&self) -> egui_term::TerminalTheme {
        let mut palette = egui_term::ColorPalette::default();
        if self.preset == Preset::Phosphor && self.visual().strength() > 0.0 {
            palette.foreground = if self.amber { "#ffce8a" } else { "#b9f8cb" }.into();
            palette.dim_foreground = if self.amber { "#ad8d62" } else { "#80ae8e" }.into();
        }
        egui_term::TerminalTheme::new(Box::new(palette))
    }
    pub fn ui(&mut self, ui: &mut egui::Ui) {
        ui.label("Grafikeffekte");
        ui.horizontal_wrapped(|ui| {
            for preset in [
                Preset::Off,
                Preset::Aurora,
                Preset::Phosphor,
                Preset::DeepSpace,
            ] {
                ui.selectable_value(&mut self.preset, preset, preset.label());
            }
        });
        ui.add_enabled_ui(self.preset != Preset::Off, |ui| {
            ui.add(egui::Slider::new(&mut self.intensity, 0.0..=1.0).text("Effektstärke"));
            ui.checkbox(&mut self.animations, "Animationen");
            ui.add_enabled(
                self.animations,
                egui::Checkbox::new(&mut self.cursor_trail, "Cursor-Schweif"),
            );
            if self.preset == Preset::Phosphor {
                ui.checkbox(&mut self.amber, "Bernstein statt Grün");
                ui.add_enabled_ui(self.animations, |ui| {
                    ui.checkbox(&mut self.text_afterglow, "Text-Nachleuchten");
                    ui.add_enabled_ui(self.text_afterglow, |ui| {
                        ui.add(egui::Slider::new(&mut self.afterglow_strength, 0.0..=1.0).text("Leuchtstärke"));
                        ui.add(egui::Slider::new(&mut self.afterglow_duration, 0.1..=1.5).suffix(" s").text("Nachleuchtdauer"));
                    });
                });
                ui.small("Neue und geänderte Zeichen leuchten kurz auf. Gelöschte Zeichen hinterlassen keine Textspuren.");
            }
        });
        ui.small("Effekte liegen hinter dem Text. Animationen laufen nur im aktiven Bereich bei fokussiertem Fenster, mit höchstens 20 Hintergrundbildern pro Sekunde. Ohne Animationen bleibt der Hintergrund statisch.");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn afterglow_settings_migrate_validate_and_roundtrip() {
        let old: Settings = serde_json::from_str(r#"{"preset":"Phosphor"}"#).unwrap();
        assert!(old.text_afterglow);
        assert_eq!(old.afterglow_duration, 0.6);
        let mut settings = Settings {
            text_afterglow: false,
            afterglow_strength: f32::NAN,
            afterglow_duration: f32::INFINITY,
            ..old
        };
        settings.validate();
        assert_eq!(settings.afterglow_strength, 0.5);
        assert_eq!(settings.afterglow_duration, 0.6);
        settings.afterglow_strength = 2.0;
        settings.afterglow_duration = -1.0;
        settings.validate();
        let restored: Settings =
            serde_json::from_str(&serde_json::to_string(&settings).unwrap()).unwrap();
        assert!(!restored.visual().text_afterglow);
        assert_eq!(restored.visual().afterglow_strength, 1.0);
        assert_eq!(restored.visual().afterglow_duration, 0.1);
    }
    #[test]
    fn persisted_settings_and_reduced_motion() {
        let mut settings = Settings {
            preset: Preset::DeepSpace,
            animations: false,
            intensity: 3.0,
            ..Settings::default()
        };
        settings.validate();
        let saved = serde_json::to_string(&settings).unwrap();
        let restored: Settings = serde_json::from_str(&saved).unwrap();
        assert_eq!(restored.preset, Preset::DeepSpace);
        assert_eq!(restored.intensity, 1.0);
        assert!(!restored.visual().animated());
        settings.preset = Preset::Off;
        assert_eq!(settings.visual().strength(), 0.0);
        settings.intensity = f32::NAN;
        settings.validate();
        assert_eq!(settings.intensity, 0.45);
    }

    fn background(
        preset: Preset,
        time: f64,
        animations: bool,
    ) -> Vec<egui::epaint::ClippedPrimitive> {
        let ctx = egui::Context::default();
        ctx.tessellation_options_mut(|o| o.prerasterized_discs = false);
        let output = ctx.run(egui::RawInput::default(), |ctx| {
            let painter = ctx.layer_painter(egui::LayerId::background());
            Settings {
                preset,
                animations,
                intensity: 0.65,
                ..Settings::default()
            }
            .visual()
            .paint_background(
                &painter,
                egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(720.0, 360.0)),
                time,
            );
        });
        ctx.tessellate(output.shapes, output.pixels_per_point)
    }

    #[test]
    fn backgrounds_are_bounded_and_reduced_motion_is_static() {
        assert!(background(Preset::Off, 0.0, true).is_empty());
        for preset in [Preset::Aurora, Preset::Phosphor, Preset::DeepSpace] {
            let first = background(preset, 0.0, false);
            assert!(!first.is_empty());
            assert_eq!(
                format!("{first:?}"),
                format!("{:?}", background(preset, 100.0, false))
            );
            let mut vertices = 0;
            for primitive in first {
                assert_eq!(
                    primitive.clip_rect,
                    egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(720.0, 360.0))
                );
                if let egui::epaint::Primitive::Mesh(mesh) = primitive.primitive {
                    assert!(mesh.is_valid());
                    vertices += mesh.vertices.len();
                    assert!(mesh
                        .vertices
                        .iter()
                        .all(|v| v.pos.x.is_finite() && v.pos.y.is_finite()));
                }
            }
            assert!(vertices < 20000);
        }
        assert_ne!(
            format!("{:?}", background(Preset::Aurora, 0.0, true)),
            format!("{:?}", background(Preset::Aurora, 10.0, true))
        );
    }

    #[test]
    #[ignore = "writes background-rendering preview meshes for visual QA"]
    fn export_background_preview() {
        let previews: Vec<_> = [Preset::Aurora, Preset::Phosphor, Preset::DeepSpace].into_iter().map(|preset| {
            let meshes: Vec<_> = background(preset, 12.0, true).into_iter().filter_map(|p| {
                if let egui::epaint::Primitive::Mesh(mesh) = p.primitive {
                    Some(serde_json::json!({"vertices":mesh.vertices.iter().map(|v| [v.pos.x, v.pos.y, v.color.r() as f32,v.color.g() as f32,v.color.b() as f32,v.color.a() as f32]).collect::<Vec<_>>(),"indices":mesh.indices}))
                } else { None }
            }).collect();
            serde_json::json!({"preset":preset.label(),"meshes":meshes})
        }).collect();
        let directory = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("work");
        std::fs::create_dir_all(&directory).unwrap();
        std::fs::write(
            directory.join("effects-preview.json"),
            serde_json::to_vec(&previews).unwrap(),
        )
        .unwrap();
    }
}
