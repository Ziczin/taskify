//! Первый запуск: раскладываем папки и ставим встроенные паки.

use std::path::{Path, PathBuf};

use anyhow::{Context, Result};

use crate::engine::Engine;
use crate::settings::Settings;

/// Каталог данных: рядом с `livespaces`, если путь переопределён.
pub fn data_root(settings: &Settings) -> PathBuf {
    settings
        .paths
        .livespaces_dir
        .as_ref()
        .and_then(|dir| dir.parent().map(|parent| parent.to_path_buf()))
        .unwrap_or_else(Settings::dir)
}

/// Папка с ресурсами: сначала рядом с exe, затем текущая папка, затем исходники.
/// Нужна для портативной сборки: GitHub-артефакт кладёт `assets` рядом с exe.
pub fn assets_root() -> PathBuf {
    if let Some(path) = std::env::var_os("TASKIFY_ASSETS") {
        return PathBuf::from(path);
    }
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            candidates.push(dir.join("assets"));
            // В macOS-бандле exe лежит в Contents/MacOS, а ресурсы — в
            // Contents/Resources. Поддерживаем оба расположения.
            if let Some(contents) = dir.parent() {
                candidates.push(contents.join("Resources").join("assets"));
            }
        }
    }
    if let Ok(cwd) = std::env::current_dir() {
        candidates.push(cwd.join("assets"));
    }
    let source = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("assets");
    candidates.push(source.clone());
    candidates
        .iter()
        .find(|candidate| candidate.join("packs/greeting/pack.toml").exists())
        .cloned()
        .unwrap_or(source)
}

pub fn packs_source() -> PathBuf {
    assets_root().join("packs")
}

pub fn packs_dir(settings: &Settings) -> PathBuf {
    settings
        .paths
        .packs_dir
        .clone()
        .unwrap_or_else(|| data_root(settings).join("packs"))
}

pub fn ensure_dirs(settings: &Settings) -> Result<()> {
    std::fs::create_dir_all(packs_dir(settings)).context("не удалось создать папку паков")?;
    std::fs::create_dir_all(settings.livespaces_dir())
        .context("не удалось создать папку лайвспейсов")?;
    if livespace_path(settings).is_none() {
        std::fs::create_dir_all(settings.livespaces_dir().join(&settings.livespace))
            .context("не удалось создать профиль")?;
    }
    Ok(())
}

/// Путь к лайвспейсу, выбранному вручную, если он существует.
pub fn livespace_path(settings: &Settings) -> Option<PathBuf> {
    settings
        .personal_livespace_dir(&settings.livespace)
        .cloned()
        .or_else(|| settings.livespace_path.as_ref().map(PathBuf::from))
        .filter(|path| path.exists())
}

/// Есть ли уже хоть один лайвспейс: ручной путь или папка в данных.
pub fn has_livespace(settings: &Settings) -> bool {
    if livespace_path(settings).is_some()
        || settings
            .paths
            .livespace_dirs
            .values()
            .any(|path| path.exists())
    {
        return true;
    }
    std::fs::read_dir(settings.livespaces_dir())
        .map(|entries| entries.flatten().any(|entry| entry.path().is_dir()))
        .unwrap_or(false)
}

