//! Применение события к состоянию и статистике.
//!
//! Функция чистая: никакого ввода-вывода, поэтому пересчёт из журнала даёт
//! тот же результат, что и инкрементальное обновление.

use chrono::NaiveDate;
use taskify_core::time::DayRules;

use crate::aggregates::Aggregates;
use crate::event::{Event, EventKind};
use crate::state::{State, StickerPlacement};

#[derive(Debug, Clone, Copy)]
pub struct Rules {
    pub days: DayRules,
    pub daily_norm: u64,
    pub weekly_goal: i64,
}

impl Default for Rules {
    fn default() -> Self {
        Self {
            days: DayRules::default(),
            daily_norm: 12,
            weekly_goal: 0,
        }
    }
}

pub fn apply(state: &mut State, aggregates: &mut Aggregates, event: &Event, rules: &Rules) {
    state.last_seq = state.last_seq.max(event.seq);
    aggregates.last_seq = aggregates.last_seq.max(event.seq);

    match &event.kind {
        EventKind::TaskCompleted(payload) => {
            let date = payload.local_date;
            state.rollover(date, &rules.days);

            let task_state = state.task(&payload.task.id);
            task_state.started = None;
            task_state.completions_total = payload.completions_total;
            task_state.streak_current = payload.streak;
            task_state.streak_max = task_state.streak_max.max(payload.streak);
            task_state.last_done = Some(date);
            task_state.last_occurrence = Some(date);
            task_state.last_skipped = None;
            task_state.subtasks_done.clear();
            task_state.subtasks_date = Some(date);
            task_state.next_due = match payload.task.schedule {
                taskify_core::task::Schedule::Interval { days } if days > 0 => {
                    Some(date + chrono::Duration::days(days as i64 - 1))
                }
                _ => None,
            };

            let awarded = payload.awarded;
            state.add_xp(awarded.xp);
            state.add_coins(awarded.coins);
            state.days_active.insert(date);
            state.day.completed += 1;
            state.day.xp = state.day.xp.saturating_add(awarded.xp);
            state.day.coins = state.day.coins.saturating_add(awarded.coins);
            if payload.over_norm {
                state.day.over_norm += 1;
            }
            state.week.xp = state.week.xp.saturating_add(awarded.xp);
            state.week.completed += 1;

            aggregates.totals.tasks_completed += 1;
            aggregates.totals.xp_earned = aggregates.totals.xp_earned.saturating_add(awarded.xp);
            aggregates.totals.coins_earned =
                aggregates.totals.coins_earned.saturating_add(awarded.coins);
            let day = aggregates.day_mut(date);
            day.completed += 1;
            day.xp = day.xp.saturating_add(awarded.xp);
            day.coins = day.coins.saturating_add(awarded.coins);

            let task = aggregates.task_mut(&payload.task.id);
            task.completions += 1;
            task.xp = task.xp.saturating_add(awarded.xp);
            task.coins = task.coins.saturating_add(awarded.coins);
            if !payload.task.title.is_empty() {
                task.title = payload.task.title.clone();
            }
            task.category = payload.task.category.clone();

            if let Some(category) = &payload.task.category {
                *aggregates.by_category.entry(category.clone()).or_insert(0) += 1;
            }
            for tag in &payload.task.tags {
                *aggregates.by_tag.entry(tag.clone()).or_insert(0) += 1;
            }
            for (name, effect) in &payload.task.metrics {
                let current = aggregates.metrics.get(name).copied().unwrap_or(0);
                aggregates
                    .metrics
                    .insert(name.clone(), effect.apply(current));
            }
        }
        EventKind::SubtaskCompleted(payload) => {
            let date = payload.local_date;
            state.rollover(date, &rules.days);
            let awarded = payload.awarded;
            state.add_xp(awarded.xp);
            state.add_coins(awarded.coins);
            state.day.xp = state.day.xp.saturating_add(awarded.xp);
            state.day.coins = state.day.coins.saturating_add(awarded.coins);
            state.week.xp = state.week.xp.saturating_add(awarded.xp);

            let task_state = state.task(&payload.task_id);
            task_state.subtasks_done.insert(payload.subtask_id.clone());
            task_state.subtasks_date = Some(date);

            aggregates.totals.subtasks_completed += 1;
            aggregates.totals.xp_earned = aggregates.totals.xp_earned.saturating_add(awarded.xp);
            aggregates.totals.coins_earned =
                aggregates.totals.coins_earned.saturating_add(awarded.coins);
            let day = aggregates.day_mut(date);
            day.xp = day.xp.saturating_add(awarded.xp);
            day.coins = day.coins.saturating_add(awarded.coins);
            let task = aggregates.task_mut(&payload.task_id);
            task.xp = task.xp.saturating_add(awarded.xp);
            task.coins = task.coins.saturating_add(awarded.coins);
            if !payload.task_title.is_empty() {
                task.title = payload.task_title.clone();
            }
        }
        EventKind::TaskCompletionReverted(payload) => {
            let date = payload.local_date;
            state.rollover(date, &rules.days);
            let awarded = payload.awarded;
            state.add_xp(-awarded.xp);
            state.add_coins(-awarded.coins);
            state.day.xp = state.day.xp.saturating_sub(awarded.xp);
            state.day.coins = state.day.coins.saturating_sub(awarded.coins);
            state.day.completed = state.day.completed.saturating_sub(1);
            state.week.xp = state.week.xp.saturating_sub(awarded.xp);
            state.week.completed = state.week.completed.saturating_sub(1);

            if let Some(task_state) = state.tasks.get_mut(&payload.target) {
                task_state.completions_total = task_state.completions_total.saturating_sub(1);
                task_state.streak_current = task_state.streak_current.saturating_sub(1);
                if task_state.last_occurrence == Some(date) {
                    task_state.last_done = None;
                    task_state.last_occurrence = None;
                }
            }

            aggregates.totals.tasks_completed = aggregates.totals.tasks_completed.saturating_sub(1);
            aggregates.totals.xp_earned = aggregates.totals.xp_earned.saturating_sub(awarded.xp);
            aggregates.totals.coins_earned =
                aggregates.totals.coins_earned.saturating_sub(awarded.coins);
            let day = aggregates.day_mut(date);
            day.completed = day.completed.saturating_sub(1);
            day.xp = day.xp.saturating_sub(awarded.xp);
            day.coins = day.coins.saturating_sub(awarded.coins);
            if let Some(task) = aggregates.by_task.get_mut(&payload.target) {
                task.completions = task.completions.saturating_sub(1);
                task.xp = task.xp.saturating_sub(awarded.xp);
                task.coins = task.coins.saturating_sub(awarded.coins);
            }
        }
        EventKind::TaskRejected(payload) => {
            let date = payload.local_date;
            state.rollover(date, &rules.days);
            state.day.rejected += 1;
            let task_state = state.task(&payload.task.id);
            task_state.started = None;
            task_state.last_rejected = Some(date);
            task_state.rejected_total += 1;
            aggregates.totals.rejections += 1;
            aggregates.day_mut(date).rejections += 1;
        }
        EventKind::HabitSkipped(payload) => {
            state.rollover(payload.local_date, &rules.days);
            let task_state = state.task(&payload.task_id);
            task_state.last_skipped = Some(payload.local_date);
            task_state.streak_current = 0;
            aggregates.totals.skips += 1;
        }
        EventKind::ComboAwarded(payload) => {
            let date = payload.local_date;
            state.rollover(date, &rules.days);
            let awarded = payload.awarded;
            state.add_xp(awarded.xp);
            state.add_coins(awarded.coins);
            state.day.xp = state.day.xp.saturating_add(awarded.xp);
            state.day.coins = state.day.coins.saturating_add(awarded.coins);
            state.week.xp = state.week.xp.saturating_add(awarded.xp);

            aggregates.totals.combos_completed += 1;
            aggregates.totals.xp_earned = aggregates.totals.xp_earned.saturating_add(awarded.xp);
            aggregates.totals.coins_earned =
                aggregates.totals.coins_earned.saturating_add(awarded.coins);
            let day = aggregates.day_mut(date);
            day.xp = day.xp.saturating_add(awarded.xp);
            day.coins = day.coins.saturating_add(awarded.coins);

            let key = if payload.window_key.is_empty() {
                date.to_string()
            } else {
                payload.window_key.clone()
            };
            let combo_state = state.combos.entry(payload.combo.clone()).or_default();
            combo_state.awarded.insert(key.clone());
            combo_state.window = key;
        }
        EventKind::AchievementUnlocked(payload) => {
            let date = payload.local_date;
            state.rollover(date, &rules.days);
            state.achievements.insert(payload.achievement.clone());
            if let Some(sticker) = &payload.sticker {
                state.purchased.insert(format!("sticker/{sticker}"));
            }
            let awarded = payload.awarded;
            state.add_xp(awarded.xp);
            state.add_coins(awarded.coins);
            aggregates.totals.achievements += 1;
            aggregates.totals.xp_earned = aggregates.totals.xp_earned.saturating_add(awarded.xp);
            aggregates.totals.coins_earned =
                aggregates.totals.coins_earned.saturating_add(awarded.coins);
            let day = aggregates.day_mut(date);
            day.xp = day.xp.saturating_add(awarded.xp);
            day.coins = day.coins.saturating_add(awarded.coins);
        }
        EventKind::Purchase(payload) => {
            state.rollover(payload.local_date, &rules.days);
            let key = format!("{}/{}", payload.token, payload.variant);
            if state.purchased.insert(key) {
                state.add_coins(-payload.price);
                aggregates.totals.purchases += 1;
                aggregates.totals.coins_spent =
                    aggregates.totals.coins_spent.saturating_add(payload.price);
            }
        }
        EventKind::BundlePurchased(payload) => {
            state.rollover(payload.local_date, &rules.days);
            let key = format!("bundle/{}", payload.bundle);
            if state.purchased.insert(key) {
                state.add_coins(-payload.price);
                for item in &payload.items {
                    state
                        .purchased
                        .insert(format!("{}/{}", item.token, item.variant));
                }
                for sticker in &payload.stickers {
                    state.purchased.insert(format!("sticker/{sticker}"));
                }
                aggregates.totals.purchases += 1;
                aggregates.totals.coins_spent =
                    aggregates.totals.coins_spent.saturating_add(payload.price);
            }
        }
        EventKind::TaskStarted(payload) => {
            let started = payload.local_date;
            state.task(&payload.task_id).started = Some(started);
        }
        EventKind::TaskStopped(payload) => {
            state.task(&payload.task_id).started = None;
        }
        EventKind::TaskViewed(payload) => {
            state.viewed_tasks.insert(payload.task_id.clone());
        }
        EventKind::TasksSeen(payload) => {
            state.viewed_tasks.extend(payload.tasks.iter().cloned());
            if payload.seed {
                state.seen_seed_version = payload.version.max(1);
            }
        }
        EventKind::TasksNotified(payload) => {
            state.notified_tasks.extend(payload.tasks.iter().cloned());
        }
        EventKind::StickerPlaced(payload) => {
            let side = payload.side.as_deref().unwrap_or("left").to_string();
            let key = crate::state::sticker_slot(&side, &payload.sticker);
            state.stickers.insert(
                key,
                StickerPlacement {
                    x: payload.x,
                    y: payload.y,
                    side,
                    moved: payload.moved,
                    scale: crate::state::clamp_sticker_scale(payload.scale),
                    rotation: crate::state::normalize_sticker_rotation(payload.rotation),
                },
            );
        }
        EventKind::StickerRemoved(payload) => match payload.side.as_deref() {
            Some(side) => {
                let key = crate::state::sticker_slot(side, &payload.sticker);
                state.stickers.remove(&key);
            }
            None => {
                for side in crate::state::STICKER_SIDES {
                    let key = crate::state::sticker_slot(side, &payload.sticker);
                    state.stickers.remove(&key);
                }
            }
        },
        EventKind::LevelUp(_) => {
            aggregates.levels_reached += 1;
        }
        EventKind::PackInstalled(payload) => {
            state
                .packs
                .insert(payload.pack.clone(), payload.version.clone());
            state.packs_disabled.remove(&payload.pack);
        }
        EventKind::PackUpdated(payload) => {
            state.packs.insert(payload.pack.clone(), payload.to.clone());
        }
        EventKind::PackDisabled(payload) => {
            state.packs_disabled.insert(payload.pack.clone());
        }
        EventKind::PackReset(payload) => {
            // История остаётся на месте: чистим только кеш прогресса.
            for task_id in &payload.tasks {
                if !payload.keep_tasks.iter().any(|keep| keep == task_id) {
                    state.tasks.remove(task_id);
                }
            }
            for combo in &payload.combos {
                state.combos.remove(combo);
            }
            for achievement in &payload.achievements {
                state.achievements.remove(achievement);
            }
        }
        EventKind::SettingsChanged(_) => {}
        EventKind::WeekClosed(payload) => {
            state.week.closed = true;
            let bonus = payload.bonus;
            if !bonus.is_zero() {
                state.add_xp(bonus.xp);
                state.add_coins(bonus.coins);
            }
        }
        EventKind::Unknown => {}
    }
}

