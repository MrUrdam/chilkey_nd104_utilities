//! UI texts in German and English.
//!
//! Locale files live in `apps/nd104-studio/locales/` and are compiled in.
//! A new language = new JSON file + entry in [`LANGUAGES`] and [`source`].

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
    serde_json::from_str(source(code)).expect("locale files are valid JSON (tested)")
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
