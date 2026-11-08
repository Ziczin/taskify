//! Валидатор пака: коды PK001–PK016 и предупреждения W001–W006.

use std::collections::{BTreeMap, BTreeSet};

use taskify_core::task::{is_valid_id, Task, KNOWN_LOCALES};

use crate::entities::{Pack, TokenKind, Unlock};
use crate::loader::{Issue, PackSet, Severity};
use crate::manifest::PACK_SCHEMA;

/// Лимиты по умолчанию: косметик-паки бывают большими.
pub const DEFAULT_MAX_BYTES: u64 = 512 * 1024 * 1024;
pub const DEFAULT_MAX_FILES: usize = 20_000;
pub const DEFAULT_MAX_RATIO: f64 = 200.0;

pub fn validate_pack(pack: &Pack) -> Vec<Issue> {
    let mut issues = Vec::new();
    let root = pack.root.display().to_string();

    let Some(manifest) = &pack.manifest else {
        issues.push(Issue::error("PK001", root, "нет манифеста pack.toml"));
        return issues;
    };

    if manifest.schema > PACK_SCHEMA {
        issues.push(Issue::error(
            "PK002",
            "pack.toml",
            format!("схема {} новее {}", manifest.schema, PACK_SCHEMA),
        ));
    }

    if !is_valid_id(&pack.id) {
        issues.push(Issue::error(
            "PK003",
            "pack.toml",
            format!("недопустимый id пака: {}", pack.id),
        ));
    }

    if manifest.official && !pack.metrics_declared.is_empty() {
        for metric in &pack.metrics_declared {
            if metric.starts_with("sys.") {
                issues.push(Issue::error(
                    "PK006",
                    "definitions/metrics.toml",
                    format!("префикс sys. зарезервирован: {metric}"),
                ));
            }
        }
    }

    for task in pack.tasks.values() {
        let file = format!("tasks/{}.toml", short(&task.id));
        for error in task.validate() {
            let code = match &error {
                taskify_core::task::TaskError::MilestoneEveryZero { .. } => "PK009",
                taskify_core::task::TaskError::MilestoneMultiplierBelowOne { .. } => "PK009",
                taskify_core::task::TaskError::InvalidCurrency { .. } => "PK008",
                other => other.code(),
            };
            issues.push(Issue::error(code, &file, format!("{error:?}")).with_field("task"));
        }

        if let Some(category) = &task.category {
            if !pack.categories.contains_key(category) {
                issues.push(Issue::error(
                    "PK005",
                    &file,
                    format!("неизвестная категория: {category}"),
                ));
            }
        }
        for tag in &task.tags {
            if !pack.tags.contains_key(tag) {
                issues.push(Issue::error(
                    "PK005",
                    &file,
                    format!("неизвестный тег: {tag}"),
                ));
            }
        }
        if let Some(icon) = &task.icon {
            if !pack.icons.contains_key(icon) {
                issues.push(Issue::error(
                    "PK005",
                    &file,
                    format!("неизвестная иконка: {icon}"),
                ));
            }
        }
        for combo in &task.combos {
            if !pack.combos.contains_key(combo) {
                issues.push(Issue::error(
                    "PK013",
                    &file,
                    format!("ссылка на несуществующее комбо: {combo}"),
                ));
            }
        }
        for metric in task.metrics.keys() {
            if !metric.starts_with("sys.") && !pack.metrics_declared.contains(metric) {
                issues.push(Issue::error(
                    "PK006",
                    &file,
                    format!("неизвестная метрика: {metric}"),
                ));
            }
        }

        if manifest.official && (task.reward.xp < 0 || task.reward.coins < 0) {
            issues.push(Issue::warning(
                "W001",
                &file,
                "отрицательная награда в официальном паке",
            ));
        }

        if !task.title.contains_key("en") {
            issues.push(Issue::warning(
                "W002",
                &file,
                "нет английского перевода, будет показан технический id",
            ));
        }
        for code in task.title.keys() {
            if !KNOWN_LOCALES.contains(&code.as_str()) {
                issues.push(Issue::error(
                    "PK004",
                    &file,
                    format!("неизвестный язык: {code}"),
                ));
            }
        }
    }

    for group in pack.groups.values() {
        let file = format!("groups/{}.toml", short(&group.id));
        if !group.title.contains_key("en") {
            issues.push(Issue::warning(
                "W002",
                &file,
                "нет английского названия группы",
            ));
        }
        for code in group.title.keys() {
            if !KNOWN_LOCALES.contains(&code.as_str()) {
                issues.push(Issue::error(
                    "PK004",
                    &file,
                    format!("неизвестный язык: {code}"),
                ));
            }
        }
    }

    for combo in pack.combos.values() {
        let file = format!("combos/{}.toml", short(&combo.id));
        for member in &combo.members {
            if !pack.tasks.contains_key(member) {
                issues.push(Issue::error(
                    "PK005",
                    &file,
                    format!("участник не существует: {member}"),
                ));
            }
        }
        // PK007: состав комбо и обратные ссылки из заданий совпадают.
        for member in &combo.members {
            if let Some(task) = pack.tasks.get(member) {
                if !task.combos.contains(&combo.id) {
                    issues.push(Issue::error(
                        "PK007",
                        &file,
                        format!("задание {member} не ссылается на комбо {}", combo.id),
                    ));
                }
            }
        }
        for (task_id, task) in &pack.tasks {
            if task.combos.contains(&combo.id) && !combo.members.contains(task_id) {
                issues.push(Issue::error(
                    "PK007",
                    &file,
                    format!("задание {task_id} ссылается на комбо, но не входит в состав"),
                ));
            }
        }
    }

    for achievement in pack.achievements.values() {
        let file = format!("achievements/{}.toml", short(&achievement.id));
        for (metric, _, _) in achievement.condition.leaves() {
            if !metric.starts_with("sys.") && !pack.metrics_declared.contains(&metric.to_string()) {
                issues.push(Issue::error(
                    "PK006",
                    &file,
                    format!("неизвестная метрика в условии: {metric}"),
                ));
            }
        }
        if let Some(sticker) = &achievement.sticker {
            let own_prefix = format!("{}/", pack.id);
            if sticker.starts_with(&own_prefix) && !pack.stickers.contains_key(sticker) {
                issues.push(Issue::error(
                    "PK015",
                    &file,
                    format!("нет стикера пака: {sticker}"),
                ));
            }
        }
        if !achievement.title.contains_key("en") {
            issues.push(Issue::warning("W002", &file, "нет английского названия"));
        }
    }

    for bundle in pack.bundles.values() {
        let file = format!("bundles/{}.toml", short(&bundle.id));
        if bundle.price <= 0 {
            issues.push(Issue::error("PK015", &file, "у набора нет цены"));
        }
        if bundle.tokens.is_empty() {
            issues.push(Issue::warning("W005", &file, "тема пустая"));
        }
        if !bundle.stickers.is_empty() {
            issues.push(Issue::warning(
                "W006",
                &file,
                "стикеры в теме игнорируются: тема — только оформление",
            ));
        }
        for item in &bundle.tokens {
            let Some((token, variant)) = item.split_once(':') else {
                issues.push(Issue::error(
                    "PK015",
                    &file,
                    format!("ссылка без варианта: {item}"),
                ));
                continue;
            };
            match pack.tokens.get(token) {
                Some(def) => {
                    if !def.variants.iter().any(|item| item.id == variant) {
                        issues.push(Issue::error(
                            "PK014",
                            &file,
                            format!("нет варианта {variant} у {token}"),
                        ));
                    }
                }
                None => issues.push(Issue::error("PK015", &file, format!("нет токена {token}"))),
            }
        }
    }

    for sticker in pack.stickers.values() {
        let file = format!("definitions/stickers/{}.toml", short(&sticker.id));
        if !is_valid_id(short(&sticker.id)) {
            issues.push(Issue::error(
                "PK003",
                &file,
                format!("недопустимый id стикера: {}", sticker.id),
            ));
        }
        if sticker.icon.trim().is_empty() && sticker.image.trim().is_empty() {
            issues.push(Issue::error(
                "PK014",
                &file,
                "у стикера нет ни icon, ни image",
            ));
        }
        if !sticker.image.trim().is_empty() {
            let image = std::path::Path::new(&sticker.image);
            if sticker.image.contains("..") || image.is_absolute() {
                issues.push(Issue::error(
                    "PK011",
                    &file,
                    format!("недопустимый путь картинки: {}", sticker.image),
                ));
            } else if !sticker.image.to_ascii_lowercase().ends_with(".png") {
                issues.push(Issue::error(
                    "PK014",
                    &file,
                    format!("картинка стикера должна быть PNG: {}", sticker.image),
                ));
            } else if !pack.root.join(&sticker.image).exists() {
                issues.push(Issue::error(
                    "PK010",
                    &file,
                    format!("нет файла картинки: {}", sticker.image),
                ));
            }
        }
        if !sticker.scale.is_finite() || sticker.scale <= 0.0 {
            issues.push(Issue::error("PK015", &file, "недопустимый scale стикера"));
        }
        if sticker.price < 0 {
            issues.push(Issue::error("PK015", &file, "у стикера отрицательная цена"));
        }
        if !sticker.name.contains_key("en") {
            issues.push(Issue::warning("W002", &file, "нет английского названия"));
        }
    }

    for token in pack.tokens.values() {
        let file = format!("cosmetics/{}.toml", short(&token.id));
        if token.variants.is_empty() {
            issues.push(Issue::warning("W003", &file, "у токена нет вариантов"));
        }
        let mut seen = BTreeSet::new();
        for variant in &token.variants {
            if !seen.insert(&variant.id) {
                issues.push(Issue::error(
                    "PK014",
                    &file,
                    format!("повторяющийся вариант: {}", variant.id),
                ));
            }
            if matches!(token.kind, TokenKind::Image | TokenKind::Sound) {
                let asset = pack.root.join(&variant.value);
                if variant.value.contains("..") || variant.value.starts_with('/') {
                    issues.push(Issue::error(
                        "PK011",
                        &file,
                        format!("недопустимый путь: {}", variant.value),
                    ));
                } else if !asset.exists() {
                    issues.push(Issue::error(
                        "PK010",
                        &file,
                        format!("нет файла: {}", variant.value),
                    ));
                }
            }
            if let Unlock::Achievement { id } = &variant.unlock {
                if !pack.achievements.contains_key(id) {
                    issues.push(Issue::error(
                        "PK013",
                        &file,
                        format!("ссылка на несуществующую ачивку: {id}"),
                    ));
                }
            }
        }
    }

    // PK012: размеры пака.
    let (bytes, files) = measure(&pack.root);
    if bytes > DEFAULT_MAX_BYTES || files > DEFAULT_MAX_FILES {
        issues.push(Issue::error(
            "PK012",
            root,
            format!("пак больше лимитов: {bytes} байт, {files} файлов"),
        ));
    }

    issues
}

