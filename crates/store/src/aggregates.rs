//! Кеш статистики: агрегаты, по которым строятся ачивки и графики.

use std::collections::BTreeMap;

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};
use taskify_core::task::Locales;

use crate::state::State;

pub const AGGREGATES_SCHEMA: u32 = 1;

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Totals {
    pub tasks_completed: u64,
    pub subtasks_completed: u64,
    pub xp_earned: i64,
    pub coins_earned: i64,
    pub coins_spent: i64,
    pub rejections: u64,
    pub skips: u64,
    pub combos_completed: u64,
    pub purchases: u64,
    pub achievements: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct DayAgg {
    pub xp: i64,
    pub coins: i64,
    pub completed: u64,
    pub rejections: u64,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TaskAgg {
    pub completions: u64,
    pub xp: i64,
    pub coins: i64,
    #[serde(default)]
    pub title: Locales,
    #[serde(default)]
    pub category: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Aggregates {
    pub schema: u32,
    pub last_seq: u64,
    #[serde(default)]
    pub totals: Totals,
    #[serde(default)]
    pub by_day: BTreeMap<NaiveDate, DayAgg>,
    #[serde(default)]
    pub by_task: BTreeMap<String, TaskAgg>,
    #[serde(default)]
    pub by_category: BTreeMap<String, u64>,
    #[serde(default)]
    pub by_tag: BTreeMap<String, u64>,
    /// Пользовательские метрики из паков.
    #[serde(default)]
    pub metrics: BTreeMap<String, i64>,
    #[serde(default)]
    pub levels_reached: u64,
}

impl Default for Aggregates {
    fn default() -> Self {
        Self {
            schema: AGGREGATES_SCHEMA,
            last_seq: 0,
            totals: Totals::default(),
            by_day: BTreeMap::new(),
            by_task: BTreeMap::new(),
            by_category: BTreeMap::new(),
            by_tag: BTreeMap::new(),
            metrics: BTreeMap::new(),
            levels_reached: 0,
        }
    }
}

impl Aggregates {
    pub fn day_mut(&mut self, date: NaiveDate) -> &mut DayAgg {
        self.by_day.entry(date).or_default()
    }

    pub fn task_mut(&mut self, id: &str) -> &mut TaskAgg {
        self.by_task.entry(id.to_string()).or_default()
    }

    pub fn bump(&mut self, name: &str, delta: i64) {
        let entry = self.metrics.entry(name.to_string()).or_insert(0);
        *entry = entry.saturating_add(delta);
    }

    /// Значение метрики: системной или пользовательской.
    pub fn value(&self, id: &str, state: &State) -> i64 {
        match id {
            "sys.tasks_completed_total" => self.totals.tasks_completed as i64,
            "sys.subtasks_completed_total" => self.totals.subtasks_completed as i64,
            "sys.xp_total" => state.xp_total,
            "sys.coins_earned_total" => self.totals.coins_earned,
            "sys.coins_spent_total" => self.totals.coins_spent,
            "sys.level" => state.level as i64,
            "sys.combos_completed_total" => self.totals.combos_completed as i64,
            "sys.days_active" => state.days_active.len() as i64,
            "sys.tasks_over_norm_today" => state.day.over_norm as i64,
            "sys.xp_today" => state.day.xp,
            "sys.tasks_today" => state.day.completed as i64,
            "sys.achievements_total" => self.totals.achievements as i64,
            _ => {
                if let Some(task) = id.strip_prefix("sys.task_completions.") {
                    return self
                        .by_task
                        .get(task)
                        .map(|entry| entry.completions as i64)
                        .unwrap_or(0);
                }
                if let Some(category) = id.strip_prefix("sys.tasks_by_category.") {
                    return self.by_category.get(category).copied().unwrap_or(0) as i64;
                }
                if let Some(tag) = id.strip_prefix("sys.tasks_by_tag.") {
                    return self.by_tag.get(tag).copied().unwrap_or(0) as i64;
                }
                self.metrics.get(id).copied().unwrap_or(0)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn system_metrics_resolve() {
        let mut aggs = Aggregates::default();
        let state = State {
            xp_total: 500,
            level: 3,
            ..State::default()
        };
        aggs.totals.tasks_completed = 42;
        aggs.task_mut("water").completions = 7;
        aggs.by_category.insert("body".into(), 5);

        assert_eq!(aggs.value("sys.tasks_completed_total", &state), 42);
        assert_eq!(aggs.value("sys.xp_total", &state), 500);
        assert_eq!(aggs.value("sys.level", &state), 3);
        assert_eq!(aggs.value("sys.task_completions.water", &state), 7);
        assert_eq!(aggs.value("sys.tasks_by_category.body", &state), 5);
        assert_eq!(aggs.value("custom.unknown", &state), 0);
    }
}