/// Начало дня у даты: удобно для тестов.
pub fn start_of_day(date: NaiveDate) -> NaiveDate {
    date
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::*;
    use std::collections::BTreeMap;
    use taskify_core::reward::Reward;
    use taskify_core::task::{Hints, Kind, Schedule, Task};

    fn task() -> Task {
        let mut title = BTreeMap::new();
        title.insert("en".to_string(), "Water".to_string());
        Task {
            id: "water".into(),
            kind: Kind::Habit,
            title,
            description: BTreeMap::new(),
            category: Some("body".into()),
            tags: vec!["health".into()],
            icon: None,
            reward: Reward::new(10, 2),
            milestones: vec![],
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

    fn completed(seq: u64, date: NaiveDate) -> Event {
        Event {
            schema: 1,
            seq,
            id: format!("e{seq}"),
            ts: chrono::DateTime::parse_from_rfc3339("2024-03-10T09:00:00+04:00").unwrap(),
            kind: EventKind::TaskCompleted(TaskCompleted {
                task: task(),
                base: Reward::new(10, 2),
                milestones: vec![],
                awarded: Reward::new(10, 2),
                completions_total: 1,
                streak: 1,
                local_date: date,
                over_norm: false,
                combos: vec![],
            }),
        }
    }

    #[test]
    fn completion_updates_everything() {
        let mut state = State::default();
        let mut aggs = Aggregates::default();
        let rules = Rules::default();
        let date = NaiveDate::from_ymd_opt(2024, 3, 10).unwrap();
        apply(&mut state, &mut aggs, &completed(1, date), &rules);

        assert_eq!(state.xp_total, 10);
        assert_eq!(state.coins, 2);
        assert_eq!(state.tasks["water"].completions_total, 1);
        assert_eq!(state.tasks["water"].streak_current, 1);
        assert_eq!(state.day.completed, 1);
        assert_eq!(aggs.totals.tasks_completed, 1);
        assert_eq!(aggs.by_task["water"].completions, 1);
        assert_eq!(aggs.by_category["body"], 1);
        assert_eq!(aggs.by_tag["health"], 1);
        assert_eq!(state.last_seq, 1);
    }

    #[test]
    fn revert_undoes_completion() {
        let mut state = State::default();
        let mut aggs = Aggregates::default();
        let rules = Rules::default();
        let date = NaiveDate::from_ymd_opt(2024, 3, 10).unwrap();
        apply(&mut state, &mut aggs, &completed(1, date), &rules);
        let revert = Event {
            schema: 1,
            seq: 2,
            id: "e2".into(),
            ts: chrono::DateTime::parse_from_rfc3339("2024-03-10T09:00:05+04:00").unwrap(),
            kind: EventKind::TaskCompletionReverted(CompletionReverted {
                target: "water".into(),
                awarded: Reward::new(10, 2),
                local_date: date,
            }),
        };
        apply(&mut state, &mut aggs, &revert, &rules);
        assert_eq!(state.xp_total, 0);
        assert_eq!(state.coins, 0);
        assert_eq!(aggs.totals.tasks_completed, 0);
        assert_eq!(state.level, 1);
    }

    #[test]
    fn pack_reset_clears_progress_but_keeps_one_shots_and_totals() {
        let mut state = State::default();
        let mut aggs = Aggregates::default();
        let rules = Rules::default();
        let date = NaiveDate::from_ymd_opt(2024, 3, 10).unwrap();
        apply(&mut state, &mut aggs, &completed(1, date), &rules);
        state.task("water/one_shot").completions_total = 1;
        state
            .combos
            .insert("water/morning".into(), Default::default());
        state.achievements.insert("water/first".into());

        let event = Event {
            schema: 1,
            seq: 2,
            id: "e2".into(),
            ts: chrono::DateTime::parse_from_rfc3339("2024-03-10T09:05:00+04:00").unwrap(),
            kind: EventKind::PackReset(PackReset {
                pack: "water".into(),
                tasks: vec!["water".into(), "water/one_shot".into()],
                keep_tasks: vec!["water/one_shot".into()],
                combos: vec!["water/morning".into()],
                achievements: vec!["water/first".into()],
                local_date: date,
            }),
        };
        apply(&mut state, &mut aggs, &event, &rules);

        assert!(!state.tasks.contains_key("water"));
        assert_eq!(state.tasks["water/one_shot"].completions_total, 1);
        assert!(state.combos.is_empty());
        assert!(state.achievements.is_empty());
        // История и глобальные итоги не переписываются.
        assert_eq!(state.xp_total, 10);
        assert_eq!(state.coins, 2);
        assert_eq!(aggs.totals.tasks_completed, 1);
    }

    #[test]
    fn rejection_is_recorded_without_reward() {
        let mut state = State::default();
        let mut aggs = Aggregates::default();
        let rules = Rules::default();
        let date = NaiveDate::from_ymd_opt(2024, 3, 10).unwrap();
        let event = Event {
            schema: 1,
            seq: 1,
            id: "e1".into(),
            ts: chrono::DateTime::parse_from_rfc3339("2024-03-10T09:00:00+04:00").unwrap(),
            kind: EventKind::TaskRejected(TaskRejected {
                task: task(),
                local_date: date,
                reason: None,
            }),
        };
        apply(&mut state, &mut aggs, &event, &rules);
        assert_eq!(state.xp_total, 0);
        assert_eq!(state.day.rejected, 1);
        assert_eq!(aggs.totals.rejections, 1);
    }

    #[test]
    fn purchase_deducts_coins_once() {
        let mut state = State::default();
        let mut aggs = Aggregates::default();
        let rules = Rules::default();
        state.coins = 100;
        let date = NaiveDate::from_ymd_opt(2024, 3, 10).unwrap();
        let event = Event {
            schema: 1,
            seq: 1,
            id: "e1".into(),
            ts: chrono::DateTime::parse_from_rfc3339("2024-03-10T09:00:00+04:00").unwrap(),
            kind: EventKind::Purchase(Purchase {
                token: "color.bg".into(),
                variant: "night".into(),
                title: BTreeMap::new(),
                price: 30,
                local_date: date,
            }),
        };
        apply(&mut state, &mut aggs, &event, &rules);
        apply(&mut state, &mut aggs, &event, &rules);
        assert_eq!(state.coins, 70);
        assert_eq!(aggs.totals.coins_spent, 30);
        assert!(state.owns("color.bg/night"));
    }

    #[test]
    fn skip_resets_streak() {
        let mut state = State::default();
        let mut aggs = Aggregates::default();
        let rules = Rules::default();
        let date = NaiveDate::from_ymd_opt(2024, 3, 10).unwrap();
        apply(&mut state, &mut aggs, &completed(1, date), &rules);
        let skip = Event {
            schema: 1,
            seq: 2,
            id: "e2".into(),
            ts: chrono::DateTime::parse_from_rfc3339("2024-03-11T09:00:00+04:00").unwrap(),
            kind: EventKind::HabitSkipped(HabitSkipped {
                task_id: "water".into(),
                local_date: date + chrono::Duration::days(1),
            }),
        };
        apply(&mut state, &mut aggs, &skip, &rules);
        assert_eq!(state.tasks["water"].streak_current, 0);
        assert_eq!(state.tasks["water"].streak_max, 1);
    }

    #[test]
    fn unknown_event_is_ignored() {
        let mut state = State::default();
        let mut aggs = Aggregates::default();
        let rules = Rules::default();
        let event = Event {
            schema: 1,
            seq: 9,
            id: "e9".into(),
            ts: chrono::DateTime::parse_from_rfc3339("2024-03-10T09:00:00+04:00").unwrap(),
            kind: EventKind::Unknown,
        };
        apply(&mut state, &mut aggs, &event, &rules);
        assert_eq!(state.last_seq, 9);
        assert_eq!(state.xp_total, 0);
    }
    #[test]
    fn sticker_events_place_both_sides_and_remove() {
        let rules = Rules::default();
        let mut state = State::default();
        let mut aggregates = Aggregates::default();
        let placed = Event {
            schema: 1,
            seq: 1,
            id: "e1".into(),
            ts: chrono::DateTime::parse_from_rfc3339("2024-03-10T09:00:00+04:00").unwrap(),
            kind: EventKind::StickerPlaced(StickerPlaced {
                sticker: "star".into(),
                x: 0.5,
                y: 0.5,
                side: Some("left".into()),
                moved: false,
                scale: 1.0,
                rotation: 0.0,
            }),
        };
        apply(&mut state, &mut aggregates, &placed, &rules);
        assert_eq!(
            state.stickers.get("left:star"),
            Some(&StickerPlacement {
                x: 0.5,
                y: 0.5,
                side: "left".into(),
                moved: false,
                scale: 1.0,
                rotation: 0.0,
            })
        );

        // Тот же стикер можно повесить и справа.
        let right = Event {
            schema: 1,
            seq: 2,
            id: "e2".into(),
            ts: chrono::DateTime::parse_from_rfc3339("2024-03-10T09:01:00+04:00").unwrap(),
            kind: EventKind::StickerPlaced(StickerPlaced {
                sticker: "star".into(),
                x: 0.5,
                y: 0.5,
                side: Some("right".into()),
                moved: false,
                scale: 1.0,
                rotation: 0.0,
            }),
        };
        apply(&mut state, &mut aggregates, &right, &rules);
        assert_eq!(state.stickers.len(), 2);

        // Снятие слева не трогает правую панель.
        let removed = Event {
            schema: 1,
            seq: 3,
            id: "e3".into(),
            ts: chrono::DateTime::parse_from_rfc3339("2024-03-10T09:05:00+04:00").unwrap(),
            kind: EventKind::StickerRemoved(StickerRemoved {
                sticker: "star".into(),
                side: Some("left".into()),
            }),
        };
        apply(&mut state, &mut aggregates, &removed, &rules);
        assert_eq!(state.stickers.len(), 1);
        assert!(state.stickers.contains_key("right:star"));

        // Старое событие без стороны снимает стикер с обеих панелей.
        let legacy = Event {
            schema: 1,
            seq: 4,
            id: "e4".into(),
            ts: chrono::DateTime::parse_from_rfc3339("2024-03-10T09:06:00+04:00").unwrap(),
            kind: EventKind::StickerRemoved(StickerRemoved {
                sticker: "star".into(),
                side: None,
            }),
        };
        apply(&mut state, &mut aggregates, &legacy, &rules);
        assert!(state.stickers.is_empty());
    }

    #[test]
    fn sticker_transform_is_clamped_and_old_events_stay_identity() {
        let rules = Rules::default();
        let mut state = State::default();
        let mut aggregates = Aggregates::default();
        let transformed = Event {
            schema: 1,
            seq: 1,
            id: "t1".into(),
            ts: chrono::DateTime::parse_from_rfc3339("2024-03-10T09:00:00+04:00").unwrap(),
            kind: EventKind::StickerPlaced(StickerPlaced {
                sticker: "star".into(),
                x: 0.5,
                y: 0.5,
                side: Some("left".into()),
                moved: true,
                scale: 99.0,
                rotation: -1.0,
            }),
        };
        apply(&mut state, &mut aggregates, &transformed, &rules);
        let placement = state.stickers.get("left:star").unwrap();
        assert!((placement.scale - crate::state::STICKER_MAX_SCALE).abs() < 0.001);
        assert!((placement.rotation - (std::f32::consts::TAU - 1.0)).abs() < 0.001);

        // Журнал до появления полей: размер 1.0, поворот 0.
        let legacy_kind: EventKind = serde_json::from_str(
            r#"{"type":"sticker_placed","data":{"sticker":"goose","x":0.2,"y":0.3}}"#,
        )
        .unwrap();
        let legacy = Event {
            schema: 1,
            seq: 2,
            id: "t2".into(),
            ts: chrono::DateTime::parse_from_rfc3339("2024-03-10T09:01:00+04:00").unwrap(),
            kind: legacy_kind,
        };
        apply(&mut state, &mut aggregates, &legacy, &rules);
        let placement = state.stickers.get("left:goose").unwrap();
        assert_eq!(placement.scale, 1.0);
        assert_eq!(placement.rotation, 0.0);
    }
    #[test]
    fn started_flag_lives_until_stopped_or_done() {
        let rules = Rules::default();
        let mut state = State::default();
        let mut aggregates = Aggregates::default();
        let date = NaiveDate::from_ymd_opt(2024, 3, 10).unwrap();

        let start = Event {
            schema: 1,
            seq: 1,
            id: "s1".into(),
            ts: chrono::DateTime::parse_from_rfc3339("2024-03-10T09:00:00+04:00").unwrap(),
            kind: EventKind::TaskStarted(TaskStarted {
                task_id: "water".into(),
                local_date: date,
            }),
        };
        apply(&mut state, &mut aggregates, &start, &rules);
        assert_eq!(state.task_ref("water").unwrap().started, Some(date));

        let stop = Event {
            schema: 1,
            seq: 2,
            id: "s2".into(),
            ts: chrono::DateTime::parse_from_rfc3339("2024-03-10T09:05:00+04:00").unwrap(),
            kind: EventKind::TaskStopped(TaskStopped {
                task_id: "water".into(),
            }),
        };
        apply(&mut state, &mut aggregates, &stop, &rules);
        assert_eq!(state.task_ref("water").unwrap().started, None);

        // Взяли снова и выполнили — флаг снимается.
        let start = Event {
            seq: 3,
            id: "s3".into(),
            ..start
        };
        apply(&mut state, &mut aggregates, &start, &rules);
        let done = completed(4, date);
        apply(&mut state, &mut aggregates, &done, &rules);
        assert_eq!(state.task_ref("water").unwrap().started, None);
    }
    #[test]
    fn bundle_grants_all_items_for_its_price() {
        let rules = Rules::default();
        let mut state = State::default();
        let mut aggregates = Aggregates::default();
        let date = NaiveDate::from_ymd_opt(2024, 5, 2).unwrap();
        state.coins = 1000;

        let event = Event {
            schema: 1,
            seq: 1,
            id: "b1".into(),
            ts: chrono::DateTime::parse_from_rfc3339("2024-05-02T12:00:00+04:00").unwrap(),
            kind: EventKind::BundlePurchased(BundlePurchased {
                bundle: "cozy".into(),
                price: 650,
                items: vec![
                    BundleItem {
                        token: "starter/color.accent".into(),
                        variant: "orange".into(),
                    },
                    BundleItem {
                        token: "starter/sound.ui".into(),
                        variant: "soft".into(),
                    },
                ],
                stickers: vec!["coffee".into()],
                local_date: date,
            }),
        };
        apply(&mut state, &mut aggregates, &event, &rules);

        assert!(state.owns("bundle/cozy"));
        assert!(state.owns("sticker/coffee"));
        assert!(state.owns("starter/color.accent/orange"));
        assert!(state.owns("starter/sound.ui/soft"));
        assert_eq!(state.coins, 350);
        assert_eq!(aggregates.totals.coins_spent, 650);

        // Повторная покупка ничего не списывает.
        apply(&mut state, &mut aggregates, &event, &rules);
        assert_eq!(state.coins, 350);
    }
}