fn short(id: &str) -> &str {
    id.split_once('/').map(|(_, rest)| rest).unwrap_or(id)
}

fn measure(root: &std::path::Path) -> (u64, usize) {
    let mut bytes = 0u64;
    let mut files = 0usize;
    for entry in walkdir::WalkDir::new(root).into_iter().flatten() {
        if entry.file_type().is_file() {
            files += 1;
            bytes += entry.metadata().map(|meta| meta.len()).unwrap_or(0);
        }
    }
    (bytes, files)
}

pub fn validate_set(set: &PackSet) -> Vec<Issue> {
    let mut issues = set.issues.clone();
    for pack in set.packs.values() {
        issues.extend(validate_pack(pack));
    }

    // Цепочки заданий: предыдущая должна существовать, а цикл не должен
    // запереть всю цепочку навсегда.
    let tasks: BTreeMap<&str, &Task> = set
        .packs
        .values()
        .flat_map(|pack| pack.tasks.values())
        .map(|task| (task.id.as_str(), task))
        .collect();
    for task in tasks.values() {
        let Some(after) = task.after.as_deref() else {
            continue;
        };
        let file = format!("tasks/{}.toml", short(&task.id));
        let mut cursor = after;
        let mut seen = BTreeSet::new();
        loop {
            if !seen.insert(cursor.to_string()) {
                issues.push(Issue::error(
                    "PK016",
                    &file,
                    format!("цикл в цепочке заданий: {cursor}"),
                ));
                break;
            }
            let Some(previous) = tasks
                .get(cursor)
                .copied()
                .or_else(|| tasks.values().copied().find(|item| item.id_in_use(cursor)))
            else {
                issues.push(Issue::error(
                    "PK016",
                    &file,
                    format!("предыдущая задача не найдена: {cursor}"),
                ));
                break;
            };
            if previous.id == task.id {
                issues.push(Issue::error(
                    "PK016",
                    &file,
                    format!("задача ссылается сама на себя: {cursor}"),
                ));
                break;
            }
            match previous.after.as_deref() {
                Some(next) => cursor = next,
                None => break,
            }
        }
    }

    issues
}

