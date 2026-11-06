//! Установка пака из zip: защита от path traversal, лимиты, атомарная замена.

use std::io::Read;
use std::path::{Path, PathBuf};

use crate::entities::Pack;
use crate::loader::{Issue, PackSet, Severity};
use crate::validate::{DEFAULT_MAX_BYTES, DEFAULT_MAX_FILES, DEFAULT_MAX_RATIO};

#[derive(Debug, Clone)]
pub struct Limits {
    pub max_bytes: u64,
    pub max_files: usize,
    pub max_ratio: f64,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_bytes: DEFAULT_MAX_BYTES,
            max_files: DEFAULT_MAX_FILES,
            max_ratio: DEFAULT_MAX_RATIO,
        }
    }
}

#[derive(Debug, Clone)]
pub struct InstallReport {
    pub pack_id: String,
    pub version: String,
    pub files: usize,
    pub bytes: u64,
    pub path: PathBuf,
    pub warnings: Vec<String>,
}

pub fn install_zip(
    archive: impl AsRef<Path>,
    packs_dir: impl AsRef<Path>,
    limits: &Limits,
) -> anyhow::Result<InstallReport> {
    let archive = archive.as_ref();
    let packs_dir = packs_dir.as_ref();
    std::fs::create_dir_all(packs_dir)?;

    let file = std::fs::File::open(archive)?;
    let mut zip = zip::ZipArchive::new(file)?;

    if zip.len() > limits.max_files {
        anyhow::bail!("в архиве {} файлов, лимит {}", zip.len(), limits.max_files);
    }

    let mut total: u64 = 0;
    let mut warnings = Vec::new();
    for index in 0..zip.len() {
        let entry = zip.by_index_raw(index)?;
        let name = entry.name().to_string();
        if !is_safe_path(&name) {
            anyhow::bail!("недопустимый путь в архиве: {name}");
        }
        if entry.is_dir() {
            continue;
        }
        total = total.saturating_add(entry.size());
        if total > limits.max_bytes {
            anyhow::bail!("распакованный размер превышает {} байт", limits.max_bytes);
        }
        if entry.compressed_size() > 0 {
            let ratio = entry.size() as f64 / entry.compressed_size() as f64;
            if ratio > limits.max_ratio {
                anyhow::bail!("подозрительная степень сжатия у {name}: {ratio:.0}:1");
            }
        }
    }

    let temp = packs_dir.join(format!(".tmp-{}", uuid::Uuid::new_v4()));
    if temp.exists() {
        std::fs::remove_dir_all(&temp)?;
    }
    std::fs::create_dir_all(&temp)?;

    let result = extract(&mut zip, &temp, limits);
    let report = match result {
        Ok(mut report) => {
            let pack_id = read_pack_id(&temp).unwrap_or_else(|| {
                warnings.push("в паке нет pack.toml: имя взято из файла архива".to_string());
                archive
                    .file_stem()
                    .map(|stem| stem.to_string_lossy().to_string())
                    .unwrap_or_else(|| "pack".to_string())
            });
            let target = packs_dir.join(&pack_id);
            if target.exists() {
                std::fs::remove_dir_all(&target)?;
            }
            std::fs::rename(&temp, &target)?;
            report.pack_id = pack_id;
            report.path = target;
            report.warnings = warnings;
            report
        }
        Err(error) => {
            let _ = std::fs::remove_dir_all(&temp);
            return Err(error);
        }
    };

    Ok(report)
}

fn extract(
    zip: &mut zip::ZipArchive<std::fs::File>,
    target: &Path,
    limits: &Limits,
) -> anyhow::Result<InstallReport> {
    let mut files = 0usize;
    let mut bytes = 0u64;
    for index in 0..zip.len() {
        let mut entry = zip.by_index(index)?;
        let name = entry.name().to_string();
        if entry.is_dir() {
            std::fs::create_dir_all(target.join(&name))?;
            continue;
        }
        let path = target.join(&name);
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut buffer = Vec::new();
        entry.read_to_end(&mut buffer)?;
        bytes = bytes.saturating_add(buffer.len() as u64);
        if bytes > limits.max_bytes {
            anyhow::bail!("распакованный размер превышает {}", limits.max_bytes);
        }
        std::fs::write(&path, &buffer)?;
        files += 1;
    }
    if files > limits.max_files {
        anyhow::bail!("файлов больше лимита {}", limits.max_files);
    }
    Ok(InstallReport {
        pack_id: String::new(),
        version: String::new(),
        files,
        bytes,
        path: target.to_path_buf(),
        warnings: Vec::new(),
    })
}

