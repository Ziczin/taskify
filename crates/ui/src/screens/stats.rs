//! Экран «Статистика»: неделя, месяц и история.
//!
//! Недели и месяцы листаются только по периодам, где есть агрегаты: пустые
//! периоды пропускаются, а если данных нет совсем — стрелки выключены.

use chrono::{Datelike, Duration, NaiveDate};
use eframe::egui;
use taskify_store::Aggregates;

use crate::app::{StatsTab, TaskifyApp};

/// Есть ли в дне хоть какое-то событие, попавшее в агрегаты.
fn day_has_data(day: &taskify_store::aggregates::DayAgg) -> bool {
    day.completed > 0 || day.xp != 0 || day.coins != 0 || day.rejections > 0
}

fn has_data_between(aggregates: &Aggregates, start: NaiveDate, end: NaiveDate) -> bool {
    aggregates
        .by_day
        .range(start..=end)
        .any(|(_, day)| day_has_data(day))
}

pub(crate) fn week_has_data(aggregates: &Aggregates, start: NaiveDate) -> bool {
    has_data_between(aggregates, start, start + Duration::days(6))
}

pub(crate) fn month_start(date: NaiveDate) -> NaiveDate {
    NaiveDate::from_ymd_opt(date.year(), date.month(), 1).expect("valid month")
}

pub(crate) fn shift_month(start: NaiveDate, delta: i32) -> NaiveDate {
    let total = start.year() * 12 + start.month0() as i32 + delta;
    let year = total.div_euclid(12);
    let month = total.rem_euclid(12) as u32 + 1;
    NaiveDate::from_ymd_opt(year, month, 1).expect("valid shifted month")
}

pub(crate) fn month_end(start: NaiveDate) -> NaiveDate {
    shift_month(start, 1) - Duration::days(1)
}

pub(crate) fn month_has_data(aggregates: &Aggregates, start: NaiveDate) -> bool {
    has_data_between(aggregates, start, month_end(start))
}

/// Ближайшая неделя назад, где есть данные. Текущая неделя разрешена всегда,
/// даже пустая: с неё открывается экран.
pub(crate) fn prev_week_shift(
    aggregates: &Aggregates,
    current: NaiveDate,
    from: i32,
) -> Option<i32> {
    let earliest = aggregates.by_day.keys().next().copied();
    let min_shift = earliest
        .map(|date| -((current - date).num_weeks() as i32 + 1))
        .unwrap_or(0);
    let mut shift = from - 1;
    while shift >= min_shift {
        let start = current + Duration::weeks(shift as i64);
        if start == current || week_has_data(aggregates, start) {
            return Some(shift);
        }
        shift -= 1;
    }
    None
}

/// Ближайшая неделя вперёд. Дальше текущей не уходим.
pub(crate) fn next_week_shift(
    aggregates: &Aggregates,
    current: NaiveDate,
    from: i32,
) -> Option<i32> {
    for shift in (from + 1)..=0 {
        let start = current + Duration::weeks(shift as i64);
        if week_has_data(aggregates, start) {
            return Some(shift);
        }
    }
    None
}

pub(crate) fn prev_month_shift(
    aggregates: &Aggregates,
    current: NaiveDate,
    from: i32,
) -> Option<i32> {
    let earliest = aggregates.by_day.keys().next().copied();
    let min_shift = earliest
        .map(|date| {
            -(((current.year() - date.year()) * 12 + current.month0() as i32
                - date.month0() as i32)
                + 1)
        })
        .unwrap_or(0);
    let mut shift = from - 1;
    while shift >= min_shift {
        let start = shift_month(current, shift);
        if start == current || month_has_data(aggregates, start) {
            return Some(shift);
        }
        shift -= 1;
    }
    None
}

pub(crate) fn next_month_shift(
    aggregates: &Aggregates,
    current: NaiveDate,
    from: i32,
) -> Option<i32> {
    for shift in (from + 1)..=0 {
        let start = shift_month(current, shift);
        if month_has_data(aggregates, start) {
            return Some(shift);
        }
    }
    None
}

