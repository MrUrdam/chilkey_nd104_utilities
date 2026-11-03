//! UI texts in German and English.
//!
//! Locale files live in `apps/nd104-studio/locales/` and are compiled in.
//! A new language = new JSON file + entry in [`LANGUAGES`] and [`source`].
//!
//! Platform variants: a key with `@linux` (or another `std::env::consts::OS`
//! name) replaces the plain key on that platform, e.g. `menu.autostart`
//! "Start with Windows" / `menu.autostart@linux` "Start at login". The tray
//! and the settings window both get the resolved texts.

use std::collections::HashMap;

/// Selectable languages: (code, native name).
pub const LANGUAGES: &[(&str, &str)] = &[("de", "Deutsch"), ("en", "English")];

const FALLBACK: &str = "en";

fn source(code: &str) -> &'static str {
    match code {
        "de" => include_str!("../../locales/de.json"),
        _ => include_str!("../../locales/en.json"),
    }
}

fn parse(code: &str) -> HashMap<String, String> {
    let texts = serde_json::from_str(source(code)).expect("locale files are valid JSON (tested)");
    for_platform(texts, std::env::consts::OS)
}

/// Applies the `key@<os>` variants of `os` and drops all variants.
fn for_platform(texts: HashMap<String, String>, os: &str) -> HashMap<String, String> {
    let suffix = format!("@{os}");
    let mut resolved: HashMap<String, String> = texts
        .iter()
        .filter(|(k, _)| !k.contains('@'))
        .map(|(k, v)| (k.clone(), v.clone()))
        .collect();
    for (key, value) in texts {
        if let Some(base) = key.strip_suffix(&suffix) {
            resolved.insert(base.to_string(), value);
        }
    }
    resolved
}

/// Resolves the `language` setting (`auto`, `de`, `en`) to a language code.
pub fn resolve(setting: &str) -> &'static str {
    if let Some((code, _)) = LANGUAGES.iter().find(|(code, _)| *code == setting) {
        return code;
    }
    let system = sys_locale::get_locale().unwrap_or_default().to_lowercase();
    LANGUAGES
        .iter()
        .map(|(code, _)| *code)
        .find(|code| system.starts_with(code))
        .unwrap_or(FALLBACK)
}

#[derive(Clone)]
pub struct I18n {
    texts: HashMap<String, String>,
    fallback: HashMap<String, String>,
}

impl I18n {
    pub fn new(setting: &str) -> Self {
        Self {
            texts: parse(resolve(setting)),
            fallback: parse(FALLBACK),
        }
    }

    pub fn t(&self, key: &str) -> String {
        self.texts
            .get(key)
            .or_else(|| self.fallback.get(key))
            .cloned()
            .unwrap_or_else(|| key.to_string())
    }

    /// All texts (fallback language filled in), for the settings window.
    pub fn all(&self) -> HashMap<String, String> {
        let mut all = self.fallback.clone();
        all.extend(self.texts.clone());
        all
    }

    /// Text with `{name}` placeholders replaced.
    pub fn tf(&self, key: &str, args: &[(&str, &str)]) -> String {
        args.iter().fold(self.t(key), |text, (name, value)| {
            text.replace(&format!("{{{name}}}"), value)
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn all_locales_have_the_same_keys() {
        let mut reference: Vec<_> = parse(FALLBACK).into_keys().collect();
        reference.sort();
        for (code, _) in LANGUAGES {
            let mut keys: Vec<_> = parse(code).into_keys().collect();
            keys.sort();
            assert_eq!(keys, reference, "locale {code} differs from {FALLBACK}");
        }
    }

    #[test]
    fn platform_variants_replace_the_plain_text() {
        let texts: HashMap<String, String> = [
            ("menu.autostart", "Start with Windows"),
            ("menu.autostart@linux", "Start at login"),
            ("menu.quit", "Quit"),
        ]
        .into_iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect();
        let linux = for_platform(texts.clone(), "linux");
        assert_eq!(linux["menu.autostart"], "Start at login");
        assert_eq!(linux["menu.quit"], "Quit");
        assert!(!linux.contains_key("menu.autostart@linux"));
        let windows = for_platform(texts, "windows");
        assert_eq!(windows["menu.autostart"], "Start with Windows");
        assert!(!windows.keys().any(|k| k.contains('@')));
    }

    #[test]
    fn every_platform_variant_has_a_plain_text() {
        let raw: HashMap<String, String> = serde_json::from_str(source(FALLBACK)).unwrap();
        for key in raw.keys().filter(|k| k.contains('@')) {
            let base = key.split('@').next().unwrap();
            assert!(raw.contains_key(base), "{key} has no plain {base}");
        }
    }

    #[test]
    fn explicit_language_and_placeholders() {
        let de = I18n::new("de");
        assert_eq!(de.t("menu.quit"), "Beenden");
        assert_eq!(
            de.tf("menu.about", &[("version", "0.2.0")]),
            "Über ND104 Studio 0.2.0"
        );
        assert_eq!(I18n::new("en").t("menu.quit"), "Quit");
        assert_eq!(de.t("no.such.key"), "no.such.key");
    }
}
