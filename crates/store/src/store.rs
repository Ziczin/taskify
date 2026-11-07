//! Фасад хранилища: открывает профиль, пишет события, держит кеши в актуальном виде.

use std::path::Path;
use std::time::Duration;

use chrono::{DateTime, FixedOffset, Local};
use taskify_core::level::level_for;

use crate::aggregates::{Aggregates, AGGREGATES_SCHEMA};
use crate::apply::{apply, Rules};
use crate::backup::{self, Tier};
use crate::cache;
use crate::event::{Event, EventKind, EVENT_SCHEMA};
use crate::livespace::{GameSettings, Livespace};
use crate::log::{self, LogWriter};
use crate::replay;
use crate::state::{State, STATE_SCHEMA};

pub struct Store {
    pub livespace: Livespace,
    pub settings: GameSettings,
    pub state: State,
    pub aggregates: Aggregates,
    pub warnings: Vec<String>,
    log: LogWriter,
    events_since_backup: u64,
}

impl Store {
    pub fn open(root: impl AsRef<Path>) -> std::io::Result<Self> {
        let livespace = Livespace::open(root)?;
        let settings = livespace.load_settings();
        let rules = Self::rules_for(&settings);

        let cached_state: Option<State> = cache::read_json(&livespace.state_file());
        let cached_aggregates: Option<Aggregates> = cache::read_json(&livespace.aggregates_file());

        let (mut state, aggregates, warnings, last_seq) = match (cached_state, cached_aggregates) {
            (Some(state), Some(aggregates))
                if state.schema == STATE_SCHEMA && aggregates.schema == AGGREGATES_SCHEMA =>
            {
                if Self::log_is_newer(&livespace, &state) {
                    let outcome = log::read_all(&livespace.log_dir())?;
                    let result = replay::rebuild(&outcome.events, &rules);
                    let mut warnings = outcome.warnings;
                    warnings.extend(result.warnings);
                    (result.state, result.aggregates, warnings, 0u64)
                } else {
                    let seq = state.last_seq;
                    (state, aggregates, Vec::new(), seq)
                }
            }
            _ => {
                let outcome = log::read_all(&livespace.log_dir())?;
                let result = replay::rebuild(&outcome.events, &rules);
                let mut warnings = outcome.warnings;
                warnings.extend(result.warnings);
                let seq = result.state.last_seq;
                (result.state, result.aggregates, warnings, seq)
            }
        };

        let log = LogWriter::open(livespace.log_dir(), last_seq.max(state.last_seq))?;
        // Уровень — производное от опыта, а не отдельное достижение. Пересчёт
        // при открытии важен после смены кривой: в кеше мог остаться старый.
        state.level = level_for(state.xp_total);

        let store = Self {
            livespace,
            settings,
            state,
            aggregates,
            warnings,
            log,
            events_since_backup: 0,
        };
        store.save_caches()?;
        Ok(store)
    }

    pub fn rules_for(settings: &GameSettings) -> Rules {
        Rules {
            days: settings.rules(),
            daily_norm: settings.daily_norm,
            weekly_goal: settings.weekly_xp_goal,
        }
    }

    pub fn rules(&self) -> Rules {
        Self::rules_for(&self.settings)
    }

    fn log_is_newer(livespace: &Livespace, _state: &State) -> bool {
        let Some(state_time) = std::fs::metadata(livespace.state_file())
            .and_then(|meta| meta.modified())
            .ok()
        else {
            return true;
        };
        let mut newest = state_time;
        if let Ok(entries) = std::fs::read_dir(livespace.log_dir()) {
            for entry in entries.flatten() {
                if let Ok(modified) = entry.metadata().and_then(|meta| meta.modified()) {
                    if modified > newest {
                        newest = modified;
                    }
                }
            }
        }
        newest > state_time
    }

    pub fn record(&mut self, kind: EventKind) -> std::io::Result<Event> {
        self.record_at(kind, Local::now().fixed_offset())
    }

    pub fn record_at(
        &mut self,
        kind: EventKind,
        ts: DateTime<FixedOffset>,
    ) -> std::io::Result<Event> {
        let seq = self.log.assign_seq();
        let event = Event {
            schema: EVENT_SCHEMA,
            seq,
            id: uuid::Uuid::new_v4().to_string(),
            ts,
            kind,
        };
        self.log.append(&event)?;
        let rules = self.rules();
        apply(&mut self.state, &mut self.aggregates, &event, &rules);
        self.save_caches()?;
        self.events_since_backup += 1;
        self.maybe_backup();
        Ok(event)
    }

    pub fn save_caches(&self) -> std::io::Result<()> {
        cache::write_json(&self.livespace.state_file(), &self.state)?;
        cache::write_json(&self.livespace.aggregates_file(), &self.aggregates)
    }

    /// Полный пересчёт из журнала: кеши удаляются и собираются заново.
    pub fn rebuild(&mut self) -> std::io::Result<usize> {
        let outcome = log::read_all(&self.livespace.log_dir())?;
        let result = replay::rebuild(&outcome.events, &self.rules());
        self.warnings = outcome.warnings;
        self.warnings.extend(result.warnings);
        self.state = result.state;
        self.aggregates = result.aggregates;
        self.log = LogWriter::open(self.livespace.log_dir(), self.state.last_seq)?;
        self.save_caches()?;
        Ok(result.events)
    }

