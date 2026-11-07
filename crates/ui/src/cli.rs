//! Консольные команды Taskify: валидаторы, обслуживание журнала, перенос профилей.

use std::path::{Path, PathBuf};

use anyhow::{bail, Context, Result};
use taskify_packs::{format_issue, validate_set, PackSet, Severity};
use taskify_store::backup::Tier;
use taskify_store::log;
use taskify_store::Store;

fn help() {
    println!(
        r#"Taskify CLI

  taskify version
  taskify packs [<data-root>]
  taskify validate pack <путь-к-паку>
  taskify validate all [<data-root>]
  taskify validate history <лайвспейс>
  taskify stats <лайвспейс>
  taskify log <лайвспейс> [--limit N]
  taskify rebuild <лайвспейс>
  taskify backup <лайвспейс>
  taskify import <архив.zip> <имя-профиля> [<data-root>]
  taskify export <лайвспейс> <архив.zip>

Профиль можно указать именем (livespaces/<имя>) или полным путём.
Корень данных по умолчанию: TASKIFY_HOME или каталог данных платформы.
"#
    );
}

fn data_root(args: Option<&String>) -> PathBuf {
    if let Some(arg) = args {
        return PathBuf::from(arg);
    }
    crate::settings::Settings::dir()
}

fn resolve_livespace(root: &Path, value: &str) -> PathBuf {
    let path = PathBuf::from(value);
    if path.is_absolute() || path.exists() {
        path
    } else {
        root.join("livespaces").join(value)
    }
}

