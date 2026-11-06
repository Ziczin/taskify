//! Экран «Склад»: всё купленное — темы, косметика и стикеры.
//!
//! Внутри — вкладки, как в маркете, и карточки товаров: у цветов образец-квадратик,
//! у стикеров миниатюра и переключатели боковых панелей.

use eframe::egui;
use taskify_packs::{Bundle, Token, TokenKind};

use crate::app::{StockTab, TaskifyApp};
use crate::stickers::{self, Sticker};

/// Ширина карточек на складе.
const CARD_WIDTH: f32 = 168.0;

/// Подвкладки косметики: каждый элемент оформления отдельно, а не всё в куче.
const COSMETIC_GROUPS: [&str; 6] = [
    "accent",
    "background",
    "wallpaper",
    "font",
    "sound",
    "other",
];

/// К какой подвкладке относится токен.
fn cosmetic_group(token: &Token) -> &'static str {
    // id бывает с префиксом пака: `starter/color.accent`. Берём последний
    // сегмент после точки — `accent` или `background`.
    let tail = token.id.rsplit('/').next().unwrap_or(&token.id);
    let role = tail.rsplit('.').next().unwrap_or(tail);
    match token.kind {
        TokenKind::Theme => "other",
        TokenKind::Color => match role {
            "accent" => "accent",
            "background" => "background",
            _ => "other",
        },
        TokenKind::Image => "wallpaper",
        TokenKind::Font => "font",
        TokenKind::Sound => "sound",
    }
}

/// Квадратик-образец цвета.
fn swatch(ui: &mut egui::Ui, color: egui::Color32) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(18.0, 18.0), egui::Sense::hover());
    ui.painter()
        .rect_filled(rect, egui::CornerRadius::same(4), color);
}

/// Высота нижней полосы действий в карточках склада.
const ACTION_ROW: f32 = 28.0;

/// Карточка склада с действием, прижатым к нижней кромке.
///
/// Содержимое рисуется в верхней части и обрезается по её границе: сколько бы
/// текста ни было, кнопка остаётся на одном и том же месте карточки.
fn stock_card(
    ui: &mut egui::Ui,
    width: f32,
    height: f32,
    content: impl FnOnce(&mut egui::Ui),
    action: impl FnOnce(&mut egui::Ui),
) {
    egui::Frame::group(ui.style())
        .inner_margin(egui::Margin::symmetric(10, 8))
        .show(ui, |ui| {
            ui.set_width(width);
            let (rect, _) = ui.allocate_exact_size(egui::vec2(width, height), egui::Sense::hover());
            let content_rect = egui::Rect::from_min_size(
                rect.min,
                egui::vec2(width, (height - ACTION_ROW - 6.0).max(20.0)),
            );
            ui.scope_builder(
                egui::UiBuilder::new()
                    .max_rect(content_rect)
                    .layout(egui::Layout::top_down(egui::Align::LEFT)),
                |ui| {
                    ui.set_width(width);
                    ui.set_clip_rect(content_rect.intersect(ui.clip_rect()));
                    content(ui);
                },
            );
            let action_rect = egui::Rect::from_min_size(
                egui::pos2(rect.left(), rect.bottom() - ACTION_ROW),
                egui::vec2(width, ACTION_ROW),
            );
            ui.scope_builder(
                egui::UiBuilder::new()
                    .max_rect(action_rect)
                    .layout(egui::Layout::left_to_right(egui::Align::Center)),
                |ui| action(ui),
            );
        });
}

/// Сетка карточек складских списков.
///
/// Явные строки, как в маркете: `horizontal_wrapped` с карточками фиксированной
/// ширины не переносит их и растягивает список в одну линию за край экрана.
/// Ширина карточки ужимается под контейнер, поэтому строка не выходит за него.
fn stock_grid<T>(
    ui: &mut egui::Ui,
    preferred_width: f32,
    gap: f32,
    items: &[T],
    mut draw: impl FnMut(&mut egui::Ui, &T, f32),
) {
    if items.is_empty() {
        return;
    }
    let available = ui.available_width().max(1.0);
    // stock_card добавляет по 10 px внутреннего отступа с каждой стороны.
    let width = preferred_width.min((available - 20.0).max(24.0));
    let slot = width + 20.0;
    let columns = (((available + gap) / (slot + gap)).floor() as usize).max(1);
    for row in items.chunks(columns) {
        ui.horizontal_top(|ui| {
            ui.spacing_mut().item_spacing.x = 0.0;
            for (index, item) in row.iter().enumerate() {
                if index > 0 {
                    ui.add_space(gap);
                }
                draw(ui, item, width);
            }
        });
        ui.add_space(gap);
    }
}