    /// Сравнение кеша с журналом: возвращает список расхождений.
    pub fn verify(&mut self) -> std::io::Result<Vec<String>> {
        let outcome = log::read_all(&self.livespace.log_dir())?;
        let result = replay::rebuild(&outcome.events, &self.rules());
        let mut problems = outcome.warnings;
        problems.extend(result.warnings);
        if result.state.xp_total != self.state.xp_total {
            problems.push(format!(
                "XP: кеш {}, журнал {}",
                self.state.xp_total, result.state.xp_total
            ));
        }
        if result.state.coins != self.state.coins {
            problems.push(format!(
                "coins: кеш {}, журнал {}",
                self.state.coins, result.state.coins
            ));
        }
        if result.aggregates.totals.tasks_completed != self.aggregates.totals.tasks_completed {
            problems.push(format!(
                "выполнений: кеш {}, журнал {}",
                self.aggregates.totals.tasks_completed, result.aggregates.totals.tasks_completed
            ));
        }
        Ok(problems)
    }

    pub fn save_settings(&mut self, settings: GameSettings) -> std::io::Result<()> {
        self.livespace.save_settings(&settings)?;
        self.settings = settings;
        Ok(())
    }

    fn maybe_backup(&mut self) {
        if !self.settings.backups.enabled {
            return;
        }
        let now = Local::now();
        let settings = self.settings.backups.clone();
        let seq = self.state.last_seq;
        let tiers = [
            (
                Tier::Minutes,
                Duration::from_secs(settings.minute_interval.max(1) * 60),
            ),
            (
                Tier::Hourly,
                Duration::from_secs(settings.hour_interval.max(1) * 3600),
            ),
            (Tier::Daily, Duration::from_secs(24 * 3600)),
            (Tier::Weekly, Duration::from_secs(7 * 24 * 3600)),
        ];
        for (tier, interval) in tiers {
            if backup::due(&self.livespace, tier, now, interval) {
                let keep = tier.keep(&settings);
                let _ = backup::create(&self.livespace, tier, now, seq, keep);
            }
        }
    }

    pub fn backup_now(&self, tier: Tier) -> std::io::Result<std::path::PathBuf> {
        let keep = tier.keep(&self.settings.backups);
        backup::create(
            &self.livespace,
            tier,
            Local::now(),
            self.state.last_seq,
            keep,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{HabitSkipped, Purchase, TaskCompleted};
    use std::collections::BTreeMap;
    use taskify_core::reward::Reward;
    use taskify_core::task::{Hints, Kind, Schedule, Task};

    fn temp() -> std::path::PathBuf {
        crate::test_support::temp_root("store")
    }

    fn task() -> Task {
        let mut title = BTreeMap::new();
        title.insert("en".to_string(), "Water".to_string());
        Task {
            id: "water".into(),
            kind: Kind::Habit,
            title,
            description: BTreeMap::new(),
            category: Some("body".into()),
            tags: vec![],
            icon: None,
            reward: Reward::new(10, 2),
            milestones: vec![],
            schedule: Schedule::Daily,
            after: None,
            combos: vec![],
            hints: Hints::default(),
            subtasks: vec![],
            metrics: BTreeMap::new(),
            former_ids: vec![],
            rev: 1,
        }
    }

    #[test]
    fn events_survive_reopen() {
        let root = temp();
        let date = chrono::NaiveDate::from_ymd_opt(2024, 3, 10).unwrap();
        {
            let mut store = Store::open(&root).unwrap();
            store
                .record(EventKind::TaskCompleted(TaskCompleted {
                    task: task(),
                    base: Reward::new(10, 2),
                    milestones: vec![],
                    awarded: Reward::new(10, 2),
                    completions_total: 1,
                    streak: 1,
                    local_date: date,
                    over_norm: false,
                    combos: vec![],
                }))
                .unwrap();
        }
        let store = Store::open(&root).unwrap();
        assert_eq!(store.state.xp_total, 10);
        assert_eq!(store.state.coins, 2);
        assert_eq!(store.aggregates.totals.tasks_completed, 1);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn level_is_recomputed_from_xp_on_open() {
        let root = temp();
        let date = chrono::NaiveDate::from_ymd_opt(2024, 3, 10).unwrap();
        {
            let mut store = Store::open(&root).unwrap();
            store
                .record(EventKind::TaskCompleted(TaskCompleted {
                    task: task(),
                    base: Reward::new(100, 10),
                    milestones: vec![],
                    awarded: Reward::new(100, 10),
                    completions_total: 1,
                    streak: 1,
                    local_date: date,
                    over_norm: false,
                    combos: vec![],
                }))
                .unwrap();
        }
        let livespace = Livespace::open(&root).unwrap();
        let mut cached: State = cache::read_json(&livespace.state_file()).unwrap();
        cached.level = 99;
        cache::write_json(&livespace.state_file(), &cached).unwrap();

        let store = Store::open(&root).unwrap();
        assert_eq!(store.state.level, 2);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn caches_can_be_deleted_and_rebuilt() {
        let root = temp();
        let date = chrono::NaiveDate::from_ymd_opt(2024, 3, 10).unwrap();
        {
            let mut store = Store::open(&root).unwrap();
            store
                .record(EventKind::HabitSkipped(HabitSkipped {
                    task_id: "water".into(),
                    local_date: date,
                }))
                .unwrap();
        }
        let livespace = Livespace::open(&root).unwrap();
        std::fs::remove_file(livespace.state_file()).unwrap();
        std::fs::remove_file(livespace.aggregates_file()).unwrap();
        let store = Store::open(&root).unwrap();
        assert_eq!(store.aggregates.totals.skips, 1);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn verify_detects_no_drift() {
        let root = temp();
        let mut store = Store::open(&root).unwrap();
        store
            .record(EventKind::Purchase(Purchase {
                token: "color.bg".into(),
                variant: "night".into(),
                title: BTreeMap::new(),
                price: 0,
                local_date: chrono::NaiveDate::from_ymd_opt(2024, 3, 10).unwrap(),
            }))
            .unwrap();
        assert!(store.verify().unwrap().is_empty());
        let _ = std::fs::remove_dir_all(&root);
    }
}