/// Простой слаг из имени: транслит, нижний регистр, дефисы.
pub fn slugify(name: &str) -> String {
    let mut out = String::new();
    for ch in name.chars() {
        let lower = ch.to_lowercase().next().unwrap_or(ch);
        let mapped = match lower {
            'а' => "a",
            'б' => "b",
            'в' => "v",
            'г' => "g",
            'д' => "d",
            'е' | 'ё' => "e",
            'ж' => "zh",
            'з' => "z",
            'и' => "i",
            'й' => "y",
            'к' => "k",
            'л' => "l",
            'м' => "m",
            'н' => "n",
            'о' => "o",
            'п' => "p",
            'р' => "r",
            'с' => "s",
            'т' => "t",
            'у' => "u",
            'ф' => "f",
            'х' => "h",
            'ц' => "c",
            'ч' => "ch",
            'ш' => "sh",
            'щ' => "sch",
            'ъ' | 'ь' => "",
            'ы' => "y",
            'э' => "e",
            'ю' => "yu",
            'я' => "ya",
            'a'..='z' | '0'..='9' => {
                out.push(lower);
                continue;
            }
            ' ' | '-' | '_' => {
                if !out.ends_with('-') && !out.is_empty() {
                    out.push('-');
                }
                continue;
            }
            _ => continue,
        };
        out.push_str(mapped);
    }
    let trimmed = out.trim_matches('-').to_string();
    if trimmed.is_empty() {
        "space".to_string()
    } else {
        trimmed
    }
}

/// Создаёт лайвспейс в выбранной папке и возвращает его путь.
pub fn create_livespace_at(parent: &Path, name: &str) -> Result<PathBuf> {
    let name = name.trim();
    if name.is_empty() {
        anyhow::bail!("пустое имя лайвспейса");
    }
    let base = slugify(name);
    let mut path = parent.join(&base);
    let mut index = 2;
    while path.exists() {
        path = parent.join(format!("{base}-{index}"));
        index += 1;
    }
    std::fs::create_dir_all(&path).context("не удалось создать папку лайвспейса")?;
    taskify_store::LivespaceBuilder::new(&path)
        .name(name)
        .build()
        .context("не удалось создать лайвспейс")?;
    Ok(path)
}

/// Принимает выбранную папку как лайвспейс: пустую инициализирует.
pub fn adopt_livespace(path: &Path) -> Result<PathBuf> {
    if !path.is_dir() {
        anyhow::bail!("это не папка");
    }
    if path.join("livespace.toml").exists() || path.join("data").exists() {
        taskify_store::Livespace::open(path).context("не удалось открыть лайвспейс")?;
        return Ok(path.to_path_buf());
    }
    let empty = std::fs::read_dir(path)
        .map(|mut entries| entries.next().is_none())
        .unwrap_or(false);
    if !empty {
        anyhow::bail!("в папке нет лайвспейса");
    }
    let name = path
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| "Лайвспейс".to_string());
    taskify_store::LivespaceBuilder::new(path)
        .name(&name)
        .build()
        .context("не удалось создать лайвспейс")?;
    Ok(path.to_path_buf())
}

/// Имя лайвспейса из манифеста или слаг.
pub fn livespace_name(path: &Path, slug: &str) -> String {
    std::fs::read_to_string(path.join("livespace.toml"))
        .ok()
        .and_then(|text| toml::from_str::<toml::Value>(&text).ok())
        .and_then(|value| {
            value
                .get("name")
                .and_then(|name| name.as_str())
                .map(str::to_string)
        })
        .unwrap_or_else(|| slug.to_string())
}

/// Копирует лайвспейс в пустую папку. Старая папка остаётся на месте:
/// приложение удалит её только после успешного открытия копии.
pub fn copy_livespace_to(from: &Path, to: &Path) -> Result<()> {
    let from = from
        .canonicalize()
        .with_context(|| format!("папка лайвспейса не найдена: {}", from.display()))?;
    if !from.join("livespace.toml").is_file() {
        anyhow::bail!("это не папка лайвспейса");
    }
    let to = std::path::absolute(to).unwrap_or_else(|_| to.to_path_buf());
    if to == from {
        anyhow::bail!("папка уже используется этим лайвспейсом");
    }
    if to.starts_with(&from) {
        anyhow::bail!("нельзя перенести лайвспейс внутрь его самого");
    }
    if to.exists() {
        let empty = std::fs::read_dir(&to)
            .map(|mut entries| entries.next().is_none())
            .unwrap_or(false);
        if !empty {
            anyhow::bail!("выбранная папка не пуста");
        }
    }
    std::fs::create_dir_all(&to).context("не удалось создать папку лайвспейса")?;
    if let Err(error) = copy_dir(&from, &to) {
        let _ = std::fs::remove_dir_all(&to);
        return Err(error).context("не удалось скопировать лайвспейс");
    }
    Ok(())
}

