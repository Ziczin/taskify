//! Журнал событий: файлы по месяцам, только добавление, fsync после каждой записи.

use std::fs::{File, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};

use chrono::{DateTime, Datelike, FixedOffset};

use crate::event::Event;

#[derive(Debug)]
pub struct ReadOutcome {
    pub events: Vec<Event>,
    /// Последний файл пришлось усечь: оборвалась недописанная строка.
    pub repaired: bool,
    pub warnings: Vec<String>,
}

pub struct LogWriter {
    dir: PathBuf,
    month: String,
    file: Option<File>,
    seq: u64,
}

impl LogWriter {
    pub fn open(dir: impl Into<PathBuf>, last_seq: u64) -> std::io::Result<Self> {
        let dir = dir.into();
        std::fs::create_dir_all(&dir)?;
        Ok(Self {
            dir,
            month: String::new(),
            file: None,
            seq: last_seq,
        })
    }

    pub fn next_seq(&self) -> u64 {
        self.seq + 1
    }

    pub fn assign_seq(&mut self) -> u64 {
        self.seq += 1;
        self.seq
    }

    fn path_for(&self, month: &str) -> PathBuf {
        self.dir.join(format!("{month}.jsonl"))
    }

    fn ensure_open(&mut self, month: &str) -> std::io::Result<()> {
        if self.month == month && self.file.is_some() {
            return Ok(());
        }
        let path = self.path_for(month);
        let file = OpenOptions::new().create(true).append(true).open(path)?;
        self.file = Some(file);
        self.month = month.to_string();
        Ok(())
    }

    /// Дописывает событие и синхронизирует файл на диск.
    pub fn append(&mut self, event: &Event) -> std::io::Result<()> {
        let month = month_key(event.ts);
        self.ensure_open(&month)?;
        let line = serde_json::to_string(event)
            .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
        let file = self
            .file
            .as_mut()
            .ok_or_else(|| std::io::Error::other("log file is not open"))?;
        file.write_all(line.as_bytes())?;
        file.write_all(b"\n")?;
        file.sync_all()
    }

    pub fn flush(&mut self) -> std::io::Result<()> {
        match self.file.as_mut() {
            Some(file) => file.sync_all(),
            None => Ok(()),
        }
    }
}

pub fn month_key(ts: DateTime<FixedOffset>) -> String {
    format!("{:04}-{:02}", ts.year(), ts.month())
}

/// Читает все события по порядку файлов. Хвост последнего файла, если он не
/// заканчивается переводом строки, отрезается — это след обрыва записи.
pub fn read_all(dir: &Path) -> std::io::Result<ReadOutcome> {
    let mut files: Vec<PathBuf> = Vec::new();
    if dir.exists() {
        for entry in std::fs::read_dir(dir)? {
            let entry = entry?;
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) == Some("jsonl") {
                files.push(path);
            }
        }
    }
    files.sort();

    let mut events = Vec::new();
    let mut warnings = Vec::new();
    let mut repaired = false;

    for (index, path) in files.iter().enumerate() {
        let is_last = index + 1 == files.len();
        let text = std::fs::read_to_string(path)?;
        let ends_with_newline = text.ends_with('\n');

        if is_last && !ends_with_newline && !text.is_empty() {
            // Обрыв записи: отрезаем недописанную строку.
            if let Some(offset) = text.rfind('\n') {
                let keep = &text[..=offset];
                std::fs::write(path, keep)?;
                warnings.push(format!(
                    "{}: оборванная запись отрезана",
                    path.file_name().unwrap_or_default().to_string_lossy()
                ));
                repaired = true;
            } else {
                std::fs::write(path, "")?;
                warnings.push(format!(
                    "{}: файл был пуст или состоял из одной битой строки",
                    path.file_name().unwrap_or_default().to_string_lossy()
                ));
                repaired = true;
            }
        }

        let reader = BufReader::new(File::open(path)?);
        for (line_no, line) in reader.lines().enumerate() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            match Event::parse_line(&line) {
                Ok(event) => events.push(event),
                Err(error) => {
                    let name = path.file_name().unwrap_or_default().to_string_lossy();
                    return Err(std::io::Error::new(
                        std::io::ErrorKind::InvalidData,
                        format!("{name}:{}: {error}", line_no + 1),
                    ));
                }
            }
        }
    }

    events.sort_by_key(|event| event.seq);
    Ok(ReadOutcome {
        events,
        repaired,
        warnings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::{EventKind, HabitSkipped};
    use chrono::TimeZone;

    fn temp_dir() -> PathBuf {
        let dir = crate::test_support::temp_root("log");
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    fn event(seq: u64, month: u32) -> Event {
        let ts = FixedOffset::east_opt(0)
            .unwrap()
            .with_ymd_and_hms(2024, month, 10, 9, 0, 0)
            .unwrap();
        Event {
            schema: 1,
            seq,
            id: format!("id-{seq}"),
            ts,
            kind: EventKind::HabitSkipped(HabitSkipped {
                task_id: "water".into(),
                local_date: ts.date_naive(),
            }),
        }
    }

    #[test]
    fn appends_and_reads_in_order() {
        let dir = temp_dir();
        let mut writer = LogWriter::open(&dir, 0).unwrap();
        for seq in 1..=4 {
            writer.append(&event(seq, 3)).unwrap();
        }
        writer.append(&event(5, 4)).unwrap();
        drop(writer);

        let outcome = read_all(&dir).unwrap();
        let seqs: Vec<u64> = outcome.events.iter().map(|event| event.seq).collect();
        assert_eq!(seqs, vec![1, 2, 3, 4, 5]);
        assert!(!outcome.repaired);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn month_rotation_creates_two_files() {
        let dir = temp_dir();
        let mut writer = LogWriter::open(&dir, 0).unwrap();
        writer.append(&event(1, 3)).unwrap();
        writer.append(&event(2, 4)).unwrap();
        drop(writer);
        let mut names: Vec<String> = std::fs::read_dir(&dir)
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().to_string())
            .collect();
        names.sort();
        assert_eq!(names, vec!["2024-03.jsonl", "2024-04.jsonl"]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn truncated_tail_is_repaired() {
        let dir = temp_dir();
        let mut writer = LogWriter::open(&dir, 0).unwrap();
        writer.append(&event(1, 3)).unwrap();
        writer.append(&event(2, 3)).unwrap();
        drop(writer);

        let path = dir.join("2024-03.jsonl");
        let text = std::fs::read_to_string(&path).unwrap();
        let mut broken = text.clone();
        broken.push_str("{\"schema\":1,\"seq\":3,\"id\":\"x\"");
        std::fs::write(&path, broken).unwrap();

        let outcome = read_all(&dir).unwrap();
        assert_eq!(outcome.events.len(), 2);
        assert!(outcome.repaired);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn deeper_corruption_is_an_error() {
        let dir = temp_dir();
        let path = dir.join("2024-03.jsonl");
        std::fs::write(&path, "{not json}\n").unwrap();
        let error = read_all(&dir).unwrap_err();
        assert!(error.to_string().contains("2024-03.jsonl:1"));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