impl TaskifyApp {
    pub(crate) fn screen_stock(&mut self, ui: &mut egui::Ui) {
        ui.heading(self.t("stock.title"));
        ui.label(egui::RichText::new(self.t("stock.hint")).small().weak());
        ui.add_space(6.0);

        let locale = self.settings.ui.locale.clone();
        // Склад показывает базовые элементы и покупки из всех паков, даже
        // выключенных: выключенный пак не должен прятать оформление.
        let tokens: Vec<Token> = self
            .engine
            .packs
            .tokens_any()
            .into_iter()
            .cloned()
            .collect();
        let owned_bundles: Vec<Bundle> = self
            .engine
            .packs
            .bundles()
            .into_iter()
            .filter(|bundle| self.engine.bundle_owned(&bundle.id))
            .cloned()
            .collect();
        let owned_stickers: Vec<Sticker> = stickers::catalog_any(&self.engine.packs)
            .into_iter()
            .filter(|sticker| self.engine.sticker_owned(&sticker.id))
            .collect();
        let profiles = self.settings.look_profiles.len();
        let owned_theme_variants: usize = tokens
            .iter()
            .filter(|token| token.kind == TokenKind::Theme)
            .map(|token| {
                token
                    .variants
                    .iter()
                    .filter(|variant| self.engine.owned(&token.id, &variant.id))
                    .count()
            })
            .sum();
        let cosmetics_count: usize = tokens
            .iter()
            .filter(|token| token.kind != TokenKind::Theme)
            .map(|token| {
                token
                    .variants
                    .iter()
                    .filter(|variant| self.engine.owned(&token.id, &variant.id))
                    .count()
            })
            .sum();
        let themes_count = owned_bundles.len() + owned_theme_variants;

        // Вкладки с количеством.
        let counts = [
            (StockTab::Themes, themes_count),
            (StockTab::Cosmetics, cosmetics_count),
            (StockTab::Stickers, owned_stickers.len()),
            (StockTab::Profiles, profiles),
        ];
        let labels: Vec<(StockTab, String)> = counts
            .iter()
            .map(|(tab, count)| (*tab, format!("{} ({count})", self.t(tab.key()))))
            .collect();
        ui.horizontal_wrapped(|ui| {
            for (tab, label) in &labels {
                if ui.selectable_label(self.stock_tab == *tab, label).clicked() {
                    self.stock_tab = *tab;
                    self.start_tab_transition(ui.ctx());
                }
            }
        });
        ui.add_space(8.0);

        // Смена вкладки двигает только содержимое, шапка и вкладки стоят.
        let progress = self.tab_progress(ui.ctx());
        Self::transition(ui, progress);

        let mut apply: Option<(String, String, String)> = None;
        let mut apply_base: Option<String> = None;
        let mut toggle: Option<(String, String)> = None;
        let mut apply_bundle: Option<String> = None;
        let mut save_profile = false;
        let mut apply_profile: Option<String> = None;
        let mut delete_profile: Option<String> = None;
        let mut overwrite_profile: Option<String> = None;
        // Темы живут отдельно от остальной косметики: один список «Темы».
        let cosmetic_tokens: Vec<Token> = tokens
            .iter()
            .filter(|token| token.kind != TokenKind::Theme)
            .cloned()
            .collect();

        match self.stock_tab {
            StockTab::Cosmetics => {
                self.stock_cosmetics(ui, &cosmetic_tokens, &locale, &mut apply);
            }
            StockTab::Themes => {
                self.stock_themes(
                    ui,
                    &owned_bundles,
                    &tokens,
                    &locale,
                    &mut apply_bundle,
                    &mut apply_base,
                );
            }
            StockTab::Stickers => {
                self.stock_stickers(ui, &owned_stickers, &locale, &mut toggle);
            }
            StockTab::Profiles => {
                self.stock_profiles(
                    ui,
                    &mut save_profile,
                    &mut apply_profile,
                    &mut delete_profile,
                    &mut overwrite_profile,
                );
            }
        }

        if save_profile {
            self.save_look_profile(ui.ctx());
        }
        if let Some(name) = apply_profile {
            self.apply_look_profile(&name, ui.ctx());
        }
        if let Some(name) = delete_profile {
            self.delete_look_profile(&name, ui.ctx());
        }
        if let Some(name) = overwrite_profile {
            self.overwrite_look_target = Some(name);
        }
        if let Some(id) = apply_bundle {
            if let Some(bundle) = self.engine.packs.bundle(&id).cloned() {
                self.apply_bundle(&bundle, ui.ctx());
            }
        }
        if let Some(base) = apply_base {
            self.apply_base_theme(&base, ui.ctx());
        }
        if let Some((id, side)) = toggle {
            let result = if self.engine.sticker_placed(&id, &side) {
                self.engine.remove_sticker(&id, &side)
            } else {
                self.engine.place_sticker(&id, &side)
            };
            if let Err(error) = result {
                self.set_status(format!("{error:#}"));
            }
        }
        if let Some((token, variant, value)) = apply {
            self.apply_cosmetic(&token, &variant, &value, ui.ctx());
        }
    }

