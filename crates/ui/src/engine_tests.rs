//! Сквозные тесты правил: награды, комбо, ачивки, отмена, покупки.

use std::path::PathBuf;

use taskify_core::group::RandomGroup;
use taskify_core::task::{Kind, Schedule};

use crate::engine::Engine;
use crate::settings::Settings;

struct Fixture {
    root: PathBuf,
    engine: Engine,
}

impl Fixture {
    fn new() -> Self {
        let root = crate::test_support::temp_root("engine");
        let mut settings = Settings::default();
        settings.paths.livespaces_dir = Some(root.join("livespaces"));
        let mut engine = crate::bootstrap::prepare(&settings).expect("профиль должен открыться");
        // Базовые паки по умолчанию выключены — для сквозных тестов включаем все.
        let ids: Vec<String> = engine.packs.packs.keys().cloned().collect();
        for id in ids {
            engine
                .set_pack_enabled(&id, true)
                .expect("пак должен включиться");
        }
        engine.apply_disabled_packs();
        Self { root, engine }
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.root);
    }
}

#[test]
fn legacy_bundle_stickers_are_not_granted_or_priced() {
    let mut fixture = Fixture::new();
    let bundle = taskify_packs::Bundle {
        id: "greeting/legacy".into(),
        price: 0,
        base: None,
        title: Default::default(),
        description: Default::default(),
        tokens: Vec::new(),
        stickers: vec!["greeting/coffee".into()],
    };
    assert_eq!(fixture.engine.bundle_full_price(&bundle), 0);
    fixture.engine.buy_bundle(&bundle).unwrap();
    assert!(fixture.engine.bundle_owned("greeting/legacy"));
    assert!(!fixture.engine.sticker_owned("greeting/coffee"));
    assert_eq!(fixture.engine.store.state.stickers.len(), 0);
}

#[test]
fn completing_a_task_gives_reward_and_unlocks_first_achievement() {
    let mut fixture = Fixture::new();
    let outcome = fixture.engine.complete("greeting/hello").unwrap();
    assert_eq!(outcome.awarded.xp, 10);
    assert_eq!(outcome.awarded.coins, 10);
    assert!(outcome
        .achievements
        .contains(&"greeting/first_step".to_string()));
    assert!(fixture.engine.store.state.xp_total > 2);
    assert!(fixture
        .engine
        .store
        .state
        .achievements
        .contains("greeting/first_step"));
}

#[test]
fn same_task_cannot_be_completed_twice_a_day() {
    let mut fixture = Fixture::new();
    fixture.engine.complete("habits/water").unwrap();
    let error = fixture.engine.complete("habits/water").unwrap_err();
    assert!(error.to_string().contains("уже выполнено"));
}

#[test]
fn completed_one_shot_never_becomes_due_again() {
    let mut fixture = Fixture::new();
    let task = fixture
        .engine
        .packs
        .task("cleaning/choose_room")
        .cloned()
        .expect("ваншот из пака уборки должен существовать");
    let today = fixture.engine.today();
    assert!(task.is_one_shot());
    assert!(fixture.engine.is_due(&task, today));

    fixture.engine.complete("cleaning/choose_room").unwrap();

    assert!(fixture.engine.is_spent_one_shot(&task));
    assert!(!fixture.engine.is_due(&task, today));
    assert!(!fixture
        .engine
        .is_due(&task, today + chrono::Duration::days(1)));
}

