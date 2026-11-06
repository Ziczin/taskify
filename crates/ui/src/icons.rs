//! Иконки интерфейса: только PNG-картинки, никакого текста вместо них.
//!
//! Кнопки, навигация, иконки заданий и стикеры берут картинки из
//! `assets/icons` (сгенерированы `tools/make_icons.py`).

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use eframe::egui;
use egui::{Color32, Response, TextureHandle, TextureOptions};

use crate::icon_data::ICONS;

pub struct Icons {
    textures: HashMap<&'static str, (TextureHandle, bool)>,
    /// PNG-стикеры паков: ключ — абсолютный путь, чтобы паки с одинаковыми
    /// id не перетирали картинки друг друга.
    pack_art: HashMap<PathBuf, PackArt>,
}

struct PackArt {
    texture: TextureHandle,
    natural: egui::Vec2,
}

impl Icons {
    pub fn load(ctx: &egui::Context) -> Self {
        let mut textures = HashMap::new();
        for (key, bytes, mono) in ICONS {
            match decode(bytes) {
                Some(image) => {
                    let handle = ctx.load_texture(*key, image, TextureOptions::LINEAR);
                    textures.insert(*key, (handle, *mono));
                }
                None => eprintln!("иконка {key}: PNG не прочитался"),
            }
        }
        Self {
            textures,
            pack_art: HashMap::new(),
        }
    }

    /// Загрузить PNG-стикеры всех установленных паков.
    pub fn load_sticker_images(&mut self, ctx: &egui::Context, packs: &taskify_packs::PackSet) {
        for pack in packs.packs.values() {
            for def in pack.stickers.values() {
                let Some(path) = def.image_path(&pack.root) else {
                    continue;
                };
                if self.pack_art.contains_key(&path) {
                    continue;
                }
                let Ok(bytes) = std::fs::read(&path) else {
                    eprintln!("стикер {}: файл не читается: {}", def.id, path.display());
                    continue;
                };
                let Some(image) = decode(&bytes) else {
                    eprintln!("стикер {}: PNG не прочитался: {}", def.id, path.display());
                    continue;
                };
                let natural = egui::vec2(image.size[0] as f32, image.size[1] as f32);
                let name = format!("pack-sticker:{}", path.display());
                let texture = ctx.load_texture(name, image, TextureOptions::LINEAR);
                self.pack_art.insert(path, PackArt { texture, natural });
            }
        }
    }

    /// Размер картинки, вписанной в квадрат `max` с сохранением пропорций.
    pub(crate) fn file_fit(&self, path: &Path, max: f32) -> Option<egui::Vec2> {
        let art = self.pack_art.get(path)?;
        let max = max.max(1.0);
        let fit = max / art.natural.x.max(art.natural.y).max(1.0);
        Some(art.natural * fit)
    }

    pub fn show_file(&self, ui: &mut egui::Ui, path: &Path, max: f32) -> Option<Response> {
        let size = self.file_fit(path, max)?;
        let art = self.pack_art.get(path)?;
        let image = egui::Image::new(egui::load::SizedTexture::new(art.texture.id(), size));
        Some(ui.add(image))
    }

    /// Нарисовать PNG пака с поворотом вокруг центра прямоугольника.
    pub(crate) fn paint_file_rotated(
        &self,
        ui: &egui::Ui,
        path: &Path,
        rect: egui::Rect,
        rotation: f32,
    ) -> bool {
        let Some(art) = self.pack_art.get(path) else {
            return false;
        };
        paint_texture_rotated(ui, art.texture.id(), rect, rotation, Color32::WHITE);
        true
    }

    /// Картинка иконки нужного размера. Для монохромных цвет берётся из темы.
    pub(crate) fn image(
        &self,
        ui: &egui::Ui,
        key: &str,
        size: f32,
        tint: Option<Color32>,
    ) -> Option<egui::Image<'static>> {
        let (handle, mono) = self.textures.get(key)?;
        let mut image = egui::Image::new(egui::load::SizedTexture::new(
            handle.id(),
            egui::vec2(size, size),
        ));
        let color = tint.or_else(|| mono.then(|| ui.visuals().text_color()));
        if let Some(color) = color {
            image = image.tint(color);
        }
        Some(image)
    }

    pub fn show(&self, ui: &mut egui::Ui, key: &str, size: f32) -> Option<Response> {
        let image = self.image(ui, key, size, None)?;
        Some(ui.add(image))
    }

    pub fn show_tint(
        &self,
        ui: &mut egui::Ui,
        key: &str,
        size: f32,
        tint: Color32,
    ) -> Option<Response> {
        let image = self.image(ui, key, size, Some(tint))?;
        Some(ui.add(image))
    }

    /// Нарисовать иконку с поворотом вокруг центра прямоугольника.
    pub(crate) fn paint_rotated(
        &self,
        ui: &egui::Ui,
        key: &str,
        rect: egui::Rect,
        tint: Color32,
        rotation: f32,
    ) {
        let Some((handle, mono)) = self.textures.get(key) else {
            return;
        };
        let color = if *mono { tint } else { Color32::WHITE };
        paint_texture_rotated(ui, handle.id(), rect, rotation, color);
    }
}

/// Один прямоугольник текстуры, повёрнутый вокруг своего центра.
fn paint_texture_rotated(
    ui: &egui::Ui,
    texture: egui::TextureId,
    rect: egui::Rect,
    rotation: f32,
    color: Color32,
) {
    let mut mesh = egui::epaint::Mesh::with_texture(texture);
    mesh.add_rect_with_uv(
        rect,
        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)),
        color,
    );
    if rotation.abs() > f32::EPSILON {
        mesh.rotate(egui::emath::Rot2::from_angle(rotation), rect.center());
    }
    ui.painter().add(egui::Shape::mesh(mesh));
}

fn decode(bytes: &[u8]) -> Option<egui::ColorImage> {
    let image = image::load_from_memory_with_format(bytes, image::ImageFormat::Png)
        .ok()?
        .to_rgba8();
    let size = [image.width() as usize, image.height() as usize];
    Some(egui::ColorImage::from_rgba_unmultiplied(
        size,
        image.as_raw(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pack_pictures_exist() {
        // Каждая иконка пака — ключ реальной PNG. Текстом символы не рисуются.
        let source = crate::bootstrap::packs_source();
        let set = taskify_packs::PackSet::load(&source);
        for pack in set.packs.values() {
            for icon in pack.icons.values() {
                assert!(!icon.image.is_empty(), "{}: пустая картинка", icon.id);
                assert!(
                    ICONS.iter().any(|(key, _, _)| *key == icon.image),
                    "{}: нет картинки {}",
                    icon.id,
                    icon.image
                );
            }
            for sticker in pack.stickers.values() {
                if !sticker.image.trim().is_empty() {
                    let path = pack.root.join(&sticker.image);
                    assert!(path.exists(), "{}: нет PNG {}", sticker.id, path.display());
                    let bytes = std::fs::read(&path).unwrap();
                    assert!(
                        decode(&bytes).is_some(),
                        "{}: PNG не читается {}",
                        sticker.id,
                        path.display()
                    );
                } else {
                    assert!(!sticker.icon.is_empty(), "{}: пустая картинка", sticker.id);
                    assert!(
                        ICONS.iter().any(|(key, _, _)| *key == sticker.icon),
                        "{}: нет картинки {}",
                        sticker.id,
                        sticker.icon
                    );
                }
            }
        }
    }

    #[test]
    fn every_icon_decodes() {
        for (key, bytes, _) in ICONS {
            assert!(decode(bytes).is_some(), "иконка {key}");
        }
    }
}