    /// Косметика: только купленное, карточками с образцом цвета.
    ///
    /// Внутри — подвкладки по элементам оформления: тема, акцент, фон, обои,
    /// шрифт, звук. Так цвета разных элементов не лежат одной кучей.
    fn stock_cosmetics(
        &mut self,
        ui: &mut egui::Ui,
        tokens: &[Token],
        locale: &str,
        apply: &mut Option<(String, String, String)>,
    ) {
        let groups: Vec<(&'static str, usize)> = COSMETIC_GROUPS
            .iter()
            .filter_map(|group| {
                let count: usize = tokens
                    .iter()
                    .filter(|token| cosmetic_group(token) == *group)
                    .map(|token| {
                        token
                            .variants
                            .iter()
                            .filter(|variant| self.engine.owned(&token.id, &variant.id))
                            .count()
                    })
                    .sum();
                (count > 0).then_some((*group, count))
            })
            .collect();
        if groups.is_empty() {
            ui.label(
                egui::RichText::new(self.t("stock.no_cosmetics"))
                    .small()
                    .weak(),
            );
            return;
        }
        if !groups
            .iter()
            .any(|(group, _)| *group == self.stock_cosmetic_group)
        {
            self.stock_cosmetic_group = groups[0].0.to_string();
        }
        let selected = self.stock_cosmetic_group.clone();
        let labels: Vec<(&'static str, String)> = groups
            .iter()
            .map(|(group, count)| {
                (
                    *group,
                    format!("{} ({count})", self.t(&format!("stock.cosmetic_{group}"))),
                )
            })
            .collect();
        ui.horizontal_wrapped(|ui| {
            for (group, label) in &labels {
                if ui.selectable_label(selected == *group, label).clicked() {
                    self.stock_cosmetic_group = (*group).to_string();
                }
            }
        });
        ui.add_space(6.0);

        let selected = self.stock_cosmetic_group.clone();
        let owned: Vec<(&Token, &taskify_packs::Variant)> = tokens
            .iter()
            .filter(|token| cosmetic_group(token) == selected.as_str())
            .flat_map(|token| {
                token
                    .variants
                    .iter()
                    .filter(|variant| self.engine.owned(&token.id, &variant.id))
                    .map(move |variant| (token, variant))
            })
            .collect();
        let any = !owned.is_empty();
        stock_grid(ui, CARD_WIDTH, 10.0, &owned, |ui, item, width| {
            let (token, variant) = *item;
            let title = taskify_core::task::tr_or(&variant.title, locale, &variant.id);
            let group = taskify_core::task::tr_or(&token.title, locale, &token.id);
            let applied = self
                .settings
                .extra
                .get(&token.id)
                .map(|value| value == &variant.id)
                .unwrap_or(false);
            let sample = if token.kind == TokenKind::Color {
                crate::app::variant_color(&self.engine.packs, &token.id, variant)
            } else {
                None
            };
            stock_card(
                ui,
                width,
                92.0,
                |ui| {
                    ui.horizontal(|ui| {
                        if let Some(color) = sample {
                            swatch(ui, color);
                        }
                        ui.add(
                            egui::Label::new(egui::RichText::new(&title).strong().size(14.0))
                                .truncate(),
                        );
                    });
                    ui.label(egui::RichText::new(&group).small().weak());
                },
                |ui| {
                    if applied {
                        ui.label(
                            egui::RichText::new(self.t("col.applied"))
                                .small()
                                .color(self.accent),
                        );
                    } else if ui.button(self.t("col.apply")).clicked() {
                        *apply =
                            Some((token.id.clone(), variant.id.clone(), variant.value.clone()));
                    }
                },
            );
        });
        if !any {
            ui.label(
                egui::RichText::new(self.t("stock.no_cosmetics"))
                    .small()
                    .weak(),
            );
        }
    }

    /// Темы: готовые наборы и базовые темы одним списком.
    fn stock_themes(
        &mut self,
        ui: &mut egui::Ui,
        bundles: &[Bundle],
        tokens: &[Token],
        locale: &str,
        apply_bundle: &mut Option<String>,
        apply_base: &mut Option<String>,
    ) {
        let mut any = false;
        if !bundles.is_empty() {
            ui.label(egui::RichText::new(self.t("stock.ready_themes")).strong());
            ui.add_space(4.0);
            self.stock_bundles(ui, bundles, locale, apply_bundle);
            any = true;
        }

        let base_owned = tokens.iter().any(|token| {
            token.kind == TokenKind::Theme
                && token
                    .variants
                    .iter()
                    .any(|variant| self.engine.owned(&token.id, &variant.id))
        });
        if base_owned {
            if any {
                ui.add_space(10.0);
            }
            ui.label(egui::RichText::new(self.t("stock.base_themes")).strong());
            ui.add_space(4.0);
            let owned: Vec<(&Token, &taskify_packs::Variant)> = tokens
                .iter()
                .filter(|token| token.kind == TokenKind::Theme)
                .flat_map(|token| {
                    token
                        .variants
                        .iter()
                        .filter(|variant| self.engine.owned(&token.id, &variant.id))
                        .map(move |variant| (token, variant))
                })
                .collect();
            any |= !owned.is_empty();
            stock_grid(ui, CARD_WIDTH, 10.0, &owned, |ui, item, width| {
                let (token, variant) = *item;
                let title = taskify_core::task::tr_or(&variant.title, locale, &variant.id);
                let group = taskify_core::task::tr_or(&token.title, locale, &token.id);
                let applied = self
                    .settings
                    .extra
                    .get(&token.id)
                    .map(|value| value == &variant.id)
                    .unwrap_or(false);
                stock_card(
                    ui,
                    width,
                    92.0,
                    |ui| {
                        ui.add(
                            egui::Label::new(egui::RichText::new(&title).strong().size(14.0))
                                .truncate(),
                        );
                        ui.label(egui::RichText::new(&group).small().weak());
                    },
                    |ui| {
                        if applied {
                            ui.label(
                                egui::RichText::new(self.t("col.applied"))
                                    .small()
                                    .color(self.accent),
                            );
                        } else if ui.button(self.t("col.apply")).clicked() {
                            *apply_base = Some(if variant.id == "dark" {
                                "dark".to_string()
                            } else {
                                "light".to_string()
                            });
                        }
                    },
                );
            });
        }

        if !any {
            ui.label(
                egui::RichText::new(self.t("stock.no_themes"))
                    .small()
                    .weak(),
            );
        }
    }

    /// Готовые темы: купленные, карточками.
    fn stock_bundles(
        &self,
        ui: &mut egui::Ui,
        bundles: &[Bundle],
        locale: &str,
        apply_bundle: &mut Option<String>,
    ) {
        if bundles.is_empty() {
            ui.label(
                egui::RichText::new(self.t("stock.no_bundles"))
                    .small()
                    .weak(),
            );
            return;
        }
        stock_grid(ui, 240.0, 10.0, bundles, |ui, bundle, width| {
            let title = bundle.title(locale);
            let description = bundle.describe(locale);
            let contents = format!("{}: {}", self.t("market.contents"), bundle.contents_count());
            let base = format!(
                "{}: {}",
                self.t("appearance.base"),
                self.base_label(bundle.theme_base().as_str())
            );
            stock_card(
                ui,
                width,
                152.0,
                |ui| {
                    ui.add(
                        egui::Label::new(egui::RichText::new(&title).strong().size(16.0))
                            .truncate(),
                    );
                    if !description.is_empty() {
                        ui.add(
                            egui::Label::new(
                                egui::RichText::new(&description)
                                    .small()
                                    .color(ui.visuals().weak_text_color()),
                            )
                            .truncate(),
                        );
                    }
                    ui.add(egui::Label::new(egui::RichText::new(&contents).small()).truncate());
                    ui.add(egui::Label::new(egui::RichText::new(&base).small()).truncate());
                },
                |ui| {
                    if ui.button(self.t("stock.apply_bundle")).clicked() {
                        *apply_bundle = Some(bundle.id.clone());
                    }
                },
            );
        });
    }

    /// Стикеры: миниатюра и переключатели «Слева»/«Справа».
    ///
    /// Стикеры висят на боковых панелях, поэтому у каждого два независимых
    /// места: один и тот же стикер может быть и слева, и справа.
    fn stock_stickers(
        &mut self,
        ui: &mut egui::Ui,
        stickers: &[Sticker],
        locale: &str,
        toggle: &mut Option<(String, String)>,
    ) {
        let grid_label = self.t("stock.sticker_grid");
        let grid_hint = self.t("stock.sticker_grid_hint");
        let opacity_label = self.t("stock.sticker_opacity");
        let opacity_hint = self.t("stock.sticker_opacity_hint");
        // В настройках хранится плотность, а ползунок показывает прозрачность.
        let mut transparency =
            100u8.saturating_sub(self.settings.ui.sticker_panel_opacity.min(100));
        let mut settings_changed = false;
        ui.horizontal_wrapped(|ui| {
            if ui
                .checkbox(&mut self.settings.ui.sticker_grid, grid_label)
                .changed()
            {
                settings_changed = true;
            }
            if self.settings.ui.sticker_grid
                && ui
                    .add(
                        egui::DragValue::new(&mut self.settings.ui.sticker_grid_size)
                            .range(8.0..=256.0)
                            .speed(1.0)
                            .suffix(" px"),
                    )
                    .changed()
            {
                settings_changed = true;
            }
            ui.add_space(8.0);
            ui.label(opacity_label).on_hover_text(&opacity_hint);
            if ui
                .add(
                    egui::DragValue::new(&mut transparency)
                        .range(0..=100)
                        .speed(1.0)
                        .suffix("%"),
                )
                .on_hover_text(&opacity_hint)
                .changed()
            {
                settings_changed = true;
            }
        });
        if settings_changed {
            self.settings.ui.sticker_grid_size =
                self.settings.ui.sticker_grid_size.clamp(8.0, 256.0);
            self.settings.ui.sticker_panel_opacity = 100u8.saturating_sub(transparency);
            self.settings.save();
        }
        ui.label(egui::RichText::new(grid_hint).small().weak());
        ui.add_space(8.0);
        if stickers.is_empty() {
            ui.label(
                egui::RichText::new(self.t("stock.no_stickers"))
                    .small()
                    .weak(),
            );
            return;
        }
        stock_grid(ui, 168.0, 10.0, stickers, |ui, sticker, width| {
            let title = sticker.title(locale);
            let left = self.engine.sticker_placed(&sticker.id, "left");
            let right = self.engine.sticker_placed(&sticker.id, "right");
            stock_card(
                ui,
                width,
                100.0,
                |ui| {
                    ui.horizontal(|ui| {
                        if let Some(path) = sticker.image_path() {
                            self.icons.show_file(ui, path, 32.0);
                        } else {
                            self.icons.show(ui, sticker.icon_key(), 32.0);
                        }
                        ui.add(egui::Label::new(egui::RichText::new(&title).strong()).truncate());
                    });
                    let mut sides = Vec::new();
                    if left {
                        sides.push(self.t("stickers.left"));
                    }
                    if right {
                        sides.push(self.t("stickers.right"));
                    }
                    if !sides.is_empty() {
                        ui.label(
                            egui::RichText::new(sides.join(" · "))
                                .small()
                                .color(self.accent),
                        );
                    }
                },
                |ui| {
                    if ui.selectable_label(left, self.t("stickers.left")).clicked() {
                        *toggle = Some((sticker.id.clone(), "left".to_string()));
                    }
                    if ui
                        .selectable_label(right, self.t("stickers.right"))
                        .clicked()
                    {
                        *toggle = Some((sticker.id.clone(), "right".to_string()));
                    }
                },
            );
        });
    }

    /// Профили оформления: временная тема, сборка, применение, время.
    fn stock_profiles(
        &mut self,
        ui: &mut egui::Ui,
        save_profile: &mut bool,
        apply_profile: &mut Option<String>,
        delete_profile: &mut Option<String>,
        overwrite_profile: &mut Option<String>,
    ) {
        // Тема окна живёт на складе: здесь её и меняем, сразу применяя.
        ui.horizontal(|ui| {
            ui.label(self.t("stock.theme"));
            let mut theme = self.settings.ui.theme.clone();
            egui::ComboBox::from_id_salt("stock_theme")
                .selected_text(&theme)
                .show_ui(ui, |ui| {
                    for value in ["system", "light", "dark"] {
                        ui.selectable_value(&mut theme, value.to_string(), value);
                    }
                });
            if theme != self.settings.ui.theme {
                self.settings.ui.theme = theme;
                // Любая правка собранной темы уходит во временную копию.
                self.settings.mark_look_draft();
                self.apply_look(ui.ctx());
                self.settings.save();
            }
        });
        ui.add_space(4.0);

        let auto_label = self.t("stock.auto_look");
        if ui
            .checkbox(&mut self.settings.auto_look, auto_label)
            .changed()
        {
            self.settings.save();
        }
        ui.label(
            egui::RichText::new(self.t("stock.auto_hint"))
                .small()
                .weak(),
        );
        ui.add_space(4.0);

        // Временная тема — это текущий вид после первой правки. Сохранённые
        // темы при этом не меняются.
        if self.settings.look_draft {
            let draft_title = self.t("stock.draft_title");
            let draft_hint = self.t("stock.draft_hint");
            let discard_label = self.t("stock.draft_discard");
            let base_name = self
                .settings
                .look_base
                .clone()
                .filter(|name| self.settings.look_profiles.contains_key(name));
            let base_text = base_name
                .as_deref()
                .map(|name| {
                    format!(
                        "{}: {}",
                        self.t("stock.draft_base"),
                        self.look_profile_title(name)
                    )
                })
                .unwrap_or_else(|| self.t("stock.draft_no_base"));
            egui::Frame::group(ui.style())
                .fill(
                    self.accent
                        .gamma_multiply(if self.settings.ui.theme == "dark" {
                            0.14
                        } else {
                            0.08
                        }),
                )
                .stroke(egui::Stroke::new(1.0, self.accent.gamma_multiply(0.45)))
                .inner_margin(egui::Margin::symmetric(12, 10))
                .show(ui, |ui| {
                    ui.label(egui::RichText::new(draft_title).strong().color(self.accent));
                    ui.label(egui::RichText::new(base_text).small().color(self.accent));
                    ui.label(egui::RichText::new(draft_hint).small());
                    if let Some(name) = base_name {
                        if ui.button(discard_label).clicked() {
                            *apply_profile = Some(name);
                        }
                    }
                });
            ui.add_space(6.0);
        }

        let profile_hint = self.t("stock.profile_name");
        let save_label = self.t("stock.save_profile");
        let profile_note = self.t("stock.profile_hint");
        ui.horizontal_wrapped(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.look_profile_name)
                    .desired_width(210.0)
                    .hint_text(&profile_hint),
            );
            if ui.button(save_label).clicked() {
                *save_profile = true;
            }
            ui.label(egui::RichText::new(profile_note).small().weak());
        });

