//! Бэкапы: четыре уровня с независимой ротацией.
//!
//! Минуты хранятся час, часы — сутки, дни — неделю, недели — три месяца.

use std::fs::File;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use chrono::{DateTime, Local};

use crate::livespace::Livespace;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Tier {
    Minutes,
    Hourly,
    Daily,
    Weekly,
}

impl Tier {
    pub fn dir_name(self) -> &'static str {
        match self {
            Tier::Minutes => "5m",
            Tier::Hourly => "hourly",
            Tier::Daily => "daily",
            Tier::Weekly => "weekly",
        }
    }

    pub fn interval(self) -> Duration {
        match self {
            Tier::Minutes => Duration::from_secs(5 * 60),
            Tier::Hourly => Duration::from_secs(60 * 60),
            Tier::Daily => Duration::from_secs(24 * 60 * 60),
            Tier::Weekly => Duration::from_secs(7 * 24 * 60 * 60),
        }
    }

    pub fn keep(self, settings: &crate::livespace::BackupSettings) -> usize {
        match self {
            Tier::Minutes => settings.keep_minutes as usize,
            Tier::Hourly => settings.keep_hours as usize,
            Tier::Daily => settings.keep_days as usize,
            Tier::Weekly => settings.keep_weeks as usize,
        }
    }
}

pub fn collect_sources(livespace: &Livespace) -> Vec<(PathBuf, String)> {
    let mut files = Vec::new();
    let mut push = |path: PathBuf, name: String| {
        if path.exists() {
            files.push((path, name));
        }
    };
    push(
        livespace.root().join("livespace.toml"),
        "livespace.toml".into(),
    );
    push(livespace.settings_file(), "settings.toml".into());
    push(livespace.state_file(), "data/state.json".into());
    push(livespace.aggregates_file(), "data/aggregates.json".into());
    push(livespace.appearance_file(), "appearance.toml".into());
    if let Ok(entries) = std::fs::read_dir(livespace.log_dir()) {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("jsonl") {
                let name = format!(
                    "data/log/{}",
                    path.file_name().unwrap_or_default().to_string_lossy()
                );
                push(path, name);
            }
        }
    }
    files
}

pub fn create(
    livespace: &Livespace,
    tier: Tier,
    now: DateTime<Local>,
    seq: u64,
    keep: usize,
) -> std::io::Result<PathBuf> {
    let dir = livespace.backups_dir().join(tier.dir_name());
    std::fs::create_dir_all(&dir)?;
    let name = format!("{}_{:010}.zip", now.format("%Y-%m-%d_%H%M%S"), seq);
    let path = dir.join(name);

    let file = File::create(&path)?;
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    for (source, name) in collect_sources(livespace) {
        let bytes = std::fs::read(&source)?;
        zip.start_file(name, options)
            .map_err(|error| std::io::Error::other(error.to_string()))?;
        zip.write_all(&bytes)?;
    }
    zip.finish()
        .map_err(|error| std::io::Error::other(error.to_string()))?;

    rotate(&dir, keep.max(1))?;
    Ok(path)
}

pub fn rotate(dir: &Path, keep: usize) -> std::io::Result<()> {
    let mut entries: Vec<(SystemTime, PathBuf)> = Vec::new();
    if let Ok(read) = std::fs::read_dir(dir) {
        for entry in read.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("zip") {
                let modified = entry
                    .metadata()
                    .and_then(|meta| meta.modified())
                    .unwrap_or(SystemTime::UNIX_EPOCH);
                entries.push((modified, path));
            }
        }
    }
    entries.sort_by_key(|entry| std::cmp::Reverse(entry.0));
    for (_, path) in entries.into_iter().skip(keep) {
        let _ = std::fs::remove_file(path);
    }
    Ok(())
}

/// Пора ли делать бэкап этого уровня: смотрим на самый свежий файл.
pub fn due(livespace: &Livespace, tier: Tier, now: DateTime<Local>, interval: Duration) -> bool {
    let dir = livespace.backups_dir().join(tier.dir_name());
    let newest = std::fs::read_dir(&dir).ok().and_then(|read| {
        read.flatten()
            .filter_map(|entry| entry.metadata().ok()?.modified().ok())
            .max()
    });
    match newest {
        None => true,
        Some(modified) => {
            let modified: DateTime<Local> = modified.into();
            (now - modified).num_seconds() >= interval.as_secs() as i64
        }
    }
}

/// Самый свежий бэкап уровня (для восстановления).
pub fn newest(livespace: &Livespace, tier: Tier) -> Option<PathBuf> {
    let dir = livespace.backups_dir().join(tier.dir_name());
    let mut entries: Vec<(SystemTime, PathBuf)> = std::fs::read_dir(dir)
        .ok()?
        .flatten()
        .filter_map(|entry| {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("zip") {
                Some((entry.metadata().ok()?.modified().ok()?, path))
            } else {
                None
            }
        })
        .collect();
    entries.sort_by_key(|entry| std::cmp::Reverse(entry.0));
    entries.pop().map(|(_, path)| path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::livespace::LivespaceBuilder;

    #[test]
    fn creates_backup_and_rotates() {
        let dir = crate::test_support::temp_root("bk");
        let livespace = LivespaceBuilder::new(&dir).build().unwrap();
        std::fs::create_dir_all(livespace.log_dir()).unwrap();
        crate::cache::write_json(&livespace.state_file(), &crate::state::State::default()).unwrap();

        let now = Local::now();
        for i in 0..4 {
            create(
                &livespace,
                Tier::Minutes,
                now + chrono::Duration::seconds(i),
                i as u64,
                2,
            )
            .unwrap();
        }
        let count = std::fs::read_dir(livespace.backups_dir().join("5m"))
            .unwrap()
            .count();
        assert_eq!(count, 2);
        assert!(newest(&livespace, Tier::Minutes).is_some());
        let _ = std::fs::remove_dir_all(&dir);
    }
}
