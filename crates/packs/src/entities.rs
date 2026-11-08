//! Сущности пака: реестры, задания, комбо, ачивки, косметика.

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use taskify_core::reward::Reward;
use taskify_core::task::{Locales, Task};

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Category {
    pub id: String,
    #[serde(default)]
    pub title: Locales,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub color: Option<String>,
    #[serde(default)]
    pub synonyms: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Tag {
    pub id: String,
    #[serde(default)]
    pub title: Locales,
    #[serde(default)]
    pub synonyms: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TaskType {
    pub id: String,
    #[serde(default)]
    pub title: Locales,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub color: Option<String>,
    #[serde(default)]
    pub default_reward: Reward,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct IconDef {
    pub id: String,
    #[serde(default)]
    pub title: Locales,
    /// Ключ картинки в `assets/icons` (`glyph_1F4A7` и подобные).
    #[serde(default)]
    pub image: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct PaletteColor {
    pub id: String,
    pub light: String,
    pub dark: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Window {
    #[default]
    Day,
    Week,
    Month,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Repeat {
    #[default]
    Once,
    EachWindow,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Combo {
    pub id: String,
    #[serde(default)]
    pub title: Locales,
    #[serde(default)]
    pub description: Locales,
    #[serde(default)]
    pub window: Window,
    #[serde(default)]
    pub repeat: Repeat,
    #[serde(default)]
    pub members: Vec<String>,
    #[serde(default)]
    pub bonus: Reward,
    #[serde(default)]
    pub hidden: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CmpOp {
    Gte,
    Gt,
    Lte,
    Lt,
    Eq,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(untagged)]
pub enum Condition {
    Leaf {
        metric: String,
        #[serde(default = "default_op")]
        op: CmpOp,
        value: i64,
    },
    All {
        all_of: Vec<Condition>,
    },
    Any {
        any_of: Vec<Condition>,
    },
}

fn default_op() -> CmpOp {
    CmpOp::Gte
}

impl Condition {
    pub fn leaves(&self) -> Vec<(&str, CmpOp, i64)> {
        match self {
            Condition::Leaf { metric, op, value } => vec![(metric.as_str(), *op, *value)],
            Condition::All { all_of } => all_of.iter().flat_map(|c| c.leaves()).collect(),
            Condition::Any { any_of } => any_of.iter().flat_map(|c| c.leaves()).collect(),
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Achievement {
    pub id: String,
    #[serde(default)]
    pub title: Locales,
    #[serde(default)]
    pub description: Locales,
    #[serde(default)]
    pub reward: Reward,
    /// Стикер в награду за ачивку (id из каталога стикеров).
    #[serde(default)]
    pub sticker: Option<String>,
    #[serde(default)]
    pub hidden: bool,
    pub condition: Condition,
}

impl Default for Condition {
    fn default() -> Self {
        Condition::Leaf {
            metric: "sys.tasks_completed_total".into(),
            op: CmpOp::Gte,
            value: 1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TokenKind {
    Color,
    Font,
    Image,
    Sound,
    /// Тема оформления: палитра из варианта.
    Theme,
}

/// Цвета темы в паке; незаполненные берутся из встроенной пары.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
pub struct ThemePalette {
    #[serde(default)]
    pub bg: Option<String>,
    #[serde(default)]
    pub panel: Option<String>,
    #[serde(default)]
    pub card: Option<String>,
    #[serde(default)]
    pub field: Option<String>,
    #[serde(default)]
    pub text: Option<String>,
    #[serde(default)]
    pub muted: Option<String>,
    #[serde(default)]
    pub accent: Option<String>,
    #[serde(default)]
    pub success: Option<String>,
    #[serde(default)]
    pub success_bg: Option<String>,
    #[serde(default)]
    pub danger: Option<String>,
    #[serde(default)]
    pub on_danger: Option<String>,
    #[serde(default)]
    pub warning: Option<String>,
    #[serde(default)]
    pub coin: Option<String>,
    #[serde(default)]
    pub border: Option<String>,
    #[serde(default)]
    pub shadow: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Unlock {
    Free,
    Coins { price: i64 },
    Level { level: u32 },
    Achievement { id: String },
    PackLevel { level: u32 },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Variant {
    pub id: String,
    pub value: String,
    #[serde(default)]
    pub title: Locales,
    #[serde(default = "free_unlock")]
    pub unlock: Unlock,
    /// Только для тем: мягкая палитра.
    #[serde(default)]
    pub palette: Option<ThemePalette>,
}

fn free_unlock() -> Unlock {
    Unlock::Free
}

/// База, от которой наследуется тема: светлая или тёмная.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum ThemeBase {
    #[default]
    Light,
    Dark,
}

impl ThemeBase {
    pub fn as_str(self) -> &'static str {
        match self {
            ThemeBase::Light => "light",
            ThemeBase::Dark => "dark",
        }
    }
}

/// Тема: набор элементов оформления одной ценой.
///
/// Стикеры в темы не входят: они покупаются и вешаются отдельно.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bundle {
    pub id: String,
    pub price: i64,
    /// Базовая тема, из которой берутся недостающие элементы.
    ///
    /// `None` — старый пак без поля: база угадывается по тёмному токену темы
    /// или фона, иначе считается светлой.
    #[serde(default)]
    pub base: Option<ThemeBase>,
    #[serde(default)]
    pub title: Locales,
    #[serde(default)]
    pub description: Locales,
    /// Ссылки `<токен>:<вариант>`; без пака — токен своего пака.
    #[serde(default)]
    pub tokens: Vec<String>,
    /// Устаревшее поле: старые паки могли класть стикеры в тему.
    /// Читается только для предупреждения валидатора и нигде не применяется.
    #[serde(default)]
    pub stickers: Vec<String>,
}

impl Bundle {
    /// База темы: явная из `base` или выведенная из тёмных токенов.
    pub fn theme_base(&self) -> ThemeBase {
        if let Some(base) = self.base {
            return base;
        }
        for item in &self.tokens {
            let Some((token, variant)) = item.split_once(':') else {
                continue;
            };
            if variant == "dark"
                && (token.ends_with("theme") || token.ends_with("color.background"))
            {
                return ThemeBase::Dark;
            }
        }
        ThemeBase::Light
    }
    pub fn title(&self, locale: &str) -> String {
        taskify_core::task::tr_or(&self.title, locale, &self.id)
    }

    pub fn describe(&self, locale: &str) -> String {
        taskify_core::task::tr_or(&self.description, locale, "")
    }

    /// Сколько элементов оформления в теме. Стикеры не считаются.
    pub fn contents_count(&self) -> usize {
        self.tokens.len()
    }
}

/// Стикер, объявленный паком.
///
/// Хранится с id вида `<пак>/<стикер>`. Можно указать либо `icon` — ключ
/// картинки из общего каталога иконок, либо `image` — PNG внутри пака.
/// `scale` задаёт размер относительно обычного слота (0.25–1.0); форма
/// картинки сохраняется, поэтому прозрачный PNG может быть широким или
/// высоким, а не только круглым.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct StickerDef {
    pub id: String,
    #[serde(default)]
    pub icon: String,
    /// Путь к PNG относительно корня пака, например `media/goose.png`.
    #[serde(default)]
    pub image: String,
    /// Размер относительно слота стикеров.
    #[serde(default = "default_sticker_scale")]
    pub scale: f32,
    #[serde(default)]
    pub price: i64,
    #[serde(default)]
    pub name: Locales,
}

fn default_sticker_scale() -> f32 {
    1.0
}

impl StickerDef {
    pub fn title(&self, locale: &str) -> String {
        taskify_core::task::tr_or(&self.name, locale, &self.id)
    }

    /// Абсолютный путь к PNG пака, если он задан и не выходит за корень пака.
    pub fn image_path(&self, root: &Path) -> Option<PathBuf> {
        let image = self.image.trim();
        let path = Path::new(image);
        if image.is_empty() || path.is_absolute() || image.contains("..") {
            return None;
        }
        Some(root.join(path))
    }

    /// Безопасный размер: не даём стикеру раздуться и перекрыть соседей.
    pub fn display_scale(&self) -> f32 {
        if self.scale.is_finite() {
            self.scale.clamp(0.25, 1.0)
        } else {
            1.0
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Token {
    pub id: String,
    #[serde(rename = "type")]
    pub kind: TokenKind,
    #[serde(default)]
    pub group: String,
    #[serde(default)]
    pub title: Locales,
    #[serde(default)]
    pub variants: Vec<Variant>,
}

/// Уровни пака, между которыми задание видно.
///
/// `from` — с какого уровня пака задание появляется (None — сразу),
/// `to` — на каком уровне пропадает (None — не пропадает).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, serde::Deserialize)]
pub struct LevelGate {
    #[serde(default)]
    pub from: Option<u32>,
    #[serde(default)]
    pub to: Option<u32>,
}

impl LevelGate {
    pub fn visible(&self, level: u32) -> bool {
        self.from.map(|from| level >= from).unwrap_or(true)
            && self.to.map(|to| level <= to).unwrap_or(true)
    }

    pub fn is_empty(&self) -> bool {
        self.from.is_none() && self.to.is_none()
    }
}

/// Загруженный пак со всеми реестрами.
#[derive(Debug, Clone, Default)]
pub struct Pack {
    pub id: String,
    pub version: String,
    pub manifest: Option<crate::manifest::Manifest>,
    pub root: std::path::PathBuf,
    pub categories: std::collections::BTreeMap<String, Category>,
    pub tags: std::collections::BTreeMap<String, Tag>,
    pub types: std::collections::BTreeMap<String, TaskType>,
    pub icons: std::collections::BTreeMap<String, IconDef>,
    pub palette: std::collections::BTreeMap<String, PaletteColor>,
    pub tasks: std::collections::BTreeMap<String, Task>,
    /// Случайные группы: отдельные сущности с условиями появления.
    pub groups: std::collections::BTreeMap<String, taskify_core::group::RandomGroup>,
    pub combos: std::collections::BTreeMap<String, Combo>,
    pub achievements: std::collections::BTreeMap<String, Achievement>,
    pub tokens: std::collections::BTreeMap<String, Token>,
    pub bundles: std::collections::BTreeMap<String, Bundle>,
    /// Стикеры, объявленные паком: id уже с префиксом пака.
    pub stickers: std::collections::BTreeMap<String, StickerDef>,
    /// Уровневые рамки заданий: id задания → от/до какого уровня пака.
    pub task_levels: std::collections::BTreeMap<String, LevelGate>,
    /// Объявленные паком метрики.
    pub metrics_declared: Vec<String>,
    pub enabled: bool,
}

#[cfg(test)]
mod gate_tests {
    use super::LevelGate;

    #[test]
    fn gate_is_open_by_default() {
        let gate = LevelGate::default();
        assert!(gate.is_empty());
        assert!(gate.visible(1));
        assert!(gate.visible(999));
    }

    #[test]
    fn gate_bounds_are_inclusive() {
        let gate = LevelGate {
            from: Some(5),
            to: Some(10),
        };
        assert!(!gate.visible(4));
        assert!(gate.visible(5));
        assert!(gate.visible(10));
        assert!(!gate.visible(11));
        assert!(!gate.is_empty());
    }

    #[test]
    fn bundle_contents_are_decoration_only() {
        let bundle = super::Bundle {
            id: "test/theme".into(),
            price: 100,
            base: None,
            title: Default::default(),
            description: Default::default(),
            tokens: vec!["color.accent:mint".into(), "sound.ui:soft".into()],
            // Старое поле читается, но в состав темы не входит.
            stickers: vec!["test/cat".into()],
        };
        assert_eq!(bundle.contents_count(), 2);
    }

    #[test]
    fn gate_with_only_from_has_no_upper_bound() {
        let gate = LevelGate {
            from: Some(7),
            to: None,
        };
        assert!(!gate.visible(6));
        assert!(gate.visible(700));
    }
}
