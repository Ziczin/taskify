//! Экран «Ачивки»: прогресс по условиям паков.
//!
//! Список фильтруется по паку и по получению, сортируется по имени или по
//! проценту выполнения — строки собираются заранее, чтобы фильтр не мешал
//! отрисовке.

use eframe::egui;

use crate::app::{AchSort, AchState, TaskifyApp};

struct Row {
    pack_id: String,
    pack_name: String,
    title: String,
    description: String,
    unlocked: bool,
    current: i64,
    target: i64,
    fraction: f32,
    reward: String,
}

impl TaskifyApp {
    pub(crate) fn screen_achievements(&mut self, ui: &mut egui::Ui) {
        ui.heading(self.t("nav.achievements"));
        ui.add_space(8.0);

        let locale = self.settings.ui.locale.clone();
        let mut rows: Vec<Row> = Vec::new();
        let mut pack_names: Vec<(String, String)> = Vec::new();

        let mut pack_ids: Vec<String> = self.engine.packs.packs.keys().cloned().collect();
        pack_ids.sort();
        for pack_id in pack_ids {
            let Some(pack) = self.engine.packs.packs.get(&pack_id) else {
                continue;
            };
            if !pack.enabled {
                continue;
            }
            let pack_name = pack
                .manifest
                .as_ref()
                .map(|manifest| taskify_core::task::tr_or(&manifest.name, &locale, &pack.id))
                .unwrap_or_else(|| pack.id.clone());
            pack_names.push((pack.id.clone(), pack_name.clone()));

            for achievement in pack.achievements.values() {
                let unlocked = self
                    .engine
                    .store
                    .state
                    .achievements
                    .contains(&achievement.id);
                let (current, target) = self.engine.achievement_progress(&achievement.condition);
                let fraction = if target > 0 {
                    (current as f32 / target as f32).clamp(0.0, 1.0)
                } else {
                    1.0
                };
                let title = taskify_core::task::tr_or(&achievement.title, &locale, &achievement.id);
                let description =
                    taskify_core::task::tr(&achievement.description, &locale).unwrap_or_default();
                let mut reward = format!(
                    "+{} {}  +{} {}",
                    achievement.reward.xp,
                    self.t("common.xp"),
                    achievement.reward.coins,
                    self.t("common.coins_short")
                );
                if let Some(sticker) = achievement.sticker.as_deref() {
                    let name = crate::stickers::find(&self.engine.packs, sticker)
                        .map(|item| item.title(&locale))
                        .unwrap_or_else(|| sticker.to_string());
                    reward.push_str(&format!("  ·  {}: {name}", self.t("ach.sticker")));
                }
                rows.push(Row {
                    pack_id: pack.id.clone(),
                    pack_name: pack_name.clone(),
                    title,
                    description,
                    unlocked,
                    current,
                    target,
                    fraction,
                    reward,
                });
            }
        }

        if rows.is_empty() {
            ui.label(self.t("packs.none"));
            return;
        }

        let unlocked_count = rows.iter().filter(|row| row.unlocked).count();
        let total_count = rows.len();
        let mut search_response: Option<egui::Response> = None;
        ui.horizontal(|ui| {
            ui.spacing_mut().interact_size.y = 26.0;
            ui.spacing_mut().button_padding = egui::vec2(10.0, 3.0);
            ui.label(self.t("ach.search"));
            let hint = self.t("ach.search_hint");
            let response = ui.add(
                egui::TextEdit::singleline(&mut self.ach_query)
                    .desired_width(280.0)
                    .hint_text(hint)
                    .margin(crate::search::SEARCH_FIELD_MARGIN),
            );
            if response.changed() {
                self.ach_suggest = 0;
            }
            if self.focus_search {
                response.request_focus();
                self.focus_search = false;
            }
            search_response = Some(response);
        });
        if let Some(response) = search_response.as_ref() {
            let items = rows.iter().map(|row| crate::search::Suggestion {
                value: row.title.clone(),
                label: row.title.clone(),
                meta: row.pack_name.clone(),
            });
            let suggestions = crate::search::suggestions(&self.ach_query, items);
            if let Some(value) = crate::search::show_suggestions(
                ui.ctx(),
                response,
                &suggestions,
                &mut self.ach_suggest,
                "ach_search_suggestions",
            ) {
                self.ach_query = value;
                self.ach_suggest = 0;
                response.surrender_focus();
            }
        }
        ui.add_space(6.0);
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().interact_size.y = 26.0;
            ui.spacing_mut().button_padding = egui::vec2(10.0, 3.0);

            let mut filter_pack = self.ach_pack.clone();
            ui.label(self.t("ach.pack"));
            let pack_text = filter_pack
                .as_ref()
                .and_then(|id| {
                    pack_names
                        .iter()
                        .find(|(pid, _)| pid == id)
                        .map(|(_, name)| name.clone())
                })
                .unwrap_or_else(|| self.t("ach.all_packs"));
            egui::ComboBox::from_id_salt("ach_pack_filter")
                .selected_text(pack_text)
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_label(filter_pack.is_none(), self.t("ach.all_packs"))
                        .clicked()
                    {
                        filter_pack = None;
                    }
                    for (id, name) in &pack_names {
                        if ui
                            .selectable_label(filter_pack.as_deref() == Some(id.as_str()), name)
                            .clicked()
                        {
                            filter_pack = Some(id.clone());
                        }
                    }
                });
            self.ach_pack = filter_pack;

            ui.label(self.t("ach.state"));
            egui::ComboBox::from_id_salt("ach_state_filter")
                .selected_text(self.t(self.ach_state.key()))
                .show_ui(ui, |ui| {
                    for state in AchState::ALL {
                        if ui
                            .selectable_label(self.ach_state == state, self.t(state.key()))
                            .clicked()
                        {
                            self.ach_state = state;
                        }
                    }
                });

            ui.label(self.t("ach.sort"));
            egui::ComboBox::from_id_salt("ach_sort")
                .selected_text(self.t(self.ach_sort.key()))
                .show_ui(ui, |ui| {
                    for sort in AchSort::ALL {
                        if ui
                            .selectable_label(self.ach_sort == sort, self.t(sort.key()))
                            .clicked()
                        {
                            self.ach_sort = sort;
                        }
                    }
                });
        });
        ui.add_space(8.0);

        rows.retain(|row| {
            let pack_ok = self
                .ach_pack
                .as_ref()
                .map(|id| &row.pack_id == id)
                .unwrap_or(true);
            let query = self.ach_query.trim().to_lowercase();
            let search_ok = query.is_empty()
                || format!("{} {} {}", row.title, row.description, row.pack_name)
                    .to_lowercase()
                    .contains(&query);
            let state_ok = match self.ach_state {
                AchState::All => true,
                AchState::Unlocked => row.unlocked,
                AchState::Locked => !row.unlocked,
            };
            pack_ok && state_ok && search_ok
        });
        match self.ach_sort {
            AchSort::Name => rows.sort_by(|a, b| {
                a.title
                    .to_lowercase()
                    .cmp(&b.title.to_lowercase())
                    .then_with(|| a.pack_name.cmp(&b.pack_name))
            }),
            AchSort::Percent => rows.sort_by(|a, b| {
                b.fraction
                    .partial_cmp(&a.fraction)
                    .unwrap_or(std::cmp::Ordering::Equal)
                    .then_with(|| a.title.to_lowercase().cmp(&b.title.to_lowercase()))
            }),
        }

        pack_names.sort_by_cached_key(|(_, name)| name.to_lowercase());

        if rows.is_empty() {
            ui.label(self.t("ach.none_match"));
            return;
        }

        for row in &rows {
            let unlocked = row.unlocked;
            egui::Frame::group(ui.style())
                .fill(if unlocked {
                    self.palette().success_bg
                } else {
                    ui.visuals().extreme_bg_color
                })
                .inner_margin(egui::Margin::symmetric(12, 8))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        let star = if unlocked {
                            "ui.star_on"
                        } else {
                            "ui.star_off"
                        };
                        let tint = if unlocked {
                            self.palette().coin
                        } else {
                            ui.visuals().weak_text_color()
                        };
                        self.icons.show_tint(ui, star, 22.0, tint);
                        ui.vertical(|ui| {
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new(&row.title).strong());
                                ui.label(
                                    egui::RichText::new(if unlocked {
                                        self.t("ach.unlocked")
                                    } else {
                                        self.t("ach.locked")
                                    })
                                    .small()
                                    .color(if unlocked {
                                        self.palette().success
                                    } else {
                                        ui.visuals().weak_text_color()
                                    }),
                                );
                                ui.label(egui::RichText::new(&row.pack_name).small().weak());
                            });
                            if !row.description.is_empty() {
                                ui.label(egui::RichText::new(&row.description).small());
                            }
                            ui.add(
                                egui::ProgressBar::new(row.fraction)
                                    .desired_width(260.0)
                                    .text(format!(
                                        "{}/{}",
                                        row.current.min(row.target),
                                        row.target
                                    )),
                            );
                        });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            ui.label(egui::RichText::new(&row.reward).small());
                        });
                    });
                });
            ui.add_space(6.0);
        }

        ui.add_space(6.0);
        ui.label(format!(
            "{} · {}/{}",
            self.t("ach.found"),
            unlocked_count,
            total_count
        ));
    }
}