pub fn has_errors(issues: &[Issue]) -> bool {
    issues.iter().any(|issue| issue.severity == Severity::Error)
}

/// Человекочитаемая строка для отчёта.
pub fn format_issue(issue: &Issue) -> String {
    let severity = match issue.severity {
        Severity::Error => "ошибка",
        Severity::Warning => "предупреждение",
    };
    match &issue.field {
        Some(field) => format!(
            "{severity} {} [{}] {}: {}",
            issue.code, issue.file, field, issue.message
        ),
        None => format!(
            "{severity} {} {}: {}",
            issue.code, issue.file, issue.message
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::entities::Condition;
    use std::path::PathBuf;

    fn temp_root() -> PathBuf {
        crate::test_support::temp_root("val")
    }

    fn write(path: &std::path::Path, text: &str) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        std::fs::write(path, text).unwrap();
    }

    #[test]
    fn clean_pack_has_no_errors() {
        let root = temp_root();
        write(
            &root.join("demo/pack.toml"),
            "id = \"demo\"\nversion = \"1.0.0\"\n",
        );
        write(
            &root.join("demo/definitions/categories/body.toml"),
            "id = \"body\"\n[title]\nen = \"Body\"\n",
        );
        write(
            &root.join("demo/tasks/water.toml"),
            "id = \"water\"\ncategory = \"body\"\n[title]\nen = \"Water\"\n",
        );
        write(
            &root.join("demo/tasks/follow.toml"),
            "id = \"follow\"\nafter = \"water\"\n[title]\nen = \"Follow\"\n",
        );
        let set = PackSet::load(&root);
        let issues = validate_set(&set);
        assert!(!has_errors(&issues), "{issues:?}");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn missing_prerequisite_is_pk016() {
        let root = temp_root();
        write(&root.join("demo/pack.toml"), "id = \"demo\"\n");
        write(
            &root.join("demo/tasks/follow.toml"),
            "id = \"follow\"\nafter = \"missing\"\n[title]\nen = \"Follow\"\n",
        );
        let set = PackSet::load(&root);
        let issues = validate_set(&set);
        assert!(
            issues
                .iter()
                .any(|issue| issue.code == "PK016" && issue.message.contains("не найдена")),
            "{issues:?}"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn prerequisite_cycle_is_pk016() {
        let root = temp_root();
        write(&root.join("demo/pack.toml"), "id = \"demo\"\n");
        write(
            &root.join("demo/tasks/a.toml"),
            "id = \"a\"\nafter = \"b\"\n[title]\nen = \"A\"\n",
        );
        write(
            &root.join("demo/tasks/b.toml"),
            "id = \"b\"\nafter = \"a\"\n[title]\nen = \"B\"\n",
        );
        let set = PackSet::load(&root);
        let issues = validate_set(&set);
        assert!(
            issues.iter().any(|issue| issue.code == "PK016"),
            "{issues:?}"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn legacy_theme_stickers_are_warned_and_not_counted() {
        let root = temp_root();
        write(&root.join("demo/pack.toml"), "id = \"demo\"\n");
        write(
            &root.join("demo/bundles/legacy.toml"),
            "id = \"legacy\"\nprice = 10\nstickers = [\"star\"]\n",
        );
        let set = PackSet::load(&root);
        let bundle = set.packs["demo"].bundles.get("demo/legacy").unwrap();
        assert_eq!(bundle.stickers.len(), 1);
        assert_eq!(bundle.contents_count(), 0);
        let issues = validate_set(&set);
        assert!(
            issues.iter().any(|issue| issue.code == "W006"),
            "{issues:?}"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn unknown_category_is_pk005() {
        let root = temp_root();
        write(&root.join("demo/pack.toml"), "id = \"demo\"\n");
        write(
            &root.join("demo/tasks/water.toml"),
            "id = \"water\"\ncategory = \"nope\"\n[title]\nen = \"Water\"\n",
        );
        let set = PackSet::load(&root);
        let issues = validate_set(&set);
        assert!(issues.iter().any(|issue| issue.code == "PK005"));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn missing_english_is_warning_w002() {
        let root = temp_root();
        write(&root.join("demo/pack.toml"), "id = \"demo\"\n");
        write(
            &root.join("demo/tasks/water.toml"),
            "id = \"water\"\n[title]\nru = \"Вода\"\n",
        );
        let set = PackSet::load(&root);
        let issues = validate_set(&set);
        assert!(issues
            .iter()
            .any(|issue| issue.code == "W002" && issue.severity == Severity::Warning));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn negative_reward_in_official_pack_warns() {
        let root = temp_root();
        write(
            &root.join("demo/pack.toml"),
            "id = \"demo\"\nofficial = true\n",
        );
        write(
            &root.join("demo/tasks/water.toml"),
            "id = \"water\"\n[title]\nen = \"Water\"\n[reward]\nxp = -5\n",
        );
        let set = PackSet::load(&root);
        let issues = validate_set(&set);
        assert!(issues.iter().any(|issue| issue.code == "W001"));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn group_without_english_is_warning_w002() {
        let root = temp_root();
        write(&root.join("demo/pack.toml"), "id = \"demo\"\n");
        write(
            &root.join("demo/groups/evening.toml"),
            "id = \"evening\"\n[title]\nru = \"Вечер\"\n",
        );
        let set = PackSet::load(&root);
        let issues = validate_set(&set);
        assert!(issues
            .iter()
            .any(|issue| issue.code == "W002" && issue.file.contains("groups/")));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn group_unknown_locale_is_pk004() {
        let root = temp_root();
        write(&root.join("demo/pack.toml"), "id = \"demo\"\n");
        write(
            &root.join("demo/groups/evening.toml"),
            "id = \"evening\"\n[title]\nen = \"Evening\"\nxx = \"Broken\"\n",
        );
        let set = PackSet::load(&root);
        let issues = validate_set(&set);
        assert!(issues
            .iter()
            .any(|issue| issue.code == "PK004" && issue.file.contains("groups/")));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn invalid_group_weight_is_t011() {
        let root = temp_root();
        write(&root.join("demo/pack.toml"), "id = \"demo\"\n");
        write(
            &root.join("demo/groups/evening.toml"),
            "id = \"evening\"\n[title]\nen = \"Evening\"\n",
        );
        write(
            &root.join("demo/tasks/a.toml"),
            "id = \"a\"\ngroup = \"evening\"\ngroup_weight = 0\n[title]\nen = \"A\"\n",
        );
        write(
            &root.join("demo/tasks/b.toml"),
            "id = \"b\"\ngroup = \"evening\"\n[title]\nen = \"B\"\n",
        );
        let set = PackSet::load(&root);
        let issues = validate_set(&set);
        assert!(
            issues
                .iter()
                .any(|issue| issue.code == "T011" && issue.message.contains("InvalidGroupWeight")),
            "{issues:?}"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn invalid_group_level_range_is_g005() {
        let root = temp_root();
        write(&root.join("demo/pack.toml"), "id = \"demo\"\n");
        write(
            &root.join("demo/groups/evening.toml"),
            "id = \"evening\"\nlevel_from = 5\nlevel_to = 2\n[title]\nen = \"Evening\"\n",
        );
        let set = PackSet::load(&root);
        let issues = validate_set(&set);
        assert!(
            issues.iter().any(|issue| issue.code == "G005"),
            "{issues:?}"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn group_weight_without_group_is_t011() {
        let root = temp_root();
        write(&root.join("demo/pack.toml"), "id = \"demo\"\n");
        write(
            &root.join("demo/tasks/a.toml"),
            "id = \"a\"\ngroup_weight = 2\n[title]\nen = \"A\"\n",
        );
        let set = PackSet::load(&root);
        let issues = validate_set(&set);
        assert!(
            issues
                .iter()
                .any(|issue| issue.code == "T011"
                    && issue.message.contains("GroupWeightWithoutGroup")),
            "{issues:?}"
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn condition_leaves_are_listed() {
        let condition: Condition =
            toml::from_str("all_of = [{ metric = \"sys.level\", value = 5 }]").unwrap();
        let leaves = condition.leaves();
        assert_eq!(leaves.len(), 1);
        assert_eq!(leaves[0].0, "sys.level");
    }
}