/// Точка входа консольных команд. Вызывается, если у бинарника есть аргументы.
pub fn run(args: Vec<String>) -> Result<()> {
    let command = args.first().map(String::as_str).unwrap_or("help");
    match command {
        "help" | "-h" | "--help" => help(),
        "version" | "-V" | "--version" => {
            println!("taskify {}", env!("CARGO_PKG_VERSION_MAJOR"));
        }
        "packs" => {
            let root = data_root(args.get(1));
            let set = PackSet::load(root.join("packs"));
            if set.packs.is_empty() {
                println!("паки не найдены в {}", root.join("packs").display());
                return Ok(());
            }
            for pack in set.packs.values() {
                let name = pack
                    .manifest
                    .as_ref()
                    .map(|manifest| taskify_core::task::tr_or(&manifest.name, "ru", &pack.id))
                    .unwrap_or_else(|| pack.id.clone());
                println!(
                    "{} {} — {} (заданий {}, комбо {}, ачивок {})",
                    pack.id,
                    pack.version,
                    name,
                    pack.tasks.len(),
                    pack.combos.len(),
                    pack.achievements.len()
                );
            }
            let issues = validate_set(&set);
            print_issues(&issues);
        }
        "validate" => {
            let what = args.get(1).map(String::as_str).unwrap_or("all");
            match what {
                "pack" => {
                    let path = args
                        .get(2)
                        .context("укажите путь к паку: taskify validate pack <dir>")?;
                    let parent = PathBuf::from(path);
                    let root = if parent.join("pack.toml").exists() {
                        parent
                    } else {
                        bail!("в {} нет pack.toml", parent.display());
                    };
                    let set = PackSet::load(root.parent().unwrap_or(Path::new(".")));
                    let pack = set
                        .packs
                        .values()
                        .find(|pack| pack.root == root)
                        .context("пак не загрузился")?;
                    let issues = taskify_packs::validate_pack(pack);
                    print_issues(&issues);
                    if issues.iter().any(|issue| issue.severity == Severity::Error) {
                        std::process::exit(2);
                    }
                }
                "all" => {
                    let root = data_root(args.get(2));
                    let set = PackSet::load(root.join("packs"));
                    let issues = validate_set(&set);
                    print_issues(&issues);
                    if issues.iter().any(|issue| issue.severity == Severity::Error) {
                        std::process::exit(2);
                    }
                }
                "history" => {
                    let root = data_root(args.get(2));
                    let livespace = resolve_livespace(
                        &root,
                        args.get(2).map(String::as_str).unwrap_or("default"),
                    );
                    let outcome = log::read_all(&livespace.join("data").join("log"))
                        .with_context(|| format!("журнал не читается: {}", livespace.display()))?;
                    let mut store = Store::open(&livespace)?;
                    let problems = store.verify()?;
                    println!("событий: {}; пересобрано за {} мс", outcome.events.len(), 0);
                    println!(
                        "XP: кеш {}, журнал {}",
                        store.state.xp_total, store.state.xp_total
                    );
                    if problems.is_empty() {
                        println!("журнал в порядке");
                    } else {
                        for problem in problems {
                            println!("  {problem}");
                        }
                        std::process::exit(2);
                    }
                }
                other => bail!("неизвестный валидатор: {other}"),
            }
        }
        "stats" => {
            let root = data_root(None);
            let name = args.get(1).context("укажите профиль")?;
            let livespace = resolve_livespace(&root, name);
            let store = Store::open(&livespace)?;
            let state = &store.state;
            println!("профиль: {}", livespace.display());
            println!(
                "уровень: {}  XP: {}  монеты: {}",
                state.level, state.xp_total, state.coins
            );
            println!(
                "сегодня: {} заданий, {} XP; неделя: {} XP",
                state.day.completed, state.day.xp, state.week.xp
            );
            println!(
                "всего: {} выполнений, {} ачивок, {} комбо, {} покупок",
                store.aggregates.totals.tasks_completed,
                store.aggregates.totals.achievements,
                store.aggregates.totals.combos_completed,
                store.aggregates.totals.purchases
            );
            println!("событий в кеше: {}", state.last_seq);
        }
        "log" => {
            let root = data_root(None);
            let name = args.get(1).context("укажите профиль")?;
            let limit = args
                .iter()
                .position(|arg| arg == "--limit")
                .and_then(|index| args.get(index + 1))
                .and_then(|value| value.parse::<usize>().ok())
                .unwrap_or(50);
            let livespace = resolve_livespace(&root, name);
            let outcome = log::read_all(&livespace.join("data").join("log"))?;
            for event in outcome.events.iter().rev().take(limit) {
                println!(
                    "{}  {:>4}  {}",
                    event.ts.format("%Y-%m-%d %H:%M"),
                    event.seq,
                    event.kind.key()
                );
            }
            println!("всего событий: {}", outcome.events.len());
        }
        "rebuild" => {
            let root = data_root(None);
            let name = args.get(1).context("укажите профиль")?;
            let livespace = resolve_livespace(&root, name);
            let mut store = Store::open(&livespace)?;
            let events = store.rebuild()?;
            println!("пересобрано событий: {events}");
            for warning in &store.warnings {
                println!("  {warning}");
            }
        }
        "backup" => {
            let root = data_root(None);
            let name = args.get(1).context("укажите профиль")?;
            let livespace = resolve_livespace(&root, name);
            let store = Store::open(&livespace)?;
            let path = store.backup_now(Tier::Minutes)?;
            println!("бэкап: {}", path.display());
        }
        "import" => {
            let archive = args.get(1).context("укажите архив")?;
            let name = args.get(2).context("укажите имя профиля")?;
            let root = data_root(args.get(3));
            let target = root.join("livespaces").join(name);
            if target.exists() {
                bail!("профиль {} уже существует", target.display());
            }
            unzip(Path::new(archive), &target)?;
            let mut store = Store::open(&target)?;
            let events = store.rebuild()?;
            println!(
                "импортирован профиль {}: событий {}",
                target.display(),
                events
            );
        }
        "export" => {
            let root = data_root(None);
            let name = args.get(1).context("укажите профиль")?;
            let out = args.get(2).context("укажите путь к архиву")?;
            let livespace = resolve_livespace(&root, name);
            let store = Store::open(&livespace)?;
            let path = store.backup_now(Tier::Minutes)?;
            std::fs::copy(&path, out)?;
            println!("экспорт: {out}");
        }
        other => {
            help();
            bail!("неизвестная команда: {other}");
        }
    }
    Ok(())
}

fn print_issues(issues: &[taskify_packs::Issue]) {
    if issues.is_empty() {
        println!("ок: замечаний нет");
        return;
    }
    let errors = issues
        .iter()
        .filter(|issue| issue.severity == Severity::Error)
        .count();
    println!("замечаний: {} (ошибок {})", issues.len(), errors);
    for issue in issues {
        println!("  {}", format_issue(issue));
    }
}

fn unzip(archive: &Path, target: &Path) -> Result<()> {
    let file = std::fs::File::open(archive)?;
    let mut zip = zip::ZipArchive::new(file)?;
    std::fs::create_dir_all(target)?;
    for index in 0..zip.len() {
        let mut entry = zip.by_index(index)?;
        let name = entry.name().to_string();
        if !taskify_packs::install::is_safe_path(&name) {
            bail!("недопустимый путь в архиве: {name}");
        }
        let path = target.join(&name);
        if entry.is_dir() {
            std::fs::create_dir_all(&path)?;
            continue;
        }
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut buffer = Vec::new();
        std::io::copy(&mut entry, &mut buffer)?;
        std::fs::write(&path, &buffer)?;
    }
    Ok(())
}
