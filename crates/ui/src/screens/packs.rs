//! Экран «Паки»: список, установка, валидатор и содержимое пака по вкладкам.
//!
//! Клик по «Открыть» проваливает внутрь пака: задания, комбо, ачивки и
//! служебная информация. Уровень пака считается по опыту его заданий:
//! сколько раз задания пака выполняли, умноженное на их награду.

use eframe::egui;
use taskify_packs::{format_issue, Pack, Severity, TokenKind, Unlock};

use crate::app::{hex_color, PackTab, PacksFilter, TaskifyApp};

impl TaskifyApp {
    pub(crate) fn screen_packs(&mut self, ui: &mut egui::Ui) {
        if let Some(id) = self.pack_detail.clone() {
            if self.engine.packs.packs.contains_key(&id) {
                self.screen_pack_detail(ui, &id);
                return;
            }
            self.pack_detail = None;
        }
        self.pack_list(ui);
    }

    /// Опыт пака: сумма опыта за все выполнения его заданий.
    fn pack_xp(&self, pack: &Pack) -> i64 {
        pack.tasks
            .values()
            .map(|task| {
                let done = self
                    .engine
                    .store
                    .state
                    .tasks
                    .get(&task.id)
                    .map(|state| state.completions_total)
                    .unwrap_or(0);
                (done as i64).saturating_mul(task.reward.xp)
            })
            .sum()
    }

    /// Уровень пака и прогресс внутри уровня.
    pub(crate) fn pack_level(&self, pack: &Pack) -> (u32, u64, u64, i64) {
        let xp = self.pack_xp(pack);
        let level = taskify_core::level_for(xp);
        let (inside, span) = taskify_core::level_progress(xp);
        (level, inside, span, xp)
    }

    fn pack_display_name(&self, pack: &Pack, locale: &str) -> String {
        pack.manifest
            .as_ref()
            .map(|manifest| taskify_core::task::tr_or(&manifest.name, locale, &pack.id))
            .unwrap_or_else(|| pack.id.clone())
    }

