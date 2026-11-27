//! UI languages: Hochdeutsch (the source texts) and East Westphalian Low German
//! (Ostwestfälisches Platt).
//!
//! Texts are keyed by their German source text: `tr("Speichern")`, or with named
//! placeholders `trf!("Bild nicht angezeigt: {error}", error = error)`. A missing
//! translation falls back to German; the tests below keep the table complete.

mod nds;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU8, Ordering};
use std::sync::LazyLock;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Language {
    #[default]
    #[serde(rename = "de")]
    German,
    #[serde(rename = "nds-owl")]
    EastWestphalian,
}

impl Language {
    pub const ALL: [Self; 2] = [Self::German, Self::EastWestphalian];

    /// Name shown in the language selector, in the language itself.
    pub fn label(self) -> &'static str {
        match self {
            Self::German => "Hochdeutsch",
            Self::EastWestphalian => "Plattdütsk (Ostwestfaolen)",
        }
    }

    /// How AI quick actions ask for the answer language.
    pub fn ai_answer_language(self) -> &'static str {
        match self {
            Self::German => "auf Deutsch",
            Self::EastWestphalian => "auf ostwestfälischem Plattdeutsch",
        }
    }
}

static CURRENT: AtomicU8 = AtomicU8::new(0);

pub fn set_language(language: Language) {
    CURRENT.store(language as u8, Ordering::Relaxed);
}

pub fn language() -> Language {
    match CURRENT.load(Ordering::Relaxed) {
        1 => Language::EastWestphalian,
        _ => Language::German,
    }
}

static EAST_WESTPHALIAN: LazyLock<HashMap<&'static str, &'static str>> =
    LazyLock::new(|| nds::TABLE.iter().copied().collect());