#[test]
fn pack_reset_clears_tasks_but_keeps_one_shots_and_optional_achievements() {
    let mut fixture = Fixture::new();
    for task in [
        "greeting/hello",
        "greeting/breath",
        "greeting/window",
        "greeting/note",
        "greeting/water",
    ] {
        fixture.engine.complete(task).unwrap();
    }
    assert!(fixture
        .engine
        .store
        .state
        .combos
        .contains_key("greeting/hello"));
    assert!(fixture
        .engine
        .store
        .state
        .achievements
        .contains("greeting/first_step"));

    let cleared = fixture.engine.reset_pack("greeting", false).unwrap();
    assert_eq!(cleared, 1, "сброситься должна только привычка water");
    assert!(!fixture
        .engine
        .store
        .state
        .tasks
        .contains_key("greeting/water"));
    assert!(
        fixture
            .engine
            .store
            .state
            .tasks
            .contains_key("greeting/hello"),
        "выполненный ваншот не должен сбрасываться"
    );
    assert!(fixture.engine.store.state.combos.is_empty());
    assert!(
        fixture
            .engine
            .store
            .state
            .achievements
            .contains("greeting/first_step"),
        "ачивки сохраняются, пока их не попросили сбросить"
    );
    let log_dir = fixture.engine.store.livespace.log_dir();
    let events_before = taskify_store::log::read_all(&log_dir).unwrap().events.len();

    // После сброса привычка снова доступна: предыдущие ваншоты остались выполненными.
    assert!(fixture.engine.complete("greeting/water").is_ok());
    fixture.engine.reset_pack("greeting", true).unwrap();
    let events_after = taskify_store::log::read_all(&log_dir).unwrap().events.len();
    assert!(
        events_after >= events_before + 2,
        "сбросы только добавляют события, историю не переписывают"
    );
    assert!(!fixture
        .engine
        .store
        .state
        .achievements
        .contains("greeting/first_step"));
    assert!(
        fixture
            .engine
            .store
            .state
            .tasks
            .contains_key("greeting/hello"),
        "ваншот остаётся выполненным и после сброса ачивок"
    );
}

#[test]
fn old_zero_coin_tasks_get_a_one_time_catchup_bonus() {
    let mut fixture = Fixture::new();
    // Профиль до 189: задания уже выполнялись, но монет почти не было.
    fixture.engine.store.aggregates.totals.tasks_completed = 60;
    fixture.engine.store.aggregates.totals.coins_earned = 8;
    let marker = fixture
        .engine
        .store
        .livespace
        .root()
        .join("data/legacy-task-coins-v1");
    let _ = std::fs::remove_file(&marker);
    let before = fixture.engine.store.state.coins;

    assert_eq!(fixture.engine.credit_legacy_task_coins().unwrap(), 52);
    assert_eq!(fixture.engine.store.state.coins, before + 52);
    assert!(marker.exists());
    assert_eq!(fixture.engine.credit_legacy_task_coins().unwrap(), 0);
    assert_eq!(fixture.engine.store.state.coins, before + 52);
}

#[test]
fn chained_task_opens_only_after_previous_completion() {
    let mut fixture = Fixture::new();
    let chained = fixture
        .engine
        .packs
        .task("greeting/breath")
        .cloned()
        .expect("второй шаг приветствия");
    let today = fixture.engine.today();

    assert!(!fixture.engine.is_unlocked(&chained));
    assert!(!fixture.engine.is_due(&chained, today));
    assert!(fixture.engine.complete("greeting/breath").is_err());
    assert!(fixture.engine.start_task("greeting/breath").is_err());

    fixture.engine.complete("greeting/hello").unwrap();

    assert!(fixture.engine.is_unlocked(&chained));
    assert!(fixture.engine.is_due(&chained, today));
    assert!(fixture.engine.complete("greeting/breath").is_ok());
}

#[test]
fn three_greeting_tasks_award_the_combo() {
    let mut fixture = Fixture::new();
    let mut combos = Vec::new();
    for task in ["greeting/hello", "greeting/breath", "greeting/window"] {
        let outcome = fixture.engine.complete(task).unwrap();
        combos.extend(outcome.combos);
    }
    assert!(combos.contains(&"greeting/hello".to_string()));
    let state = &fixture.engine.store.state;
    assert!(
        state.combos["greeting/hello"].awarded.contains("once")
            || !state.combos["greeting/hello"].awarded.is_empty()
    );
}

#[test]
fn rejection_gives_nothing_and_task_returns() {
    let mut fixture = Fixture::new();
    fixture
        .engine
        .reject("habits/water", Some("передумал".into()))
        .unwrap();
    let state = &fixture.engine.store.state;
    assert_eq!(state.xp_total, 0);
    assert_eq!(state.coins, 0);
    assert_eq!(state.tasks["habits/water"].rejected_total, 1);
    assert!(!fixture
        .engine
        .is_done_today("habits/water", fixture.engine.today()));
}

#[test]
fn undo_returns_reward_within_window() {
    let mut fixture = Fixture::new();
    fixture.engine.complete("habits/water").unwrap();
    let xp_after = fixture.engine.store.state.xp_total;
    assert!(xp_after > 0);
    assert!(fixture.engine.undo_last().unwrap());
    assert!(fixture.engine.store.state.xp_total < xp_after);
}

