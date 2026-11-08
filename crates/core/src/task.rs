//! Модель задания.

use std::collections::BTreeMap;
use std::fmt;

use chrono::{NaiveTime, Timelike};
use serde::{Deserialize, Serialize};

use crate::metrics::MetricEffect;
use crate::reward::{Milestone, Reward};

/// Строка на нескольких языках: код языка → текст.
pub type Locales = BTreeMap<String, String>;

pub const KNOWN_LOCALES: [&str; 8] = ["ru", "en", "ja", "uk", "pl", "es", "zh", "de"];

/// Текст на выбранном языке, затем на английском.
pub fn tr(map: &Locales, locale: &str) -> Option<String> {
    map.get(locale)
        .or_else(|| map.get("en"))
        .filter(|value| !value.trim().is_empty())
        .cloned()
}

/// Текст с последним фолбэком на технический id.
pub fn tr_or(map: &Locales, locale: &str, id: &str) -> String {
    tr(map, locale).unwrap_or_else(|| format!("[{id}]"))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    /// Каждый день.
    Habit,
    /// Раз в N дней.
    Repeat,
    /// Ваншот: выполняется один раз и больше не появляется.
    #[serde(alias = "one_shot", alias = "oneshot")]
    Once,
}

impl Kind {
    pub fn key(self) -> &'static str {
        match self {
            Kind::Habit => "habit",
            Kind::Repeat => "repeat",
            Kind::Once => "once",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum Schedule {
    /// Появляется каждый день.
    #[default]
    Daily,
    /// Появляется раз в `days` дней.
    Interval { days: u32 },
    /// Появляется один раз, без даты.
    #[serde(alias = "one_shot", alias = "oneshot")]
    Once,
    /// Не появляется сама, только вручную.
    Manual,
}

/// Время суток `ЧЧ:ММ` (секунды допустимы при разборе, но не нужны окну).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct ClockTime(NaiveTime);

impl ClockTime {
    pub fn parse(value: &str) -> Option<Self> {
        let mut parts = value.trim().split(':');
        let hour: u32 = parts.next()?.parse().ok()?;
        let minute: u32 = parts.next()?.parse().ok()?;
        let second: u32 = match parts.next() {
            Some(second) => second.parse().ok()?,
            None => 0,
        };
        if parts.next().is_some() {
            return None;
        }
        NaiveTime::from_hms_opt(hour, minute, second).map(Self)
    }

    pub fn time(self) -> NaiveTime {
        self.0
    }
}

impl fmt::Display for ClockTime {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.0.second() == 0 {
            write!(f, "{:02}:{:02}", self.0.hour(), self.0.minute())
        } else {
            write!(
                f,
                "{:02}:{:02}:{:02}",
                self.0.hour(),
                self.0.minute(),
                self.0.second()
            )
        }
    }
}

impl Serialize for ClockTime {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

impl<'de> Deserialize<'de> for ClockTime {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::parse(&value).ok_or_else(|| {
            serde::de::Error::custom(format!(
                "неверное время «{value}»: нужно ЧЧ:ММ, например 09:00"
            ))
        })
    }
}

/// Окно, в которое задание можно выполнять.
///
/// `from > to` означает переход через полночь: `22:00–06:00`.
/// `from == to` — задание доступно весь день.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct TimeWindow {
    pub from: ClockTime,
    pub to: ClockTime,
}

impl TimeWindow {
    pub fn contains(&self, time: NaiveTime) -> bool {
        if self.from == self.to {
            return true;
        }
        if self.from < self.to {
            self.from.time() <= time && time <= self.to.time()
        } else {
            time >= self.from.time() || time <= self.to.time()
        }
    }

