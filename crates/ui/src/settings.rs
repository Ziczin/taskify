//! Настройки приложения: окно, язык, тема.
//!
//! Живут в `%APPDATA%\taskify\settings.toml`, читаются до создания окна.

use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

pub const DEFAULT_WIDTH: f32 = 1280.0;
pub const DEFAULT_HEIGHT: f32 = 800.0;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowState {
    pub width: f32,
    pub height: f32,
    pub x: Option<f32>,
    pub y: Option<f32>,
    pub maximized: bool,
    pub fullscreen: bool,
}

impl Default for WindowState {
    fn default() -> Self {
        Self {
            width: DEFAULT_WIDTH,
            height: DEFAULT_HEIGHT,
            x: None,
            y: None,
            maximized: false,
            fullscreen: false,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UiSettings {
    pub locale: String,
    /// `system`, `light`, `dark`.
    pub theme: String,
    /// Сколько колонок в сетке заданий.
    #[serde(default = "default_columns")]
    pub columns: usize,
    /// Сколько карточек в строке витрины маркета.
    #[serde(default = "default_market_columns")]
    pub market_columns: usize,
    /// Устаревшая настройка доски стикеров: оставлена, чтобы старые
    /// `settings.toml` читались без ошибок. Стикеры теперь висят на панелях.
    #[serde(default = "default_board_columns")]
    pub board_columns: usize,
    /// Множитель скорости анимаций: 1.0 — обычная.
    #[serde(default = "default_anim_speed")]
    pub anim_speed: f32,
    /// Множитель размера шрифтов интерфейса.
    #[serde(default = "default_font_scale")]
    pub font_scale: f32,
    /// Показывать сетку на боковых панелях со стикерами.
    #[serde(default)]
    pub sticker_grid: bool,
    /// Шаг сетки стикеров в пикселях. Центры стикеров прилипают к её узлам.
    #[serde(default = "default_sticker_grid_size")]
    pub sticker_grid_size: f32,
    /// Плотность подложки боковых панелей со стикерами, 0–100 %.
    #[serde(default = "default_sticker_panel_opacity")]
    pub sticker_panel_opacity: u8,
}

pub fn default_columns() -> usize {
    4
}

pub fn default_market_columns() -> usize {
    5
}

pub fn default_board_columns() -> usize {
    6
}

pub fn default_anim_speed() -> f32 {
    1.0
}

pub fn default_font_scale() -> f32 {
    1.0
}

pub fn default_sticker_grid_size() -> f32 {
    32.0
}

pub fn default_sticker_panel_opacity() -> u8 {
    59
}

impl Default for UiSettings {
    fn default() -> Self {
        Self {
            locale: "ru".to_string(),
            theme: "light".to_string(),
            columns: default_columns(),
            market_columns: default_market_columns(),
            board_columns: default_board_columns(),
            anim_speed: default_anim_speed(),
            font_scale: default_font_scale(),
            sticker_grid: false,
            sticker_grid_size: default_sticker_grid_size(),
            sticker_panel_opacity: default_sticker_panel_opacity(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Paths {
    /// Общая папка, в которой создаются и ищутся лайвспейсы.
    #[serde(default)]
    pub livespaces_dir: Option<PathBuf>,
    /// Папка установленных паков. Пусто — `packs` рядом с данными.
    #[serde(default)]
    pub packs_dir: Option<PathBuf>,
    /// Личные папки лайвспейсов по слагу. Пусто — общая папка.
    #[serde(default)]
    pub livespace_dirs: BTreeMap<String, PathBuf>,
}

/// Ключ встроенного профиля «базовая светлая тема».
pub const BASE_PROFILE_LIGHT: &str = "base.light";
/// Ключ встроенного профиля «базовая тёмная тема».
pub const BASE_PROFILE_DARK: &str = "base.dark";

/// Встроенные профили: их нельзя удалить, они есть всегда.
pub fn is_base_profile(name: &str) -> bool {
    name == BASE_PROFILE_LIGHT || name == BASE_PROFILE_DARK
}

/// Ключ базового профиля для светлой или тёмной темы.
pub fn base_profile_key(theme: &str) -> &'static str {
    if theme == "dark" {
        BASE_PROFILE_DARK
    } else {
        BASE_PROFILE_LIGHT
    }
}

/// Встроенный профиль: тема плюс базовые элементы оформления.
///
/// Ключ `accent` намеренно не задаётся — палитра темы сама подставляет свой
/// акцент, поэтому светлая и тёмная базы выглядят уместно.
fn base_profile(theme: &str) -> LookProfile {
    let mut extra = BTreeMap::new();
    extra.insert(
        "appearance.theme_token".to_string(),
        "starter/theme".to_string(),
    );
    extra.insert("starter/theme".to_string(), theme.to_string());
    extra.insert("starter/color.background".to_string(), theme.to_string());
    extra.insert("starter/color.accent".to_string(), "blue".to_string());
    extra.insert("starter/font.ui".to_string(), "sans".to_string());
    extra.insert("starter/sound.ui".to_string(), "quiet".to_string());
    LookProfile {
        theme: theme.to_string(),
        extra,
        at: None,
    }
}

/// Сохранённый профиль оформления: тема и вся косметика.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct LookProfile {
    #[serde(default)]
    pub theme: String,
    #[serde(default)]
    pub extra: BTreeMap<String, String>,
    /// Время включения профиля, `ЧЧ:ММ`. Пусто — профиль только ручной.
    #[serde(default)]
    pub at: Option<String>,
}

/// Разбирает `ЧЧ:ММ` в минуты от полуночи.
pub fn parse_at(text: &str) -> Option<u32> {
    let (hours, minutes) = text.trim().split_once(':')?;
    let hours: u32 = hours.trim().parse().ok()?;
    let minutes: u32 = minutes.trim().parse().ok()?;
    if hours > 23 || minutes > 59 {
        return None;
    }
    Some(hours * 60 + minutes)
}

/// Какой профиль должен быть активен в это время суток.
///
/// Берётся ближайшее прошедшее время включения; если сегодня ещё ничего не
/// включалось — вчерашний последний профиль (переход через полночь).
pub fn active_profile(profiles: &BTreeMap<String, LookProfile>, minutes: u32) -> Option<String> {
    let mut best: Option<(u32, String)> = None;
    let mut fallback: Option<(u32, String)> = None;
    for (name, profile) in profiles {
        let Some(at) = profile.at.as_deref().and_then(parse_at) else {
            continue;
        };
        if at <= minutes && best.as_ref().map(|(value, _)| at > *value).unwrap_or(true) {
            best = Some((at, name.clone()));
        }
        if fallback
            .as_ref()
            .map(|(value, _)| at > *value)
            .unwrap_or(true)
        {
            fallback = Some((at, name.clone()));
        }
    }
    best.map(|(_, name)| name)
        .or_else(|| fallback.map(|(_, name)| name))
}

impl LookProfile {
    pub fn items(&self) -> usize {
        self.extra.len()
    }
}

/// Почему текущий вид нельзя сохранить как новую тему.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LookSaveError {
    /// Пустое имя.
    Empty,
    /// Тема с таким именем уже есть; её можно только перезаписать явно.
    Exists,
    /// Встроенная базовая тема: её нельзя перезаписывать.
    Base,
    /// Такой темы уже нет.
    Missing,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Settings {
    #[serde(default)]
    pub window: WindowState,
    #[serde(default)]
    pub ui: UiSettings,
    #[serde(default)]
    pub paths: Paths,
    #[serde(default)]
    pub extra: BTreeMap<String, String>,
    /// Имя игрока в шапке.
    #[serde(default = "default_player")]
    pub player: String,
    /// Слаг последнего открытого лайвспейса.
    #[serde(default = "default_livespace")]
    pub livespace: String,
    /// Полный путь к лайвспейсу, если он лежит вне папки данных.
    #[serde(default)]
    pub livespace_path: Option<String>,
    /// Сохранённые профили оформления, по имени.
    #[serde(default)]
    pub look_profiles: BTreeMap<String, LookProfile>,
    /// Тема, с которой начата текущая временная тема.
    ///
    /// `None` — временная тема собрана с нуля. Пока `look_draft` равен false,
    /// здесь лежит активная сохранённая тема.
    #[serde(default)]
    pub look_base: Option<String>,
    /// Текущий вид — временная тема, а не сохранённая.
    #[serde(default)]
    pub look_draft: bool,
    /// Переключать профили оформления по времени суток.
    #[serde(default)]
    pub auto_look: bool,
}

/// Настройки без файла: лайвспейс по умолчанию — `default`, иначе пустой
/// слаг открывал корень `livespaces` как профиль.
impl Default for Settings {
    fn default() -> Self {
        Self {
            window: WindowState::default(),
            ui: UiSettings::default(),
            paths: Paths::default(),
            extra: BTreeMap::new(),
            player: default_player(),
            livespace: default_livespace(),
            livespace_path: None,
            look_profiles: BTreeMap::new(),
            look_base: None,
            look_draft: false,
            auto_look: false,
        }
    }
}

pub fn default_livespace() -> String {
    "default".to_string()
}

fn default_player() -> String {
    "Игрок".to_string()
}

/// Роль оформления: хвост id без пака (`color.accent`, `background.image`).
///
/// Роль, а не `group` из токена: акцент, фон, шрифт и звук живут в группе
/// `theme`, но должны быть активны одновременно.
fn cosmetic_role(key: &str) -> &str {
    key.rsplit('/').next().unwrap_or(key)
}

impl Settings {
    /// Включает вариант косметики, выключая другие варианты той же роли.
    ///
    /// Роль — хвост id без пака: два пака могут объявлять акцент или обои, но
    /// активным остаётся один. Разные роли (акцент, цвет фона, картинка фона,
    /// шрифт, звук) живут вместе.
    pub fn apply_cosmetic(&mut self, token: &str, variant: &str) {
        let role = cosmetic_role(token).to_string();
        self.extra
            .retain(|key, _| !key.contains('/') || key == token || cosmetic_role(key) != role);
        self.extra.insert(token.to_string(), variant.to_string());
    }

    /// Сбрасывает оформление на базовую светлую или тёмную тему.
    pub fn apply_base_theme(&mut self, theme: &str) {
        let theme = if theme == "dark" { "dark" } else { "light" };
        let base = base_profile(theme);
        self.ui.theme = base.theme;
        self.extra = base.extra;
    }

    /// Применяет тему: сначала полная база, затем элементы темы.
    ///
    /// Всё, чего в теме нет (шрифт, звук, акцент, обои), остаётся от базовой
    /// светлой или тёмной темы.
    pub fn apply_theme_items(&mut self, base: &str, items: &[(String, String, String)]) {
        self.apply_base_theme(base);
        for (token, variant, value) in items {
            self.apply_cosmetic(token, variant);
            match cosmetic_role(token) {
                "theme" => {
                    self.extra
                        .insert("appearance.theme_token".to_string(), token.clone());
                    self.ui.theme = if value == "dark" { "dark" } else { "light" }.to_string();
                }
                "color.accent" => {
                    self.extra.insert("accent".to_string(), value.clone());
                }
                "color.background" => {
                    self.ui.theme = if value == "dark" { "dark" } else { "light" }.to_string();
                }
                _ => {}
            }
        }
    }

    /// Оставляет по одному варианту на роль: чинит настройки, накопленные до
    /// появления эксклюзивного режима.
    pub fn dedupe_cosmetics(&mut self) -> bool {
        let before = self.extra.len();
        let mut seen = std::collections::BTreeSet::new();
        let extra = std::mem::take(&mut self.extra);
        for (key, value) in extra {
            if !key.contains('/') || seen.insert(cosmetic_role(&key).to_string()) {
                self.extra.insert(key, value);
            }
        }
        self.extra.len() != before
    }

    pub fn dir() -> PathBuf {
        // TASKIFY_HOME позволяет держать данные рядом с проектом (портативный режим).
        if let Some(home) = std::env::var_os("TASKIFY_HOME") {
            return PathBuf::from(home);
        }
        let base = std::env::var_os("APPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir);
        base.join("taskify")
    }

    pub fn path() -> PathBuf {
        Self::dir().join("settings.toml")
    }

    #[allow(dead_code)] // используется с v0.52
    pub fn livespaces_dir(&self) -> PathBuf {
        self.paths
            .livespaces_dir
            .clone()
            .unwrap_or_else(|| Self::dir().join("livespaces"))
    }

    /// Личная папка лайвспейса, если она задана.
    pub fn personal_livespace_dir(&self, slug: &str) -> Option<&PathBuf> {
        self.paths.livespace_dirs.get(slug)
    }

    /// Запоминает личную папку лайвспейса; `None` возвращает общую.
    pub fn set_personal_livespace_dir(&mut self, slug: &str, path: Option<PathBuf>) {
        match path {
            Some(path) => {
                self.paths.livespace_dirs.insert(slug.to_string(), path);
            }
            None => {
                self.paths.livespace_dirs.remove(slug);
            }
        }
    }

    /// Текущий вид как тема. Расписание `at` в новый снимок не входит.
    pub fn current_look(&self, at: Option<String>) -> LookProfile {
        LookProfile {
            theme: self.ui.theme.clone(),
            extra: self.extra.clone(),
            at,
        }
    }

    /// Текущий вид стал временной темой: сохранённые темы не меняются.
    pub fn mark_look_draft(&mut self) {
        self.look_draft = true;
    }

    /// Собирает новую тему из текущего вида.
    ///
    /// Существующие темы не перезаписываются: для этого есть отдельное
    /// подтверждённое действие «Перезаписать».
    pub fn save_look_as(&mut self, name: &str) -> Result<(), LookSaveError> {
        let name = name.trim();
        if name.is_empty() {
            return Err(LookSaveError::Empty);
        }
        if is_base_profile(name) {
            return Err(LookSaveError::Base);
        }
        if self.look_profiles.contains_key(name) {
            return Err(LookSaveError::Exists);
        }
        self.look_profiles
            .insert(name.to_string(), self.current_look(None));
        self.look_base = Some(name.to_string());
        self.look_draft = false;
        Ok(())
    }

    /// Перезаписывает только пользовательскую тему текущим видом.
    ///
    /// Время включения сохраняется: перезапись меняет оформление, а не
    /// расписание. Базовые темы перезаписать нельзя.
    pub fn overwrite_look(&mut self, name: &str) -> Result<(), LookSaveError> {
        if is_base_profile(name) {
            return Err(LookSaveError::Base);
        }
        let Some(profile) = self.look_profiles.get(name) else {
            return Err(LookSaveError::Missing);
        };
        let at = profile.at.clone();
        self.look_profiles
            .insert(name.to_string(), self.current_look(at));
        self.look_base = Some(name.to_string());
        self.look_draft = false;
        Ok(())
    }

    /// Применяет сохранённую тему и делает её активной, без временной копии.
    pub fn apply_saved_look(&mut self, name: &str) -> bool {
        let Some(profile) = self.look_profiles.get(name).cloned() else {
            return false;
        };
        self.ui.theme = profile.theme;
        self.extra = profile.extra;
        self.dedupe_cosmetics();
        self.look_base = Some(name.to_string());
        self.look_draft = false;
        true
    }

    /// Для старых настроек: если текущий вид совпал с сохранённой темой,
    /// считаем её активной. Это только метка, сами настройки не меняются.
    pub fn detect_active_look(&mut self) -> bool {
        if self.look_base.is_some() || self.look_draft {
            return false;
        }
        let theme = self.ui.theme.clone();
        let extra = self.extra.clone();
        let name = self
            .look_profiles
            .iter()
            .find(|(_, profile)| profile.theme == theme && profile.extra == extra)
            .map(|(name, _)| name.clone());
        if let Some(name) = name {
            self.look_base = Some(name);
            true
        } else {
            false
        }
    }

    pub fn load() -> Self {
        let path = Self::path();
        let mut settings = match std::fs::read_to_string(&path) {
            Ok(text) => toml::from_str(&text).unwrap_or_default(),
            Err(_) => Self::default(),
        };
        let mut migrated = false;
        if settings.paths.livespace_dirs.is_empty() {
            if let Some(path) = settings.livespace_path.clone() {
                settings
                    .paths
                    .livespace_dirs
                    .insert(settings.livespace.clone(), PathBuf::from(path));
                migrated = true;
            }
        }
        let deduped = settings.dedupe_cosmetics();
        let bases = settings.ensure_base_profiles();
        let active = settings.detect_active_look();
        if bases || deduped || migrated || active {
            settings.save();
        }
        settings
    }

    /// Добавляет встроенные базовые профили, если их ещё нет.
    pub fn ensure_base_profiles(&mut self) -> bool {
        let mut added = false;
        for (key, theme) in [(BASE_PROFILE_LIGHT, "light"), (BASE_PROFILE_DARK, "dark")] {
            if !self.look_profiles.contains_key(key) {
                self.look_profiles
                    .insert(key.to_string(), base_profile(theme));
                added = true;
            }
        }
        added
    }

    pub fn save(&self) {
        let path = Self::path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        if let Ok(text) = toml::to_string_pretty(self) {
            let tmp = path.with_extension("toml.tmp");
            if std::fs::write(&tmp, text).is_ok() {
                let _ = std::fs::rename(&tmp, &path);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_are_sane() {
        let s = Settings::default();
        assert_eq!(s.ui.locale, "ru");
        assert_eq!(s.window.width, DEFAULT_WIDTH);
        assert!(!s.window.fullscreen);
    }

    #[test]
    fn sticker_grid_settings_are_parsed() {
        let text = r#"
livespace = "default"

[ui]
locale = "ru"
theme = "light"
sticker_grid = true
sticker_grid_size = 48.0
sticker_panel_opacity = 20
"#;
        let settings: Settings = toml::from_str(text).unwrap();
        assert!(settings.ui.sticker_grid);
        assert!((settings.ui.sticker_grid_size - 48.0).abs() < 0.001);
        assert_eq!(settings.ui.sticker_panel_opacity, 20);
        assert_eq!(
            Settings::default().ui.sticker_panel_opacity,
            default_sticker_panel_opacity()
        );
    }

    #[test]
    fn paths_are_parsed_and_personal_dirs_roundtrip() {
        let text = r#"
livespace = "default"

[paths]
packs_dir = "D:/Taskify/packs"

[paths.livespace_dirs]
default = "D:/Sync/Taskify/default"
"#;
        let mut settings: Settings = toml::from_str(text).unwrap();
        assert_eq!(
            settings.paths.packs_dir.as_deref(),
            Some(std::path::Path::new("D:/Taskify/packs"))
        );
        assert_eq!(
            settings.personal_livespace_dir("default"),
            Some(&PathBuf::from("D:/Sync/Taskify/default"))
        );
        settings.set_personal_livespace_dir("default", None);
        assert!(settings.personal_livespace_dir("default").is_none());
        let text = toml::to_string(&settings).unwrap();
        let back: Settings = toml::from_str(&text).unwrap();
        assert_eq!(back.paths.packs_dir, settings.paths.packs_dir);
        assert!(back.paths.livespace_dirs.is_empty());
    }

    #[test]
    fn base_profiles_are_always_present() {
        let mut settings = Settings::default();
        assert!(settings.ensure_base_profiles());
        assert!(!settings.ensure_base_profiles());
        assert_eq!(settings.look_profiles.len(), 2);
        let dark = settings
            .look_profiles
            .get(BASE_PROFILE_DARK)
            .expect("тёмная база");
        assert_eq!(dark.theme, "dark");
        assert_eq!(
            dark.extra
                .get("starter/color.background")
                .map(String::as_str),
            Some("dark")
        );
        assert!(is_base_profile(BASE_PROFILE_DARK));
        assert!(!is_base_profile("Вечер"));
    }

    #[test]
    fn assembling_theme_never_overwrites_a_saved_one() {
        let mut settings = Settings::default();
        settings.ui.theme = "light".to_string();
        settings
            .extra
            .insert("accent".to_string(), "#111111".to_string());

        assert_eq!(settings.save_look_as("   "), Err(LookSaveError::Empty));
        assert_eq!(
            settings.save_look_as(BASE_PROFILE_LIGHT),
            Err(LookSaveError::Base)
        );
        assert_eq!(settings.save_look_as("Вечер"), Ok(()));
        let saved = settings.look_profiles.get("Вечер").cloned().unwrap();

        // Меняем текущий вид и пробуем сохранить под тем же именем.
        settings.ui.theme = "dark".to_string();
        settings
            .extra
            .insert("accent".to_string(), "#222222".to_string());
        settings.mark_look_draft();
        assert_eq!(settings.save_look_as("Вечер"), Err(LookSaveError::Exists));
        assert_eq!(settings.look_profiles.get("Вечер"), Some(&saved));
        assert_eq!(settings.look_base.as_deref(), Some("Вечер"));
        assert!(settings.look_draft);
    }

    #[test]
    fn overwrite_is_explicit_and_keeps_schedule() {
        let mut settings = Settings::default();
        settings.save_look_as("День").unwrap();
        if let Some(profile) = settings.look_profiles.get_mut("День") {
            profile.at = Some("08:00".to_string());
        }
        settings.ui.theme = "dark".to_string();
        settings
            .extra
            .insert("accent".to_string(), "#333333".to_string());
        settings.mark_look_draft();

        assert_eq!(
            settings.overwrite_look(BASE_PROFILE_DARK),
            Err(LookSaveError::Base)
        );
        assert_eq!(
            settings.overwrite_look("Нет такой"),
            Err(LookSaveError::Missing)
        );
        assert_eq!(settings.overwrite_look("День"), Ok(()));

        let profile = settings.look_profiles.get("День").unwrap();
        assert_eq!(profile.theme, "dark");
        assert_eq!(profile.at.as_deref(), Some("08:00"));
        assert_eq!(settings.look_base.as_deref(), Some("День"));
        assert!(!settings.look_draft);
    }

    #[test]
    fn applying_a_saved_theme_clears_the_draft() {
        let mut settings = Settings::default();
        settings.ui.theme = "light".to_string();
        settings.save_look_as("Утро").unwrap();
        settings.ui.theme = "dark".to_string();
        settings.extra.clear();
        settings.mark_look_draft();

        assert!(settings.apply_saved_look("Утро"));
        assert_eq!(settings.ui.theme, "light");
        assert_eq!(settings.look_base.as_deref(), Some("Утро"));
        assert!(!settings.look_draft);
        assert!(!settings.apply_saved_look("Нет такой"));
    }

    #[test]
    fn look_profiles_survive_roundtrip() {
        let mut s = Settings::default();
        let mut extra = BTreeMap::new();
        extra.insert("accent".to_string(), "#d98fa8".to_string());
        extra.insert("beauty/color.accent".to_string(), "rose".to_string());
        s.look_profiles.insert(
            "Вечер".to_string(),
            LookProfile {
                theme: "dark".to_string(),
                extra,
                at: Some("20:30".to_string()),
            },
        );
        s.look_base = Some("Вечер".to_string());
        s.look_draft = true;
        let text = toml::to_string(&s).unwrap();
        let back: Settings = toml::from_str(&text).unwrap();
        let profile = back.look_profiles.get("Вечер").expect("профиль");
        assert_eq!(profile.theme, "dark");
        assert_eq!(profile.items(), 2);
        assert_eq!(profile.at.as_deref(), Some("20:30"));
        assert_eq!(back.look_base.as_deref(), Some("Вечер"));
        assert!(back.look_draft);
    }

    #[test]
    fn schedule_picks_the_latest_activation() {
        let mut profiles = BTreeMap::new();
        let make = |at: &str| LookProfile {
            theme: "light".to_string(),
            extra: BTreeMap::new(),
            at: Some(at.to_string()),
        };
        profiles.insert("Утро".to_string(), make("08:00"));
        profiles.insert("Вечер".to_string(), make("20:00"));
        profiles.insert("Ручной".to_string(), LookProfile::default());

        assert_eq!(active_profile(&profiles, 7 * 60).as_deref(), Some("Вечер"));
        assert_eq!(active_profile(&profiles, 8 * 60).as_deref(), Some("Утро"));
        assert_eq!(active_profile(&profiles, 13 * 60).as_deref(), Some("Утро"));
        assert_eq!(active_profile(&profiles, 21 * 60).as_deref(), Some("Вечер"));
        assert_eq!(parse_at("23:59"), Some(23 * 60 + 59));
        assert_eq!(parse_at("24:00"), None);
        assert_eq!(parse_at("8-00"), None);
        assert_eq!(parse_at(""), None);
        assert_eq!(active_profile(&BTreeMap::new(), 600), None);
    }

    #[test]
    fn one_variant_per_role_stays_active() {
        let mut settings = Settings::default();
        settings.extra.insert("accent".into(), "#111111".into());
        settings
            .extra
            .insert("starter/font.ui".into(), "sans".into());

        settings.apply_cosmetic("starter/color.accent", "green");
        settings.apply_cosmetic("beauty/color.accent", "rose");
        settings.apply_cosmetic("starter/color.accent", "blue");

        assert_eq!(
            settings
                .extra
                .get("starter/color.accent")
                .map(String::as_str),
            Some("blue")
        );
        assert!(!settings.extra.contains_key("beauty/color.accent"));
        assert_eq!(
            settings.extra.get("accent").map(String::as_str),
            Some("#111111")
        );
        assert_eq!(
            settings.extra.get("starter/font.ui").map(String::as_str),
            Some("sans")
        );
    }

    #[test]
    fn theme_keeps_missing_elements_from_base() {
        let mut settings = Settings::default();
        settings.apply_cosmetic("starter/font.ui", "serif");
        settings.apply_cosmetic("starter/sound.ui", "chime");
        settings.apply_cosmetic("beauty/background.image", "silk");
        settings.apply_theme_items(
            "dark",
            &[(
                "starter/color.accent".to_string(),
                "purple".to_string(),
                "#9b7bd4".to_string(),
            )],
        );

        assert_eq!(settings.ui.theme, "dark");
        assert_eq!(
            settings.extra.get("starter/font.ui").map(String::as_str),
            Some("sans")
        );
        assert_eq!(
            settings.extra.get("starter/sound.ui").map(String::as_str),
            Some("quiet")
        );
        assert!(!settings.extra.contains_key("beauty/background.image"));
        assert_eq!(
            settings
                .extra
                .get("starter/color.accent")
                .map(String::as_str),
            Some("purple")
        );
        assert_eq!(
            settings.extra.get("accent").map(String::as_str),
            Some("#9b7bd4")
        );
    }

    #[test]
    fn background_color_and_image_live_together_but_images_are_exclusive() {
        let mut settings = Settings::default();
        settings.extra.insert("accent".into(), "#d98fa8".into());
        settings.apply_cosmetic("starter/color.background", "sepia");
        settings.apply_cosmetic("starter/background.image", "paper");
        assert!(settings.extra.contains_key("starter/color.background"));
        assert_eq!(
            settings
                .extra
                .get("starter/background.image")
                .map(String::as_str),
            Some("paper")
        );

        settings.apply_cosmetic("beauty/background.image", "silk");
        assert!(!settings.extra.contains_key("starter/background.image"));
        assert_eq!(
            settings
                .extra
                .get("beauty/background.image")
                .map(String::as_str),
            Some("silk")
        );
        assert!(settings.extra.contains_key("starter/color.background"));
        assert!(settings.extra.contains_key("accent"));
    }

    #[test]
    fn dedupe_keeps_one_entry_per_role() {
        let mut settings = Settings::default();
        settings
            .extra
            .insert("starter/color.accent".into(), "green".into());
        settings
            .extra
            .insert("beauty/color.accent".into(), "rose".into());
        settings
            .extra
            .insert("starter/background.image".into(), "paper".into());
        settings.extra.insert("accent".into(), "#111111".into());
        settings.dedupe_cosmetics();
        let accents = settings
            .extra
            .keys()
            .filter(|key| key.ends_with("/color.accent"))
            .count();
        assert_eq!(accents, 1);
        assert!(settings.extra.contains_key("starter/background.image"));
        assert!(settings.extra.contains_key("accent"));
    }

    #[test]
    fn applying_same_role_from_another_pack_replaces_it() {
        let mut settings = Settings::default();
        settings.apply_cosmetic("starter/background.image", "paper");
        settings.apply_cosmetic("beauty/background.image", "dusk");
        let images: Vec<&String> = settings
            .extra
            .keys()
            .filter(|key| key.ends_with("/background.image"))
            .collect();
        assert_eq!(images.len(), 1);
        assert_eq!(images[0], "beauty/background.image");
    }

    #[test]
    fn roundtrip_toml() {
        let s = Settings::default();
        let text = toml::to_string(&s).unwrap();
        let back: Settings = toml::from_str(&text).unwrap();
        assert_eq!(back.ui.theme, s.ui.theme);
    }
}
