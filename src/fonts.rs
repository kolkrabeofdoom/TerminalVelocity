//! Fonts: Consolas for the terminal plus Windows fallback fonts for characters that
//! egui's built-in fonts lack (symbols such as ✓ ⏺ ◆, emoji, Chinese/Japanese).
//! AI CLIs print many of these; without a fallback they show up as boxes.

use crate::model;

/// A font file under `%SystemRoot%\Fonts`. `index` selects the face in a `.ttc` collection.
struct SystemFont {
    name: &'static str,
    file: &'static str,
    index: u32,
}

/// Preferred terminal font, placed first in the monospace family.
const PRIMARY: SystemFont = SystemFont {
    name: "consolas",
    file: "consola.ttf",
    index: 0,
};

/// Fallbacks, tried in this order after egui's own fonts. Missing files are skipped.
const FALLBACKS: [SystemFont; 4] = [
    // Arrows, geometric shapes, dingbats, technical symbols (⏺ ✓ ◆ ⚠ …).
    SystemFont {
        name: "segoe-ui-symbol",
        file: "seguisym.ttf",
        index: 0,
    },
    // Emoji; egui draws the monochrome outlines.
    SystemFont {
        name: "segoe-ui-emoji",
        file: "seguiemj.ttf",
        index: 0,
    },
    // Chinese and most Japanese/Korean ideographs; face 0 of the collection.
    SystemFont {
        name: "microsoft-yahei",
        file: "msyh.ttc",
        index: 0,
    },
    // Korean Hangul.
    SystemFont {
        name: "malgun-gothic",
        file: "malgun.ttf",
        index: 0,
    },
];

/// Build font definitions. `load` returns a font file's bytes or `None` if it is missing;
/// injectable so the ordering can be tested without real font files.
fn definitions(load: impl Fn(&str) -> Option<Vec<u8>>) -> egui::FontDefinitions {
    let mut fonts = egui::FontDefinitions::default();
    let mut add = |font: &SystemFont| -> bool {
        let Some(bytes) = load(font.file) else {
            return false;
        };
        let mut data = egui::FontData::from_owned(bytes);
        data.index = font.index;
        fonts.font_data.insert(font.name.into(), data.into());
        true
    };
    let primary = add(&PRIMARY);
    let fallbacks: Vec<&str> = FALLBACKS
        .iter()
        .filter(|font| add(font))
        .map(|font| font.name)
        .collect();
    for family in [egui::FontFamily::Monospace, egui::FontFamily::Proportional] {
        let list = fonts.families.entry(family.clone()).or_default();
        if primary && family == egui::FontFamily::Monospace {
            list.insert(0, PRIMARY.name.into());
        }
        list.extend(fallbacks.iter().map(|name| name.to_string()));
    }
    fonts
}

/// Install Consolas and the fallback fonts that exist on this system.
pub fn install(ctx: &egui::Context) {
    ctx.set_fonts(definitions(|file| {
        model::system_font_path(file, |key| std::env::var(key).ok())
            .and_then(|path| std::fs::read(path).ok())
    }));
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn consolas_comes_first_and_fallbacks_after_the_builtin_fonts() {
        let defaults = egui::FontDefinitions::default();
        let fonts = definitions(|file| (file != "malgun.ttf").then(|| vec![0]));
        let mono = &fonts.families[&egui::FontFamily::Monospace];
        let prop = &fonts.families[&egui::FontFamily::Proportional];
        assert_eq!(mono[0], "consolas");
        // egui's own fonts keep priority over the Windows fallbacks.
        let builtin_mono = &defaults.families[&egui::FontFamily::Monospace];
        assert_eq!(&mono[1..=builtin_mono.len()], builtin_mono.as_slice());
        assert_eq!(
            &mono[mono.len() - 3..],
            ["segoe-ui-symbol", "segoe-ui-emoji", "microsoft-yahei"]
        );
        assert!(!prop.contains(&"consolas".to_string()));
        assert!(prop.ends_with(&[
            "segoe-ui-symbol".to_string(),
            "segoe-ui-emoji".to_string(),
            "microsoft-yahei".to_string()
        ]));
        // Missing files are skipped entirely.
        assert!(!fonts.font_data.contains_key("malgun-gothic"));
        assert!(!mono.contains(&"malgun-gothic".to_string()));
    }

    #[test]
    fn without_system_fonts_the_builtin_fonts_remain() {
        let fonts = definitions(|_| None);
        let defaults = egui::FontDefinitions::default();
        assert_eq!(
            fonts.families[&egui::FontFamily::Monospace],
            defaults.families[&egui::FontFamily::Monospace]
        );
        assert_eq!(fonts.font_data.len(), defaults.font_data.len());
    }

    /// With the real Windows fonts, characters that AI CLIs print and egui's built-in
    /// fonts lack (✓ ✗ ⎿ ✻, CJK) get a glyph. Characters whose fallback font file is
    /// missing on this machine are skipped.
    #[cfg(windows)]
    #[test]
    fn windows_fallbacks_cover_symbols_and_cjk() {
        let font_dir = model::system_font_path("", |key| std::env::var(key).ok()).unwrap();
        let load = |file: &str| std::fs::read(font_dir.join(file)).ok();
        let id = egui::FontId::monospace(14.0);
        let builtin = egui::epaint::text::Fonts::new(1.0, 2048, egui::FontDefinitions::default());
        let installed = egui::epaint::text::Fonts::new(1.0, 2048, definitions(load));
        let cases = [
            ('✓', "seguisym.ttf"),
            ('✗', "seguisym.ttf"),
            ('⎿', "seguisym.ttf"),
            ('✻', "seguisym.ttf"),
            ('中', "msyh.ttc"),
            ('あ', "msyh.ttc"),
            ('한', "malgun.ttf"),
        ];
        let mut checked = 0;
        for (c, file) in cases {
            assert!(!builtin.has_glyph(&id, c), "egui already has {c}");
            if !font_dir.join(file).is_file() {
                continue;
            }
            assert!(installed.has_glyph(&id, c), "no glyph for {c} with {file}");
            checked += 1;
        }
        assert!(
            checked >= 4,
            "only {checked} characters checked; fonts missing?"
        );
    }
}
