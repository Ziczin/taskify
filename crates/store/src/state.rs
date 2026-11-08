//! Кеш состояния профиля.

use std::collections::{BTreeMap, BTreeSet};

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use taskify_core::level::level_for;
use taskify_core::time::DayRules;

pub const STATE_SCHEMA: u32 = 2;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TaskState {
    pub completions_total: u64,
    pub streak_current: u32,
    pub streak_max: u32,
    pub last_done: Option<NaiveDate>,
    /// Последнее оплаченное вхождение (день для привычки, дата для повтора).
    pub last_occurrence: Option<NaiveDate>,
    pub next_due: Option<NaiveDate>,
    pub last_rejected: Option<NaiveDate>,
    pub last_skipped: Option<NaiveDate>,
    /// Отмеченные подзадачи текущего вхождения.
    #[serde(default)]
    pub subtasks_done: BTreeSet<String>,
    #[serde(default)]
    pub subtasks_date: Option<NaiveDate>,
    #[serde(default)]
    pub rejected_total: u64,
    /// День, когда задание взяли в работу (для подсветки и фильтра).
    #[serde(default)]
    pub started: Option<NaiveDate>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DayState {
    pub date: Option<NaiveDate>,
    pub completed: u64,
    pub xp: i64,
    pub coins: i64,
    pub rejected: u64,
    pub over_norm: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct WeekState {
    pub start: Option<NaiveDate>,
    pub xp: i64,
    pub completed: u64,
    pub closed: bool,
    pub goal_rewarded: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ComboState {
    /// Ключ окна, в котором считается прогресс.
    pub window: String,
    pub members_done: BTreeSet<String>,
    /// Окна, в которых бонус уже выдан.
    pub awarded: BTreeSet<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct State {
    pub schema: u32,
    pub last_seq: u64,
    pub xp_total: i64,
    pub coins: i64,
    pub level: u32,
    #[serde(default)]
    pub tasks: BTreeMap<String, TaskState>,
    #[serde(default)]
    pub day: DayState,
    #[serde(default)]
    pub week: WeekState,
    #[serde(default)]
    pub combos: BTreeMap<String, ComboState>,
    #[serde(default)]
    pub purchased: BTreeSet<String>,
    #[serde(default)]
    pub achievements: BTreeSet<String>,
    #[serde(default)]
    pub days_active: BTreeSet<NaiveDate>,
    #[serde(default)]
    pub packs: BTreeMap<String, String>,
    #[serde(default)]
    pub packs_disabled: BTreeSet<String>,
    /// Задания, которые пользователь уже видел. Новые id подсвечиваются.
    #[serde(default)]
    pub viewed_tasks: BTreeSet<String>,
    /// Версия разовой заливки «уже виденных»; 0 — заливки ещё не было.
    #[serde(default)]
    pub seen_seed_version: u32,
    /// Задания, о появлении которых уже отправляли системное уведомление.
    #[serde(default)]
    pub notified_tasks: BTreeSet<String>,
    /// Купленные стикеры и их место на боковых панелях: `сторона:id` →
    /// позиция. Один стикер может висеть и слева, и справа.
    #[serde(default)]
    pub stickers: BTreeMap<String, StickerPlacement>,
}

/// Сторона, на которой висит стикер.
pub const STICKER_SIDES: [&str; 2] = ["left", "right"];

/// Ключ стикера на панели: `left:star`, `right:star`.
pub fn sticker_slot(side: &str, sticker: &str) -> String {
    format!("{side}:{sticker}")
}

/// Нижняя граница пользовательского размера: защита от нуля и NaN.
pub const STICKER_MIN_SCALE: f32 = 0.05;
/// Верхняя граница — не художественное ограничение, а защита от чисел,
/// которые уже нельзя нарисовать. 64× — в 32 раза больше прежних 200 %.
pub const STICKER_MAX_SCALE: f32 = 64.0;

/// Безопасный размер: NaN и бесконечность не должны ломать раскладку.
pub fn clamp_sticker_scale(scale: f32) -> f32 {
    if scale.is_finite() {
        scale.clamp(STICKER_MIN_SCALE, STICKER_MAX_SCALE)
    } else {
        1.0
    }
}

/// Поворот в радианах, приведённый к диапазону `0..2π`.
pub fn normalize_sticker_rotation(rotation: f32) -> f32 {
    if rotation.is_finite() {
        rotation.rem_euclid(std::f32::consts::TAU)
    } else {
        0.0
    }
}

/// Положение стикера на боковой панели в долях её ширины и высоты.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct StickerPlacement {
    pub x: f32,
    pub y: f32,
    /// `left` или `right`; у старых записей поля нет — считается левой.
    #[serde(default = "default_side")]
    pub side: String,
    /// Стикер сдвинут вручную. До первого перетаскивания он лежит в плитке.
    #[serde(default)]
    pub moved: bool,
    /// Пользовательский размер поверх `scale` из пака.
    #[serde(default = "default_sticker_placement_scale")]
    pub scale: f32,
    /// Поворот картинки вокруг центра, радианы.
    #[serde(default)]
    pub rotation: f32,
}

fn default_side() -> String {
    "left".to_string()
}

fn default_sticker_placement_scale() -> f32 {
    1.0
}

impl Default for StickerPlacement {
    fn default() -> Self {
        Self {
            x: 0.0,
            y: 0.0,
            side: default_side(),
            moved: false,
            scale: 1.0,
            rotation: 0.0,
        }
    }
}

impl Default for State {
    fn default() -> Self {
        Self {
            schema: STATE_SCHEMA,
            last_seq: 0,
            xp_total: 0,
            coins: 0,
            level: 1,
            tasks: BTreeMap::new(),
            day: DayState::default(),
            week: WeekState::default(),
            combos: BTreeMap::new(),
            purchased: BTreeSet::new(),
            achievements: BTreeSet::new(),
            days_active: BTreeSet::new(),
            packs: BTreeMap::new(),
            packs_disabled: BTreeSet::new(),
            viewed_tasks: BTreeSet::new(),
            seen_seed_version: 0,
            notified_tasks: BTreeSet::new(),
            stickers: BTreeMap::new(),
        }
    }
}

impl State {
    pub fn task(&mut self, id: &str) -> &mut TaskState {
        self.tasks.entry(id.to_string()).or_default()
    }

    pub fn task_ref(&self, id: &str) -> Option<&TaskState> {
        self.tasks.get(id)
    }

    pub fn add_xp(&mut self, xp: i64) {
        self.xp_total = self.xp_total.saturating_add(xp);
        self.level = level_for(self.xp_total);
    }

    pub fn add_coins(&mut self, coins: i64) {
        self.coins = self.coins.saturating_add(coins);
    }

    pub fn owns(&self, key: &str) -> bool {
        self.purchased.contains(key)
    }

    /// Переключает день и неделю, если событие относится к другому дню.
    pub fn rollover(&mut self, date: NaiveDate, rules: &DayRules) {
        if self.day.date != Some(date) {
            self.day = DayState {
                date: Some(date),
                ..DayState::default()
            };
        }
        let week_start = Some(rules.week_start(date));
        if self.week.start != week_start {
            self.week = WeekState {
                start: week_start,
                ..WeekState::default()
            };
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rollover_resets_day_but_keeps_week() {
        let mut state = State::default();
        let rules = DayRules::default();
        let monday = NaiveDate::from_ymd_opt(2024, 3, 4).unwrap();
        state.rollover(monday, &rules);
        state.day.completed = 3;
        state.week.completed = 3;

        state.rollover(monday, &rules);
        assert_eq!(state.day.completed, 3);

        let tuesday = monday + chrono::Duration::days(1);
        state.rollover(tuesday, &rules);
        assert_eq!(state.day.completed, 0);
        assert_eq!(state.week.completed, 3);

        let next_monday = monday + chrono::Duration::days(7);
        state.rollover(next_monday, &rules);
        assert_eq!(state.week.completed, 0);
    }

    #[test]
    fn level_follows_xp() {
        let mut state = State::default();
        state.add_xp(49);
        assert_eq!(state.level, 1);
        state.add_xp(1);
        assert_eq!(state.level, 2);
        state.add_xp(60);
        assert_eq!(state.level, 3);
        state.add_xp(70);
        assert_eq!(state.level, 4);
    }
}