/// Все лайвспейсы: `(слаг, имя из манифеста)`, по алфавиту.
///
/// Личные папки важнее общей: если для слага задан свой путь, показываем
/// лайвспейс оттуда.
pub fn livespaces(settings: &Settings) -> Vec<(String, String)> {
    let dir = settings.livespaces_dir();
    let mut found = std::collections::BTreeMap::<String, String>::new();
    if let Ok(entries) = std::fs::read_dir(&dir) {
        for entry in entries.flatten() {
            let path = entry.path();
            if !path.is_dir() {
                continue;
            }
            let slug = entry.file_name().to_string_lossy().to_string();
            found.insert(slug.clone(), livespace_name(&path, &slug));
        }
    }
    for (slug, path) in &settings.paths.livespace_dirs {
        if path.is_dir() {
            found.insert(slug.clone(), livespace_name(path, slug));
        }
    }
    if let Some(path) = livespace_path(settings) {
        let slug = settings.livespace.clone();
        found
            .entry(slug.clone())
            .or_insert_with(|| livespace_name(&path, &slug));
    }
    let mut found: Vec<(String, String)> = found.into_iter().collect();
    found.sort_by_key(|(_, name)| name.to_lowercase());
    found
}

/// Создаёт новый пустой лайвспейс и возвращает его слаг.
pub fn create_livespace(settings: &Settings, name: &str) -> Result<String> {
    let name = name.trim();
    if name.is_empty() {
        anyhow::bail!("пустое имя лайвспейса");
    }
    let dir = settings.livespaces_dir();
    let mut index = 1;
    let slug = loop {
        let candidate = format!("space-{index}");
        if !dir.join(&candidate).exists() && !settings.paths.livespace_dirs.contains_key(&candidate)
        {
            break candidate;
        }
        index += 1;
    };
    taskify_store::LivespaceBuilder::new(dir.join(&slug))
        .name(name)
        .build()
        .with_context(|| "не удалось создать лайвспейс")?;
    Ok(slug)
}

/// Удаляет каталог лайвспейса целиком.
///
/// Проверяет, что это действительно лайвспейс (`livespace.toml`), и не даёт
/// удалить корень данных или каталог без манифеста.
pub fn delete_livespace_dir(root: &Path) -> Result<()> {
    let root = root
        .canonicalize()
        .with_context(|| format!("каталог не найден: {}", root.display()))?;
    if !root.join("livespace.toml").is_file() {
        anyhow::bail!("это не каталог лайвспейса");
    }
    if root.parent().is_none() {
        anyhow::bail!("нельзя удалить корень диска");
    }
    std::fs::remove_dir_all(&root)
        .with_context(|| format!("не удалось удалить каталог: {}", root.display()))?;
    Ok(())
}

/// Версия пака из его `pack.toml`: `(major, minor, patch)`.
fn pack_version(path: &Path) -> Option<(u32, u32, u32)> {
    let text = std::fs::read_to_string(path).ok()?;
    let line = text
        .lines()
        .find(|line| line.trim_start().starts_with("version"))?;
    let value = line.split('"').nth(1)?;
    let mut parts = value.split('.');
    let major = parts.next()?.parse().ok()?;
    let minor = parts.next()?.parse().ok()?;
    let patch = parts.next().and_then(|part| part.parse().ok()).unwrap_or(0);
    Some((major, minor, patch))
}

/// Паки, которые были встроенными раньше и больше не поставляются.
/// Их удаляем из профиля, чтобы после обновления не оставались старые карточки.
const RETIRED_BUILTIN_PACKS: &[&str] = &[
    "beauty", "focus", "gifts", "health", "home", "mind", "money", "sport", "starter", "test_all",
    "welcome",
];

