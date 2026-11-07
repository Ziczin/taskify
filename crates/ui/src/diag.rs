//! Диагностика запуска: куда писать, если профиль не открылся.
//!
//! Ошибки вида «Отказано в доступе» зависят от машины, поэтому при сбое мы
//! оставляем рядом след: пути, переменные окружения, проверку записи и вывод
//! `icacls`. Без этого причину видно только по диалогу.

use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::OnceLock;

const LOG_NAME: &str = "taskify-startup.log";

static DESTINATION: OnceLock<Option<PathBuf>> = OnceLock::new();

/// Пишет шапку: номер сборки, пути, пользователь, переменные окружения.
pub fn init() {
    write(&format!(
        "=== {} taskify {} pid={} exe={} cwd={} user={}",
        chrono::Local::now().to_rfc3339(),
        env!("CARGO_PKG_VERSION_MAJOR"),
        std::process::id(),
        std::env::current_exe()
            .map(|path| path.display().to_string())
            .unwrap_or_default(),
        std::env::current_dir()
            .map(|path| path.display().to_string())
            .unwrap_or_default(),
        std::env::var("USERNAME").unwrap_or_default(),
    ));
    for key in [
        "APPDATA",
        "LOCALAPPDATA",
        "TEMP",
        "TMP",
        "USERPROFILE",
        "HOME",
        "XDG_DATA_HOME",
        "TASKIFY_HOME",
    ] {
        if let Ok(value) = std::env::var(key) {
            write(&format!("env {key}={value}"));
        }
    }
}

fn candidates() -> Vec<PathBuf> {
    let mut paths = Vec::new();
    paths.push(std::env::temp_dir().join(LOG_NAME));
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            paths.push(dir.join(LOG_NAME));
        }
    }
    paths.push(crate::settings::Settings::dir().join(LOG_NAME));
    if let Some(home) = std::env::var_os("HOME") {
        paths.push(PathBuf::from(home).join(LOG_NAME));
    }
    paths
}

fn destination() -> &'static Option<PathBuf> {
    DESTINATION.get_or_init(|| {
        candidates().into_iter().find(|candidate| {
            std::fs::OpenOptions::new()
                .create(true)
                .append(true)
                .open(candidate)
                .is_ok()
        })
    })
}

/// Путь к файлу журнала, если его удалось создать.
pub fn log_path() -> Option<&'static Path> {
    destination().as_deref()
}

/// Открывает журнал запуска в системном блокноте.
///
/// Путь уже известен `init()`; если журнал не создался, возвращаем понятную
/// ошибку, а не молча ничего не делаем.
pub fn open_log() -> Result<PathBuf, String> {
    let path = log_path()
        .ok_or_else(|| "журнал запуска не создан".to_string())?
        .to_path_buf();
    let opener = if cfg!(windows) {
        "notepad.exe"
    } else if cfg!(target_os = "macos") {
        "open"
    } else {
        "xdg-open"
    };
    std::process::Command::new(opener)
        .arg(&path)
        .spawn()
        .map_err(|error| format!("не удалось открыть журнал: {error}"))?;
    Ok(path)
}

pub fn write(line: &str) {
    if let Some(path) = destination() {
        if let Ok(mut file) = std::fs::OpenOptions::new().append(true).open(path) {
            let _ = writeln!(file, "{line}");
        }
    }
}

/// Проверяет, что в каталог действительно можно писать, и объясняет отказ.
pub fn probe(dir: &Path) -> Result<(), String> {
    std::fs::create_dir_all(dir).map_err(|error| describe("создать каталог", error))?;
    let file = dir.join(format!(".taskify-probe-{}", std::process::id()));
    std::fs::write(&file, b"probe").map_err(|error| describe("записать файл", error))?;
    let _ = std::fs::remove_file(&file);
    Ok(())
}

/// Выгружает права на каталог в журнал: там видно и запрещающие записи.
#[cfg(windows)]
pub fn dump_acl(dir: &Path) {
    let output = std::process::Command::new("icacls").arg(dir).output();
    match output {
        Ok(output) => {
            let text = String::from_utf8_lossy(&output.stdout);
            write(&format!("icacls {}\n{text}", dir.display()));
        }
        Err(error) => write(&format!("icacls {} не запустился: {error}", dir.display())),
    }
}

#[cfg(not(windows))]
pub fn dump_acl(dir: &Path) {
    // На Unix права проверяет `probe`; отдельная утилита не нужна.
    let _ = dir;
}

fn describe(step: &str, error: std::io::Error) -> String {
    match error.raw_os_error() {
        Some(code) => format!("{step}: {error} (код {code})"),
        None => format!("{step}: {error}"),
    }
}

/// Разбирает профиль по шагам: какой каталог и какой файл не открываются.
///
/// Запускается только после отказа, поэтому стоит дорого — зато в журнале
/// остаётся точная операция, а не общее «отказано в доступе».
pub fn investigate(root: &Path) {
    write(&format!("--- разбор профиля {}", root.display()));
    let live = root.join("livespaces").join("default");
    let data = live.join("data");
    let log = data.join("log");

    for dir in [
        root.to_path_buf(),
        root.join("packs"),
        root.join("livespaces"),
        live.clone(),
        data.clone(),
        log.clone(),
        data.join("backups"),
    ] {
        match std::fs::metadata(&dir) {
            Ok(meta) => write(&format!(
                "{}: есть, каталог={}",
                dir.display(),
                meta.is_dir()
            )),
            Err(error) => write(&format!("{}: нет ({error})", dir.display())),
        }
        if let Err(error) = probe(&dir) {
            write(&format!("{}: запись не проходит — {error}", dir.display()));
        }
    }

    for file in [
        root.join("settings.toml"),
        live.join("livespace.toml"),
        live.join("settings.toml"),
        data.join("state.json"),
        data.join("aggregates.json"),
        log.join(format!("{}.jsonl", chrono::Local::now().format("%Y-%m"))),
    ] {
        match std::fs::OpenOptions::new().read(true).open(&file) {
            Ok(_) => {}
            Err(error) => write(&format!("{}: чтение не проходит — {error}", file.display())),
        }
        let temp = file.with_extension("probe");
        match std::fs::write(&temp, b"probe") {
            Ok(()) => {
                let _ = std::fs::remove_file(&temp);
            }
            Err(error) => write(&format!("{}: запись не проходит — {error}", file.display())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_reports_writable_directory() {
        let root = crate::test_support::temp_root("diag-probe");
        assert!(probe(&root).is_ok());
    }

    #[test]
    fn probe_explains_path_that_is_a_file() {
        let root = crate::test_support::temp_root("diag-file");
        std::fs::create_dir_all(&root).unwrap();
        let file = root.join("blocker");
        std::fs::write(&file, b"x").unwrap();
        let error = probe(&file).unwrap_err();
        assert!(error.contains("создать каталог"), "{error}");
    }
}