#[test]
fn purchase_requires_coins() {
    let mut fixture = Fixture::new();
    let error = fixture
        .engine
        .purchase("greeting/color.accent", "green")
        .unwrap_err();
    assert!(error.to_string().contains("недоступен"));

    fixture.engine.store.state.coins = 500;
    fixture
        .engine
        .purchase("greeting/color.accent", "green")
        .unwrap();
    assert!(fixture.engine.owned("greeting/color.accent", "green"));
    assert_eq!(fixture.engine.store.state.coins, 400);
}

#[test]
fn base_cosmetics_work_with_disabled_pack() {
    let mut fixture = Fixture::new();
    fixture.engine.set_pack_enabled("greeting", false).unwrap();
    fixture.engine.apply_disabled_packs();
    assert!(!fixture.engine.packs.packs["greeting"].enabled);
    assert!(fixture.engine.owned("greeting/theme", "dark"));
    assert_eq!(
        fixture
            .engine
            .variant_value("greeting/theme", "dark")
            .as_deref(),
        Some("dark")
    );
    assert_eq!(
        fixture
            .engine
            .packs
            .token_any("greeting/theme")
            .map(|token| token.kind),
        Some(taskify_packs::TokenKind::Theme)
    );
    assert!(fixture
        .engine
        .packs
        .tokens_any()
        .iter()
        .any(|token| token.id == "greeting/theme"));
}

#[test]
fn sticker_hangs_on_both_panels() {
    let mut fixture = Fixture::new();
    fixture
        .engine
        .store
        .state
        .purchased
        .insert("sticker/star".to_string());
    fixture.engine.place_sticker("star", "left").unwrap();
    fixture.engine.place_sticker("star", "right").unwrap();
    assert!(fixture.engine.sticker_placed("star", "left"));
    assert!(fixture.engine.sticker_placed("star", "right"));

    fixture.engine.remove_sticker("star", "left").unwrap();
    assert!(!fixture.engine.sticker_placed("star", "left"));
    assert!(fixture.engine.sticker_placed("star", "right"));
}

#[test]
fn sticker_can_be_moved_within_its_panel() {
    let mut fixture = Fixture::new();
    fixture
        .engine
        .store
        .state
        .purchased
        .insert("sticker/star".to_string());
    fixture.engine.place_sticker("star", "left").unwrap();
    fixture
        .engine
        .move_sticker("star", "left", 0.25, 0.7)
        .unwrap();
    let placement = fixture
        .engine
        .store
        .state
        .stickers
        .get("left:star")
        .unwrap();
    assert!(placement.moved);
    assert!((placement.x - 0.25).abs() < 0.001);
    assert!((placement.y - 0.7).abs() < 0.001);
    assert!(!fixture.engine.sticker_placed("star", "right"));
}

#[test]
fn sticker_can_be_resized_and_rotated_without_losing_position() {
    let mut fixture = Fixture::new();
    fixture
        .engine
        .store
        .state
        .purchased
        .insert("sticker/star".to_string());
    fixture.engine.place_sticker("star", "left").unwrap();
    fixture
        .engine
        .move_sticker("star", "left", 0.25, 0.7)
        .unwrap();
    fixture
        .engine
        .transform_sticker("star", "left", 1.8, -0.5)
        .unwrap();
    let placement = fixture
        .engine
        .store
        .state
        .stickers
        .get("left:star")
        .unwrap();
    assert!((placement.x - 0.25).abs() < 0.001);
    assert!((placement.y - 0.7).abs() < 0.001);
    assert!((placement.scale - 1.8).abs() < 0.001);
    assert!(
        (placement.rotation - (std::f32::consts::TAU - 0.5)).abs() < 0.001,
        "поворот должен нормализоваться в 0..2π"
    );

    // Последующее перетаскивание не сбрасывает размер и поворот.
    fixture
        .engine
        .move_sticker("star", "left", 0.4, 0.6)
        .unwrap();
    let placement = fixture
        .engine
        .store
        .state
        .stickers
        .get("left:star")
        .unwrap();
    assert!((placement.scale - 1.8).abs() < 0.001);
    assert!((placement.rotation - (std::f32::consts::TAU - 0.5)).abs() < 0.001);
}