    fn pack_list(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading(self.t("nav.packs"));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button(self.t("packs.install_zip")).clicked() {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("zip", &["zip"])
                        .pick_file()
                    {
                        self.install_pack(&path, ui.ctx());
                    }
                }
                if ui.button(self.t("common.refresh")).clicked() {
                    self.engine.reload_packs();
                    self.engine.apply_disabled_packs();
                }
            });
        });
        ui.add_space(8.0);

        let locale = self.settings.ui.locale.clone();
        let packs: Vec<Pack> = self.engine.packs.packs.values().cloned().collect();
        if packs.is_empty() {
            ui.label(self.t("packs.none"));
            return;
        }

        let disabled_count = packs
            .iter()
            .filter(|pack| self.engine.is_pack_disabled(&pack.id))
            .count();
        let enabled_count = packs.len() - disabled_count;

        // Поиск и фильтр по состоянию — в одной строке.
        let mut filter = self.packs_filter;
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().interact_size.y = 26.0;
            ui.spacing_mut().button_padding = egui::vec2(10.0, 3.0);
            let hint = self.t("packs.search_hint");
            ui.add_sized(
                [260.0, 26.0],
                egui::TextEdit::singleline(&mut self.packs_query).hint_text(hint),
            );
            ui.add_space(8.0);
            for value in PacksFilter::ALL {
                let count = match value {
                    PacksFilter::All => packs.len(),
                    PacksFilter::Enabled => enabled_count,
                    PacksFilter::Disabled => disabled_count,
                };
                let label = format!("{} {count}", self.t(value.key()));
                if ui.selectable_label(filter == value, label).clicked() {
                    filter = value;
                }
            }
        });
        self.packs_filter = filter;
        ui.add_space(6.0);

        let query = self.packs_query.trim().to_lowercase();
        let mut visible: Vec<&Pack> = packs
            .iter()
            .filter(|pack| match filter {
                PacksFilter::All => true,
                PacksFilter::Enabled => !self.engine.is_pack_disabled(&pack.id),
                PacksFilter::Disabled => self.engine.is_pack_disabled(&pack.id),
            })
            .filter(|pack| {
                if query.is_empty() {
                    return true;
                }
                let name = self.pack_display_name(pack, &locale);
                let description = pack
                    .manifest
                    .as_ref()
                    .map(|manifest| taskify_core::task::tr_or(&manifest.description, &locale, ""))
                    .unwrap_or_default();
                name.to_lowercase().contains(&query)
                    || pack.id.to_lowercase().contains(&query)
                    || description.to_lowercase().contains(&query)
            })
            .collect();
        // Обучающие паки (default_enabled) — первыми, дальше по названию.
        visible.sort_by_key(|pack| {
            let pinned = pack
                .manifest
                .as_ref()
                .map(|manifest| !manifest.default_enabled)
                .unwrap_or(true);
            (pinned, self.pack_display_name(pack, &locale).to_lowercase())
        });
        if visible.is_empty() {
            ui.label(self.t("packs.no_match"));
            return;
        }

        let mut toggle: Option<(String, bool)> = None;
        let mut open: Option<String> = None;
        for pack in visible {
            let name = self.pack_display_name(pack, &locale);
            let disabled = self.engine.is_pack_disabled(&pack.id);
            let (level, inside, span, xp) = self.pack_level(pack);

            egui::Frame::group(ui.style())
                .inner_margin(egui::Margin::symmetric(12, 8))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    // Кнопки пака — в верхнем правом углу карточки.
                    ui.horizontal_top(|ui| {
                        ui.vertical(|ui| {
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new(&name).strong().size(16.0));
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{} {}",
                                        self.t("packs.version"),
                                        pack.version
                                    ))
                                    .small()
                                    .weak(),
                                );
                                if pack
                                    .manifest
                                    .as_ref()
                                    .map(|manifest| manifest.official)
                                    .unwrap_or(false)
                                {
                                    ui.label(
                                        egui::RichText::new(self.t("packs.official"))
                                            .small()
                                            .color(self.accent),
                                    );
                                }
                                if disabled {
                                    ui.label(
                                        egui::RichText::new(self.t("packs.disabled"))
                                            .small()
                                            .color(self.palette().danger),
                                    );
                                }
                                ui.label(
                                    egui::RichText::new(format!(
                                        "{} {}",
                                        self.t("packs.level"),
                                        level
                                    ))
                                    .small()
                                    .strong()
                                    .color(self.accent),
                                )
                                .on_hover_text(format!(
                                    "{}: {xp} · {inside}/{span}",
                                    self.t("packs.pack_xp")
                                ));
                            });
                            ui.label(
                                egui::RichText::new(format!(
                                    "{} {} · {} {} · {} {}",
                                    pack.tasks.len(),
                                    self.t("packs.tasks"),
                                    pack.combos.len(),
                                    self.t("packs.combos"),
                                    pack.achievements.len(),
                                    self.t("packs.achievements")
                                ))
                                .small()
                                .weak(),
                            );
                            // Подробное описание пака: видно сразу, без раскрытия.
                            if let Some(manifest) = &pack.manifest {
                                let description =
                                    taskify_core::task::tr_or(&manifest.description, &locale, "");
                                if !description.is_empty() {
                                    ui.add(
                                        egui::Label::new(
                                            egui::RichText::new(description).small().weak(),
                                        )
                                        .wrap(),
                                    );
                                }
                            }
                        });
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::TOP), |ui| {
                            let label = if disabled {
                                self.t("packs.enable")
                            } else {
                                self.t("packs.disable")
                            };
                            if ui.button(label).clicked() {
                                toggle = Some((pack.id.clone(), disabled));
                            }
                            if ui.button(self.t("packs.open")).clicked() {
                                open = Some(pack.id.clone());
                            }
                        });
                    });
                });
            ui.add_space(6.0);
        }

        if let Some(id) = open {
            self.pack_tab = self
                .engine
                .packs
                .packs
                .get(&id)
                .and_then(|pack| PackTab::for_pack(pack).first().copied())
                .unwrap_or(PackTab::Info);
            self.pack_detail = Some(id);
        }

        if let Some((id, enable)) = toggle {
            match self.engine.set_pack_enabled(&id, enable) {
                Ok(()) => {
                    self.engine.apply_disabled_packs();
                    let text = if enable {
                        self.t("status.pack_on")
                    } else {
                        self.t("status.pack_off")
                    };
                    self.status.clear();
                    self.toast(text, ui.ctx());
                }
                Err(error) => self.set_status(format!("{error:#}")),
            }
        }

        ui.add_space(10.0);
        ui.separator();
        let issues = taskify_packs::validate_set(&self.engine.packs);
        if issues.is_empty() {
            ui.horizontal(|ui| {
                self.icons
                    .show_tint(ui, "ui.check", 16.0, self.palette().success);
                ui.label(
                    egui::RichText::new("OK")
                        .color(self.palette().success)
                        .strong(),
                );
            });
            return;
        }
        ui.label(egui::RichText::new(self.t("packs.issues")).strong());
        let errors = issues
            .iter()
            .filter(|issue| issue.severity == Severity::Error)
            .count();
        ui.label(format!(
            "{}: {} · {}: {}",
            self.t("packs.issues"),
            issues.len(),
            "ошибок",
            errors
        ));
        for issue in issues.iter().take(20) {
            let color = match issue.severity {
                Severity::Error => self.palette().danger,
                Severity::Warning => self.palette().warning,
            };
            ui.label(
                egui::RichText::new(format_issue(issue))
                    .small()
                    .color(color),
            );
        }
    }

    /// Внутренность пака: заголовок с уровнем и вкладки.
    fn screen_pack_detail(&mut self, ui: &mut egui::Ui, id: &str) {
        let Some(pack) = self.engine.packs.packs.get(id).cloned() else {
            self.pack_detail = None;
            return;
        };
        let locale = self.settings.ui.locale.clone();
        let name = self.pack_display_name(&pack, &locale);
        let disabled = self.engine.is_pack_disabled(&pack.id);
        let (level, inside, span, xp) = self.pack_level(&pack);

        ui.horizontal(|ui| {
            if ui.button(self.t("packs.back")).clicked() {
                self.pack_detail = None;
            }
            ui.heading(&name);
            if pack
                .manifest
                .as_ref()
                .map(|manifest| manifest.official)
                .unwrap_or(false)
            {
                ui.label(
                    egui::RichText::new(self.t("packs.official"))
                        .small()
                        .color(self.accent),
                );
            }
            if disabled {
                ui.label(
                    egui::RichText::new(self.t("packs.disabled"))
                        .small()
                        .color(self.palette().danger),
                );
            }
        });
        ui.add_space(4.0);

        ui.horizontal(|ui| {
            ui.label(
                egui::RichText::new(format!("{} {}", self.t("packs.level"), level))
                    .size(18.0)
                    .strong()
                    .color(self.accent),
            );
            let fraction = if span > 0 {
                inside as f32 / span as f32
            } else {
                1.0
            };
            ui.add(
                egui::ProgressBar::new(fraction.clamp(0.0, 1.0))
                    .desired_width(240.0)
                    .text(format!("{inside} / {span}")),
            )
            .on_hover_text(format!("{}: {xp}", self.t("packs.pack_xp")));
            ui.label(
                egui::RichText::new(format!(
                    "{} · {} {}",
                    pack.id,
                    self.t("packs.version"),
                    pack.version
                ))
                .small()
                .weak(),
            );
        });
        if let Some(manifest) = &pack.manifest {
            let description = taskify_core::task::tr_or(&manifest.description, &locale, "");
            if !description.is_empty() {
                ui.add(egui::Label::new(egui::RichText::new(description).small().weak()).wrap());
            }
        }
        ui.add_space(8.0);

        let tabs = PackTab::for_pack(&pack);
        let mut tab = if tabs.contains(&self.pack_tab) {
            self.pack_tab
        } else {
            tabs.first().copied().unwrap_or(PackTab::Info)
        };
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().item_spacing.x = 6.0;
            for value in tabs {
                let count = value.count(&pack);
                let label = if count > 0 {
                    format!("{} {count}", self.t(value.key()))
                } else {
                    self.t(value.key())
                };
                if ui.selectable_label(tab == value, label).clicked() {
                    tab = value;
                }
            }
        });
        self.pack_tab = tab;
        ui.add_space(8.0);

        match tab {
            PackTab::Tasks => self.pack_tasks_tab(ui, &pack, &locale),
            PackTab::Combos => self.pack_combos_tab(ui, &pack, &locale),
            PackTab::Achievements => self.pack_achievements_tab(ui, &pack, &locale),
            PackTab::Categories => self.pack_categories_tab(ui, &pack, &locale),
            PackTab::Tags => self.pack_tags_tab(ui, &pack, &locale),
            PackTab::Types => self.pack_types_tab(ui, &pack, &locale),
            PackTab::Icons => self.pack_icons_tab(ui, &pack, &locale),
            PackTab::Palette => self.pack_palette_tab(ui, &pack),
            PackTab::Tokens => self.pack_tokens_tab(ui, &pack, &locale),
            PackTab::Bundles => self.pack_bundles_tab(ui, &pack, &locale),
            PackTab::Stickers => self.pack_stickers_tab(ui, &pack, &locale),
            PackTab::Metrics => self.pack_metrics_tab(ui, &pack),
            PackTab::Info => self.pack_info_tab(ui, &pack, &locale),
        }
    }

    fn pack_tasks_tab(&mut self, ui: &mut egui::Ui, pack: &Pack, locale: &str) {
        if pack.tasks.is_empty() {
            ui.label(self.t("packs.empty"));
            return;
        }
        let mut tasks: Vec<_> = pack.tasks.values().collect();
        tasks.sort_by_key(|task| taskify_core::task::tr_or(&task.title, locale, &task.id));
        for task in tasks {
            let title = taskify_core::task::tr_or(&task.title, locale, &task.id);
            let description = taskify_core::task::tr(&task.description, locale).unwrap_or_default();
            let done = self
                .engine
                .store
                .state
                .tasks
                .get(&task.id)
                .map(|state| state.completions_total)
                .unwrap_or(0);
            let category = task
                .category
                .as_ref()
                .and_then(|id| pack.categories.get(id))
                .map(|category| taskify_core::task::tr_or(&category.title, locale, &category.id));
            let tags: Vec<String> = task
                .tags
                .iter()
                .filter_map(|id| pack.tags.get(id))
                .map(|tag| taskify_core::task::tr_or(&tag.title, locale, &tag.id))
                .collect();

            egui::Frame::group(ui.style())
                .inner_margin(egui::Margin::symmetric(12, 8))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        if let Some(icon) = task.icon.as_deref() {
                            self.icons.show(ui, icon, 18.0);
                        }
                        ui.label(egui::RichText::new(&title).strong());
                        ui.label(
                            egui::RichText::new(self.t(&format!("today.{}", task.kind.key())))
                                .small()
                                .weak(),
                        );
                        if let Some(category) = &category {
                            ui.label(egui::RichText::new(category).small().color(self.accent));
                        }
                        if !tags.is_empty() {
                            ui.label(egui::RichText::new(tags.join(" · ")).small().weak());
                        }
                    });
                    if !description.is_empty() {
                        ui.label(egui::RichText::new(&description).small().weak());
                    }
                    let mut stats = format!(
                        "+{} {} · +{} {} · {}: {}",
                        task.reward.xp,
                        self.t("common.xp"),
                        task.reward.coins,
                        self.t("common.coins_short"),
                        self.t("packs.done_times"),
                        done
                    );
                    if let Some(schedule) = schedule_text(task, locale) {
                        stats.push_str(&format!(" · {schedule}"));
                    }
                    if let Some(gate) = pack.task_levels.get(&task.id) {
                        if let Some(from) = gate.from {
                            stats.push_str(&format!(
                                " · {} {}",
                                self.t("packs.appears_from"),
                                from
                            ));
                        }
                        if let Some(to) = gate.to {
                            stats.push_str(&format!(" · {} {}", self.t("packs.disappears_at"), to));
                        }
                    }
                    ui.label(egui::RichText::new(stats).small().weak());
                });
            ui.add_space(4.0);
        }
    }

    fn pack_combos_tab(&mut self, ui: &mut egui::Ui, pack: &Pack, locale: &str) {
        if pack.combos.is_empty() {
            ui.label(self.t("packs.empty"));
            return;
        }
        let mut combos: Vec<_> = pack.combos.values().collect();
        combos.sort_by_key(|combo| taskify_core::task::tr_or(&combo.title, locale, &combo.id));
        for combo in combos {
            let title = taskify_core::task::tr_or(&combo.title, locale, &combo.id);
            let description =
                taskify_core::task::tr(&combo.description, locale).unwrap_or_default();
            let names: Vec<String> = combo
                .members
                .iter()
                .map(|id| {
                    pack.tasks
                        .get(id)
                        .map(|task| taskify_core::task::tr_or(&task.title, locale, id))
                        .unwrap_or_else(|| id.clone())
                })
                .collect();
            egui::Frame::group(ui.style())
                .inner_margin(egui::Margin::symmetric(12, 8))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(&title).strong());
                        ui.label(
                            egui::RichText::new(format!(
                                "+{} {} · +{} {}",
                                combo.bonus.xp,
                                self.t("common.xp"),
                                combo.bonus.coins,
                                self.t("common.coins_short")
                            ))
                            .small()
                            .weak(),
                        );
                    });
                    if !description.is_empty() {
                        ui.label(egui::RichText::new(&description).small().weak());
                    }
                    if !names.is_empty() {
                        ui.label(egui::RichText::new(names.join(" · ")).small().weak());
                    }
                });
            ui.add_space(4.0);
        }
    }

    fn pack_achievements_tab(&mut self, ui: &mut egui::Ui, pack: &Pack, locale: &str) {
        if pack.achievements.is_empty() {
            ui.label(self.t("packs.empty"));
            return;
        }
        let mut achievements: Vec<_> = pack.achievements.values().collect();
        achievements.sort_by_key(|achievement| {
            taskify_core::task::tr_or(&achievement.title, locale, &achievement.id)
        });
        for achievement in achievements {
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
            let title = taskify_core::task::tr_or(&achievement.title, locale, &achievement.id);
            let description =
                taskify_core::task::tr(&achievement.description, locale).unwrap_or_default();
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
                        self.icons.show_tint(
                            ui,
                            if unlocked {
                                "ui.star_on"
                            } else {
                                "ui.star_off"
                            },
                            18.0,
                            if unlocked {
                                self.palette().coin
                            } else {
                                ui.visuals().weak_text_color()
                            },
                        );
                        ui.label(egui::RichText::new(&title).strong());
                        ui.label(
                            egui::RichText::new(if unlocked {
                                self.t("ach.unlocked")
                            } else {
                                self.t("ach.locked")
                            })
                            .small()
                            .weak(),
                        );
                    });
                    if !description.is_empty() {
                        ui.label(egui::RichText::new(&description).small().weak());
                    }
                    ui.add(
                        egui::ProgressBar::new(fraction)
                            .desired_width(240.0)
                            .text(format!("{}/{}", current.min(target), target)),
                    );
                });
            ui.add_space(4.0);
        }
    }

    fn pack_categories_tab(&mut self, ui: &mut egui::Ui, pack: &Pack, locale: &str) {
        if pack.categories.is_empty() {
            ui.label(self.t("packs.empty"));
            return;
        }
        let mut categories: Vec<_> = pack.categories.values().collect();
        categories.sort_by_key(|category| {
            taskify_core::task::tr_or(&category.title, locale, &category.id)
        });
        for category in categories {
            let title = taskify_core::task::tr_or(&category.title, locale, &category.id);
            let color = category
                .color
                .as_deref()
                .and_then(|value| pack_color(pack, value));
            egui::Frame::group(ui.style())
                .inner_margin(egui::Margin::symmetric(12, 8))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        if let Some(icon) = category.icon.as_deref() {
                            self.show_pack_icon(ui, pack, icon, 20.0);
                        }
                        if let Some(color) = color {
                            color_dot(ui, color);
                        }
                        ui.label(egui::RichText::new(&title).strong());
                        ui.label(egui::RichText::new(&category.id).small().weak());
                    });
                    if !category.synonyms.is_empty() {
                        ui.label(
                            egui::RichText::new(category.synonyms.join(" · "))
                                .small()
                                .weak(),
                        );
                    }
                });
            ui.add_space(4.0);
        }
    }

    fn pack_tags_tab(&mut self, ui: &mut egui::Ui, pack: &Pack, locale: &str) {
        if pack.tags.is_empty() {
            ui.label(self.t("packs.empty"));
            return;
        }
        let mut tags: Vec<_> = pack.tags.values().collect();
        tags.sort_by_key(|tag| taskify_core::task::tr_or(&tag.title, locale, &tag.id));
        for tag in tags {
            let title = taskify_core::task::tr_or(&tag.title, locale, &tag.id);
            egui::Frame::group(ui.style())
                .inner_margin(egui::Margin::symmetric(12, 8))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(&title).strong());
                        ui.label(egui::RichText::new(&tag.id).small().weak());
                    });
                    if !tag.synonyms.is_empty() {
                        ui.label(egui::RichText::new(tag.synonyms.join(" · ")).small().weak());
                    }
                });
            ui.add_space(4.0);
        }
    }

    fn pack_types_tab(&mut self, ui: &mut egui::Ui, pack: &Pack, locale: &str) {
        if pack.types.is_empty() {
            ui.label(self.t("packs.empty"));
            return;
        }
        let mut types: Vec<_> = pack.types.values().collect();
        types.sort_by_key(|kind| taskify_core::task::tr_or(&kind.title, locale, &kind.id));
        for kind in types {
            let title = taskify_core::task::tr_or(&kind.title, locale, &kind.id);
            let color = kind
                .color
                .as_deref()
                .and_then(|value| pack_color(pack, value));
            egui::Frame::group(ui.style())
                .inner_margin(egui::Margin::symmetric(12, 8))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        if let Some(icon) = kind.icon.as_deref() {
                            self.show_pack_icon(ui, pack, icon, 20.0);
                        }
                        if let Some(color) = color {
                            color_dot(ui, color);
                        }
                        ui.label(egui::RichText::new(&title).strong());
                        ui.label(egui::RichText::new(&kind.id).small().weak());
                    });
                    ui.label(
                        egui::RichText::new(format!(
                            "+{} {} · +{} {}",
                            kind.default_reward.xp,
                            self.t("common.xp"),
                            kind.default_reward.coins,
                            self.t("common.coins_short")
                        ))
                        .small()
                        .weak(),
                    );
                });
            ui.add_space(4.0);
        }
    }

    fn pack_icons_tab(&mut self, ui: &mut egui::Ui, pack: &Pack, locale: &str) {
        if pack.icons.is_empty() {
            ui.label(self.t("packs.empty"));
            return;
        }
        let mut icons: Vec<_> = pack.icons.values().collect();
        icons.sort_by_key(|icon| taskify_core::task::tr_or(&icon.title, locale, &icon.id));
        for icon in icons {
            let title = taskify_core::task::tr_or(&icon.title, locale, &icon.id);
            egui::Frame::group(ui.style())
                .inner_margin(egui::Margin::symmetric(12, 8))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        if !icon.image.is_empty() {
                            self.icons.show(ui, &icon.image, 24.0);
                        }
                        ui.label(egui::RichText::new(&title).strong());
                        ui.label(egui::RichText::new(&icon.id).small().weak());
                        ui.label(egui::RichText::new(&icon.image).small().weak());
                    });
                });
            ui.add_space(4.0);
        }
    }

    fn pack_palette_tab(&mut self, ui: &mut egui::Ui, pack: &Pack) {
        if pack.palette.is_empty() {
            ui.label(self.t("packs.empty"));
            return;
        }
        let mut colors: Vec<_> = pack.palette.values().collect();
        colors.sort_by_key(|color| &color.id);
        for color in colors {
            egui::Frame::group(ui.style())
                .inner_margin(egui::Margin::symmetric(12, 8))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(&color.id).strong());
                        if let Some(light) = hex_color(&color.light) {
                            color_dot(ui, light);
                        }
                        ui.label(egui::RichText::new(&color.light).small().weak());
                        if let Some(dark) = hex_color(&color.dark) {
                            color_dot(ui, dark);
                        }
                        ui.label(egui::RichText::new(&color.dark).small().weak());
                    });
                });
            ui.add_space(4.0);
        }
    }

    fn pack_tokens_tab(&mut self, ui: &mut egui::Ui, pack: &Pack, locale: &str) {
        if pack.tokens.is_empty() {
            ui.label(self.t("packs.empty"));
            return;
        }
        let mut tokens: Vec<_> = pack.tokens.values().collect();
        tokens.sort_by_key(|token| taskify_core::task::tr_or(&token.title, locale, &token.id));
        for token in tokens {
            let title = taskify_core::task::tr_or(&token.title, locale, &token.id);
            egui::Frame::group(ui.style())
                .inner_margin(egui::Margin::symmetric(12, 8))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(&title).strong());
                        ui.label(
                            egui::RichText::new(self.pack_token_kind_label(token.kind))
                                .small()
                                .weak(),
                        );
                        ui.label(egui::RichText::new(&token.id).small().weak());
                    });
                    for variant in &token.variants {
                        let variant_title =
                            taskify_core::task::tr_or(&variant.title, locale, &variant.id);
                        let swatch = pack_color(pack, &variant.value);
                        ui.horizontal(|ui| {
                            if let Some(color) = swatch {
                                color_dot(ui, color);
                            }
                            ui.label(egui::RichText::new(variant_title).small());
                            if swatch.is_none() {
                                ui.label(egui::RichText::new(&variant.value).small().weak());
                            }
                            ui.label(
                                egui::RichText::new(self.pack_unlock_note(&variant.unlock))
                                    .small()
                                    .color(self.accent),
                            );
                        });
                    }
                });
            ui.add_space(4.0);
        }
    }

    fn pack_bundles_tab(&mut self, ui: &mut egui::Ui, pack: &Pack, locale: &str) {
        if pack.bundles.is_empty() {
            ui.label(self.t("packs.empty"));
            return;
        }
        let mut bundles: Vec<_> = pack.bundles.values().collect();
        bundles.sort_by_key(|bundle| taskify_core::task::tr_or(&bundle.title, locale, &bundle.id));
        for bundle in bundles {
            let title = taskify_core::task::tr_or(&bundle.title, locale, &bundle.id);
            let description =
                taskify_core::task::tr(&bundle.description, locale).unwrap_or_default();
            egui::Frame::group(ui.style())
                .inner_margin(egui::Margin::symmetric(12, 8))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        ui.label(egui::RichText::new(&title).strong());
                        ui.label(
                            egui::RichText::new(format!(
                                "{} {} · {} {}",
                                bundle.price,
                                self.t("common.coins_short"),
                                bundle.contents_count(),
                                self.t("market.contents")
                            ))
                            .small()
                            .weak(),
                        );
                    });
                    if !description.is_empty() {
                        ui.label(egui::RichText::new(description).small().weak());
                    }
                    if !bundle.tokens.is_empty() {
                        ui.label(
                            egui::RichText::new(bundle.tokens.join(" · "))
                                .small()
                                .weak(),
                        );
                    }
                });
            ui.add_space(4.0);
        }
    }

    fn pack_stickers_tab(&mut self, ui: &mut egui::Ui, pack: &Pack, locale: &str) {
        if pack.stickers.is_empty() {
            ui.label(self.t("packs.empty"));
            return;
        }
        let mut stickers: Vec<_> = pack.stickers.values().collect();
        stickers.sort_by_key(|sticker| sticker.title(locale));
        for sticker in stickers {
            let title = sticker.title(locale);
            egui::Frame::group(ui.style())
                .inner_margin(egui::Margin::symmetric(12, 8))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.horizontal(|ui| {
                        let mut drawn = false;
                        if let Some(path) = sticker.image_path(&pack.root) {
                            drawn = self.icons.show_file(ui, &path, 30.0).is_some();
                        }
                        if !drawn && !sticker.icon.is_empty() {
                            drawn = self.icons.show(ui, &sticker.icon, 26.0).is_some();
                        }
                        if !drawn {
                            ui.label(egui::RichText::new("—").weak());
                        }
                        ui.vertical(|ui| {
                            ui.label(egui::RichText::new(&title).strong());
                            ui.label(
                                egui::RichText::new(format!(
                                    "{} {} · {}",
                                    sticker.price,
                                    self.t("common.coins_short"),
                                    sticker.id
                                ))
                                .small()
                                .weak(),
                            );
                        });
                    });
                });
            ui.add_space(4.0);
        }
    }

    fn pack_metrics_tab(&mut self, ui: &mut egui::Ui, pack: &Pack) {
        if pack.metrics_declared.is_empty() {
            ui.label(self.t("packs.empty"));
            return;
        }
        let mut metrics = pack.metrics_declared.clone();
        metrics.sort();
        for metric in metrics {
            egui::Frame::group(ui.style())
                .inner_margin(egui::Margin::symmetric(12, 8))
                .show(ui, |ui| {
                    ui.set_width(ui.available_width());
                    ui.monospace(metric);
                });
            ui.add_space(4.0);
        }
    }

    fn show_pack_icon(&self, ui: &mut egui::Ui, pack: &Pack, icon: &str, size: f32) {
        if let Some(key) = pack_icon_key(pack, icon) {
            self.icons.show(ui, key, size);
        }
    }

    fn pack_unlock_note(&self, unlock: &Unlock) -> String {
        match unlock {
            Unlock::Free => self.t("col.free"),
            Unlock::Coins { price } => {
                format!("{price} {}", self.t("common.coins_short"))
            }
            Unlock::Level { level } | Unlock::PackLevel { level } => {
                format!("{} {}", self.t("common.level"), level)
            }
            Unlock::Achievement { id } => format!("{} {id}", self.t("market.achievement")),
        }
    }

    fn pack_token_kind_label(&self, kind: TokenKind) -> String {
        self.t(match kind {
            TokenKind::Color => "market.t_color",
            TokenKind::Font => "market.t_font",
            TokenKind::Image => "market.t_image",
            TokenKind::Sound => "market.t_sound",
            TokenKind::Theme => "market.t_theme",
        })
    }

    fn install_pack(&mut self, path: &std::path::Path, ctx: &egui::Context) {
        let packs_dir = crate::bootstrap::packs_dir(&self.settings);
        match taskify_packs::install::install_zip(path, &packs_dir, &Default::default()) {
            Ok(report) => {
                // Установленный пак сразу включаем: базовые паки по умолчанию
                // выключены, но установка из zip — явное намерение.
                self.engine.reload_packs();
                let _ = self.engine.set_pack_enabled(&report.pack_id, true);
                self.engine.apply_disabled_packs();
                let text = format!(
                    "{}: {} ({} файлов)",
                    self.t("status.install_ok"),
                    report.pack_id,
                    report.files
                );
                self.toast(text, ctx);
            }
            Err(error) => {
                let text = format!("{}: {error}", self.t("status.install_fail"));
                self.toast(text, ctx);
            }
        }
    }

    fn pack_info_tab(&mut self, ui: &mut egui::Ui, pack: &Pack, locale: &str) {
        let (level, inside, span, xp) = self.pack_level(pack);
        let mut rows: Vec<(String, String)> = vec![
            (self.t("packs.info_id"), pack.id.clone()),
            (self.t("packs.version"), pack.version.clone()),
            (
                self.t("packs.pack_xp"),
                format!("{xp} · {} {level} · {inside}/{span}", self.t("packs.level")),
            ),
            (self.t("packs.tasks"), pack.tasks.len().to_string()),
            (self.t("packs.combos"), pack.combos.len().to_string()),
            (
                self.t("packs.achievements"),
                pack.achievements.len().to_string(),
            ),
            (
                self.t("packs.categories"),
                pack.categories.len().to_string(),
            ),
            (self.t("packs.tags"), pack.tags.len().to_string()),
            (self.t("packs.types"), pack.types.len().to_string()),
            (self.t("packs.icons"), pack.icons.len().to_string()),
            (self.t("packs.tab_palette"), pack.palette.len().to_string()),
            (self.t("packs.tokens"), pack.tokens.len().to_string()),
            (self.t("packs.stickers"), pack.stickers.len().to_string()),
            (self.t("packs.bundles"), pack.bundles.len().to_string()),
            (
                self.t("packs.tab_metrics"),
                pack.metrics_declared.len().to_string(),
            ),
            (self.t("packs.folder"), pack.root.display().to_string()),
        ];
        if let Some(manifest) = &pack.manifest {
            if let Some(license) = manifest.license.clone() {
                rows.push((self.t("packs.license"), license));
            }
        }
        for (label, value) in rows {
            ui.horizontal(|ui| {
                ui.add_sized(
                    [180.0, 20.0],
                    egui::Label::new(egui::RichText::new(label).small().weak()),
                );
                ui.label(egui::RichText::new(value).small());
            });
        }
        if let Some(manifest) = &pack.manifest {
            let description = taskify_core::task::tr_or(&manifest.description, locale, "");
            if !description.is_empty() {
                ui.add_space(6.0);
                ui.label(egui::RichText::new(description).small());
            }
        }
    }
}

