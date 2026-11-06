//! Экран «Маркет»: витрина карточек с фильтрами по типу и паку.

use std::path::PathBuf;

use eframe::egui;
use taskify_packs::{TokenKind, Unlock};

use crate::app::{MarketKind, TaskifyApp};
use crate::stickers;

/// Что можно купить с карточки.
enum CardAction {
    Bundle(String),
    Token(String, String),
    Sticker(String),
}

/// Один товар витрины.
enum Card {
    Bundle {
        id: String,
        title: String,
        pack: String,
        description: String,
        price: i64,
        full: i64,
        contents: usize,
        base: &'static str,
        owned: bool,
    },
    Cosmetic {
        token: String,
        variant: String,
        title: String,
        pack: String,
        note: String,
        owned: bool,
        buyable: bool,
        swatch: Option<egui::Color32>,
        kind: TokenKind,
    },
    Sticker {
        id: String,
        title: String,
        pack: String,
        price: i64,
        owned: bool,
        icon: Option<String>,
        image: Option<PathBuf>,
    },
}

impl Card {
    fn order(&self) -> u8 {
        match self {
            Card::Bundle { .. } => 0,
            Card::Cosmetic { .. } => 1,
            Card::Sticker { .. } => 2,
        }
    }

    fn pack(&self) -> &str {
        match self {
            Card::Bundle { pack, .. }
            | Card::Cosmetic { pack, .. }
            | Card::Sticker { pack, .. } => pack,
        }
    }

    /// Куплено ли уже: такие товары с витрины убираются.
    fn owned(&self) -> bool {
        match self {
            Card::Bundle { owned, .. }
            | Card::Cosmetic { owned, .. }
            | Card::Sticker { owned, .. } => *owned,
        }
    }

    fn title(&self) -> &str {
        match self {
            Card::Bundle { title, .. }
            | Card::Cosmetic { title, .. }
            | Card::Sticker { title, .. } => title,
        }
    }
}

/// Обрезает строку до `max` символов, добавляя многоточие.
fn clip_line(text: &str, max: usize) -> String {
    let count = text.chars().count();
    if count <= max {
        return text.to_string();
    }
    let mut out: String = text.chars().take(max.saturating_sub(1)).collect();
    out.push('…');
    out
}

/// Переносит текст по словам вручную: egui-метка в узкой карточке иногда
/// растягивается вместо переноса, поэтому ширину держим сами.
fn wrap_text(text: &str, max: usize, lines: usize) -> String {
    let mut result: Vec<String> = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        let word = clip_line(word, max);
        if current.is_empty() {
            current = word;
        } else if current.chars().count() + 1 + word.chars().count() <= max {
            current.push(' ');
            current.push_str(&word);
        } else {
            result.push(std::mem::take(&mut current));
            current = word;
        }
    }
    if !current.is_empty() {
        result.push(current);
    }
    if result.len() > lines {
        result.truncate(lines);
        if let Some(last) = result.last_mut() {
            *last = clip_line(last, max);
        }
    }
    result.join("\n")
}

fn pack_of(id: &str) -> String {
    id.split_once('/')
        .map(|(pack, _)| pack.to_string())
        .unwrap_or_else(|| id.to_string())
}

impl TaskifyApp {
    /// Пометка о том, как открывается вариант: цена, уровень или ачивка.
    fn unlock_note(&self, unlock: &Unlock) -> String {
        match unlock {
            Unlock::Free => self.t("col.free"),
            Unlock::Coins { price } => {
                format!("{price} {}", self.t("common.coins_short"))
            }
            Unlock::Level { level } => {
                format!("{} {}", self.t("common.level"), level)
            }
            Unlock::Achievement { id } => format!("{} {id}", self.t("market.achievement")),
            Unlock::PackLevel { level } => {
                format!("{} {}", self.t("common.level"), level)
            }
        }
    }