#[test]
fn removed_sticker_returns_with_default_position_size_and_rotation() {
    let mut fixture = Fixture::new();
    fixture
        .engine
        .store
        .state
        .purchased
        .insert("sticker/star".to_string());
    fixture.engine.place_sticker("star", "left").unwrap();
    fixture
        .engine
        .move_sticker("star", "left", 0.25, 0.7)
        .unwrap();
    fixture
        .engine
        .transform_sticker("star", "left", 3.5, 0.4)
        .unwrap();

    fixture.engine.remove_sticker("star", "left").unwrap();
    assert!(!fixture.engine.sticker_placed("star", "left"));

    fixture.engine.place_sticker("star", "left").unwrap();
    let placement = fixture
        .engine
        .store
        .state
        .stickers
        .get("left:star")
        .expect("стикер должен вернуться");
    assert!(!placement.moved, "стикер возвращается как новый");
    assert!((placement.x - 0.5).abs() < 0.001);
    assert!((placement.y - 0.5).abs() < 0.001);
    assert!((placement.scale - 1.0).abs() < 0.001);
    assert!(placement.rotation.abs() < 0.001);
}

#[test]
fn sticker_transform_rejects_a_sticker_that_is_not_on_the_panel() {
    let mut fixture = Fixture::new();
    assert!(fixture
        .engine
        .transform_sticker("star", "left", 1.5, 0.0)
        .is_err());
}

#[test]
fn subtask_can_be_completed_once() {
    let mut fixture = Fixture::new();
    let reward = fixture
        .engine
        .complete_subtask("habits/exercise", "squats")
        .unwrap();
    assert_eq!(reward.xp, 1);
    assert!(fixture
        .engine
        .store
        .state
        .task_ref("habits/exercise")
        .unwrap()
        .subtasks_done
        .contains("squats"));
}

#[test]
fn availability_window_is_checked_against_local_time() {
    let fixture = Fixture::new();
    let task = fixture.engine.task("habits/morning").unwrap();
    let morning = chrono::NaiveTime::from_hms_opt(6, 0, 0).unwrap();
    let night = chrono::NaiveTime::from_hms_opt(2, 0, 0).unwrap();
    assert!(fixture.engine.is_available_at(task, morning));
    assert!(!fixture.engine.is_available_at(task, night));
}

#[test]
fn random_group_shows_one_member_and_completes_as_group() {
    let mut fixture = Fixture::new();
    let date = fixture.engine.today();
    {
        let pack = fixture.engine.packs.packs.get_mut("greeting").unwrap();
        let group = RandomGroup {
            id: "greeting/random".into(),
            title: [("en".to_string(), "Random".to_string())].into(),
            kind: Kind::Habit,
            schedule: Schedule::Daily,
            window: None,
            after: None,
            level_from: None,
            level_to: None,
        };
        pack.groups.insert(group.id.clone(), group);
        pack.tasks.get_mut("greeting/hello").unwrap().group = Some("greeting/random".into());
        pack.tasks.get_mut("greeting/breath").unwrap().group = Some("greeting/random".into());
    }

    let visible = fixture.engine.today_tasks(date);
    let card = visible
        .iter()
        .find(|task| task.id == "greeting/random")
        .expect("группа должна быть в «Сегодня»");
    assert!(fixture.engine.is_group_task(&card.id));
    // breath закрыт своим after = hello, поэтому группа обязана пропустить
    // его и показать hello.
    assert_eq!(
        card.title,
        fixture.engine.task("greeting/hello").unwrap().title
    );
    assert!(!visible.iter().any(|task| task.id == "greeting/hello"));
    assert!(!visible.iter().any(|task| task.id == "greeting/breath"));

    // Выбор не меняется от кадра к кадру.
    let again = fixture
        .engine
        .today_tasks(date)
        .into_iter()
        .find(|task| task.id == "greeting/random")
        .unwrap();
    assert_eq!(again.title.clone(), card.title.clone());

    let member_reward = card.reward;
    let outcome = fixture.engine.complete("greeting/random").unwrap();
    assert_eq!(outcome.awarded, member_reward);
    assert!(fixture.engine.is_done_today("greeting/random", date));
    let group = fixture
        .engine
        .display_task("greeting/random", date)
        .unwrap();
    assert!(!fixture.engine.is_due(&group, date));
}