/// Удаляет устаревшие официальные паки, не трогая пользовательские папки.
fn remove_retired_builtin_packs(root: &Path) {
    for id in RETIRED_BUILTIN_PACKS {
        let dir = root.join(id);
        let manifest = dir.join("pack.toml");
        let Ok(text) = std::fs::read_to_string(&manifest) else {
            continue;
        };
        if !text.lines().any(|line| line.trim() == "official = true") {
            continue;
        }
        if let Err(error) = std::fs::remove_dir_all(&dir) {
            eprintln!("не удалось удалить устаревший пак {id}: {error}");
        }
    }
}

/// Ставит встроенные паки, которых нет в профиле, и обновляет официальные,
/// если встроенная версия новее. Возвращает число добавленных и обновлённых.
pub fn install_builtin_packs(settings: &Settings) -> Result<usize> {
    let source = packs_source();
    if !source.join("greeting").join("pack.toml").exists() {
        anyhow::bail!("встроенные паки не найдены рядом с приложением");
    }
    let target_root = packs_dir(settings);
    remove_retired_builtin_packs(&target_root);
    let mut installed = 0;
    for entry in std::fs::read_dir(&source).context("не удалось прочитать встроенные паки")?
    {
        let entry = entry?;
        let path = entry.path();
        if !path.is_dir() || !path.join("pack.toml").exists() {
            continue;
        }
        let target = target_root.join(entry.file_name());
        let bundled = pack_version(&path.join("pack.toml"));
        if target.join("pack.toml").exists() {
            // Пак уже стоит: обновляем только если встроенный новее.
            let current = pack_version(&target.join("pack.toml"));
            if let (Some(bundled), Some(current)) = (bundled, current) {
                if bundled > current {
                    std::fs::remove_dir_all(&target)
                        .context("не удалось обновить пак: папка занята")?;
                    copy_dir(&path, &target)?;
                    installed += 1;
                }
            }
            continue;
        }
        copy_dir(&path, &target)?;
        installed += 1;
    }
    Ok(installed)
}

pub fn copy_dir(from: &Path, to: &Path) -> Result<()> {
    std::fs::create_dir_all(to)?;
    for entry in std::fs::read_dir(from)? {
        let entry = entry?;
        let path = entry.path();
        let target = to.join(entry.file_name());
        if path.is_dir() {
            copy_dir(&path, &target)?;
        } else {
            std::fs::copy(&path, &target)?;
        }
    }
    Ok(())
}

