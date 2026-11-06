//! Фразы-уведомления: мешок без повторов и отдельная история показов.

use std::path::PathBuf;

use rand::seq::SliceRandom;

use crate::i18n::I18n;

const MAX_PHRASES: usize = 12;

pub struct Notifications {
    bag: Vec<String>,
    history: PathBuf,
}

impl Notifications {
    pub fn new(history: PathBuf) -> Self {
        Self {
            bag: Vec::new(),
            history,
        }
    }

    fn refill(&mut self, i18n: &I18n, locale: &str) {
        let mut keys: Vec<String> = Vec::new();
        for index in 1..=MAX_PHRASES {
            let key = format!("notify.{index}");
            if i18n.has(locale, &key) || i18n.has("en", &key) {
                keys.push(key);
            }
        }
        keys.shuffle(&mut rand::rng());
        self.bag = keys;
    }

    /// Следующая фраза: мешок опустошается без повторов.
    pub fn next(&mut self, i18n: &I18n, locale: &str) -> Option<String> {
        if self.bag.is_empty() {
            self.refill(i18n, locale);
        }
        self.bag.pop().map(|key| i18n.t(&key))
    }

    /// Показ пишется отдельно от основного журнала.
    pub fn log(&self, phrase: &str, task: &str) {
        let line = serde_json::json!({
            "ts": chrono::Local::now().to_rfc3339(),
            "phrase": phrase,
            "task": task,
        });
        if let Some(parent) = self.history.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.history)
        {
            use std::io::Write;
            let _ = writeln!(file, "{line}");
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bag_does_not_repeat_before_exhaustion() {
        let i18n = I18n::load("ru");
        let mut notifications =
            Notifications::new(crate::test_support::temp_root("notify").join("n.jsonl"));
        let mut seen = Vec::new();
        for _ in 0..5 {
            if let Some(phrase) = notifications.next(&i18n, "ru") {
                seen.push(phrase);
            }
        }
        let unique: std::collections::BTreeSet<_> = seen.iter().collect();
        assert_eq!(unique.len(), seen.len(), "фразы повторились раньше времени");
        assert!(!seen.is_empty());
    }
}