#[test]
fn random_group_weight_biases_selection() {
    let mut fixture = Fixture::new();
    let start = fixture.engine.today();
    let hello_title = fixture.engine.task("greeting/hello").unwrap().title.clone();
    {
        let pack = fixture.engine.packs.packs.get_mut("greeting").unwrap();
        pack.groups.insert(
            "greeting/weighted".into(),
            RandomGroup {
                id: "greeting/weighted".into(),
                title: [("en".to_string(), "Weighted".to_string())].into(),
                kind: Kind::Habit,
                schedule: Schedule::Daily,
                window: None,
                after: None,
                level_from: None,
                level_to: None,
            },
        );
        let hello = pack.tasks.get_mut("greeting/hello").unwrap();
        hello.group = Some("greeting/weighted".into());
        hello.group_weight = Some(9.0);
        let breath = pack.tasks.get_mut("greeting/breath").unwrap();
        breath.group = Some("greeting/weighted".into());
        breath.group_weight = Some(1.0);
        // Убираем собственную цепочку, чтобы оба участника были доступны.
        breath.after = None;
    }
    let group = fixture
        .engine
        .packs
        .group("greeting/weighted")
        .unwrap()
        .clone();
    let mut heavy = 0;
    let mut light = 0;
    for offset in 0..200 {
        let date = start + chrono::Duration::days(offset);
        let card = fixture
            .engine
            .group_task(&group, date)
            .expect("группа должна выбрать участника");
        if card.title == hello_title {
            heavy += 1;
        } else {
            light += 1;
        }
    }
    assert!(
        heavy > light,
        "вес 9 должен выпадать чаще веса 1: {heavy} против {light}"
    );
}

#[test]
fn random_group_disappears_when_no_member_can_appear() {
    let mut fixture = Fixture::new();
    let date = fixture.engine.today();
    {
        let pack = fixture.engine.packs.packs.get_mut("greeting").unwrap();
        pack.groups.insert(
            "greeting/blocked".into(),
            RandomGroup {
                id: "greeting/blocked".into(),
                title: [("en".to_string(), "Blocked".to_string())].into(),
                kind: Kind::Habit,
                schedule: Schedule::Daily,
                window: None,
                after: None,
                level_from: None,
                level_to: None,
            },
        );
        pack.tasks.get_mut("greeting/hello").unwrap().group = Some("greeting/blocked".into());
        pack.tasks.get_mut("greeting/breath").unwrap().group = Some("greeting/blocked".into());
        pack.tasks.get_mut("greeting/hello").unwrap().after = Some("greeting/missing".into());
        pack.tasks.get_mut("greeting/breath").unwrap().after = Some("greeting/hello".into());
    }
    assert!(fixture
        .engine
        .display_task("greeting/blocked", date)
        .is_none());
    assert!(!fixture
        .engine
        .today_tasks(date)
        .iter()
        .any(|task| task.id == "greeting/blocked"));
}

#[test]
fn random_group_conditions_come_from_group_not_members() {
    let mut fixture = Fixture::new();
    let date = fixture.engine.today();
    let window = taskify_core::task::TimeWindow {
        from: taskify_core::task::ClockTime::parse("09:00").unwrap(),
        to: taskify_core::task::ClockTime::parse("10:00").unwrap(),
    };
    {
        let pack = fixture.engine.packs.packs.get_mut("greeting").unwrap();
        let group = RandomGroup {
            id: "greeting/conditions".into(),
            title: [("en".to_string(), "Conditions".to_string())].into(),
            kind: Kind::Habit,
            schedule: Schedule::Daily,
            window: Some(window),
            after: Some("greeting/smile".into()),
            level_from: Some(99),
            level_to: None,
        };
        pack.groups.insert(group.id.clone(), group);
        pack.tasks.get_mut("greeting/hello").unwrap().group = Some("greeting/conditions".into());
        pack.tasks.get_mut("greeting/breath").unwrap().group = Some("greeting/conditions".into());
        // Собственное окно участника не должно попадать в карточку группы;
        // оно только решает, можно ли вообще показать этого участника.
        pack.tasks.get_mut("greeting/hello").unwrap().window =
            Some(taskify_core::task::TimeWindow {
                from: taskify_core::task::ClockTime::parse("00:00").unwrap(),
                to: taskify_core::task::ClockTime::parse("00:00").unwrap(),
            });
    }
    let card = fixture
        .engine
        .display_task("greeting/conditions", date)
        .expect("группа");
    assert_eq!(card.window, Some(window));
    assert_eq!(card.after.as_deref(), Some("greeting/smile"));
    assert!(!fixture.engine.task_level_visible(&card));
    assert!(!fixture.engine.is_unlocked(&card));
    assert!(!fixture
        .engine
        .is_available_at(&card, chrono::NaiveTime::from_hms_opt(11, 0, 0).unwrap()));
}

