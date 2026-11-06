//! Экран «История»: журнал событий с фильтрами.

use eframe::egui;
use taskify_store::{Event, EventKind};

use crate::app::TaskifyApp;

impl TaskifyApp {
    fn load_history(&mut self) {
        let log_dir = self.engine.store.livespace.log_dir();
        match taskify_store::log::read_all(&log_dir) {
            Ok(outcome) => {
                self.history.events = outcome.events;
                self.history.loaded = true;
            }
            Err(error) => {
                self.history.loaded = true;
                self.set_status(format!("журнал не читается: {error}"));
            }
        }
    }

    fn event_line(&self, event: &Event) -> (String, String, String) {
        let locale = self.settings.ui.locale.clone();
        let kind = event.kind.key().to_string();
        let date = event.ts.format("%Y-%m-%d %H:%M").to_string();
        let detail = match &event.kind {
            EventKind::TaskCompleted(payload) => format!(
                "{}  +{} xp  +{} {}",
                payload.task.title(&locale),
                payload.awarded.xp,
                payload.awarded.coins,
                self.t("common.coins_short")
            ),
            EventKind::TaskCompletionReverted(payload) => {
                format!("{}  {} xp", payload.target, payload.awarded.xp)
            }
            EventKind::SubtaskCompleted(payload) => {
                let title =
                    taskify_core::task::tr_or(&payload.subtask_title, &locale, &payload.subtask_id);
                format!("{}  {}", payload.task_id, title)
            }
            EventKind::TaskRejected(payload) => payload.task.title(&locale),
            EventKind::HabitSkipped(payload) => payload.task_id.clone(),
            EventKind::ComboAwarded(payload) => {
                let title = taskify_core::task::tr_or(&payload.title, &locale, &payload.combo);
                format!("{title}  +{} xp", payload.awarded.xp)
            }
            EventKind::AchievementUnlocked(payload) => {
                let title =
                    taskify_core::task::tr_or(&payload.title, &locale, &payload.achievement);
                format!("{title}  +{} xp", payload.awarded.xp)
            }
            EventKind::Purchase(payload) => format!(
                "{} / {}  {} {}",
                payload.token,
                payload.variant,
                payload.price,
                self.t("common.coins_short")
            ),
            EventKind::BundlePurchased(payload) => {
                format!(
                    "{}  {} {}  {} шт",
                    payload.bundle,
                    payload.price,
                    self.t("common.coins_short"),
                    payload.items.len()
                )
            }
            EventKind::TaskStarted(payload) => {
                format!("{} {}", payload.task_id, payload.local_date)
            }
            EventKind::TaskStopped(payload) => payload.task_id.clone(),
            EventKind::StickerPlaced(payload) => {
                format!("{}  {:.2} / {:.2}", payload.sticker, payload.x, payload.y)
            }
            EventKind::StickerRemoved(payload) => payload.sticker.clone(),
            EventKind::LevelUp(payload) => format!("level {}", payload.level),
            EventKind::PackInstalled(payload) => format!("{} {}", payload.pack, payload.version),
            EventKind::PackUpdated(payload) => {
                format!("{} {} → {}", payload.pack, payload.from, payload.to)
            }
            EventKind::PackDisabled(payload) => payload.pack.clone(),
            EventKind::PackReset(payload) => format!(
                "{}  −{} / −{} / −{}",
                payload.pack,
                payload.tasks.len().saturating_sub(payload.keep_tasks.len()),
                payload.combos.len(),
                payload.achievements.len()
            ),
            EventKind::SettingsChanged(payload) => {
                format!("{} = {}", payload.key, payload.value)
            }
            EventKind::WeekClosed(payload) => format!(
                "{}: {} xp, {}",
                payload.week_start, payload.xp, payload.completed
            ),
            EventKind::Unknown => String::new(),
        };
        (date, kind, detail)
    }

    /// Варианты подсказок по событиям с учётом выбранного типа и дат.
    fn history_suggestions(&self) -> Vec<crate::search::Suggestion> {
        let query = self.history.query.trim().to_string();
        if query.is_empty() {
            return Vec::new();
        }
        let kind_filter = self.history.kind.as_str();
        let from = self.history.from.trim();
        let to = self.history.to.trim();
        let mut items = Vec::new();
        for event in self.history.events.iter().rev() {
            let date = event.ts.format("%Y-%m-%d").to_string();
            if !kind_filter.is_empty() && event.kind.key() != kind_filter {
                continue;
            }
            if !from.is_empty() && date.as_str() < from {
                continue;
            }
            if !to.is_empty() && date.as_str() > to {
                continue;
            }
            let (date, kind, detail) = self.event_line(event);
            if detail.is_empty() {
                continue;
            }
            items.push(crate::search::Suggestion {
                value: detail.clone(),
                label: detail,
                meta: format!("{date}  ·  {kind}"),
            });
            items.push(crate::search::Suggestion {
                value: kind.clone(),
                label: kind,
                meta: format!("{}  ·  {date}", self.t("hist.type")),
            });
        }
        crate::search::suggestions(&query, items)
    }

