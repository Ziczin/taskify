//! Атомарная запись кешей.

use std::path::Path;

use serde::de::DeserializeOwned;
use serde::Serialize;

pub fn read_json<T: DeserializeOwned>(path: &Path) -> Option<T> {
    let text = std::fs::read_to_string(path).ok()?;
    serde_json::from_str(&text).ok()
}

pub fn write_json<T: Serialize>(path: &Path, value: &T) -> std::io::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let text = serde_json::to_string_pretty(value)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))?;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, text)?;
    std::fs::rename(&tmp, path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde::Deserialize;

    #[derive(Serialize, Deserialize, PartialEq, Debug)]
    struct Sample {
        a: u32,
    }

    #[test]
    fn roundtrip_and_missing_file() {
        let dir = crate::test_support::temp_root("cache");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("state.json");
        assert_eq!(read_json::<Sample>(&path), None);
        write_json(&path, &Sample { a: 5 }).unwrap();
        assert_eq!(read_json::<Sample>(&path), Some(Sample { a: 5 }));
        let _ = std::fs::remove_dir_all(&dir);
    }
}