#[test]
fn random_group_level_gate_uses_group_level() {
    let mut fixture = Fixture::new();
    let date = fixture.engine.today();
    {
        let pack = fixture.engine.packs.packs.get_mut("habits").unwrap();
        pack.groups.insert(
            "habits/gated".into(),
            RandomGroup {
                id: "habits/gated".into(),
                title: [("en".to_string(), "Gated".to_string())].into(),
                kind: Kind::Habit,
                schedule: Schedule::Daily,
                window: None,
                after: None,
                level_from: Some(99),
                level_to: None,
            },
        );
        pack.tasks.get_mut("habits/water").unwrap().group = Some("habits/gated".into());
        let morning = pack.tasks.get_mut("habits/morning").unwrap();
        morning.group = Some("habits/gated".into());
        morning.window = None;
    }
    let card = fixture.engine.display_task("habits/gated", date).unwrap();
    assert!(!fixture.engine.task_level_visible(&card));
    fixture
        .engine
        .packs
        .packs
        .get_mut("habits")
        .unwrap()
        .groups
        .get_mut("habits/gated")
        .unwrap()
        .level_from = Some(1);
    assert!(fixture.engine.task_level_visible(&card));
}

#[test]
fn random_group_window_and_chain_come_from_group() {
    let mut fixture = Fixture::new();
    let date = fixture.engine.today();
    {
        let pack = fixture.engine.packs.packs.get_mut("habits").unwrap();
        let window = taskify_core::task::TimeWindow {
            from: taskify_core::task::ClockTime::parse("09:00").unwrap(),
            to: taskify_core::task::ClockTime::parse("10:00").unwrap(),
        };
        pack.groups.insert(
            "habits/timed".into(),
            RandomGroup {
                id: "habits/timed".into(),
                title: [("en".to_string(), "Timed".to_string())].into(),
                kind: Kind::Habit,
                schedule: Schedule::Daily,
                window: Some(window),
                after: Some("habits/exercise".into()),
                level_from: None,
                level_to: None,
            },
        );
        pack.tasks.get_mut("habits/water").unwrap().group = Some("habits/timed".into());
        let morning = pack.tasks.get_mut("habits/morning").unwrap();
        morning.group = Some("habits/timed".into());
        morning.window = None;
    }
    let card = fixture.engine.display_task("habits/timed", date).unwrap();
    assert_eq!(
        card.window,
        fixture.engine.group("habits/timed").unwrap().window
    );
    assert!(fixture
        .engine
        .is_available_at(&card, chrono::NaiveTime::from_hms_opt(9, 30, 0).unwrap()));
    assert!(!fixture
        .engine
        .is_available_at(&card, chrono::NaiveTime::from_hms_opt(11, 0, 0).unwrap()));
    assert!(!fixture.engine.is_unlocked(&card));
    fixture.engine.complete("habits/exercise").unwrap();
    assert!(fixture.engine.is_unlocked(&card));
}

#[test]
fn random_group_one_shot_is_spent_after_completion() {
    let mut fixture = Fixture::new();
    let date = fixture.engine.today();
    {
        let pack = fixture.engine.packs.packs.get_mut("habits").unwrap();
        pack.groups.insert(
            "habits/once".into(),
            RandomGroup {
                id: "habits/once".into(),
                title: [("en".to_string(), "Once".to_string())].into(),
                kind: Kind::Once,
                schedule: Schedule::Once,
                window: None,
                after: None,
                level_from: None,
                level_to: None,
            },
        );
        pack.tasks.get_mut("habits/water").unwrap().group = Some("habits/once".into());
        let morning = pack.tasks.get_mut("habits/morning").unwrap();
        morning.group = Some("habits/once".into());
        morning.window = None;
    }
    let card = fixture.engine.display_task("habits/once", date).unwrap();
    assert!(card.is_one_shot());
    fixture.engine.complete("habits/once").unwrap();
    assert_eq!(
        fixture
            .engine
            .store
            .state
            .task_ref("habits/once")
            .unwrap()
            .completions_total,
        1
    );
    assert!(!fixture.engine.is_due(&card, date));
    assert!(!fixture
        .engine
        .is_due(&card, date + chrono::Duration::days(1)));
}

