//! Событие — единица истории.

use chrono::{DateTime, FixedOffset, NaiveDate};
use serde::{Deserialize, Serialize};

use taskify_core::reward::{MilestoneHit, Reward};
use taskify_core::task::{Locales, Task};

/// Версия формата события. Меняется только через миграторы.
pub const EVENT_SCHEMA: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
    pub schema: u32,
    pub seq: u64,
    pub id: String,
    pub ts: DateTime<FixedOffset>,
    #[serde(flatten)]
    pub kind: EventKind,
}

impl Event {
    pub fn kind_key(&self) -> &'static str {
        self.kind.key()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "data", rename_all = "snake_case")]
pub enum EventKind {
    TaskCompleted(TaskCompleted),
    TaskCompletionReverted(CompletionReverted),
    SubtaskCompleted(SubtaskCompleted),
    TaskRejected(TaskRejected),
    HabitSkipped(HabitSkipped),
    ComboAwarded(ComboAwarded),
    AchievementUnlocked(AchievementUnlocked),
    Purchase(Purchase),
    BundlePurchased(BundlePurchased),
    TaskStarted(TaskStarted),
    TaskStopped(TaskStopped),
    StickerPlaced(StickerPlaced),
    StickerRemoved(StickerRemoved),
    LevelUp(LevelUp),
    PackInstalled(PackInstalled),
    PackUpdated(PackUpdated),
    PackDisabled(PackDisabled),
    SettingsChanged(SettingsChanged),
    WeekClosed(WeekClosed),
    /// Событие неизвестного типа: строка сохраняется, но не применяется.
    Unknown,
}

impl Event {
    /// Разбор строки журнала. Неизвестный тип не считается ошибкой.
    pub fn parse_line(line: &str) -> Result<Event, serde_json::Error> {
        let value: serde_json::Value = serde_json::from_str(line)?;
        let known = value
            .get("type")
            .and_then(|kind| kind.as_str())
            .map(|kind| KNOWN_TYPES.contains(&kind))
            .unwrap_or(false);
        if known {
            return serde_json::from_value(value);
        }
        let schema = value.get("schema").and_then(|v| v.as_u64()).unwrap_or(1) as u32;
        let seq = value.get("seq").and_then(|v| v.as_u64()).unwrap_or(0);
        let id = value
            .get("id")
            .and_then(|v| v.as_str())
            .unwrap_or_default()
            .to_string();
        let ts = value
            .get("ts")
            .and_then(|v| v.as_str())
            .and_then(|text| chrono::DateTime::parse_from_rfc3339(text).ok())
            .unwrap_or_else(|| {
                chrono::DateTime::parse_from_rfc3339("1970-01-01T00:00:00+00:00")
                    .expect("epoch must parse")
            });
        Ok(Event {
            schema,
            seq,
            id,
            ts,
            kind: EventKind::Unknown,
        })
    }
}

pub const KNOWN_TYPES: [&str; 19] = [
    "task_completed",
    "task_completion_reverted",
    "subtask_completed",
    "task_rejected",
    "habit_skipped",
    "combo_awarded",
    "achievement_unlocked",
    "purchase",
    "bundle_purchased",
    "task_started",
    "task_stopped",
    "sticker_placed",
    "sticker_removed",
    "level_up",
    "pack_installed",
    "pack_updated",
    "pack_disabled",
    "settings_changed",
    "week_closed",
];

