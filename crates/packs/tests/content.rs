//! Встроенные паки обязаны проходить валидатор: это часть сборки.

use taskify_packs::{format_issue, has_errors, validate_set, PackSet};

const PACKS: [&str; 7] = [
    "greeting",
    "guide",
    "cleaning",
    "habits",
    "inspiration",
    "social",
    "devday",
];

fn packs_root() -> std::path::PathBuf {
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("assets")
        .join("packs")
}

#[test]
fn builtin_packs_are_valid() {
    let set = PackSet::load(packs_root());
    assert_eq!(
        set.packs.len(),
        PACKS.len(),
        "должны остаться только новые паки: {:?}",
        set.packs.keys().collect::<Vec<_>>()
    );
    for id in PACKS {
        assert!(set.packs.contains_key(id), "нет пака {id}");
    }

    let issues = validate_set(&set);
    let report: Vec<String> = issues.iter().map(format_issue).collect();
    assert!(
        !has_errors(&issues),
        "ошибки валидации:\n{}",
        report.join("\n")
    );

    for pack in set.packs.values() {
        assert_eq!(
            pack.manifest.as_ref().map(|manifest| manifest.id.as_str()),
            Some(pack.id.as_str()),
            "манифест {}",
            pack.id
        );
        assert_eq!(
            pack.manifest.as_ref().map(|manifest| manifest.official),
            Some(true),
            "встроенный пак {} должен быть официальным",
            pack.id
        );
        assert!(!pack.tasks.is_empty(), "пак {} без заданий", pack.id);
        assert!(!pack.categories.is_empty(), "пак {} без категорий", pack.id);
        for task in pack.tasks.values() {
            assert!(
                task.reward.xp >= 10,
                "у {}/{} меньше 10 опыта",
                pack.id,
                task.id
            );
            assert!(
                task.reward.coins >= 10,
                "у {}/{} меньше 10 монет",
                pack.id,
                task.id
            );
            assert!(
                task.title.contains_key("en") && task.title.contains_key("ru"),
                "у {}/{} нет ru/en названия",
                pack.id,
                task.id
            );
            let category = task.category.as_deref().unwrap_or_default();
            assert!(
                pack.categories.contains_key(category)
                    || pack
                        .categories
                        .contains_key(&format!("{}/{}", pack.id, category)),
                "у {}/{} неизвестная категория {category}",
                pack.id,
                task.id
            );
        }
    }
}

#[test]
fn greeting_and_guide_are_enabled_by_default() {
    let set = PackSet::load(packs_root());
    for id in PACKS {
        let manifest = set.packs.get(id).and_then(|pack| pack.manifest.as_ref());
        let enabled = manifest.map(|manifest| manifest.default_enabled);
        assert_eq!(
            enabled,
            Some(id == "greeting" || id == "guide"),
            "у пака {id} неверный default_enabled"
        );
    }
}

#[test]
fn greeting_is_one_chain_with_a_combo_and_achievement() {
    let set = PackSet::load(packs_root());
    let pack = set.packs.get("greeting").expect("пак greeting");
    let chain = ["hello", "breath", "window", "note", "water", "smile"];
    let mut previous: Option<&str> = None;
    for id in chain {
        let task_id = format!("greeting/{id}");
        let task = pack.tasks.get(&task_id).expect("шаг цепочки");
        assert_eq!(
            task.after.as_deref(),
            previous.map(|prev| format!("greeting/{prev}")).as_deref(),
            "у {task_id} неверная связь"
        );
        previous = Some(id);
    }
    assert!(pack.combos.contains_key("greeting/hello"));
    assert!(pack.achievements.contains_key("greeting/first_step"));
}

#[test]
fn guide_is_a_seven_step_chain() {
    let set = PackSet::load(packs_root());
    let pack = set.packs.get("guide").expect("пак guide");
    let chain = [
        "open_today",
        "pick_step",
        "mark_done",
        "open_week",
        "open_market",
        "apply_style",
        "finish",
    ];
    assert_eq!(pack.tasks.len(), chain.len());
    let mut previous: Option<&str> = None;
    for id in chain {
        let task_id = format!("guide/{id}");
        let task = pack.tasks.get(&task_id).expect("шаг знакомства");
        assert_eq!(
            task.after.as_deref(),
            previous.map(|prev| format!("guide/{prev}")).as_deref(),
            "у {task_id} неверная связь"
        );
        assert!(
            task.reward.xp >= 10 && task.reward.coins >= 10,
            "у {task_id} меньше 10 опыта или монет"
        );
        previous = Some(id);
    }
}

#[test]
fn cleaning_has_a_room_by_room_chain() {
    let set = PackSet::load(packs_root());
    let pack = set.packs.get("cleaning").expect("пак cleaning");
    assert_eq!(pack.tasks.len(), 52, "план дома должен остаться подробным");
    let first = pack
        .tasks
        .get("cleaning/kitchen_trash")
        .expect("первый шаг кухни");
    assert_eq!(first.after.as_deref(), Some("cleaning/choose_room"));
    let wash = pack
        .tasks
        .get("cleaning/kitchen_wash")
        .expect("мытьё посуды");
    assert_eq!(wash.after.as_deref(), Some("cleaning/kitchen_collect"));
    for room in ["kitchen", "bath", "bed", "living", "hall"] {
        assert!(
            pack.tasks
                .keys()
                .any(|id| id.starts_with(&format!("cleaning/{room}_"))),
            "нет цепочки комнаты {room}"
        );
    }
}

