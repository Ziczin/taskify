//! Встроенные паки обязаны проходить валидатор: это часть сборки.

use taskify_packs::{format_issue, has_errors, validate_set, PackSet};

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
    assert!(
        set.packs.len() >= 7,
        "ожидалось не меньше семи встроенных паков, а нашлось {}",
        set.packs.len()
    );

    let issues = validate_set(&set);
    let report: Vec<String> = issues.iter().map(format_issue).collect();
    assert!(
        !has_errors(&issues),
        "ошибки валидации:\n{}",
        report.join("\n")
    );

    for pack in set.packs.values() {
        let name = pack.manifest.as_ref().map(|manifest| manifest.id.as_str());
        assert_eq!(name, Some(pack.id.as_str()), "манифест {}", pack.id);
        assert_eq!(
            pack.manifest.as_ref().map(|manifest| manifest.official),
            Some(true),
            "встроенный пак {} должен быть официальным",
            pack.id
        );
        // Пак может быть чисто косметическим или ачивочным — пустым он быть не должен.
        assert!(
            !(pack.tasks.is_empty()
                && pack.achievements.is_empty()
                && pack.tokens.is_empty()
                && pack.bundles.is_empty()),
            "пак {} пустой",
            pack.id
        );
        assert!(
            !pack.categories.is_empty() || !pack.tokens.is_empty() || !pack.achievements.is_empty(),
            "в паке {} нет ни категорий, ни косметики, ни ачивок",
            pack.id
        );
        for task in pack.tasks.values() {
            assert!(
                task.title.contains_key("en"),
                "у {}/{} нет английского названия",
                pack.id,
                task.id
            );
            assert!(
                task.title.contains_key("ru"),
                "у {}/{} нет русского названия",
                pack.id,
                task.id
            );
        }
    }
}

#[test]
fn starter_pack_keeps_its_shape() {
    let set = PackSet::load(packs_root());
    let pack = set.packs.get("starter").expect("пак starter");
    assert_eq!(pack.tasks.len(), 20, "двадцать заданий");
    assert_eq!(pack.combos.len(), 3, "три комбо");
    assert_eq!(pack.achievements.len(), 10, "десять ачивок");
    assert!(pack.categories.len() >= 5);
    assert!(pack.icons.len() >= 15);
    assert!(pack.tokens.len() >= 4);

    let theme = pack
        .tokens
        .values()
        .find(|token| token.kind == taskify_packs::TokenKind::Theme)
        .expect("в стартовом паке должна быть тема");
    let ids: Vec<&str> = theme
        .variants
        .iter()
        .map(|variant| variant.id.as_str())
        .collect();
    assert!(ids.contains(&"light"), "нет светлой темы: {ids:?}");
    assert!(ids.contains(&"dark"), "нет тёмной темы: {ids:?}");
    for variant in &theme.variants {
        assert!(
            variant
                .palette
                .as_ref()
                .and_then(|palette| palette.bg.as_ref())
                .is_some(),
            "у темы {} нет цвета фона",
            variant.id
        );
    }
}

#[test]
fn demo_pack_has_multiple_dark_themes() {
    let set = PackSet::load(packs_root());
    let pack = set.packs.get("test_all").expect("демопак");
    let theme = pack
        .tokens
        .values()
        .find(|token| token.kind == taskify_packs::TokenKind::Theme)
        .expect("в демопаке должна быть тема");
    let dark: Vec<_> = theme
        .variants
        .iter()
        .filter(|variant| variant.value == "dark")
        .collect();
    assert!(
        dark.len() >= 4,
        "в демопаке мало тёмных тем: {}",
        dark.len()
    );
    for variant in dark {
        let palette = variant.palette.as_ref().expect("палитра тёмной темы");
        for (name, value) in [
            ("bg", &palette.bg),
            ("card", &palette.card),
            ("text", &palette.text),
            ("accent", &palette.accent),
        ] {
            assert!(value.is_some(), "у {} нет {}", variant.id, name);
        }
    }
    let dark_bundles = pack
        .bundles
        .values()
        .filter(|bundle| bundle.theme_base().as_str() == "dark")
        .count();
    assert!(
        dark_bundles >= 4,
        "в демопаке мало тёмных наборов: {dark_bundles}"
    );
}

#[test]
fn packs_are_loadable_by_id() {
    let set = PackSet::load(packs_root());
    assert!(set.task("starter/water").is_some());
    assert!(set.combo("starter/morning").is_some());
    assert!(set.achievement("starter/first_step").is_some());
    assert!(set.category("starter/body").is_some());
    // Сумма по пакам: индекс не теряет и не дублирует сущности.
    let tasks: usize = set.packs.values().map(|pack| pack.tasks.len()).sum();
    let combos: usize = set.packs.values().map(|pack| pack.combos.len()).sum();
    let achievements: usize = set.packs.values().map(|pack| pack.achievements.len()).sum();
    assert_eq!(set.tasks().len(), tasks);
    assert_eq!(set.combos().len(), combos);
    assert_eq!(set.achievements().len(), achievements);
    assert!(set.packs.contains_key("welcome"), "нет обучающего пака");
}