impl TaskifyApp {
    pub(crate) fn screen_stats(&mut self, ui: &mut egui::Ui) {
        ui.heading(self.t("stats.title"));
        ui.add_space(2.0);
        ui.horizontal(|ui| {
            for tab in StatsTab::ALL {
                let selected = self.stats_tab == tab;
                if ui.selectable_label(selected, self.t(tab.key())).clicked() {
                    self.stats_tab = tab;
                }
            }
        });
        ui.separator();
        ui.add_space(6.0);

        match self.stats_tab {
            StatsTab::Week => self.screen_week(ui),
            StatsTab::Month => self.screen_month(ui),
            StatsTab::History => self.screen_history(ui),
        }
    }

    fn screen_month(&mut self, ui: &mut egui::Ui) {
        let current = month_start(self.engine.today());
        let mut shift = self.stats_month_shift;
        let prev = prev_month_shift(&self.engine.store.aggregates, current, shift);
        let next = next_month_shift(&self.engine.store.aggregates, current, shift);

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
            let label = format!(
                "{} {}",
                self.t(&format!("months.{}", shift_month(current, shift).month())),
                shift_month(current, shift).year()
            );
            ui.add_sized(
                [220.0, 24.0],
                egui::Label::new(egui::RichText::new(label).strong()),
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
        self.stats_month_shift = shift;

        let start = shift_month(current, shift);
        let end = month_end(start);
        if !month_has_data(&self.engine.store.aggregates, start) {
            ui.add_space(14.0);
            ui.label(self.t("stats.no_data"));
            return;
        }

        let mut total_completed = 0u64;
        let mut total_xp = 0i64;
        let mut total_coins = 0i64;
        let mut active_days = 0u32;
        let mut best: Option<(NaiveDate, i64)> = None;
        let mut date = start;
        while date <= end {
            if let Some(day) = self.engine.store.aggregates.by_day.get(&date) {
                total_completed += day.completed;
                total_xp += day.xp;
                total_coins += day.coins;
                if day_has_data(day) {
                    active_days += 1;
                }
                if best.map(|(_, xp)| day.xp > xp).unwrap_or(true) {
                    best = Some((date, day.xp));
                }
            }
            date += Duration::days(1);
        }

        ui.add_space(12.0);
        egui::Grid::new("stats_month_totals")
            .num_columns(4)
            .spacing([28.0, 4.0])
            .show(ui, |ui| {
                ui.label(egui::RichText::new(self.t("week.completed")).strong());
                ui.label(egui::RichText::new(self.t("common.xp")).strong());
                ui.label(egui::RichText::new(self.t("common.coins")).strong());
                ui.label(egui::RichText::new(self.t("stats.active_days")).strong());
                ui.end_row();
                ui.label(total_completed.to_string());
                ui.label(total_xp.to_string());
                ui.label(total_coins.to_string());
                ui.label(active_days.to_string());
                ui.end_row();
            });
        ui.add_space(6.0);
        ui.label(
            egui::RichText::new(format!(
                "{}: {}",
                self.t("stats.average_xp"),
                total_xp / end.day() as i64
            ))
            .small()
            .weak(),
        );
        if let Some((date, xp)) = best {
            ui.label(
                egui::RichText::new(format!(
                    "{}: {} · {} xp",
                    self.t("stats.best_day"),
                    date.format("%d.%m"),
                    xp
                ))
                .small()
                .weak(),
            );
        }

        ui.add_space(12.0);
        let today = self.engine.today();
        let cell_width = ((ui.available_width() - 6.0 * 6.0) / 7.0).clamp(58.0, 118.0);
        egui::Grid::new("stats_month_grid")
            .num_columns(7)
            .spacing([6.0, 6.0])
            .show(ui, |ui| {
                for key in [
                    "weekdays.mon",
                    "weekdays.tue",
                    "weekdays.wed",
                    "weekdays.thu",
                    "weekdays.fri",
                    "weekdays.sat",
                    "weekdays.sun",
                ] {
                    ui.label(egui::RichText::new(self.t(key)).small().strong());
                }
                ui.end_row();

                for _ in 0..start.weekday().num_days_from_monday() {
                    ui.label("");
                }
                let mut date = start;
                while date <= end {
                    let day = self.engine.store.aggregates.by_day.get(&date);
                    let completed = day.map(|item| item.completed).unwrap_or(0);
                    let xp = day.map(|item| item.xp).unwrap_or(0);
                    let mut frame = egui::Frame::group(ui.style())
                        .inner_margin(egui::Margin::symmetric(5, 4))
                        .corner_radius(8);
                    if date == today {
                        frame = frame.fill(self.accent.gamma_multiply(0.14));
                    }
                    let response = frame
                        .show(ui, |ui| {
                            ui.set_min_size(egui::vec2(cell_width - 10.0, 52.0));
                            ui.vertical(|ui| {
                                let marker = if date == today { "▸ " } else { "" };
                                ui.label(
                                    egui::RichText::new(format!("{marker}{}", date.day())).strong(),
                                );
                                if day.map(day_has_data).unwrap_or(false) {
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "{} {}",
                                            completed,
                                            self.t("stats.done_short")
                                        ))
                                        .small(),
                                    );
                                    ui.label(
                                        egui::RichText::new(format!(
                                            "{} {}",
                                            xp,
                                            self.t("stats.xp_short")
                                        ))
                                        .small()
                                        .weak(),
                                    );
                                } else {
                                    ui.label(egui::RichText::new("—").small().weak());
                                }
                            });
                        })
                        .response;
                    response.on_hover_text(format!(
                        "{} · {} {} · {} xp",
                        date.format("%d.%m.%Y"),
                        completed,
                        self.t("week.completed"),
                        xp
                    ));
                    if date.weekday().num_days_from_monday() == 6 {
                        ui.end_row();
                    }
                    date += Duration::days(1);
                }
            });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn date(year: i32, month: u32, day: u32) -> NaiveDate {
        NaiveDate::from_ymd_opt(year, month, day).unwrap()
    }

    #[test]
    fn weeks_without_data_are_skipped() {
        let current = date(2026, 11, 2);
        let mut aggregates = Aggregates::default();
        aggregates.day_mut(current - Duration::weeks(3)).completed = 1;
        aggregates.day_mut(current).completed = 1;
        assert_eq!(prev_week_shift(&aggregates, current, 0), Some(-3));
        assert_eq!(next_week_shift(&aggregates, current, -3), Some(0));
        assert_eq!(prev_week_shift(&aggregates, current, -3), None);
        assert_eq!(next_week_shift(&aggregates, current, 0), None);
    }

    #[test]
    fn empty_history_keeps_both_arrows_off() {
        let current = date(2026, 11, 2);
        let aggregates = Aggregates::default();
        assert_eq!(prev_week_shift(&aggregates, current, 0), None);
        assert_eq!(next_week_shift(&aggregates, current, 0), None);
        assert_eq!(prev_month_shift(&aggregates, month_start(current), 0), None);
        assert_eq!(next_month_shift(&aggregates, month_start(current), 0), None);
    }

    #[test]
    fn months_without_data_are_skipped_across_year() {
        let current = date(2026, 1, 1);
        let mut aggregates = Aggregates::default();
        aggregates.day_mut(date(2025, 11, 15)).completed = 1;
        aggregates.day_mut(current).completed = 1;
        assert_eq!(prev_month_shift(&aggregates, current, 0), Some(-2));
        assert_eq!(next_month_shift(&aggregates, current, -2), Some(0));
        assert_eq!(prev_month_shift(&aggregates, current, -2), None);
        assert_eq!(shift_month(current, -1), date(2025, 12, 1));
        assert_eq!(month_end(date(2024, 2, 1)), date(2024, 2, 29));
    }
}
