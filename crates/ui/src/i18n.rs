//! Локализация интерфейса.
//!
//! Строки статические, без склонений и подстановок. Цепочка:
//! `выбранный язык → en → [technical.id]`.

use std::collections::BTreeMap;

pub const LOCALES: [&str; 8] = ["ru", "en", "ja", "uk", "pl", "es", "zh", "de"];

const SOURCES: [(&str, &str); 8] = [
    ("ru", include_str!("../locales/ui.ru.toml")),
    ("en", include_str!("../locales/ui.en.toml")),
    ("ja", include_str!("../locales/ui.ja.toml")),
    ("uk", include_str!("../locales/ui.uk.toml")),
    ("pl", include_str!("../locales/ui.pl.toml")),
    ("es", include_str!("../locales/ui.es.toml")),
    ("zh", include_str!("../locales/ui.zh.toml")),
    ("de", include_str!("../locales/ui.de.toml")),
];

pub struct I18n {
    locale: String,
    tables: BTreeMap<String, BTreeMap<String, String>>,
}

fn flatten(value: &toml::Value, prefix: &str, out: &mut BTreeMap<String, String>) {
    match value {
        toml::Value::Table(table) => {
            for (key, child) in table {
                let full = if prefix.is_empty() {
                    key.clone()
                } else {
                    format!("{prefix}.{key}")
                };
                flatten(child, &full, out);
            }
        }
        toml::Value::String(text) => {
            out.insert(prefix.to_string(), text.clone());
        }
        other => {
            out.insert(prefix.to_string(), other.to_string());
        }
    }
}

impl I18n {
    pub fn load(locale: &str) -> Self {
        let mut tables = BTreeMap::new();
        for (code, raw) in SOURCES {
            let parsed: toml::Value = toml::from_str(raw).expect("locale file must be valid TOML");
            let mut flat = BTreeMap::new();
            flatten(&parsed, "", &mut flat);
            tables.insert(code.to_string(), flat);
        }
        let locale = if LOCALES.contains(&locale) {
            locale.to_string()
        } else {
            "en".to_string()
        };
        Self { locale, tables }
    }

    #[allow(dead_code)]
    pub fn locale(&self) -> &str {
        &self.locale
    }

    #[allow(dead_code)] // экран настроек появится в v0.58
    pub fn set_locale(&mut self, locale: &str) {
        if LOCALES.contains(&locale) {
            self.locale = locale.to_string();
        }
    }

    pub fn t(&self, key: &str) -> String {
        if let Some(value) = self.tables.get(&self.locale).and_then(|m| m.get(key)) {
            return value.clone();
        }
        if let Some(value) = self.tables.get("en").and_then(|m| m.get(key)) {
            return value.clone();
        }
        format!("[{key}]")
    }

    #[allow(dead_code)]
    pub fn has(&self, locale: &str, key: &str) -> bool {
        self.tables
            .get(locale)
            .map(|m| m.contains_key(key))
            .unwrap_or(false)
    }

    #[allow(dead_code)]
    pub fn keys(&self, locale: &str) -> Vec<String> {
        self.tables
            .get(locale)
            .map(|m| m.keys().cloned().collect())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_locale_is_a_subset_of_english_and_russian_is_complete() {
        let i18n = I18n::load("en");
        let en_keys: std::collections::BTreeSet<String> = i18n.keys("en").into_iter().collect();
        assert!(!en_keys.is_empty(), "english table must not be empty");
        // Частичный перевод разрешён: недостающие ключи берутся из английского.
        for code in LOCALES {
            let keys: std::collections::BTreeSet<String> = i18n.keys(code).into_iter().collect();
            let extra: Vec<_> = keys.difference(&en_keys).cloned().collect();
            assert!(extra.is_empty(), "{code} has unknown keys: {extra:?}");
        }
        let ru_keys: std::collections::BTreeSet<String> = i18n.keys("ru").into_iter().collect();
        let missing: Vec<_> = en_keys.difference(&ru_keys).cloned().collect();
        assert!(missing.is_empty(), "ru missing keys: {missing:?}");
    }

    #[test]
    fn fallback_goes_through_english_then_technical_id() {
        let mut i18n = I18n::load("de");
        i18n.set_locale("de");
        assert_eq!(i18n.t("nav.today"), "Heute");
        assert_eq!(i18n.t("missing.key"), "[missing.key]");
    }

    #[test]
    fn unknown_locale_falls_back_to_english() {
        let i18n = I18n::load("xx");
        assert_eq!(i18n.locale(), "en");
        assert_eq!(i18n.t("nav.today"), "Today");
    }
}
