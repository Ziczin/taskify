//! Лайвспейс: расположение данных и настройки профиля.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

/// Текущая версия формата данных профиля.
pub const LIVESPACE_SCHEMA: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupSettings {
    pub enabled: bool,
    /// Дополнительный бэкап каждые N событий.
    pub every_events: u64,
    /// Пятнадцатиминутные… вернее, минутные снимки: как часто (в минутах).
    pub minute_interval: u64,
    pub keep_minutes: u64,
    pub hour_interval: u64,
    pub keep_hours: u64,
    pub keep_days: u64,
    pub keep_weeks: u64,
}

impl Default for BackupSettings {
    fn default() -> Self {
        Self {
            enabled: true,
            every_events: 500,
            minute_interval: 5,
            keep_minutes: 12,
            hour_interval: 1,
            keep_hours: 24,
            keep_days: 7,
            keep_weeks: 13,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GameSettings {
    #[serde(default = "schema")]
    pub schema: u32,
    /// Начало дня, 0–23.
    pub day_start_hour: u32,
    pub week_starts_monday: bool,
    pub undo_seconds: u64,
    pub weekly_xp_goal: i64,
    /// Сколько заданий в день считается нормой (для `over_norm`).
    pub daily_norm: u64,
    #[serde(default)]
    pub backups: BackupSettings,
}

fn schema() -> u32 {
    LIVESPACE_SCHEMA
}

impl Default for GameSettings {
    fn default() -> Self {
        Self {
            schema: LIVESPACE_SCHEMA,
            day_start_hour: 0,
            week_starts_monday: true,
            undo_seconds: 5,
            weekly_xp_goal: 0,
            daily_norm: 12,
            backups: BackupSettings::default(),
        }
    }
}

impl GameSettings {
    pub fn rules(&self) -> taskify_core::time::DayRules {
        taskify_core::time::DayRules::new(self.day_start_hour, self.week_starts_monday)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
struct LivespaceManifest {
    schema: u32,
    id: String,
    name: String,
    created_at: String,
}

#[derive(Debug, Clone)]
pub struct Livespace {
    root: PathBuf,
    slug: String,
}

impl Livespace {
    pub fn at(root: impl Into<PathBuf>, slug: impl Into<String>) -> Self {
        Self {
            root: root.into(),
            slug: slug.into(),
        }
    }

    /// Открывает профиль по пути, читая имя из манифеста.
    pub fn open(root: impl AsRef<Path>) -> std::io::Result<Self> {
        let root = root.as_ref().to_path_buf();
        std::fs::create_dir_all(&root)?;
        let slug = root
            .file_name()
            .map(|name| name.to_string_lossy().to_string())
            .unwrap_or_else(|| "default".to_string());
        let manifest_path = root.join("livespace.toml");
        if !manifest_path.exists() {
            let manifest = LivespaceManifest {
                schema: LIVESPACE_SCHEMA,
                id: uuid::Uuid::new_v4().to_string(),
                name: slug.clone(),
                created_at: chrono::Local::now().to_rfc3339(),
            };
            let text = toml::to_string_pretty(&manifest)
                .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
            std::fs::write(&manifest_path, text)?;
        }
        Ok(Self { root, slug })
    }

    pub fn slug(&self) -> &str {
        &self.slug
    }

    pub fn root(&self) -> &Path {
        &self.root
    }

    pub fn log_dir(&self) -> PathBuf {
        self.root.join("data").join("log")
    }

    pub fn data_dir(&self) -> PathBuf {
        self.root.join("data")
    }

    pub fn state_file(&self) -> PathBuf {
        self.data_dir().join("state.json")
    }

    pub fn aggregates_file(&self) -> PathBuf {
        self.data_dir().join("aggregates.json")
    }

    pub fn settings_file(&self) -> PathBuf {
        self.root.join("settings.toml")
    }

    pub fn backups_dir(&self) -> PathBuf {
        self.data_dir().join("backups")
    }

    pub fn migrations_log(&self) -> PathBuf {
        self.data_dir().join("migrations.log")
    }

    pub fn notifications_history(&self) -> PathBuf {
        self.data_dir().join("notifications_history.jsonl")
    }

    pub fn local_tasks_dir(&self) -> PathBuf {
        self.root.join("local").join("tasks")
    }

    pub fn appearance_file(&self) -> PathBuf {
        self.root.join("appearance.toml")
    }

    pub fn load_settings(&self) -> GameSettings {
        std::fs::read_to_string(self.settings_file())
            .ok()
            .and_then(|text| toml::from_str(&text).ok())
            .unwrap_or_default()
    }

    pub fn save_settings(&self, settings: &GameSettings) -> std::io::Result<()> {
        let text = toml::to_string_pretty(settings)
            .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
        let path = self.settings_file();
        let tmp = path.with_extension("toml.tmp");
        std::fs::write(&tmp, text)?;
        std::fs::rename(&tmp, &path)
    }
}

#[derive(Debug, Clone)]
pub struct LivespaceBuilder {
    root: PathBuf,
    name: Option<String>,
}

impl LivespaceBuilder {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        Self {
            root: root.into(),
            name: None,
        }
    }

    pub fn name(mut self, name: impl Into<String>) -> Self {
        self.name = Some(name.into());
        self
    }

    pub fn build(self) -> std::io::Result<Livespace> {
        let livespace = Livespace::open(&self.root)?;
        if let Some(name) = self.name {
            let path = self.root.join("livespace.toml");
            let mut manifest: LivespaceManifest = std::fs::read_to_string(&path)
                .ok()
                .and_then(|text| toml::from_str(&text).ok())
                .unwrap_or_default();
            manifest.name = name;
            if manifest.schema == 0 {
                manifest.schema = LIVESPACE_SCHEMA;
            }
            if manifest.id.is_empty() {
                manifest.id = uuid::Uuid::new_v4().to_string();
            }
            if manifest.created_at.is_empty() {
                manifest.created_at = chrono::Local::now().to_rfc3339();
            }
            if let Ok(text) = toml::to_string_pretty(&manifest) {
                std::fs::write(&path, text)?;
            }
        }
        Ok(livespace)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_settings_are_stable() {
        let settings = GameSettings::default();
        assert_eq!(settings.day_start_hour, 0);
        assert!(settings.week_starts_monday);
        assert_eq!(settings.undo_seconds, 5);
        assert_eq!(settings.backups.keep_weeks, 13);
    }

    #[test]
    fn open_creates_manifest() {
        let dir = crate::test_support::temp_root("ls");
        let livespace = Livespace::open(&dir).unwrap();
        assert!(dir.join("livespace.toml").exists());
        assert_eq!(livespace.log_dir(), dir.join("data").join("log"));
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn settings_roundtrip() {
        let dir = crate::test_support::temp_root("ls");
        let livespace = Livespace::open(&dir).unwrap();
        let settings = GameSettings {
            day_start_hour: 4,
            ..GameSettings::default()
        };
        livespace.save_settings(&settings).unwrap();
        let back = livespace.load_settings();
        assert_eq!(back.day_start_hour, 4);
        let _ = std::fs::remove_dir_all(&dir);
    }
}