    pub(crate) fn screen_history(&mut self, ui: &mut egui::Ui) {
        if !self.history.loaded {
            self.load_history();
        }

        ui.horizontal(|ui| {
            ui.heading(self.t("nav.history"));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                if ui.button(self.t("common.refresh")).clicked() {
                    self.load_history();
                }
            });
        });
        ui.add_space(6.0);

        let mut query_response: Option<egui::Response> = None;
        ui.horizontal_wrapped(|ui| {
            // Поля и выпадашка в одной строке должны быть одной высоты.
            ui.spacing_mut().interact_size.y = 26.0;
            ui.spacing_mut().button_padding = egui::vec2(10.0, 3.0);
            ui.label(self.t("hist.search"));
            let search_hint = self.t("hist.search_hint");
            let response = ui.add(
                egui::TextEdit::singleline(&mut self.history.query)
                    .desired_width(220.0)
                    .hint_text(search_hint)
                    .margin(crate::search::SEARCH_FIELD_MARGIN),
            );
            if response.changed() {
                self.history.suggest = 0;
            }
            if self.focus_search {
                response.request_focus();
                self.focus_search = false;
            }
            query_response = Some(response);
            egui::ComboBox::from_id_salt("hist_kind")
                .selected_text(if self.history.kind.is_empty() {
                    self.t("common.all")
                } else {
                    self.history.kind.clone()
                })
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_label(self.history.kind.is_empty(), self.t("common.all"))
                        .clicked()
                    {
                        self.history.kind.clear();
                    }
                    for kind in taskify_store::event::KNOWN_TYPES {
                        if ui
                            .selectable_label(self.history.kind == kind, kind)
                            .clicked()
                        {
                            self.history.kind = kind.to_string();
                        }
                    }
                });
            ui.label(self.t("hist.from"));
            ui.add(
                egui::TextEdit::singleline(&mut self.history.from)
                    .desired_width(90.0)
                    .margin(crate::search::SEARCH_FIELD_MARGIN),
            );
            ui.label(self.t("hist.to"));
            ui.add(
                egui::TextEdit::singleline(&mut self.history.to)
                    .desired_width(90.0)
                    .margin(crate::search::SEARCH_FIELD_MARGIN),
            );
        });
        if let Some(response) = query_response.as_ref() {
            let suggestions = self.history_suggestions();
            if let Some(value) = crate::search::show_suggestions(
                ui.ctx(),
                response,
                &suggestions,
                &mut self.history.suggest,
                "history_search_suggestions",
            ) {
                self.history.query = value;
                self.history.suggest = 0;
                response.surrender_focus();
            }
        }

        ui.add_space(6.0);
        let query = self.history.query.trim().to_lowercase();
        let kind = self.history.kind.clone();
        let from = self.history.from.trim().to_string();
        let to = self.history.to.trim().to_string();

        let rows: Vec<(String, String, String)> = self
            .history
            .events
            .iter()
            .rev()
            .filter(|event| {
                if !kind.is_empty() && event.kind.key() != kind {
                    return false;
                }
                let date = event.ts.format("%Y-%m-%d").to_string();
                if !from.is_empty() && date < from {
                    return false;
                }
                if !to.is_empty() && date > to {
                    return false;
                }
                true
            })
            .map(|event| self.event_line(event))
            .filter(|(date, kind, detail)| {
                query.is_empty()
                    || format!("{date} {kind} {detail}")
                        .to_lowercase()
                        .contains(&query)
            })
            .take(self.history.limit)
            .collect();

        ui.label(
            egui::RichText::new(format!(
                "{}: {}",
                self.t("hist.count"),
                self.history.events.len()
            ))
            .small()
            .weak(),
        );

        if rows.is_empty() {
            ui.add_space(16.0);
            ui.label(if self.history.events.is_empty() {
                self.t("hist.empty")
            } else {
                self.t("hist.no_match")
            });
            return;
        }

        egui::Grid::new("history_grid")
            .num_columns(3)
            .striped(true)
            .spacing([16.0, 4.0])
            .show(ui, |ui| {
                for (date, kind, detail) in rows {
                    ui.label(egui::RichText::new(date).small().weak());
                    ui.label(egui::RichText::new(kind).small().color(self.accent));
                    ui.label(detail);
                    ui.end_row();
                }
            });
    }
}