    /// Локализованное имя пака по его id.
    fn pack_name(&self, id: &str) -> String {
        let locale = &self.settings.ui.locale;
        self.engine
            .packs
            .packs
            .get(id)
            .and_then(|pack| pack.manifest.as_ref())
            .map(|manifest| taskify_core::task::tr_or(&manifest.name, locale, id))
            .unwrap_or_else(|| id.to_string())
    }

    fn kind_label(&self, kind: TokenKind) -> String {
        match kind {
            TokenKind::Color => self.t("market.t_color"),
            TokenKind::Font => self.t("market.t_font"),
            TokenKind::Image => self.t("market.t_image"),
            TokenKind::Sound => self.t("market.t_sound"),
            TokenKind::Theme => self.t("market.t_theme"),
        }
    }

    pub(crate) fn screen_market(&mut self, ui: &mut egui::Ui) {
        ui.heading(self.t("market.title"));
        ui.label(egui::RichText::new(self.t("market.hint")).small().weak());
        ui.add_space(8.0);

        let locale = self.settings.ui.locale.clone();
        let sticker_catalog = stickers::catalog(&self.engine.packs);

        // --- фильтр по типу -------------------------------------------------
        let mut kind = self.market_kind;
        let kinds: Vec<(MarketKind, String)> = MarketKind::ALL
            .iter()
            .map(|kind| (*kind, self.t(kind.key())))
            .collect();
        ui.horizontal_wrapped(|ui| {
            for (value, label) in &kinds {
                if ui.selectable_label(kind == *value, label).clicked() {
                    kind = *value;
                }
            }
        });
        self.market_kind = kind;

        // --- фильтр по паку --------------------------------------------------
        let mut packs: Vec<String> = Vec::new();
        for bundle in self.engine.packs.bundles() {
            if self.engine.bundle_owned(&bundle.id) {
                continue;
            }
            let pack = pack_of(&bundle.id);
            if !packs.contains(&pack) {
                packs.push(pack);
            }
        }
        for token in self.engine.packs.tokens() {
            let any_left = token
                .variants
                .iter()
                .any(|variant| !self.engine.owned(&token.id, &variant.id));
            if !any_left {
                continue;
            }
            let pack = pack_of(&token.id);
            if !packs.contains(&pack) {
                packs.push(pack);
            }
        }
        for sticker in sticker_catalog.iter().filter(|sticker| sticker.price > 0) {
            if self.engine.sticker_owned(&sticker.id) {
                continue;
            }
            if !packs.contains(&sticker.pack) {
                packs.push(sticker.pack.clone());
            }
        }
        packs.sort();

        let mut pack_filter = self.market_pack.clone();
        let all_packs = self.t("market.all_packs");
        let pack_labels: Vec<(Option<String>, String)> = std::iter::once((None, all_packs))
            .chain(
                packs
                    .iter()
                    .map(|pack| (Some(pack.clone()), self.pack_name(pack))),
            )
            .collect();
        ui.horizontal_wrapped(|ui| {
            ui.label(
                egui::RichText::new(self.t("market.pack_filter"))
                    .small()
                    .weak(),
            );
            for (value, label) in &pack_labels {
                if ui.selectable_label(pack_filter == *value, label).clicked() {
                    pack_filter = value.clone();
                }
            }
        });
        self.market_pack = pack_filter.clone();
        ui.add_space(8.0);

        // --- сборка витрины ---------------------------------------------------
        let mut cards: Vec<Card> = Vec::new();
        if matches!(self.market_kind, MarketKind::All | MarketKind::Themes) {
            for bundle in self.engine.packs.bundles() {
                cards.push(Card::Bundle {
                    id: bundle.id.clone(),
                    title: bundle.title(&locale),
                    pack: pack_of(&bundle.id),
                    description: bundle.describe(&locale),
                    price: bundle.price,
                    full: self.engine.bundle_full_price(bundle),
                    contents: bundle.contents_count(),
                    base: bundle.theme_base().as_str(),
                    owned: self.engine.bundle_owned(&bundle.id),
                });
            }
        }
        if matches!(
            kind,
            MarketKind::All | MarketKind::Themes | MarketKind::Cosmetics
        ) {
            for token in self.engine.packs.tokens() {
                let is_theme = token.kind == TokenKind::Theme;
                if (kind == MarketKind::Themes && !is_theme)
                    || (kind == MarketKind::Cosmetics && is_theme)
                {
                    continue;
                }
                for variant in &token.variants {
                    let swatch = if token.kind == TokenKind::Color {
                        crate::app::variant_color(&self.engine.packs, &token.id, variant)
                    } else {
                        None
                    };
                    let note = self.unlock_note(&variant.unlock);
                    cards.push(Card::Cosmetic {
                        token: token.id.clone(),
                        variant: variant.id.clone(),
                        title: taskify_core::task::tr_or(&variant.title, &locale, &variant.id),
                        pack: pack_of(&token.id),
                        note,
                        owned: self.engine.owned(&token.id, &variant.id),
                        buyable: self.engine.can_buy(&variant.unlock),
                        swatch,
                        kind: token.kind,
                    });
                }
            }
        }
        if matches!(self.market_kind, MarketKind::All | MarketKind::Stickers) {
            for sticker in sticker_catalog.iter().filter(|sticker| sticker.price > 0) {
                cards.push(Card::Sticker {
                    id: sticker.id.clone(),
                    title: sticker.title(&locale),
                    pack: sticker.pack.clone(),
                    price: sticker.price,
                    owned: self.engine.sticker_owned(&sticker.id),
                    icon: Some(sticker.icon.clone()),
                    image: sticker.image.clone(),
                });
            }
        }
        // Купленное с витрины пропадает: повторно покупать нечего.
        cards.retain(|card| !card.owned());
        if let Some(pack) = &self.market_pack {
            cards.retain(|card| card.pack() == pack.as_str());
        }
        cards.sort_by(|left, right| {
            left.order()
                .cmp(&right.order())
                .then_with(|| left.pack().cmp(right.pack()))
                .then_with(|| {
                    left.title()
                        .to_lowercase()
                        .cmp(&right.title().to_lowercase())
                })
        });

        if cards.is_empty() {
            ui.label(
                egui::RichText::new(self.t("market.all_owned"))
                    .small()
                    .weak(),
            );
            return;
        }

        // Раскладка «плиткой»: карточки фиксированной ширины сами переносятся
        // на новую строку. Grid здесь не годится — он измеряет ячейки без
        // ограничения ширины, и длинное описание растягивает витрину за экран.
        let spacing = 10.0;
        let mut action: Option<CardAction> = None;
        // Сетка строится явными строками: horizontal_wrapped в прокрутке
        // растягивался в бесконечную «ёлочку» и не переносил карточки.
        let available = ui.available_width();
        // Колонок не больше, чем влезает минимальных карточек: иначе строка
        // снова уезжает за правый край.
        let wanted = self.settings.ui.market_columns.clamp(1, 8);
        let fit = (((available + spacing) / (170.0 + spacing)).floor() as usize).max(1);
        let columns = wanted.min(fit);
        // Внешние поля карточки — по 12 px слева и справа.
        let card_width = ((available - spacing * (columns - 1) as f32) / columns as f32 - 24.0)
            .clamp(140.0, 300.0);
        for row in cards.chunks(columns) {
            ui.horizontal_top(|ui| {
                ui.spacing_mut().item_spacing = egui::vec2(0.0, spacing);
                for (index, card) in row.iter().enumerate() {
                    if index > 0 {
                        ui.add_space(spacing + 12.0);
                    }
                    self.market_card(ui, card, &locale, &mut action, card_width);
                }
            });
            ui.add_space(spacing);
        }

        // --- покупки после обхода витрины -------------------------------------
        match action {
            Some(CardAction::Bundle(id)) => {
                let Some(bundle) = self.engine.packs.bundle(&id).cloned() else {
                    self.set_status(format!("тема не найдена: {id}"));
                    return;
                };
                match self.engine.buy_bundle(&bundle) {
                    Ok(()) => {
                        let text = format!("{}: {}", self.t("market.buy"), bundle.title(&locale));
                        self.toast(text, ui.ctx());
                    }
                    Err(error) => self.set_status(format!("{error:#}")),
                }
            }
            Some(CardAction::Token(token, variant)) => {
                match self.engine.purchase(&token, &variant) {
                    Ok(()) => {
                        let text = format!("{}: {variant}", self.t("market.buy"));
                        self.toast(text, ui.ctx());
                    }
                    Err(error) => self.set_status(format!("{error:#}")),
                }
            }
            Some(CardAction::Sticker(id)) => {
                let Some(sticker) = stickers::find(&self.engine.packs, &id) else {
                    self.set_status(format!("стикер не найден: {id}"));
                    return;
                };
                match self.engine.buy_sticker(&sticker) {
                    Ok(()) => {
                        let text = format!("{}: {}", self.t("market.buy"), sticker.title(&locale));
                        self.toast(text, ui.ctx());
                    }
                    Err(error) => self.set_status(format!("{error:#}")),
                }
            }
            None => {}
        }
    }