/// Иконка пака: `icon` может быть локальным id или полным `<пак>/<id>`.
fn pack_icon_key<'a>(pack: &'a Pack, icon: &'a str) -> Option<&'a str> {
    if let Some(def) = pack.icons.get(icon) {
        return Some(def.image.as_str());
    }
    let qualified = if icon.contains('/') {
        icon.to_string()
    } else {
        format!("{}/{}", pack.id, icon)
    };
    pack.icons
        .get(&qualified)
        .map(|def| def.image.as_str())
        .or(Some(icon))
}

/// Цвет из `#rrggbb` или имя цвета в палитре пака.
fn pack_color(pack: &Pack, value: &str) -> Option<egui::Color32> {
    hex_color(value).or_else(|| {
        pack.palette
            .get(value)
            .and_then(|color| hex_color(&color.light))
    })
}

fn color_dot(ui: &mut egui::Ui, color: egui::Color32) {
    let (rect, _) = ui.allocate_exact_size(egui::vec2(14.0, 14.0), egui::Sense::hover());
    ui.painter()
        .rect_filled(rect, egui::CornerRadius::same(3), color);
}

/// Человеческое описание расписания задания для страницы пака.
fn schedule_text(task: &taskify_core::task::Task, locale: &str) -> Option<String> {
    use taskify_core::task::Schedule;
    let russian = locale != "en";
    match &task.schedule {
        Schedule::Daily => None,
        Schedule::Interval { days } => Some(format!(
            "{}: {days}",
            if russian {
                "раз в N дней"
            } else {
                "every N days"
            }
        )),
        Schedule::Once => Some(if russian { "один раз" } else { "once" }.to_string()),
        Schedule::Manual => Some(if russian { "вручную" } else { "manual" }.to_string()),
    }
}
