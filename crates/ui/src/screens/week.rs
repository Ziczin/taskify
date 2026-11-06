//! Экран «Неделя»: сводка, цель и дни выбранной недели.

use chrono::Duration;
use eframe::egui;

use crate::app::TaskifyApp;
use crate::screens::stats::{next_week_shift, prev_week_shift, week_has_data};

impl TaskifyApp {
    pub(crate) fn screen_week(&mut self, ui: &mut egui::Ui) {
        let rules = self.engine.store.rules().days;
        let today = self.engine.today();
        let current = rules.week_start(today);

        let mut shift = self.stats_week_shift;
        let prev = prev_week_shift(&self.engine.store.aggregates, current, shift);
        let next = next_week_shift(&self.engine.store.aggregates, current, shift);
        ui.horizontal(|ui| {
            if ui
                .add_enabled(
                    prev.is_some(),
                    egui::Button::new("‹").min_size(egui::vec2(30.0, 24.0)),
                )
                .on_hover_text(self.t("stats.prev"))
                .clicked()
            {
                if let Some(value) = prev {
                    shift = value;
                }
            }
            let start = current + Duration::weeks(shift as i64);
            let end = start + Duration::days(6);
            ui.add_sized(
                [230.0, 24.0],
                egui::Label::new(
                    egui::RichText::new(format!(
                        "{} — {}",
                        start.format("%d.%m"),
                        end.format("%d.%m.%Y")
                    ))
                    .strong(),
                ),
            );
            if ui
                .add_enabled(
                    next.is_some(),
                    egui::Button::new("›").min_size(egui::vec2(30.0, 24.0)),
                )
                .on_hover_text(self.t("stats.next"))
                .clicked()
            {
                if let Some(value) = next {
                    shift = value;
                }
            }
        });
        self.stats_week_shift = shift;

        let start = current + Duration::weeks(shift as i64);
        if !week_has_data(&self.engine.store.aggregates, start) {
            ui.add_space(14.0);
            ui.label(self.t("stats.no_data"));
            return;
        }

        ui.add_space(8.0);
        let goal = self.engine.store.settings.weekly_xp_goal;
        let mut total_completed = 0u64;
        let mut total_xp = 0i64;
        let mut total_coins = 0i64;
        let mut rows = Vec::new();
        for offset in 0..7 {
            let date = start + Duration::days(offset);
            let agg = self.engine.store.aggregates.by_day.get(&date);
            let completed = agg.map(|item| item.completed).unwrap_or(0);
            let xp = agg.map(|item| item.xp).unwrap_or(0);
            let coins = agg.map(|item| item.coins).unwrap_or(0);
            total_completed += completed;
            total_xp += xp;
            total_coins += coins;
            rows.push((date, completed, xp, coins));
        }

        if goal > 0 {
            let fraction = (total_xp as f32 / goal as f32).clamp(0.0, 1.0);
            ui.horizontal(|ui| {
                ui.label(self.t("week.goal"));
                ui.add(
                    egui::ProgressBar::new(fraction)
                        .desired_width(320.0)
                        .text(format!("{total_xp} / {goal}")),
                );
                if start == current && self.engine.store.state.week.closed {
                    ui.label(
                        egui::RichText::new(self.t("week.closed"))
                            .color(self.palette().success)
                            .strong(),
                    );
                }
            });
        } else {
            ui.label(format!("{}: {total_xp}", self.t("week.xp")));
        }

        ui.add_space(10.0);
        egui::Grid::new("week_grid")
            .num_columns(4)
            .spacing([24.0, 6.0])
            .striped(true)
            .show(ui, |ui| {
                ui.label(egui::RichText::new("День").strong());
                ui.label(egui::RichText::new(self.t("week.completed")).strong());
                ui.label(egui::RichText::new(self.t("common.xp")).strong());
                ui.label(egui::RichText::new(self.t("common.coins")).strong());
                ui.end_row();

                for (date, completed, xp, coins) in &rows {
                    let marker = if *date == today { "▸ " } else { "" };
                    ui.label(format!("{marker}{}", date.format("%a %d.%m")));
                    ui.label(completed.to_string());
                    ui.label(xp.to_string());
                    ui.label(coins.to_string());
                    ui.end_row();
                }
                ui.label(egui::RichText::new("Итого").strong());
                ui.label(egui::RichText::new(total_completed.to_string()).strong());
                ui.label(egui::RichText::new(total_xp.to_string()).strong());
                ui.label(egui::RichText::new(total_coins.to_string()).strong());
                ui.end_row();
            });

        if start == current {
            ui.add_space(12.0);
            ui.separator();
            ui.label(format!(
                "{}: {}",
                self.t("today.remaining"),
                crate::engine::remaining_today(&self.engine, today)
            ));
            ui.label(format!(
                "{}: {}",
                self.t("today.done"),
                self.engine.store.state.day.completed
            ));
        }
    }
}