#[test]
fn random_group_xp_uses_average_member_reward() {
    let mut fixture = Fixture::new();
    {
        let pack = fixture.engine.packs.packs.get_mut("habits").unwrap();
        pack.groups.insert(
            "habits/xp".into(),
            RandomGroup {
                id: "habits/xp".into(),
                title: [("en".to_string(), "XP".to_string())].into(),
                kind: Kind::Habit,
                schedule: Schedule::Daily,
                window: None,
                after: None,
                level_from: None,
                level_to: None,
            },
        );
        let water = pack.tasks.get_mut("habits/water").unwrap();
        water.group = Some("habits/xp".into());
        water.group_weight = Some(100.0);
        let morning = pack.tasks.get_mut("habits/morning").unwrap();
        morning.group = Some("habits/xp".into());
        morning.window = None;
        morning.after = Some("habits/water".into());
        morning.reward.xp = 30;
        morning.group_weight = Some(1.0);
    }
    let pack = fixture.engine.packs.packs.get("habits").unwrap().clone();
    let before = fixture.engine.pack_xp(&pack);
    let outcome = fixture.engine.complete("habits/xp").unwrap();
    assert_eq!(outcome.awarded.xp, 10);
    assert_eq!(fixture.engine.pack_xp(&pack) - before, 20);
}

#[test]
fn random_group_done_card_survives_member_window_close() {
    let mut fixture = Fixture::new();
    let date = fixture.engine.today();
    {
        let pack = fixture.engine.packs.packs.get_mut("habits").unwrap();
        pack.groups.insert(
            "habits/done".into(),
            RandomGroup {
                id: "habits/done".into(),
                title: [("en".to_string(), "Done".to_string())].into(),
                kind: Kind::Habit,
                schedule: Schedule::Daily,
                window: None,
                after: None,
                level_from: None,
                level_to: None,
            },
        );
        pack.tasks.get_mut("habits/water").unwrap().group = Some("habits/done".into());
        let morning = pack.tasks.get_mut("habits/morning").unwrap();
        morning.group = Some("habits/done".into());
        morning.window = None;
    }
    fixture.engine.complete("habits/done").unwrap();
    {
        let pack = fixture.engine.packs.packs.get_mut("habits").unwrap();
        for id in ["habits/water", "habits/morning"] {
            pack.tasks.get_mut(id).unwrap().window = Some(taskify_core::task::TimeWindow {
                from: taskify_core::task::ClockTime::parse("23:00").unwrap(),
                to: taskify_core::task::ClockTime::parse("23:30").unwrap(),
            });
        }
    }
    assert!(fixture.engine.display_task("habits/done", date).is_some());
}

#[test]
fn random_group_subtask_ids_are_prefixed_by_member() {
    let mut fixture = Fixture::new();
    let date = fixture.engine.today();
    {
        let pack = fixture.engine.packs.packs.get_mut("habits").unwrap();
        pack.groups.insert(
            "habits/subtasks".into(),
            RandomGroup {
                id: "habits/subtasks".into(),
                title: [("en".to_string(), "Subtasks".to_string())].into(),
                kind: Kind::Habit,
                schedule: Schedule::Daily,
                window: None,
                after: None,
                level_from: None,
                level_to: None,
            },
        );
        let water = pack.tasks.get_mut("habits/water").unwrap();
        water.group = Some("habits/subtasks".into());
        water.after = Some("habits/missing".into());
        let morning = pack.tasks.get_mut("habits/morning").unwrap();
        morning.group = Some("habits/subtasks".into());
        morning.window = None;
    }
    let card = fixture
        .engine
        .display_task("habits/subtasks", date)
        .unwrap();
    assert_eq!(card.id, "habits/subtasks");
    fixture
        .engine
        .complete_subtask("habits/subtasks", "habits/morning::teeth")
        .unwrap();
    assert!(fixture
        .engine
        .store
        .state
        .task_ref("habits/subtasks")
        .unwrap()
        .subtasks_done
        .contains("habits/morning::teeth"));
}

#[test]
fn start_and_stop_keep_the_started_flag() {
    let mut fixture = Fixture::new();
    let date = fixture.engine.today();
    fixture.engine.start_task("habits/water").unwrap();
    assert_eq!(
        fixture
            .engine
            .store
            .state
            .task_ref("habits/water")
            .unwrap()
            .started,
        Some(date)
    );
    fixture.engine.stop_task("habits/water").unwrap();
    assert_eq!(
        fixture
            .engine
            .store
            .state
            .task_ref("habits/water")
            .unwrap()
            .started,
        None
    );
}

