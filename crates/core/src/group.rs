//! Случайная группа: отдельная сущность с условиями появления.
//!
//! Состав группы хранится не здесь: задания-участники ссылаются на группу
//! полем `group`. Группа задаёт расписание, окно, цепочку, тип и уровни,
//! а карточка берётся у выбранного участника.

use serde::{Deserialize, Serialize};

use crate::task::{is_valid_id, Kind, Locales, Schedule, TimeWindow};

fn default_kind() -> Kind {
    Kind::Habit
}

fn default_pick() -> usize {
    1
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct RandomGroup {
    pub id: String,
    #[serde(default)]
    pub title: Locales,
    #[serde(default = "default_kind")]
    pub kind: Kind,
    #[serde(default)]
    pub schedule: Schedule,
    #[serde(default)]
    pub window: Option<TimeWindow>,
    #[serde(default, alias = "previous", alias = "prev")]
    pub after: Option<String>,
    #[serde(default)]
    pub level_from: Option<u32>,
    #[serde(default)]
    pub level_to: Option<u32>,
    /// Общий пул участников. Несколько групп могут ссылаться на один пул,
    /// не дублируя задания.
    #[serde(default)]
    pub pool: Option<String>,
    /// Сколько участников выбирать за один розыгрыш.
    #[serde(default = "default_pick", alias = "draw", alias = "count")]
    pub pick: usize,
    /// Служебная группа-пул: не показывается в «Сегодня».
    #[serde(default)]
    pub hidden: bool,
}

impl Default for RandomGroup {
    fn default() -> Self {
        Self {
            id: String::new(),
            title: Locales::new(),
            kind: Kind::Habit,
            schedule: Schedule::Daily,
            window: None,
            after: None,
            level_from: None,
            level_to: None,
            pool: None,
            pick: 1,
            hidden: false,
        }
    }
}

impl RandomGroup {
    pub fn is_one_shot(&self) -> bool {
        self.kind == Kind::Once || self.schedule == Schedule::Once
    }

    pub fn visible(&self, level: u32) -> bool {
        self.level_from.map(|from| level >= from).unwrap_or(true)
            && self.level_to.map(|to| level <= to).unwrap_or(true)
    }

    pub fn validate(&self) -> Vec<GroupError> {
        let mut errors = Vec::new();
        if self.id.trim().is_empty() {
            errors.push(GroupError::EmptyId);
        } else if !is_valid_id(&self.id) {
            errors.push(GroupError::BadId);
        }
        if self.title.values().all(|value| value.trim().is_empty()) {
            errors.push(GroupError::EmptyTitle);
        }
        if let Schedule::Interval { days } = self.schedule {
            if days == 0 {
                errors.push(GroupError::IntervalDaysZero);
            }
        }
        if let Some(after) = &self.after {
            if after.trim().is_empty() {
                errors.push(GroupError::EmptyPrerequisite);
            } else if after == &self.id {
                errors.push(GroupError::PrerequisiteMatchesSelf { id: after.clone() });
            }
        }
        if let Some(from) = self.level_from {
            if let Some(to) = self.level_to {
                if from > to {
                    errors.push(GroupError::BadLevelRange { from, to });
                }
            }
        }
        if self.pick == 0 {
            errors.push(GroupError::PickZero);
        }
        if let Some(pool) = &self.pool {
            if pool.trim().is_empty() {
                errors.push(GroupError::EmptyPool);
            }
        }
        errors
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum GroupError {
    EmptyId,
    BadId,
    EmptyTitle,
    IntervalDaysZero,
    EmptyPrerequisite,
    PrerequisiteMatchesSelf { id: String },
    BadLevelRange { from: u32, to: u32 },
    PickZero,
    EmptyPool,
}

impl GroupError {
    pub fn code(&self) -> &'static str {
        match self {
            GroupError::EmptyId | GroupError::BadId => "G001",
            GroupError::EmptyTitle => "G002",
            GroupError::IntervalDaysZero => "G003",
            GroupError::EmptyPrerequisite | GroupError::PrerequisiteMatchesSelf { .. } => "G004",
            GroupError::BadLevelRange { .. } => "G005",
            GroupError::PickZero | GroupError::EmptyPool => "G006",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn group() -> RandomGroup {
        RandomGroup {
            id: "random".into(),
            title: [("en".to_string(), "Random".to_string())].into(),
            kind: Kind::Habit,
            schedule: Schedule::Daily,
            window: None,
            after: None,
            level_from: None,
            level_to: None,
            ..RandomGroup::default()
        }
    }

    #[test]
    fn group_is_valid_by_default() {
        assert!(group().validate().is_empty());
    }

    #[test]
    fn group_checks_title_schedule_and_levels() {
        let mut broken = group();
        broken.title.clear();
        broken.schedule = Schedule::Interval { days: 0 };
        broken.level_from = Some(3);
        broken.level_to = Some(1);
        let codes: Vec<&str> = broken.validate().iter().map(GroupError::code).collect();
        assert!(codes.contains(&"G002"));
        assert!(codes.contains(&"G003"));
        assert!(codes.contains(&"G005"));
    }

    #[test]
    fn group_rejects_bad_id_and_self_prerequisite() {
        let mut broken = group();
        broken.id = "Bad Id".into();
        broken.after = Some("Bad Id".into());
        let errors = broken.validate();
        assert!(errors
            .iter()
            .any(|error| matches!(error, GroupError::BadId)));
        assert!(errors
            .iter()
            .any(|error| matches!(error, GroupError::PrerequisiteMatchesSelf { .. })));
        assert_eq!(GroupError::BadId.code(), "G001");
        assert_eq!(
            GroupError::PrerequisiteMatchesSelf { id: "x".into() }.code(),
            "G004"
        );
    }

    #[test]
    fn group_rejects_empty_prerequisite() {
        let mut broken = group();
        broken.after = Some("  ".into());
        assert!(broken
            .validate()
            .iter()
            .any(|error| matches!(error, GroupError::EmptyPrerequisite)));
    }

    #[test]
    fn group_pool_and_pick_are_validated() {
        let mut broken = group();
        broken.pool = Some("  ".into());
        broken.pick = 0;
        let codes: Vec<&str> = broken.validate().iter().map(GroupError::code).collect();
        assert!(codes.contains(&"G006"));
        assert_eq!(GroupError::PickZero.code(), "G006");
        assert_eq!(GroupError::EmptyPool.code(), "G006");

        let mut valid = group();
        valid.pool = Some("food".into());
        valid.pick = 2;
        valid.hidden = true;
        assert!(valid.validate().is_empty());
    }

    #[test]
    fn group_toml_accepts_pool_pick_and_hidden() {
        let group: RandomGroup = toml::from_str(
            "id = \"warmup\"\npool = \"exercises\"\npick = 2\nhidden = true\n[title]\nen = \"Warmup\"\n",
        )
        .unwrap();
        assert_eq!(group.pool.as_deref(), Some("exercises"));
        assert_eq!(group.pick, 2);
        assert!(group.hidden);
    }

    #[test]
    fn group_visibility_bounds_are_inclusive() {
        let mut limited = group();
        limited.level_from = Some(2);
        limited.level_to = Some(4);
        assert!(!limited.visible(1));
        assert!(limited.visible(2));
        assert!(limited.visible(4));
        assert!(!limited.visible(5));
    }

    #[test]
    fn group_is_one_shot_by_kind_or_schedule() {
        let mut by_kind = group();
        by_kind.kind = Kind::Once;
        assert!(by_kind.is_one_shot());

        let mut by_schedule = group();
        by_schedule.kind = Kind::Habit;
        by_schedule.schedule = Schedule::Once;
        assert!(by_schedule.is_one_shot());

        let mut interval = group();
        interval.schedule = Schedule::Interval { days: 2 };
        assert!(!interval.is_one_shot());
    }

    #[test]
    fn group_after_aliases_parse_from_toml() {
        let previous: RandomGroup = toml::from_str("id = \"g\"\nprevious = \"before\"\n").unwrap();
        assert_eq!(previous.after.as_deref(), Some("before"));
        let prev: RandomGroup = toml::from_str("id = \"g\"\nprev = \"before\"\n").unwrap();
        assert_eq!(prev.after.as_deref(), Some("before"));
    }
}
