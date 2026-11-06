//! Каталог стикеров: покупаются за монеты и вешаются на боковые панели.

use std::path::{Path, PathBuf};
use std::sync::OnceLock;

use taskify_core::task::{tr, Locales};
use taskify_packs::PackSet;

const SOURCE: &str = include_str!("../../../assets/stickers.toml");

#[derive(Debug, serde::Deserialize)]
struct File {
    #[serde(default)]
    sticker: Vec<Entry>,
}

#[derive(Debug, serde::Deserialize)]
struct Entry {
    id: String,
    icon: String,
    price: i64,
    #[serde(default = "default_pack")]
    pack: String,
    #[serde(default)]
    name: Locales,
}

fn default_pack() -> String {
    "starter".to_string()
}

/// Один стикер из каталога.
#[derive(Debug, Clone)]
pub struct Sticker {
    pub id: String,
    /// Ключ картинки в `assets/icons`.
    pub icon: String,
    /// PNG самого пака: рисуется без круга, с прозрачным фоном.
    pub image: Option<PathBuf>,
    pub price: i64,
    /// Размер относительно обычного слота стикеров.
    pub scale: f32,
    /// Пак, из которого стикер: нужен фильтру маркета.
    pub pack: String,
    pub name: Locales,
}

impl Sticker {
    pub fn title(&self, locale: &str) -> String {
        tr(&self.name, locale).unwrap_or_else(|| self.id.clone())
    }

    pub fn icon_key(&self) -> &str {
        &self.icon
    }

    pub fn image_path(&self) -> Option<&Path> {
        self.image.as_deref()
    }

    pub fn display_scale(&self) -> f32 {
        if self.scale.is_finite() {
            self.scale.clamp(0.25, 1.0)
        } else {
            1.0
        }
    }
}

pub fn all() -> &'static [Sticker] {
    static CACHE: OnceLock<Vec<Sticker>> = OnceLock::new();
    CACHE.get_or_init(|| {
        let file: File = toml::from_str(SOURCE).expect("assets/stickers.toml не читается");
        file.sticker
            .into_iter()
            .map(|entry| Sticker {
                icon: entry.icon,
                image: None,
                id: entry.id,
                price: entry.price,
                scale: 1.0,
                pack: entry.pack,
                name: entry.name,
            })
            .collect()
    })
}

/// Каталог для маркета: встроенные стикеры и стикеры включённых паков.
pub fn catalog(packs: &PackSet) -> Vec<Sticker> {
    catalog_with(packs, false)
}

/// Каталог для склада и боковых панелей: стикеры всех установленных паков.
///
/// Выключение пака не должно отбирать уже купленный стикер.
pub fn catalog_any(packs: &PackSet) -> Vec<Sticker> {
    catalog_with(packs, true)
}

fn catalog_with(packs: &PackSet, include_disabled: bool) -> Vec<Sticker> {
    let mut result = all().to_vec();
    for pack in packs.packs.values() {
        if !include_disabled && !pack.enabled {
            continue;
        }
        for def in pack.stickers.values() {
            result.push(Sticker {
                id: def.id.clone(),
                icon: def.icon.clone(),
                image: def.image_path(&pack.root),
                price: def.price,
                scale: def.display_scale(),
                pack: pack.id.clone(),
                name: def.name.clone(),
            });
        }
    }
    result
}

/// Стикер из встроенного каталога или из любого установленного пака.
pub fn find(packs: &PackSet, id: &str) -> Option<Sticker> {
    if let Some(sticker) = all().iter().find(|sticker| sticker.id == id) {
        return Some(sticker.clone());
    }
    for pack in packs.packs.values() {
        if let Some(def) = pack.stickers.get(id) {
            return Some(Sticker {
                id: def.id.clone(),
                icon: def.icon.clone(),
                image: def.image_path(&pack.root),
                price: def.price,
                scale: def.display_scale(),
                pack: pack.id.clone(),
                name: def.name.clone(),
            });
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reward_stickers_belong_to_gifts() {
        for id in ["moon", "heart", "spark", "crown", "gift", "comet"] {
            let sticker =
                find(&PackSet::default(), id).unwrap_or_else(|| panic!("нет стикера {id}"));
            assert_eq!(sticker.pack, "gifts", "стикер {id}");
            assert_eq!(sticker.price, 0, "наградной стикер {id} не продаётся");
        }
        let star = find(&PackSet::default(), "star").expect("звезда");
        assert_eq!(star.pack, "starter");
    }

    #[test]
    fn catalog_is_sane() {
        let stickers = all();
        assert!(stickers.len() >= 8, "мало стикеров");
        let mut ids: Vec<&str> = stickers.iter().map(|sticker| sticker.id.as_str()).collect();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), stickers.len(), "id стикеров повторяются");
        for sticker in stickers {
            assert!(sticker.price >= 0, "у {} отрицательная цена", sticker.id);
            assert!(
                sticker.name.contains_key("ru") && sticker.name.contains_key("en"),
                "у {} нет названия",
                sticker.id
            );
            assert!(!sticker.pack.is_empty(), "у {} не указан пак", sticker.id);
        }
    }

    #[test]
    fn pack_stickers_join_catalog() {
        let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../assets/packs");
        let set = taskify_packs::PackSet::load(&root);
        let sticker = find(&set, "test_all/test_spark").expect("стикер тестового пака");
        assert_eq!(sticker.pack, "test_all");
        assert_eq!(sticker.price, 70);
        let goose = find(&set, "test_all/goose").expect("гусь");
        let path = goose.image_path().expect("у гуся своя PNG");
        assert!(path.exists());
        assert_eq!(goose.display_scale(), 1.0);
        assert_eq!(find(&set, "test_all/gosling").unwrap().display_scale(), 0.5);
        assert!(catalog_any(&set)
            .iter()
            .any(|item| item.id == "test_all/test_dragon"));
    }

    #[test]
    fn every_sticker_has_picture() {
        for sticker in all() {
            assert!(
                crate::icon_data::ICONS
                    .iter()
                    .any(|(key, _, _)| *key == sticker.icon_key()),
                "нет картинки для стикера {} ({})",
                sticker.id,
                sticker.icon
            );
        }
    }
}
