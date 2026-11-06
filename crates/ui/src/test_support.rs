//! Общие помощники для тестов крейта: временные каталоги внутри `target`.

use std::path::PathBuf;

/// Временный каталог под `target/tmp`.
///
/// `std::env::temp_dir()` не всегда доступен на запись (песочница, политики
/// ОС), поэтому тесты создают файлы рядом с артефактами сборки.
pub fn temp_root(prefix: &str) -> PathBuf {
    let base = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("..")
        .join("..")
        .join("target")
        .join("tmp");
    let _ = std::fs::create_dir_all(&base);
    let dir = base.join(format!("{prefix}-{}", uuid::Uuid::new_v4()));
    let _ = std::fs::create_dir_all(&dir);
    dir
}
