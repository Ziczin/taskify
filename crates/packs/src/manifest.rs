//! Манифест пака.

use serde::{Deserialize, Serialize};
use taskify_core::task::Locales;

pub const PACK_SCHEMA: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Progression {
    #[serde(default = "default_base")]
    pub base: f64,
    #[serde(default = "default_exponent")]
    pub exponent: f64,
    #[serde(default = "default_max_level")]
    pub max_level: u32,
}

fn default_base() -> f64 {
    50.0
}

fn default_exponent() -> f64 {
    1.4
}

fn default_max_level() -> u32 {
    10
}

impl Default for Progression {
    fn default() -> Self {
        Self {
            base: default_base(),
            exponent: default_exponent(),
            max_level: default_max_level(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Manifest {
    #[serde(default = "default_schema")]
    pub schema: u32,
    pub id: String,
    #[serde(default = "default_version")]
    pub version: String,
    #[serde(default)]
    pub name: Locales,
    #[serde(default)]
    pub description: Locales,
    #[serde(default)]
    pub author: Option<String>,
    #[serde(default)]
    pub homepage: Option<String>,
    #[serde(default)]
    pub license: Option<String>,
    #[serde(default)]
    pub official: bool,
    /// Включать пак сразу при установке (обучающие паки).
    #[serde(default)]
    pub default_enabled: bool,
    #[serde(default)]
    pub locales: Vec<String>,
    /// Минимальная версия приложения.
    #[serde(default)]
    pub requires: Option<String>,
    #[serde(default)]
    pub progression: Progression,
}

fn default_schema() -> u32 {
    PACK_SCHEMA
}

fn default_version() -> String {
    "0.1.0".to_string()
}

impl Manifest {
    pub fn title(&self, locale: &str) -> String {
        taskify_core::task::tr_or(&self.name, locale, &self.id)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_enabled_flag_parses() {
        let manifest: Manifest =
            toml::from_str("id = \"welcome\"\ndefault_enabled = true\n").unwrap();
        assert!(manifest.default_enabled);
        assert!(!manifest.official);
    }

    #[test]
    fn minimal_manifest_parses() {
        let manifest: Manifest = toml::from_str("id = \"starter\"\nversion = \"1.0.0\"\n").unwrap();
        assert_eq!(manifest.schema, 1);
        assert_eq!(manifest.progression.base, 50.0);
        assert_eq!(manifest.progression.max_level, 10);
        assert!(!manifest.official);
    }
}