#[test]
fn cannot_start_a_locked_task() {
    let mut fixture = Fixture::new();
    let error = fixture.engine.start_task("greeting/breath").unwrap_err();
    assert!(error.to_string().contains("закрыто"));
}

#[test]
fn skip_habit_resets_streak_and_records_the_day() {
    let mut fixture = Fixture::new();
    let date = fixture.engine.today();
    fixture.engine.complete("habits/water").unwrap();
    assert_eq!(
        fixture
            .engine
            .store
            .state
            .task_ref("habits/water")
            .unwrap()
            .streak_current,
        1
    );
    fixture.engine.skip_habit("habits/water").unwrap();
    let state = fixture.engine.store.state.task_ref("habits/water").unwrap();
    assert_eq!(state.streak_current, 0);
    assert_eq!(state.last_skipped, Some(date));
    assert_eq!(fixture.engine.store.aggregates.totals.skips, 1);
}

#[test]
fn remaining_today_decreases_after_completion() {
    let mut fixture = Fixture::new();
    let date = fixture.engine.today();
    let before = crate::engine::remaining_today(&fixture.engine, date);
    assert!(before > 0);
    fixture.engine.complete("habits/water").unwrap();
    assert_eq!(
        crate::engine::remaining_today(&fixture.engine, date),
        before - 1
    );
}

#[test]
fn new_task_stays_marked_until_viewed() {
    let mut fixture = Fixture::new();
    // Задание без истории взаимодействий — новое.
    assert!(fixture.engine.is_task_new("habits/evening"));
    fixture.engine.mark_task_viewed("habits/evening").unwrap();
    assert!(!fixture.engine.is_task_new("habits/evening"));
    assert!(fixture.engine.is_task_new("greeting/hello"));
}

#[test]
fn new_tasks_are_notified_only_once() {
    let mut fixture = Fixture::new();
    let date = fixture.engine.today();
    let tasks = fixture.engine.unnotified_new_tasks(date);
    assert!(tasks.iter().any(|id| id == "greeting/hello"));
    fixture.engine.mark_tasks_notified(&tasks).unwrap();
    assert!(fixture.engine.unnotified_new_tasks(date).is_empty());
}

#[test]
fn settings_roundtrip_through_engine() {
    let mut fixture = Fixture::new();
    let mut settings = fixture.engine.settings().clone();
    settings.day_start_hour = 4;
    settings.weekly_xp_goal = 300;
    fixture.engine.set_settings(settings.clone()).unwrap();
    assert_eq!(fixture.engine.settings().day_start_hour, 4);
    assert_eq!(fixture.engine.settings().weekly_xp_goal, 300);
}

#[test]
fn greeting_and_guide_are_enabled_by_default_while_others_stay_off() {
    let root = crate::test_support::temp_root("greeting_default");
    let mut settings = Settings::default();
    settings.paths.livespaces_dir = Some(root.join("livespaces"));
    let engine = crate::bootstrap::prepare(&settings).expect("профиль должен открыться");
    let enabled: Vec<&str> = engine
        .packs
        .packs
        .values()
        .filter(|pack| pack.enabled)
        .map(|pack| pack.id.as_str())
        .collect();
    assert_eq!(enabled, vec!["greeting", "guide"]);
    assert_eq!(engine.packs.packs.get("greeting").unwrap().tasks.len(), 6);
    assert_eq!(engine.packs.packs.get("guide").unwrap().tasks.len(), 7);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn pack_state_matches_enable_and_disable_actions() {
    let root = crate::test_support::temp_root("pack_toggle");
    let mut settings = Settings::default();
    settings.paths.livespaces_dir = Some(root.join("livespaces"));
    let mut engine = crate::bootstrap::prepare(&settings).expect("профиль должен открыться");
    assert!(
        !engine.is_pack_disabled("greeting"),
        "стартовый пак включён"
    );
    assert!(engine.is_pack_disabled("habits"), "пак привычек выключен");

    engine
        .set_pack_enabled("habits", true)
        .expect("пак включается");
    engine.apply_disabled_packs();
    assert!(!engine.is_pack_disabled("habits"));
    assert!(engine.packs.packs["habits"].enabled);

    engine
        .set_pack_enabled("habits", false)
        .expect("пак выключается");
    engine.apply_disabled_packs();
    assert!(engine.is_pack_disabled("habits"));
    assert!(!engine.packs.packs["habits"].enabled);
    let _ = std::fs::remove_dir_all(&root);
}