impl EventKind {
    pub fn key(&self) -> &'static str {
        match self {
            EventKind::TaskCompleted(_) => "task_completed",
            EventKind::TaskCompletionReverted(_) => "task_completion_reverted",
            EventKind::SubtaskCompleted(_) => "subtask_completed",
            EventKind::TaskRejected(_) => "task_rejected",
            EventKind::HabitSkipped(_) => "habit_skipped",
            EventKind::ComboAwarded(_) => "combo_awarded",
            EventKind::AchievementUnlocked(_) => "achievement_unlocked",
            EventKind::Purchase(_) => "purchase",
            EventKind::BundlePurchased(_) => "bundle_purchased",
            EventKind::TaskStarted(_) => "task_started",
            EventKind::TaskStopped(_) => "task_stopped",
            EventKind::StickerPlaced(_) => "sticker_placed",
            EventKind::StickerRemoved(_) => "sticker_removed",
            EventKind::LevelUp(_) => "level_up",
            EventKind::PackInstalled(_) => "pack_installed",
            EventKind::PackUpdated(_) => "pack_updated",
            EventKind::PackDisabled(_) => "pack_disabled",
            EventKind::SettingsChanged(_) => "settings_changed",
            EventKind::WeekClosed(_) => "week_closed",
            EventKind::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskCompleted {
    /// Полный снимок задания: история читается без пака.
    pub task: Task,
    pub base: Reward,
    #[serde(default)]
    pub milestones: Vec<MilestoneHit>,
    pub awarded: Reward,
    pub completions_total: u64,
    pub streak: u32,
    pub local_date: NaiveDate,
    #[serde(default)]
    pub over_norm: bool,
    #[serde(default)]
    pub combos: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionReverted {
    pub target: String,
    pub awarded: Reward,
    pub local_date: NaiveDate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SubtaskCompleted {
    pub task_id: String,
    pub task_title: Locales,
    pub subtask_id: String,
    pub subtask_title: Locales,
    pub awarded: Reward,
    pub local_date: NaiveDate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskRejected {
    pub task: Task,
    pub local_date: NaiveDate,
    #[serde(default)]
    pub reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HabitSkipped {
    pub task_id: String,
    pub local_date: NaiveDate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComboAwarded {
    pub combo: String,
    pub title: Locales,
    pub awarded: Reward,
    #[serde(default)]
    pub members: Vec<String>,
    /// Окно, за которое выдан бонус: дата, начало недели, месяц или `once`.
    #[serde(default)]
    pub window_key: String,
    pub local_date: NaiveDate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AchievementUnlocked {
    pub achievement: String,
    pub title: Locales,
    pub awarded: Reward,
    /// Стикер-награда: попадает на склад вместе с ачивкой.
    #[serde(default)]
    pub sticker: Option<String>,
    pub local_date: NaiveDate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Purchase {
    pub token: String,
    pub variant: String,
    pub title: Locales,
    pub price: i64,
    pub local_date: NaiveDate,
}

/// Один предмет набора.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundleItem {
    pub token: String,
    pub variant: String,
}

/// Куплен набор целиком: все его предметы становятся доступны сразу.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BundlePurchased {
    pub bundle: String,
    pub price: i64,
    pub items: Vec<BundleItem>,
    #[serde(default)]
    pub stickers: Vec<String>,
    pub local_date: NaiveDate,
}

/// Задание взяли в работу.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskStarted {
    pub task_id: String,
    pub local_date: NaiveDate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskStopped {
    pub task_id: String,
}

/// Стикер поставлен на доску: координаты — доли площади доски (0..1).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StickerPlaced {
    pub sticker: String,
    pub x: f32,
    pub y: f32,
    /// Боковая панель; в старых журналах поля нет — стикер считается левым.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub side: Option<String>,
    /// Стикер перетащен вручную, позиция важнее плитки.
    #[serde(default)]
    pub moved: bool,
    /// Пользовательский размер поверх размера из пака; в старых журналах — 1.0.
    #[serde(default = "default_sticker_placement_scale")]
    pub scale: f32,
    /// Поворот вокруг центра в радианах; в старых журналах — 0.
    #[serde(default)]
    pub rotation: f32,
}

fn default_sticker_placement_scale() -> f32 {
    1.0
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StickerRemoved {
    pub sticker: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub side: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LevelUp {
    pub level: u32,
    pub local_date: NaiveDate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackInstalled {
    pub pack: String,
    pub version: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackUpdated {
    pub pack: String,
    pub from: String,
    pub to: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PackDisabled {
    pub pack: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettingsChanged {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WeekClosed {
    pub week_start: NaiveDate,
    pub xp: i64,
    pub completed: u64,
    #[serde(default)]
    pub goal_reached: bool,
    #[serde(default)]
    pub bonus: Reward,
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;
    use taskify_core::task::{Hints, Kind, Schedule};

    fn sample_task() -> Task {
        let mut title = BTreeMap::new();
        title.insert("en".to_string(), "Drink water".to_string());
        Task {
            id: "water".into(),
            kind: Kind::Habit,
            title,
            description: Locales::new(),
            category: None,
            tags: vec![],
            icon: None,
            reward: Reward::new(10, 2),
            milestones: vec![],
            schedule: Schedule::Daily,
            combos: vec![],
            hints: Hints::default(),
            subtasks: vec![],
            metrics: BTreeMap::new(),
            former_ids: vec![],
            rev: 1,
        }
    }

    fn stamp(kind: EventKind) -> Event {
        Event {
            schema: EVENT_SCHEMA,
            seq: 1,
            id: "01H".into(),
            ts: chrono::DateTime::parse_from_rfc3339("2024-03-10T09:00:00+04:00").unwrap(),
            kind,
        }
    }

    fn all_kinds() -> Vec<EventKind> {
        let date = chrono::NaiveDate::from_ymd_opt(2024, 3, 10).unwrap();
        let task = sample_task();
        vec![
            EventKind::TaskCompleted(TaskCompleted {
                task: task.clone(),
                base: Reward::new(10, 2),
                milestones: vec![],
                awarded: Reward::new(10, 2),
                completions_total: 1,
                streak: 1,
                local_date: date,
                over_norm: false,
                combos: vec![],
            }),
            EventKind::TaskCompletionReverted(CompletionReverted {
                target: "abc".into(),
                awarded: Reward::new(10, 2),
                local_date: date,
            }),
            EventKind::SubtaskCompleted(SubtaskCompleted {
                task_id: "water".into(),
                task_title: task.title.clone(),
                subtask_id: "glass".into(),
                subtask_title: Locales::new(),
                awarded: Reward::new(1, 0),
                local_date: date,
            }),
            EventKind::TaskRejected(TaskRejected {
                task: task.clone(),
                local_date: date,
                reason: None,
            }),
            EventKind::HabitSkipped(HabitSkipped {
                task_id: "water".into(),
                local_date: date,
            }),
            EventKind::ComboAwarded(ComboAwarded {
                combo: "morning".into(),
                title: Locales::new(),
                awarded: Reward::new(5, 1),
                members: vec!["water".into()],
                window_key: date.to_string(),
                local_date: date,
            }),
            EventKind::AchievementUnlocked(AchievementUnlocked {
                achievement: "first".into(),
                title: Locales::new(),
                awarded: Reward::new(20, 5),
                sticker: None,
                local_date: date,
            }),
            EventKind::Purchase(Purchase {
                token: "color.bg".into(),
                variant: "night".into(),
                title: Locales::new(),
                price: 30,
                local_date: date,
            }),
            EventKind::LevelUp(LevelUp {
                level: 2,
                local_date: date,
            }),
            EventKind::PackInstalled(PackInstalled {
                pack: "starter".into(),
                version: "1.0.0".into(),
            }),
            EventKind::PackUpdated(PackUpdated {
                pack: "starter".into(),
                from: "1.0.0".into(),
                to: "1.1.0".into(),
            }),
            EventKind::PackDisabled(PackDisabled {
                pack: "starter".into(),
            }),
            EventKind::SettingsChanged(SettingsChanged {
                key: "day_start_hour".into(),
                value: "4".into(),
            }),
            EventKind::WeekClosed(WeekClosed {
                week_start: date,
                xp: 100,
                completed: 10,
                goal_reached: true,
                bonus: Reward::new(5, 0),
            }),
        ]
    }

    #[test]
    fn roundtrip_every_kind() {
        for kind in all_kinds() {
            let event = stamp(kind);
            let json = serde_json::to_string(&event).unwrap();
            let back: Event = serde_json::from_str(&json).unwrap();
            assert_eq!(back.kind_key(), event.kind_key());
            assert_eq!(back.seq, event.seq);
        }
    }

    #[test]
    fn unknown_type_does_not_break_parsing() {
        let json = r#"{"schema":1,"seq":5,"id":"x","ts":"2024-03-10T09:00:00+04:00","type":"future_event","data":{"a":1}}"#;
        let event = Event::parse_line(json).unwrap();
        assert_eq!(event.kind_key(), "unknown");
        assert_eq!(event.seq, 5);
    }

    #[test]
    fn broken_line_reports_position() {
        let err = serde_json::from_str::<Event>("{not json}").unwrap_err();
        assert!(!err.to_string().is_empty());
    }
}