#[test]
fn habits_grow_with_pack_level() {
    let set = PackSet::load(packs_root());
    let pack = set.packs.get("habits").expect("пак habits");
    let basic = pack
        .task_levels
        .get("habits/morning")
        .expect("рамка базовой рутины");
    let advanced = pack
        .task_levels
        .get("habits/morning_plus")
        .expect("рамка продвинутой рутины");
    assert_eq!(basic.to, Some(2));
    assert_eq!(advanced.from, Some(3));

    let exercise = pack.tasks.get("habits/exercise").expect("физкульт-привет");
    assert_eq!(exercise.subtasks.len(), 6, "шесть упражнений");
    assert!(exercise.subtasks.iter().any(|subtask| subtask
        .title
        .get("ru")
        .is_some_and(|title| title.contains("Присесть"))));
}

#[test]
fn devday_has_full_menu_and_schedule() {
    let set = PackSet::load(packs_root());
    let pack = set.packs.get("devday").expect("пак devday");

    let mains: Vec<_> = pack
        .tasks
        .values()
        .filter(|task| task.group.as_deref() == Some("devday/main-pool"))
        .collect();
    let snacks: Vec<_> = pack
        .tasks
        .values()
        .filter(|task| task.group.as_deref() == Some("devday/snack-pool"))
        .collect();
    assert_eq!(mains.len(), 100, "основных блюд должно быть 100");
    assert_eq!(snacks.len(), 50, "перекусов должно быть 50");

    let first = pack.tasks.get("devday/main-001").expect("первое блюдо");
    assert_eq!(
        first.title("ru"),
        "- Гречка\n- куриное бедро с овощами в мультиварке."
    );
    assert!(first
        .description("ru")
        .contains("- Гречка 50 г сухой\n- вода 150 мл"));
    assert!(first
        .description("ru")
        .ends_with("КБЖУ: 664 ккал, Б 58, Ж 28, У 44."));

    for task in mains.iter().chain(snacks.iter()) {
        assert!(
            !task.title("ru").contains(" + "),
            "{}: плюс в названии",
            task.id
        );
        assert!(
            !task.description("ru").contains(" + "),
            "{}: плюс в описании",
            task.id
        );
        assert!(!task.description("ru").trim().is_empty());
    }

    let charge = pack.tasks.get("devday/charge").expect("зарядка");
    assert_eq!(charge.subtasks.len(), 8, "в зарядке 8 упражнений");
    let window = charge.window.expect("окно зарядки");
    assert_eq!(window.from.to_string(), "10:00");
    assert_eq!(window.to.to_string(), "11:00");

    let walk = pack.tasks.get("devday/walk").expect("прогулка");
    let walk_window = walk.window.expect("окно прогулки");
    assert_eq!(walk_window.from.to_string(), "19:00");
    assert_eq!(walk_window.to.to_string(), "02:00");

    let visible: Vec<_> = pack.groups.values().filter(|group| !group.hidden).collect();
    let hidden: Vec<_> = pack.groups.values().filter(|group| group.hidden).collect();
    assert_eq!(hidden.len(), 3, "три служебных пула");
    assert_eq!(visible.len(), 14, "5 приёмов пищи и 9 разминок");
    for hour in 11..20 {
        let group = pack
            .groups
            .get(&format!("devday/warmup-{hour}"))
            .expect("группа разминки");
        assert_eq!(group.pick, 2);
        assert_eq!(pack.members_of(group).len(), 12);
    }
    for (name, from, to) in [
        ("breakfast", "10:00", "11:00"),
        ("lunch", "13:00", "13:30"),
        ("dinner", "16:00", "17:00"),
        ("supper", "19:30", "20:30"),
        ("evening-snack", "22:30", "23:00"),
    ] {
        let group = pack
            .groups
            .get(&format!("devday/{name}"))
            .expect("группа приёма пищи");
        let window = group.window.expect("окно приёма пищи");
        assert_eq!(window.from.to_string(), from);
        assert_eq!(window.to.to_string(), to);
        assert_eq!(
            pack.members_of(group).len(),
            if name.contains("snack") || name == "lunch" {
                50
            } else {
                100
            }
        );
    }
}

#[test]
fn packs_are_indexed_without_loss() {
    let set = PackSet::load(packs_root());
    let tasks: usize = set.packs.values().map(|pack| pack.tasks.len()).sum();
    let combos: usize = set.packs.values().map(|pack| pack.combos.len()).sum();
    let achievements: usize = set.packs.values().map(|pack| pack.achievements.len()).sum();
    assert_eq!(set.tasks().len(), tasks);
    assert_eq!(set.combos().len(), combos);
    assert_eq!(set.achievements().len(), achievements);
    assert!(set.task("greeting/hello").is_some());
}

#[test]
fn greeting_keeps_base_cosmetics_and_media() {
    let set = PackSet::load(packs_root());
    let pack = set.packs.get("greeting").expect("пак greeting");
    let theme = pack
        .tokens
        .values()
        .find(|token| token.kind == taskify_packs::TokenKind::Theme)
        .expect("базовая тема");
    let ids: Vec<&str> = theme
        .variants
        .iter()
        .map(|variant| variant.id.as_str())
        .collect();
    assert!(ids.contains(&"light") && ids.contains(&"dark"), "{ids:?}");
    assert!(!pack.bundles.is_empty(), "нет наборов косметики");
    let image = pack
        .tokens
        .values()
        .find(|token| token.kind == taskify_packs::TokenKind::Image)
        .expect("картинка фона");
    for variant in &image.variants {
        let path = pack.root.join(&variant.value);
        assert!(path.exists(), "нет файла {}", path.display());
    }
}