pub fn prepare(settings: &Settings) -> Result<Engine> {
    ensure_dirs(settings)?;
    install_builtin_packs(settings)?;
    let root = data_root(settings);
    let packs = packs_dir(settings);
    if let Some(path) = livespace_path(settings) {
        return Engine::open_at_with_packs(&path, &root, &packs);
    }
    Engine::open_at_with_packs(
        &settings.livespaces_dir().join(&settings.livespace),
        &root,
        &packs,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn livespaces_can_be_created_and_listed() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/livespace-test")
            .join(std::process::id().to_string());
        let _ = std::fs::remove_dir_all(&root);
        let mut settings = Settings::default();
        settings.paths.livespaces_dir = Some(root.join("livespaces"));

        let slug = create_livespace(&settings, "Мастерская").expect("создание");
        assert_eq!(slug, "space-1");
        let list = livespaces(&settings);
        assert!(
            list.iter()
                .any(|(found, name)| found == &slug && name == "Мастерская"),
            "список: {list:?}"
        );

        let second = create_livespace(&settings, "Отдых").expect("создание");
        assert_eq!(second, "space-2");

        // Движок открывает любой из них, а не только default.
        let data_root = settings
            .paths
            .livespaces_dir
            .clone()
            .and_then(|dir| dir.parent().map(|parent| parent.to_path_buf()))
            .expect("корень данных");
        let engine = Engine::open_livespace(&data_root, &slug).expect("открытие space-1");
        assert_eq!(engine.store.livespace.slug(), slug);
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn slugify_makes_safe_folder_names() {
        assert_eq!(slugify("Мастерская"), "masterskaya");
        assert_eq!(slugify("My Space 2!"), "my-space-2");
        assert_eq!(slugify("   "), "space");
    }

    #[test]
    fn livespace_directory_is_deleted_whole() {
        let root = PathBuf::from("target/livespace-delete-test");
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        let path = create_livespace_at(&root, "Удаляемый").unwrap();
        assert!(path.join("livespace.toml").exists());
        delete_livespace_dir(&path).unwrap();
        assert!(!path.exists());
        assert!(delete_livespace_dir(&root).is_err());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn livespace_can_be_created_and_adopted_in_chosen_folder() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/setup-test")
            .join(std::process::id().to_string());
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).expect("папка");

        let path = create_livespace_at(&root, "Мастерская").expect("создание");
        assert_eq!(path.file_name().unwrap(), "masterskaya");
        assert!(path.join("livespace.toml").exists());
        assert_eq!(
            create_livespace_at(&root, "Мастерская").unwrap(),
            root.join("masterskaya-2")
        );

        let empty = root.join("gallery");
        std::fs::create_dir_all(&empty).unwrap();
        assert_eq!(adopt_livespace(&empty).unwrap(), empty);
        assert!(empty.join("livespace.toml").exists());

        let busy = root.join("busy");
        std::fs::create_dir_all(&busy).unwrap();
        std::fs::write(busy.join("note.txt"), b"x").unwrap();
        assert!(adopt_livespace(&busy).is_err());

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn personal_livespace_path_is_listed_and_can_be_copied() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/path-test")
            .join(std::process::id().to_string());
        let _ = std::fs::remove_dir_all(&root);
        let mut settings = Settings::default();
        settings.paths.livespaces_dir = Some(root.join("livespaces"));
        let slug = create_livespace(&settings, "Мастерская").expect("создание");
        let source = settings.livespaces_dir().join(&slug);
        let destination = root.join("external").join("workshop");

        copy_livespace_to(&source, &destination).expect("копия");
        assert!(destination.join("livespace.toml").is_file());

        settings.livespace = slug.clone();
        settings.set_personal_livespace_dir(&slug, Some(destination.clone()));
        assert_eq!(
            livespace_path(&settings).as_deref(),
            Some(destination.as_path())
        );
        let list = livespaces(&settings);
        assert!(
            list.iter()
                .any(|(found, name)| found == &slug && name == "Мастерская"),
            "список: {list:?}"
        );

        let busy = root.join("busy");
        std::fs::create_dir_all(&busy).unwrap();
        std::fs::write(busy.join("note.txt"), b"x").unwrap();
        assert!(copy_livespace_to(&source, &busy).is_err());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn bundled_pack_version_is_parsed() {
        let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../assets/packs/greeting/pack.toml");
        assert_eq!(pack_version(&path), Some((1, 1, 0)));
    }

    #[test]
    fn assets_root_finds_the_greeting_pack() {
        assert!(assets_root().join("packs/greeting/pack.toml").exists());
    }

    #[test]
    fn retired_official_packs_are_removed_but_user_packs_survive() {
        let root = crate::test_support::temp_root("retired_packs");
        let old = root.join("starter");
        let custom = root.join("my_pack");
        std::fs::create_dir_all(&old).unwrap();
        std::fs::create_dir_all(&custom).unwrap();
        std::fs::write(old.join("pack.toml"), "id = \"starter\"\nofficial = true\n").unwrap();
        std::fs::write(
            custom.join("pack.toml"),
            "id = \"my_pack\"\nofficial = false\n",
        )
        .unwrap();

        remove_retired_builtin_packs(&root);

        assert!(!old.exists(), "устаревший официальный пак должен удалиться");
        assert!(custom.exists(), "пользовательский пак трогать нельзя");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn missing_version_is_none() {
        assert_eq!(pack_version(Path::new("нет-такого-файла.toml")), None);
    }
}