        let profile_names: Vec<String> = self.settings.look_profiles.keys().cloned().collect();
        if profile_names.is_empty() {
            ui.label(
                egui::RichText::new(self.t("stock.no_profiles"))
                    .small()
                    .weak(),
            );
            return;
        }

        let apply_label = self.t("col.apply");
        let delete_label = self.t("stock.delete");
        let overwrite_label = self.t("stock.overwrite");
        let active_label = self.t("stock.active_now");
        let at_label = self.t("stock.at_time");
        let light = self.t("stock.profile_light");
        let dark = self.t("stock.profile_dark");
        let bad_time = self.t("stock.bad_time");
        let accent = self.accent;
        let dirty = self.settings.look_draft;
        let active = self.settings.look_base.clone();
        let mut at_changes: Vec<(String, Option<String>)> = Vec::new();
        stock_grid(ui, 240.0, 10.0, &profile_names, |ui, name, width| {
            let Some(profile) = self.settings.look_profiles.get(name) else {
                return;
            };
            let theme = if profile.theme == "dark" {
                dark.clone()
            } else {
                light.clone()
            };
            let summary = format!("{theme}  ·  {}", profile.items());
            // Активна только сохранённая тема; временная показана отдельно.
            let current = !dirty && active.as_deref() == Some(name.as_str());
            let at_text = profile.at.clone().unwrap_or_default();
            let name_owned = name.clone();
            let overwrite_name = name.clone();
            let at_owned = at_text.clone();
            let display = self.look_profile_title(name);
            let builtin = crate::settings::is_base_profile(name);
            stock_card(
                ui,
                width,
                156.0,
                |ui| {
                    ui.horizontal(|ui| {
                        ui.add(egui::Label::new(egui::RichText::new(&display).strong()).truncate());
                        if builtin {
                            ui.label(
                                egui::RichText::new(self.t("stock.base_mark"))
                                    .small()
                                    .weak(),
                            );
                        }
                        if current {
                            ui.label(egui::RichText::new(&active_label).small().color(accent));
                        }
                    });
                    ui.label(egui::RichText::new(&summary).small().weak());
                    ui.add_space(2.0);
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(&at_label).small());
                        let mut edit = at_owned;
                        let response = ui.add(
                            egui::TextEdit::singleline(&mut edit)
                                .desired_width(64.0)
                                .hint_text("08:00"),
                        );
                        if response.changed() {
                            let value = edit.trim().to_string();
                            at_changes.push((
                                name_owned.clone(),
                                if value.is_empty() { None } else { Some(value) },
                            ));
                        }
                        if !edit.trim().is_empty() && crate::settings::parse_at(&edit).is_none() {
                            ui.label(egui::RichText::new(&bad_time).small().color(accent));
                        }
                    });
                    if !builtin {
                        ui.horizontal(|ui| {
                            ui.with_layout(
                                egui::Layout::right_to_left(egui::Align::Center),
                                |ui| {
                                    if ui
                                        .add(egui::Button::new(
                                            egui::RichText::new(&overwrite_label).small(),
                                        ))
                                        .clicked()
                                    {
                                        *overwrite_profile = Some(overwrite_name.clone());
                                    }
                                },
                            );
                        });
                    }
                },
                |ui| {
                    if ui.button(&apply_label).clicked() {
                        *apply_profile = Some(name_owned.clone());
                    }
                    // Встроенные базовые профили не удаляются.
                    if !builtin && ui.button(&delete_label).clicked() {
                        *delete_profile = Some(name_owned.clone());
                    }
                },
            );
        });
        if !at_changes.is_empty() {
            for (name, at) in at_changes {
                if let Some(profile) = self.settings.look_profiles.get_mut(&name) {
                    profile.at = at;
                }
            }
            self.settings.save();
        }
    }

    /// Применяет косметику: цвет, фон, тема, шрифт или звук.
    fn apply_cosmetic(&mut self, token: &str, variant: &str, value: &str, ctx: &egui::Context) {
        self.settings.apply_cosmetic(token, variant);
        self.settings.mark_look_draft();
        let is_theme = self
            .engine
            .packs
            .token_any(token)
            .map(|found| found.kind == TokenKind::Theme)
            .unwrap_or(false);
        if is_theme {
            self.settings
                .extra
                .insert("appearance.theme_token".to_string(), token.to_string());
            self.settings.ui.theme = if variant == "dark" { "dark" } else { "light" }.to_string();
        }
        if token.ends_with("color.accent") {
            self.settings
                .extra
                .insert("accent".to_string(), value.to_string());
        }
        if token.ends_with("color.background") {
            self.settings.ui.theme = if variant == "dark" { "dark" } else { "light" }.to_string();
        }
        self.apply_look(ctx);
        self.settings.save();
        let text = format!("{}: {variant}", self.t("col.applied"));
        self.toast(text, ctx);
    }

    /// Применяет базовую светлую или тёмную тему целиком.
    fn apply_base_theme(&mut self, base: &str, ctx: &egui::Context) {
        self.settings.apply_base_theme(base);
        // Выбор базовой темы — это готовая тема, а не временная правка.
        self.settings.look_base = Some(crate::settings::base_profile_key(base).to_string());
        self.settings.look_draft = false;
        self.apply_look(ctx);
        self.settings.save();
        let text = format!("{}: {}", self.t("col.applied"), self.base_label(base));
        self.toast(text, ctx);
    }

    /// Применяет тему целиком: косметика, обои, звук, шрифт и стикеры.
    fn apply_bundle(&mut self, bundle: &Bundle, ctx: &egui::Context) {
        let mut items = Vec::new();
        for item in &bundle.tokens {
            let Some((token, variant)) = item.split_once(':') else {
                continue;
            };
            let value = self
                .engine
                .variant_value(token, variant)
                .unwrap_or_default();
            items.push((token.to_string(), variant.to_string(), value));
        }
        // База, затем элементы темы: недостающее останется от базы.
        self.settings
            .apply_theme_items(bundle.theme_base().as_str(), &items);
        // Готовый набор — основа новой временной темы, пока его не собрали.
        self.settings.look_base = None;
        self.settings.look_draft = true;

        self.apply_look(ctx);
        self.settings.save();
        let text = format!(
            "{}: {}",
            self.t("stock.apply_bundle"),
            bundle.title(&self.settings.ui.locale)
        );
        self.toast(text, ctx);
    }

    /// Собирает новую тему из текущего вида; существующие не перезаписывает.
    fn save_look_profile(&mut self, ctx: &egui::Context) {
        let name = self.look_profile_name.trim().to_string();
        match self.settings.save_look_as(&name) {
            Ok(()) => {
                self.settings.save();
                self.look_profile_name.clear();
                let text = format!("{}: {name}", self.t("stock.profile_saved"));
                self.toast(text, ctx);
            }
            Err(crate::settings::LookSaveError::Empty) => {
                self.set_status(self.t("stock.profile_need_name"));
            }
            Err(crate::settings::LookSaveError::Exists) => {
                self.set_status(self.t("stock.profile_exists"));
            }
            Err(crate::settings::LookSaveError::Base) => {
                self.set_status(self.t("stock.profile_base"));
            }
            Err(crate::settings::LookSaveError::Missing) => {
                self.set_status(self.t("stock.profile_missing"));
            }
        }
    }

    /// Применяет сохранённый профиль целиком.
    fn apply_look_profile(&mut self, name: &str, ctx: &egui::Context) {
        if !self.settings.apply_saved_look(name) {
            self.set_status(self.t("stock.profile_missing"));
            return;
        }
        self.apply_look(ctx);
        self.settings.save();
        let text = format!(
            "{}: {}",
            self.t("stock.profile_applied"),
            self.look_profile_title(name)
        );
        self.toast(text, ctx);
    }

    /// Перезаписывает пользовательскую тему текущим временным видом.
    pub(crate) fn overwrite_look_profile(&mut self, name: &str, ctx: &egui::Context) {
        match self.settings.overwrite_look(name) {
            Ok(()) => {
                self.apply_look(ctx);
                self.settings.save();
                let text = format!(
                    "{}: {}",
                    self.t("stock.overwrite_done"),
                    self.look_profile_title(name)
                );
                self.toast(text, ctx);
            }
            Err(crate::settings::LookSaveError::Base) => {
                self.set_status(self.t("stock.profile_base"));
            }
            Err(_) => self.set_status(self.t("stock.profile_missing")),
        }
    }

    fn delete_look_profile(&mut self, name: &str, ctx: &egui::Context) {
        if self.settings.look_profiles.remove(name).is_some() {
            // Удалили активную тему: текущий вид остаётся временной темой.
            if self.settings.look_base.as_deref() == Some(name) {
                self.settings.look_base = None;
                self.settings.look_draft = true;
            }
            self.settings.save();
            let text = format!("{}: {name}", self.t("stock.profile_deleted"));
            self.toast(text, ctx);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::COSMETIC_GROUPS;
    use crate::i18n::I18n;

    #[test]
    fn cosmetic_group_titles_are_localized() {
        for locale in ["ru", "en"] {
            let i18n = I18n::load(locale);
            for group in COSMETIC_GROUPS.iter().copied().chain(["theme"]) {
                let key = format!("stock.cosmetic_{group}");
                assert!(i18n.has(locale, &key), "{locale}: нет перевода {key}");
                assert_ne!(i18n.t(&key), key, "{locale}: {key} без перевода");
            }
        }
    }
}
