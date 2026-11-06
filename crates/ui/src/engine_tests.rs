//! Сквозные тесты правил: награды, комбо, ачивки, отмена, покупки.

use std::path::PathBuf;

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
        id: "starter/legacy".into(),
        price: 0,
        base: None,
        title: Default::default(),
        description: Default::default(),
        tokens: Vec::new(),
        stickers: vec!["starter/coffee".into()],
    };
    assert_eq!(fixture.engine.bundle_full_price(&bundle), 0);
    fixture.engine.buy_bundle(&bundle).unwrap();
    assert!(fixture.engine.bundle_owned("starter/legacy"));
    assert!(!fixture.engine.sticker_owned("starter/coffee"));
    assert_eq!(fixture.engine.store.state.stickers.len(), 0);
}

#[test]
fn completing_a_task_gives_reward_and_unlocks_first_achievement() {
    let mut fixture = Fixture::new();
    let outcome = fixture.engine.complete("starter/water").unwrap();
    assert_eq!(outcome.awarded.xp, 10);
    assert_eq!(outcome.awarded.coins, 2);
    assert!(outcome
        .achievements
        .contains(&"starter/first_step".to_string()));
    // Ачивки могут идти каскадом (ачивка за уровень даёт опыт и новый уровень),
    // поэтому проверяем участие первой ачивки и рост опыта, а не точную сумму.
    assert!(outcome
        .achievements
        .iter()
        .any(|id| id == "starter/first_step"));
    assert!(fixture.engine.store.state.xp_total > 10);
    assert!(fixture
        .engine
        .store
        .state
        .achievements
        .contains("starter/first_step"));
}

#[test]
fn same_task_cannot_be_completed_twice_a_day() {
    let mut fixture = Fixture::new();
    fixture.engine.complete("starter/water").unwrap();
    let error = fixture.engine.complete("starter/water").unwrap_err();
    assert!(error.to_string().contains("уже выполнено"));
}

#[test]
fn completed_one_shot_never_becomes_due_again() {
    let mut fixture = Fixture::new();
    let task = fixture
        .engine
        .packs
        .task("starter/trip")
        .cloned()
        .expect("ваншот из стартового пака должен существовать");
    let today = fixture.engine.today();
    assert!(task.is_one_shot());
    assert!(fixture.engine.is_due(&task, today));

    fixture.engine.complete("starter/trip").unwrap();

    assert!(fixture.engine.is_spent_one_shot(&task));
    assert!(!fixture.engine.is_due(&task, today));
    assert!(!fixture
        .engine
        .is_due(&task, today + chrono::Duration::days(1)));
}

#[test]
fn three_morning_tasks_award_the_combo() {
    let mut fixture = Fixture::new();
    let mut combos = Vec::new();
    for task in ["starter/water", "starter/stretch", "starter/journal"] {
        let outcome = fixture.engine.complete(task).unwrap();
        combos.extend(outcome.combos);
    }
    assert!(combos.contains(&"starter/morning".to_string()));
    let state = &fixture.engine.store.state;
    assert!(
        state.combos["starter/morning"].awarded.contains("once")
            || !state.combos["starter/morning"].awarded.is_empty()
    );
}

#[test]
fn rejection_gives_nothing_and_task_returns() {
    let mut fixture = Fixture::new();
    fixture
        .engine
        .reject("starter/water", Some("передумал".into()))
        .unwrap();
    let state = &fixture.engine.store.state;
    assert_eq!(state.xp_total, 0);
    assert_eq!(state.coins, 0);
    assert_eq!(state.tasks["starter/water"].rejected_total, 1);
    assert!(!fixture
        .engine
        .is_done_today("starter/water", fixture.engine.today()));
}

#[test]
fn undo_returns_reward_within_window() {
    let mut fixture = Fixture::new();
    fixture.engine.complete("starter/water").unwrap();
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
        .purchase("starter/color.accent", "green")
        .unwrap_err();
    assert!(error.to_string().contains("недоступен"));

    fixture.engine.store.state.coins = 500;
    fixture
        .engine
        .purchase("starter/color.accent", "green")
        .unwrap();
    assert!(fixture.engine.owned("starter/color.accent", "green"));
    assert_eq!(fixture.engine.store.state.coins, 400);
}

#[test]
fn base_cosmetics_work_with_disabled_pack() {
    let mut fixture = Fixture::new();
    fixture.engine.set_pack_enabled("starter", false).unwrap();
    fixture.engine.apply_disabled_packs();
    assert!(!fixture.engine.packs.packs["starter"].enabled);
    assert!(fixture.engine.owned("starter/theme", "dark"));
    assert_eq!(
        fixture
            .engine
            .variant_value("starter/theme", "dark")
            .as_deref(),
        Some("dark")
    );
    assert_eq!(
        fixture
            .engine
            .packs
            .token_any("starter/theme")
            .map(|token| token.kind),
        Some(taskify_packs::TokenKind::Theme)
    );
    assert!(fixture
        .engine
        .packs
        .tokens_any()
        .iter()
        .any(|token| token.id == "starter/theme"));
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
        .complete_subtask("starter/trip", "passport")
        .unwrap();
    assert_eq!(reward.xp, 2);
    assert!(fixture
        .engine
        .store
        .state
        .task_ref("starter/trip")
        .unwrap()
        .subtasks_done
        .contains("passport"));
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
fn welcome_pack_is_enabled_by_default_while_base_packs_stay_off() {
    let root = crate::test_support::temp_root("welcome_default");
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
    assert_eq!(enabled, vec!["welcome"]);
    assert!(engine.packs.packs.get("welcome").unwrap().tasks.len() >= 10);
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn pack_state_matches_enable_and_disable_actions() {
    let root = crate::test_support::temp_root("pack_toggle");
    let mut settings = Settings::default();
    settings.paths.livespaces_dir = Some(root.join("livespaces"));
    let mut engine = crate::bootstrap::prepare(&settings).expect("профиль должен открыться");
    assert!(engine.is_pack_disabled("starter"), "базовый пак выключен");
    assert!(!engine.is_pack_disabled("welcome"), "обучающий пак включён");

    engine
        .set_pack_enabled("starter", true)
        .expect("пак включается");
    engine.apply_disabled_packs();
    assert!(!engine.is_pack_disabled("starter"));
    assert!(engine.packs.packs["starter"].enabled);

    engine
        .set_pack_enabled("starter", false)
        .expect("пак выключается");
    engine.apply_disabled_packs();
    assert!(engine.is_pack_disabled("starter"));
    assert!(!engine.packs.packs["starter"].enabled);
    let _ = std::fs::remove_dir_all(&root);
}