    pub fn label(&self) -> String {
        format!("{}–{}", self.from, self.to)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HintCost {
    pub amount: f64,
    #[serde(default = "default_currency")]
    pub currency: String,
}

fn default_currency() -> String {
    "RUB".to_string()
}

#[derive(Debug, Clone, PartialEq, Default, Serialize, Deserialize)]
pub struct Hints {
    #[serde(default)]
    pub duration_min: Option<u32>,
    #[serde(default)]
    pub cost: Option<HintCost>,
    #[serde(default)]
    pub requires: Option<Locales>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Subtask {
    pub id: String,
    #[serde(default)]
    pub title: Locales,
    #[serde(default)]
    pub description: Locales,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub reward: Reward,
    #[serde(default)]
    pub metrics: BTreeMap<String, MetricEffect>,
}

impl Subtask {
    pub fn title(&self, locale: &str) -> String {
        tr_or(&self.title, locale, &self.id)
    }
}

/// Настройка случайной группы: из `members` при каждом появлении выбирается
/// одно задание. Само задание-группа задаёт расписание, окно, цепочку и
/// уровни; содержимое карточки берётся у выбранного участника.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
pub struct RandomGroup {
    #[serde(default)]
    pub members: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Task {
    pub id: String,
    #[serde(default = "default_kind")]
    pub kind: Kind,
    #[serde(default)]
    pub title: Locales,
    #[serde(default)]
    pub description: Locales,
    #[serde(default)]
    pub category: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub icon: Option<String>,
    #[serde(default)]
    pub reward: Reward,
    #[serde(default)]
    pub milestones: Vec<Milestone>,
    #[serde(default)]
    pub schedule: Schedule,
    /// Время суток, когда задание доступно. `None` — доступно всегда.
    #[serde(default)]
    pub window: Option<TimeWindow>,
    /// Id задания, которое нужно выполнить хотя бы раз, чтобы открыть это.
    #[serde(default, alias = "previous", alias = "prev")]
    pub after: Option<String>,
    /// Случайная группа: участники и правила выбора.
    #[serde(default)]
    pub random_group: Option<RandomGroup>,
    #[serde(default)]
    pub combos: Vec<String>,
    #[serde(default)]
    pub hints: Hints,
    #[serde(default)]
    pub subtasks: Vec<Subtask>,
    #[serde(default)]
    pub metrics: BTreeMap<String, MetricEffect>,
    #[serde(default)]
    pub former_ids: Vec<String>,
    #[serde(default = "default_rev")]
    pub rev: u32,
}

fn default_kind() -> Kind {
    Kind::Habit
}

fn default_rev() -> u32 {
    1
}

/// Снимок задания в событии: полная копия, читается без пака.
pub type TaskSnapshot = Task;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TaskError {
    EmptyTitle,
    EmptyId,
    BadId,
    InvalidLocale { code: String },
    DuplicateSubtaskId { subtask: String },
    EmptySubtaskTitle { subtask: String },
    IntervalDaysZero,
    MilestoneEveryZero { index: usize },
    MilestoneMultiplierBelowOne { index: usize },
    DuplicateFormerId { id: String },
    FormerIdMatchesSelf { id: String },
    EmptyPrerequisite,
    PrerequisiteMatchesSelf { id: String },
    InvalidCurrency { currency: String },
    EmptyRandomGroup,
    RandomGroupTooSmall,
    RandomGroupSelfMember,
    DuplicateRandomMember { member: String },
}

impl TaskError {
    pub fn code(&self) -> &'static str {
        match self {
            TaskError::EmptyTitle => "T001",
            TaskError::EmptyId | TaskError::BadId => "T002",
            TaskError::InvalidLocale { .. } => "T003",
            TaskError::DuplicateSubtaskId { .. } | TaskError::EmptySubtaskTitle { .. } => "T004",
            TaskError::IntervalDaysZero => "T005",
            TaskError::MilestoneEveryZero { .. }
            | TaskError::MilestoneMultiplierBelowOne { .. } => "T006",
            TaskError::DuplicateFormerId { .. } | TaskError::FormerIdMatchesSelf { .. } => "T007",
            TaskError::EmptyPrerequisite | TaskError::PrerequisiteMatchesSelf { .. } => "T009",
            TaskError::EmptyRandomGroup
            | TaskError::RandomGroupTooSmall
            | TaskError::RandomGroupSelfMember
            | TaskError::DuplicateRandomMember { .. } => "T010",
            TaskError::InvalidCurrency { .. } => "T008",
        }
    }
}

pub fn is_valid_currency(code: &str) -> bool {
    const KNOWN: [&str; 12] = [
        "RUB", "USD", "EUR", "GBP", "JPY", "CNY", "PLN", "UAH", "KZT", "BYN", "CHF", "TRY",
    ];
    KNOWN.contains(&code)
}

pub fn is_valid_id(id: &str) -> bool {
    !id.is_empty()
        && id
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit() || matches!(c, '-' | '_' | '.'))
}

/// Id с префиксом пака: `pack/id`, каждый сегмент — обычный id.
pub fn is_valid_qualified_id(id: &str) -> bool {
    if !id.contains('/') {
        return is_valid_id(id);
    }
    let mut parts = id.split('/');
    match (parts.next(), parts.next(), parts.next()) {
        (Some(pack), Some(local), None) => is_valid_id(pack) && is_valid_id(local),
        _ => false,
    }
}

impl Task {
    pub fn title(&self, locale: &str) -> String {
        tr_or(&self.title, locale, &self.id)
    }

    pub fn description(&self, locale: &str) -> String {
        tr(&self.description, locale).unwrap_or_default()
    }

    pub fn interval_days(&self) -> Option<u32> {
        match self.schedule {
            Schedule::Interval { days } => Some(days),
            _ => None,
        }
    }

    /// Ваншот: задание выполняется ровно один раз и после этого исчезает.
    pub fn is_one_shot(&self) -> bool {
        self.kind == Kind::Once || self.schedule == Schedule::Once
    }

    /// Текущий id или один из прежних.
    pub fn id_in_use(&self, id: &str) -> bool {
        self.id == id || self.former_ids.iter().any(|former| former == id)
    }

    pub fn all_ids(&self) -> Vec<&str> {
        let mut ids = vec![self.id.as_str()];
        ids.extend(self.former_ids.iter().map(String::as_str));
        ids
    }

    pub fn subtask(&self, id: &str) -> Option<&Subtask> {
        self.subtasks.iter().find(|subtask| subtask.id == id)
    }

    pub fn validate(&self) -> Vec<TaskError> {
        let mut errors = Vec::new();

        if self.id.trim().is_empty() {
            errors.push(TaskError::EmptyId);
        } else if !is_valid_qualified_id(&self.id) {
            errors.push(TaskError::BadId);
        }

        if tr(&self.title, "en").is_none() && self.title.is_empty() {
            errors.push(TaskError::EmptyTitle);
        }

        for code in self.title.keys().chain(self.description.keys()) {
            if !KNOWN_LOCALES.contains(&code.as_str()) {
                errors.push(TaskError::InvalidLocale { code: code.clone() });
            }
        }

        if let Schedule::Interval { days } = self.schedule {
            if days == 0 {
                errors.push(TaskError::IntervalDaysZero);
            }
        }

        if let Some(after) = &self.after {
            if after.trim().is_empty() {
                errors.push(TaskError::EmptyPrerequisite);
            } else if after == &self.id {
                errors.push(TaskError::PrerequisiteMatchesSelf { id: after.clone() });
            }
        }

        if let Some(group) = &self.random_group {
            if group.members.is_empty() {
                errors.push(TaskError::EmptyRandomGroup);
            } else if group.members.len() < 2 {
                errors.push(TaskError::RandomGroupTooSmall);
            }
            let mut members = std::collections::BTreeSet::new();
            for member in &group.members {
                if member == &self.id {
                    errors.push(TaskError::RandomGroupSelfMember);
                }
                if !members.insert(member) {
                    errors.push(TaskError::DuplicateRandomMember {
                        member: member.clone(),
                    });
                }
            }
        }

        for (index, milestone) in self.milestones.iter().enumerate() {
            if milestone.every == 0 {
                errors.push(TaskError::MilestoneEveryZero { index });
            }
            if milestone.xp_mult < 1.0 || milestone.coins_mult < 1.0 {
                errors.push(TaskError::MilestoneMultiplierBelowOne { index });
            }
        }

        let mut seen = std::collections::BTreeSet::new();
        for subtask in &self.subtasks {
            if !seen.insert(subtask.id.clone()) {
                errors.push(TaskError::DuplicateSubtaskId {
                    subtask: subtask.id.clone(),
                });
            }
            if tr(&subtask.title, "en").is_none() {
                errors.push(TaskError::EmptySubtaskTitle {
                    subtask: subtask.id.clone(),
                });
            }
        }

        let mut former_seen = std::collections::BTreeSet::new();
        for former in &self.former_ids {
            if former == &self.id {
                errors.push(TaskError::FormerIdMatchesSelf { id: former.clone() });
            }
            if !former_seen.insert(former.clone()) {
                errors.push(TaskError::DuplicateFormerId { id: former.clone() });
            }
        }

        if let Some(cost) = &self.hints.cost {
            if !is_valid_currency(&cost.currency) {
                errors.push(TaskError::InvalidCurrency {
                    currency: cost.currency.clone(),
                });
            }
        }

        errors
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Task {
        let mut title = Locales::new();
        title.insert("en".into(), "Drink water".into());
        title.insert("ru".into(), "Пить воду".into());
        Task {
            id: "water".into(),
            kind: Kind::Habit,
            title,
            description: Locales::new(),
            category: Some("body".into()),
            tags: vec!["health".into()],
            icon: None,
            reward: Reward::new(10, 2),
            milestones: vec![Milestone {
                every: 7,
                xp_mult: 2.0,
                coins_mult: 2.0,
            }],
            schedule: Schedule::Daily,
            window: None,
            after: None,
            random_group: None,
            combos: vec![],
            hints: Hints::default(),
            subtasks: vec![],
            metrics: BTreeMap::new(),
            former_ids: vec![],
            rev: 1,
        }
    }

    #[test]
    fn sample_is_valid() {
        assert!(sample().validate().is_empty());
    }

    #[test]
    fn title_falls_back_through_english() {
        let task = sample();
        assert_eq!(task.title("ru"), "Пить воду");
        assert_eq!(task.title("de"), "Drink water");
        assert_eq!(task.title("xx"), "Drink water");
    }

    #[test]
    fn interval_zero_is_invalid() {
        let mut task = sample();
        task.schedule = Schedule::Interval { days: 0 };
        assert!(task
            .validate()
            .iter()
            .any(|e| matches!(e, TaskError::IntervalDaysZero)));
    }

    #[test]
    fn once_kind_or_schedule_marks_a_one_shot() {
        let mut task = sample();
        assert!(!task.is_one_shot());
        task.kind = Kind::Once;
        assert!(task.is_one_shot());
        task.kind = Kind::Repeat;
        task.schedule = Schedule::Once;
        assert!(task.is_one_shot());
    }

    #[test]
    fn prerequisite_must_be_filled_and_not_self() {
        let mut task = sample();
        task.after = Some(String::new());
        assert!(task
            .validate()
            .iter()
            .any(|error| matches!(error, TaskError::EmptyPrerequisite)));

        task.after = Some(task.id.clone());
        assert!(task
            .validate()
            .iter()
            .any(|error| matches!(error, TaskError::PrerequisiteMatchesSelf { .. })));
    }

    #[test]
    fn duplicate_subtasks_are_caught() {
        let mut task = sample();
        let subtask = Subtask {
            id: "a".into(),
            title: Locales::new(),
            description: Locales::new(),
            tags: vec![],
            reward: Reward::default(),
            metrics: BTreeMap::new(),
        };
        task.subtasks = vec![subtask.clone(), subtask];
        assert!(task
            .validate()
            .iter()
            .any(|e| matches!(e, TaskError::DuplicateSubtaskId { .. })));
    }

    #[test]
    fn former_ids_resolve() {
        let mut task = sample();
        task.id = "water2".into();
        task.former_ids = vec!["water".into()];
        assert!(task.id_in_use("water"));
        assert!(task.id_in_use("water2"));
        assert!(!task.id_in_use("tea"));
    }

    #[test]
    fn random_group_needs_two_distinct_members() {
        let mut task = sample();
        task.random_group = Some(RandomGroup { members: vec![] });
        assert!(task
            .validate()
            .iter()
            .any(|error| matches!(error, TaskError::EmptyRandomGroup)));

        task.random_group = Some(RandomGroup {
            members: vec!["one".into()],
        });
        assert!(task
            .validate()
            .iter()
            .any(|error| matches!(error, TaskError::RandomGroupTooSmall)));

        task.random_group = Some(RandomGroup {
            members: vec!["one".into(), "one".into()],
        });
        assert!(task
            .validate()
            .iter()
            .any(|error| matches!(error, TaskError::DuplicateRandomMember { .. })));

        task.random_group = Some(RandomGroup {
            members: vec!["one".into(), task.id.clone()],
        });
        assert!(task
            .validate()
            .iter()
            .any(|error| matches!(error, TaskError::RandomGroupSelfMember)));
    }

    #[test]
    fn random_group_is_parsed_from_toml() {
        let task: Task = toml::from_str(
            r#"
id = "random"
kind = "habit"
title = { en = "Random" }

[random_group]
members = ["a", "b"]
"#,
        )
        .unwrap();
        let group = task.random_group.expect("группа");
        assert_eq!(group.members, vec!["a", "b"]);
    }

    #[test]
    fn availability_window_is_parsed_and_wraps_midnight() {
        let task: Task = toml::from_str(
            r#"
id = "morning"
kind = "habit"
title = { ru = "Утро", en = "Morning" }

[reward]
xp = 10
coins = 10

[window]
from = "09:00"
to = "18:00"
"#,
        )
        .unwrap();
        let window = task.window.expect("окно");
        assert!(window.contains(NaiveTime::from_hms_opt(9, 0, 0).unwrap()));
        assert!(window.contains(NaiveTime::from_hms_opt(18, 0, 0).unwrap()));
        assert!(!window.contains(NaiveTime::from_hms_opt(18, 1, 0).unwrap()));

        let overnight: TimeWindow = toml::from_str("from = \"22:00\"\nto = \"06:00\"").unwrap();
        assert!(overnight.contains(NaiveTime::from_hms_opt(23, 30, 0).unwrap()));
        assert!(overnight.contains(NaiveTime::from_hms_opt(5, 0, 0).unwrap()));
        assert!(!overnight.contains(NaiveTime::from_hms_opt(12, 0, 0).unwrap()));
        assert_eq!(overnight.label(), "22:00–06:00");
    }

    #[test]
    fn invalid_window_time_is_rejected() {
        let result: Result<TimeWindow, _> = toml::from_str("from = \"25:00\"\nto = \"06:00\"");
        assert!(result.is_err());
    }

    #[test]
    fn toml_roundtrip() {
        let task = sample();
        let text = toml::to_string(&task).unwrap();
        let back: Task = toml::from_str(&text).unwrap();
        assert_eq!(back, task);
    }
}