    /// Одна карточка витрины: тип, название, пак, описание и покупка.
    ///
    /// Карточка рисуется на заранее выделенном прямоугольнике: Frame в узкой
    /// строке измерялся по содержимому и расползался шире сетки.
    fn market_card(
        &self,
        ui: &mut egui::Ui,
        card: &Card,
        _locale: &str,
        action: &mut Option<CardAction>,
        card_width: f32,
    ) {
        let (
            type_label,
            description,
            price,
            full,
            owned,
            buyable,
            swatch,
            icon,
            image,
            action_value,
        ) = match card {
            Card::Bundle {
                id,
                description,
                price,
                full,
                contents,
                base,
                owned,
                ..
            } => (
                format!(
                    "{} · {} · {contents}",
                    self.t("market.t_theme"),
                    self.base_label(base)
                ),
                description.clone(),
                Some(*price),
                Some(*full),
                *owned,
                !*owned,
                None,
                Some("ui.gift".to_string()),
                None,
                CardAction::Bundle(id.clone()),
            ),
            Card::Cosmetic {
                token,
                variant,
                kind,
                note,
                owned,
                buyable,
                swatch,
                ..
            } => (
                self.kind_label(*kind),
                note.clone(),
                None,
                None,
                *owned,
                *buyable,
                *swatch,
                None,
                None,
                CardAction::Token(token.clone(), variant.clone()),
            ),
            Card::Sticker {
                id,
                price,
                owned,
                icon,
                image,
                ..
            } => (
                self.t("market.single_sticker"),
                String::new(),
                Some(*price),
                None,
                *owned,
                true,
                None,
                icon.clone(),
                image.clone(),
                CardAction::Sticker(id.clone()),
            ),
        };

        let accent = ui.visuals().hyperlink_color;
        let card_height = 198.0;
        let (outer, _) = ui.allocate_exact_size(
            egui::vec2(card_width + 24.0, card_height),
            egui::Sense::hover(),
        );
        let visuals = ui.visuals().widgets.noninteractive;
        ui.painter().rect(
            outer,
            egui::CornerRadius::same(6),
            visuals.bg_fill,
            visuals.bg_stroke,
            egui::StrokeKind::Inside,
        );

        // Ряд действий — отдельная полоса у нижней кромки карточки: её
        // положение зависит только от карточки, а не от длины текста.
        let strip = 30.0;
        let content_rect = egui::Rect::from_min_size(
            outer.min + egui::vec2(12.0, 10.0),
            egui::vec2(card_width, card_height - 10.0 - strip - 14.0),
        );
        ui.scope_builder(
            egui::UiBuilder::new()
                .max_rect(content_rect)
                .layout(egui::Layout::top_down(egui::Align::LEFT)),
            |ui| {
                ui.set_width(card_width);
                ui.set_clip_rect(content_rect.intersect(ui.clip_rect()));
                ui.label(egui::RichText::new(type_label).small().weak());
                ui.horizontal(|ui| {
                    if let Some(path) = &image {
                        self.icons.show_file(ui, path, 30.0);
                    } else if let Some(key) = &icon {
                        self.icons.show(ui, key, 30.0);
                    }
                    if let Some(swatch) = swatch {
                        let (rect, _) =
                            ui.allocate_exact_size(egui::vec2(16.0, 16.0), egui::Sense::hover());
                        ui.painter()
                            .rect_filled(rect, egui::CornerRadius::same(3), swatch);
                    }
                });

                let text_chars = (card_width / 7.5).max(8.0) as usize;
                ui.label(
                    egui::RichText::new(clip_line(card.title(), text_chars))
                        .strong()
                        .size(15.0),
                );
                ui.label(
                    egui::RichText::new(clip_line(&self.pack_name(card.pack()), text_chars))
                        .small()
                        .color(accent),
                );
                if !description.is_empty() {
                    ui.add(
                        egui::Label::new(
                            egui::RichText::new(wrap_text(&description, text_chars, 3))
                                .small()
                                .color(ui.visuals().weak_text_color()),
                        )
                        .wrap(),
                    );
                }
            },
        );

        let action_rect = egui::Rect::from_min_size(
            egui::pos2(outer.left() + 12.0, outer.bottom() - 10.0 - strip),
            egui::vec2(card_width, strip),
        );
        ui.scope_builder(
            egui::UiBuilder::new()
                .max_rect(action_rect)
                .layout(egui::Layout::right_to_left(egui::Align::Center)),
            |ui| {
                if owned {
                    ui.label(
                        egui::RichText::new(self.t("col.owned"))
                            .small()
                            .color(self.palette().success),
                    );
                } else {
                    if buyable || matches!(card, Card::Sticker { .. }) {
                        if ui.button(self.t("market.buy")).clicked() {
                            *action = Some(action_value);
                        }
                    } else if matches!(card, Card::Cosmetic { .. }) {
                        ui.label(egui::RichText::new(self.t("col.locked")).small().weak());
                    }
                    if let Some(full) = full {
                        if full > price.unwrap_or(0) {
                            ui.label(
                                egui::RichText::new(format!(
                                    "{} {}",
                                    full,
                                    self.t("common.coins_short")
                                ))
                                .small()
                                .weak()
                                .strikethrough(),
                            );
                        }
                    }
                    if let Some(price) = price {
                        ui.label(
                            egui::RichText::new(format!(
                                "{} {}",
                                price,
                                self.t("common.coins_short")
                            ))
                            .strong(),
                        );
                        self.icons.show(ui, "ui.coin", 14.0);
                    }
                }
            },
        );

        if self.layout_debug {
            eprintln!(
                "market card {:.1} {:.1} {:.1} {:.1}",
                outer.min.x,
                outer.min.y,
                outer.width(),
                outer.height()
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pack_of_splits_qualified_ids() {
        assert_eq!(pack_of("beauty/glow"), "beauty");
        assert_eq!(pack_of("starter/color.accent"), "starter");
        assert_eq!(pack_of("star"), "star");
    }

    #[test]
    fn hex_color_parses_rgb() {
        assert_eq!(
            crate::app::hex_color("#ff8800"),
            Some(egui::Color32::from_rgb(255, 136, 0))
        );
        assert_eq!(
            crate::app::hex_color("ff8800"),
            Some(egui::Color32::from_rgb(255, 136, 0))
        );
        assert_eq!(crate::app::hex_color("#fff"), None);
        assert_eq!(crate::app::hex_color("зелёный"), None);
    }
}