/// Translation of a German source text into `language`.
pub fn translate(language: Language, text: &'static str) -> &'static str {
    match language {
        Language::German => text,
        Language::EastWestphalian => EAST_WESTPHALIAN.get(text).copied().unwrap_or(text),
    }
}

/// Translation into the current UI language.
pub fn tr(text: &'static str) -> &'static str {
    translate(language(), text)
}

/// Replace `{name}` placeholders with the given values. Unknown names stay as they are.
pub fn fill(template: &str, args: &[(&str, &dyn std::fmt::Display)]) -> String {
    let mut out = String::with_capacity(template.len() + 32);
    let mut rest = template;
    while let Some(start) = rest.find('{') {
        out.push_str(&rest[..start]);
        let after = &rest[start + 1..];
        let Some(end) = after.find('}') else {
            out.push_str(&rest[start..]);
            return out;
        };
        let name = &after[..end];
        match args.iter().find(|(key, _)| *key == name) {
            Some((_, value)) => out.push_str(&value.to_string()),
            None => {
                out.push('{');
                out.push_str(name);
                out.push('}');
            }
        }
        rest = &after[end + 1..];
    }
    out.push_str(rest);
    out
}

/// `trf!("Text {name}", name = value)`: translate, then fill named placeholders.
macro_rules! trf {
    ($text:literal $(, $name:ident = $value:expr)* $(,)?) => {
        crate::i18n::fill(
            crate::i18n::tr($text),
            &[$((stringify!($name), &$value as &dyn ::std::fmt::Display)),*],
        )
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    fn placeholders(text: &str) -> BTreeSet<String> {
        let mut found = BTreeSet::new();
        let mut rest = text;
        while let Some(start) = rest.find('{') {
            let after = &rest[start + 1..];
            let Some(end) = after.find('}') else { break };
            found.insert(after[..end].to_string());
            rest = &after[end + 1..];
        }
        found
    }

    /// Minimal unescape for the string literals used with `tr`/`trf!`.
    fn unescape(raw: &str) -> String {
        let mut out = String::new();
        let mut chars = raw.chars();
        while let Some(c) = chars.next() {
            if c != '\\' {
                out.push(c);
                continue;
            }
            match chars.next() {
                Some('n') => out.push('\n'),
                Some('t') => out.push('\t'),
                Some('"') => out.push('"'),
                Some('\\') => out.push('\\'),
                Some('u') => {
                    let code: String = chars.by_ref().skip(1).take_while(|&c| c != '}').collect();
                    out.push(char::from_u32(u32::from_str_radix(&code, 16).unwrap()).unwrap());
                }
                other => panic!("unsupported escape {other:?} in {raw}"),
            }
        }
        out
    }

    /// German texts passed to `tr("…")` or `trf!("…"` anywhere in `src/`.
    fn source_keys() -> BTreeSet<String> {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
        let mut stack = vec![root];
        let mut keys = BTreeSet::new();
        while let Some(dir) = stack.pop() {
            for entry in std::fs::read_dir(dir).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    stack.push(path);
                    continue;
                }
                if path.extension().is_none_or(|ext| ext != "rs")
                    || path.ends_with("i18n/nds.rs")
                    || path.ends_with("i18n/mod.rs")
                {
                    continue;
                }
                let source = std::fs::read_to_string(&path).unwrap();
                for opener in ["tr(", "trf!("] {
                    let mut rest = source.as_str();
                    while let Some(start) = rest.find(opener) {
                        // Skip identifiers that merely end in `tr`, e.g. `attr(`.
                        let before = rest[..start].chars().next_back();
                        let after = rest[start + opener.len()..].trim_start();
                        rest = &rest[start + opener.len()..];
                        if before.is_some_and(|c| c.is_alphanumeric() || c == '_')
                            || !after.starts_with('"')
                        {
                            continue;
                        }
                        let literal = &after[1..];
                        let bytes = literal.as_bytes();
                        let mut end = 0;
                        while bytes[end] != b'"' {
                            end += if bytes[end] == b'\\' { 2 } else { 1 };
                        }
                        keys.insert(unescape(&literal[..end]));
                    }
                }
            }
        }
        keys
    }

    #[test]
    fn every_ui_text_has_an_east_westphalian_translation() {
        let table: BTreeSet<String> = nds::TABLE.iter().map(|(de, _)| de.to_string()).collect();
        let used = source_keys();
        assert!(used.len() > 250, "only {} texts found", used.len());
        let missing: Vec<_> = used.difference(&table).collect();
        assert!(missing.is_empty(), "missing translations: {missing:#?}");
        let unused: Vec<_> = table.difference(&used).collect();
        assert!(
            unused.is_empty(),
            "translations without source text: {unused:#?}"
        );
    }

    #[test]
    fn translations_keep_placeholders_and_are_unique() {
        let mut seen = BTreeSet::new();
        for (german, platt) in nds::TABLE {
            assert!(seen.insert(*german), "duplicate entry: {german}");
            assert!(!platt.trim().is_empty(), "empty translation: {german}");
            assert_eq!(placeholders(german), placeholders(platt), "{german}");
        }
    }

    #[test]
    fn lookup_falls_back_to_german_and_fills_placeholders() {
        assert_eq!(translate(Language::German, "Abbrechen"), "Abbrechen");
        assert_ne!(
            translate(Language::EastWestphalian, "Abbrechen"),
            "Abbrechen"
        );
        assert_eq!(
            translate(Language::EastWestphalian, "kein Eintrag"),
            "kein Eintrag"
        );
        let value = 3;
        assert_eq!(
            fill("{a} / {b} {x}", &[("a", &value), ("b", &"vier")]),
            "3 / vier {x}"
        );
        assert_eq!(fill("offen {", &[]), "offen {");
    }

    #[test]
    fn language_setting_is_backwards_compatible() {
        assert_eq!(serde_json::to_string(&Language::German).unwrap(), "\"de\"");
        assert_eq!(
            serde_json::from_str::<Language>("\"nds-owl\"").unwrap(),
            Language::EastWestphalian
        );
    }
}