fn read_pack_id(dir: &Path) -> Option<String> {
    let text = std::fs::read_to_string(dir.join("pack.toml")).ok()?;
    let manifest: crate::manifest::Manifest = toml::from_str(&text).ok()?;
    Some(manifest.id)
}

pub fn is_safe_path(name: &str) -> bool {
    if name.is_empty() || name.starts_with('/') || name.starts_with('\\') {
        return false;
    }
    if name.len() > 2 && name.as_bytes()[1] == b':' {
        return false;
    }
    if name.contains('\0') {
        return false;
    }
    !name.split(['/', '\\']).any(|part| part == "..")
}

/// Установка папки-пака (для разработки и локальных паков).
pub fn install_dir(
    source: impl AsRef<Path>,
    packs_dir: impl AsRef<Path>,
) -> anyhow::Result<InstallReport> {
    let source = source.as_ref();
    let packs_dir = packs_dir.as_ref();
    let id = read_pack_id(source).ok_or_else(|| anyhow::anyhow!("в папке нет pack.toml"))?;
    let target = packs_dir.join(&id);
    if target.exists() {
        std::fs::remove_dir_all(&target)?;
    }
    std::fs::create_dir_all(packs_dir)?;
    copy_dir(source, &target)?;
    let report = InstallReport {
        pack_id: id,
        version: String::new(),
        files: count_files(&target),
        bytes: count_bytes(&target),
        path: target,
        warnings: Vec::new(),
    };
    Ok(report)
}

fn copy_dir(from: &Path, to: &Path) -> std::io::Result<()> {
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

fn count_files(dir: &Path) -> usize {
    walkdir::WalkDir::new(dir)
        .into_iter()
        .flatten()
        .filter(|entry| entry.file_type().is_file())
        .count()
}

fn count_bytes(dir: &Path) -> u64 {
    walkdir::WalkDir::new(dir)
        .into_iter()
        .flatten()
        .filter(|entry| entry.file_type().is_file())
        .map(|entry| entry.metadata().map(|meta| meta.len()).unwrap_or(0))
        .sum()
}

/// Удобная проверка установленного пака без запуска приложения.
pub fn inspect(path: impl AsRef<Path>) -> Vec<Issue> {
    let path = path.as_ref();
    let set = PackSet::load(path);
    let mut issues = set.issues.clone();
    for pack in set.packs.values() {
        issues.extend(crate::validate::validate_pack(pack));
    }
    issues.retain(|issue| issue.severity == Severity::Error);
    issues
}

pub fn pack_of(path: impl AsRef<Path>) -> Option<Pack> {
    let set = PackSet::load(path);
    set.packs.into_values().next()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn temp() -> PathBuf {
        crate::test_support::temp_root("inst")
    }

    fn make_zip(path: &Path, entries: &[(&str, &str)]) {
        let file = std::fs::File::create(path).unwrap();
        let mut zip = zip::ZipWriter::new(file);
        let options = zip::write::SimpleFileOptions::default();
        for (name, content) in entries {
            zip.start_file(*name, options).unwrap();
            zip.write_all(content.as_bytes()).unwrap();
        }
        zip.finish().unwrap();
    }

    #[test]
    fn installs_valid_pack() {
        let root = temp();
        std::fs::create_dir_all(&root).unwrap();
        let archive = root.join("starter.zip");
        make_zip(
            &archive,
            &[
                ("pack.toml", "id = \"starter\"\nversion = \"1.0.0\"\n"),
                (
                    "tasks/water.toml",
                    "id = \"water\"\n[title]\nen = \"Water\"\n",
                ),
            ],
        );
        let report = install_zip(&archive, root.join("packs"), &Limits::default()).unwrap();
        assert_eq!(report.pack_id, "starter");
        assert!(root.join("packs/starter/pack.toml").exists());
        assert!(root.join("packs/starter/tasks/water.toml").exists());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn rejects_traversal() {
        let root = temp();
        std::fs::create_dir_all(&root).unwrap();
        let archive = root.join("evil.zip");
        make_zip(&archive, &[("../evil.toml", "id = \"evil\"\n")]);
        let error = install_zip(&archive, root.join("packs"), &Limits::default()).unwrap_err();
        assert!(error.to_string().contains("недопустимый путь"));
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn safe_path_rules() {
        assert!(is_safe_path("tasks/a.toml"));
        assert!(!is_safe_path("../a.toml"));
        assert!(!is_safe_path("C:/a.toml"));
        assert!(!is_safe_path("/etc/passwd"));
    }
}
