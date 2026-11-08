//! Экран «Сегодня»: карточки заданий в четыре колонки.

use eframe::egui;
use taskify_core::task::{Kind, Task};

use crate::app::TaskifyApp;

#[derive(Debug, Clone)]
enum Action {
    Start(String),
    Stop(String),
    Complete(String),
    Reject(String),
    Skip(String),
    Subtask(String, String),
}

/// Высота карточки задания: кнопки прижаты к нижней кромке.
const CARD_HEIGHT: f32 = 170.0;
/// Высота ряда кнопок вместе с отступом.
const BUTTON_ROW: f32 = 26.0;

/// Смешать два цвета: `t = 0` — первый, `t = 1` — второй.
fn mix(a: egui::Color32, b: egui::Color32, t: f32) -> egui::Color32 {
    let t = t.clamp(0.0, 1.0);
    let lerp = |x: u8, y: u8| (x as f32 * (1.0 - t) + y as f32 * t).round() as u8;
    egui::Color32::from_rgb(lerp(a.r(), b.r()), lerp(a.g(), b.g()), lerp(a.b(), b.b()))
}

impl TaskifyApp {
    pub(crate) fn screen_today(&mut self, ui: &mut egui::Ui) {
        let date = self.engine.today();
        ui.horizontal(|ui| {
            ui.label(egui::RichText::new(self.t("nav.today")).size(20.0).strong());
            let remaining = crate::engine::remaining_today(&self.engine, date);
            let text = if remaining == 0 {
                self.t("today.all_done")
            } else {
                format!("{}: {remaining}", self.t("today.remaining"))
            };
            ui.label(egui::RichText::new(text).small().weak());
            if self.engine.store.state.day.over_norm > 0 {
                ui.label(
                    egui::RichText::new(format!(
                        "{}: {}",
                        self.t("today.over_norm"),
                        self.engine.store.state.day.over_norm
                    ))
                    .small()
                    .color(self.palette().warning),
                );
            }
        });

        ui.add_space(8.0);
        self.filters_row(ui);
        ui.add_space(10.0);

        let tasks: Vec<Task> = self.engine.packs.tasks().into_iter().cloned().collect();
        let candidates: Vec<Task> = tasks
            .into_iter()
            .filter(|task| self.task_matches(task, date))
            .collect();
        let done_tasks: Vec<Task> = candidates
            .iter()
            .filter(|task| self.engine.is_done_today(&task.id, date))
            .cloned()
            .collect();
        let rejected_tasks: Vec<Task> = candidates
            .iter()
            .filter(|task| {
                !self.engine.is_done_today(&task.id, date)
                    && self.engine.is_rejected_today(&task.id, date)
            })
            .cloned()
            .collect();
        let mut available_tasks: Vec<Task> = candidates
            .iter()
            .filter(|task| {
                !self.engine.is_done_today(&task.id, date)
                    && !self.engine.is_rejected_today(&task.id, date)
            })
            .cloned()
            .collect();
        if self.filters.started_only {
            available_tasks.retain(|task| self.engine.is_started(&task.id, date));
        }
        // Задания, у которых указаны уровни пака, видны только внутри рамки.
        available_tasks.retain(|task| self.task_level_visible(task));
        // Взятые в работу — первыми: их видно сразу.
        available_tasks.sort_by(|a, b| {
            let started_a = self.engine.is_started(&a.id, date);
            let started_b = self.engine.is_started(&b.id, date);
            started_b
                .cmp(&started_a)
                .then_with(|| a.category.cmp(&b.category))
                .then_with(|| a.id.cmp(&b.id))
        });

        // Три вкладки: отдельной для «в работе» нет — это фильтр доступных.
        ui.horizontal(|ui| {
            let tabs = [
                (
                    crate::app::TodayTab::Available,
                    self.t("today.tab_available"),
                    available_tasks.len(),
                ),
                (
                    crate::app::TodayTab::Done,
                    self.t("today.tab_done"),
                    done_tasks.len(),
                ),
                (
                    crate::app::TodayTab::Rejected,
                    self.t("today.tab_rejected"),
                    rejected_tasks.len(),
                ),
            ];
            for (tab, label, count) in tabs {
                let text = format!("{label} ({count})");
                let clicked = ui.selectable_label(self.today_tab == tab, text).clicked();
                if clicked && self.today_tab != tab {
                    self.today_tab = tab;
                    // Анимация относится только к контейнеру карточек.
                    self.start_tab_transition(ui.ctx());
                }
            }
            if self.today_tab == crate::app::TodayTab::Available {
                ui.add_space(8.0);
                let label = self.t("today.in_progress");
                ui.toggle_value(&mut self.filters.started_only, label);
            }
        });
        ui.add_space(8.0);

        // Появление карточек при смене статуса: шапка и вкладки остаются на
        // месте, двигается и проявляется только область с карточками.
        let cards_progress = self.tab_progress(ui.ctx());
        Self::transition(ui, cards_progress);

        let (visible, allow_start, empty_key) = match self.today_tab {
            crate::app::TodayTab::Available => (available_tasks, true, "today.empty"),
            crate::app::TodayTab::Done => (done_tasks, false, "today.empty_done"),
            crate::app::TodayTab::Rejected => (rejected_tasks, false, "today.empty_rejected"),
        };

        if visible.is_empty() {
            ui.add_space(24.0);
            ui.label(egui::RichText::new(self.t(empty_key)).weak().size(16.0));
            return;
        }

        // Колонок ровно столько, сколько задано в настройках. Задачи идут
        // слева направо и вниз, в колонке лежат вплотную друг к другу.
        let mut actions: Vec<Action> = Vec::new();
        let available = ui.available_size();
        if self.layout_debug {
            eprintln!("screen available {:.1}x{:.1}", available.x, available.y);
        }
        let gap = 10.0;
        let tight = gap;
        // Полоса прокрутки живёт в этом отступе, а не поверх крайней карточки.
        let scroll_gutter = 14.0;
        let grid_width = (available.x - scroll_gutter).max(160.0);
        let columns = self.settings.ui.columns.clamp(1, 6);
        let width = ((grid_width - gap * (columns - 1) as f32) / columns as f32).max(120.0);

        egui::ScrollArea::vertical()
            .id_salt("today_columns")
            .auto_shrink([false, false])
            .show(ui, |ui| {
                ui.allocate_ui(egui::vec2(grid_width, available.y), |ui| {
                    let origin = ui.cursor().min;
                    // Позиции анимируются: при смене числа колонок или фильтра
                    // карточки переезжают, а не прыгают.
                    let dt = ui.input(|input| input.stable_dt).min(0.05);
                    let factor = 1.0 - (-dt * 12.0).exp();
                    let mut column_heights = vec![0.0f32; columns];
                    let mut placements: Vec<(usize, egui::Vec2, f32, usize)> =
                        Vec::with_capacity(visible.len());
                    let mut moving = false;
                    for (index, task) in visible.iter().enumerate() {
                        let column = index % columns;
                        let height = self.card_heights.get(&task.id).copied().unwrap_or(205.0);
                        // Позиции считаем в координатах содержимого: иначе при
                        // прокрутке цели смещаются и карточки «плывут».
                        let target =
                            egui::vec2(column as f32 * (width + gap), column_heights[column]);
                        column_heights[column] += height + tight;
                        let current = self.card_anim.get(&task.id).copied().unwrap_or(target);
                        let next = current + (target - current) * factor;
                        if (target - next).length() > 0.4 {
                            moving = true;
                        }
                        self.card_anim.insert(task.id.clone(), next);
                        placements.push((index, next, height, column));
                    }
                    self.card_anim
                        .retain(|id, _| visible.iter().any(|task| &task.id == id));
                    let content_height = column_heights.iter().copied().fold(available.y, f32::max);
                    ui.allocate_rect(
                        egui::Rect::from_min_size(
                            origin,
                            egui::vec2(ui.available_width(), content_height),
                        ),
                        egui::Sense::hover(),
                    );
                    if moving {
                        ui.ctx().request_repaint();
                    }
                    // Пока карточки едут, не даём им наехать друг на друга:
                    // позиция не выше низа предыдущей карточки в колонке.
                    let mut drawn_bottom = vec![0.0f32; columns];
                    for (index, position, height, column) in placements {
                        let task = &visible[index];
                        let top = position.y.max(drawn_bottom[column]);
                        let rect = egui::Rect::from_min_size(
                            origin + egui::vec2(position.x, top),
                            egui::vec2(width, height),
                        );
                        let inner = ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
                            ui.set_width(width);
                            let started = self.engine.is_started(&task.id, date);
                            self.task_card(ui, task, date, &mut actions, started, allow_start)
                        });
                        let measured = inner.response.rect.height();
                        if measured > 0.0 {
                            drawn_bottom[column] = top + measured;
                            // Высота следует за карточкой в обе стороны: при
                            // сворачивании подзадач нижние карточки должны
                            // вернуться на место, а не остаться сдвинутыми.
                            self.card_heights.insert(task.id.clone(), measured);
                        }
                        if self.layout_debug {
                            let r = inner.response.rect;
                            eprintln!(
                                "card {} {:.1} {:.1} {:.1} {:.1}",
                                task.id,
                                r.min.x,
                                r.min.y,
                                r.width(),
                                r.height()
                            );
                        }
                    }
                    self.card_heights
                        .retain(|id, _| visible.iter().any(|task| &task.id == id));
                    if self.layout_debug {
                        eprintln!(
                            "grid columns={columns} width={width:.1} content={content_height:.1}"
                        );
                    }
                });
            });

        for action in actions {
            self.apply_action(action, date, ui.ctx());
        }
    }

    fn filters_row(&mut self, ui: &mut egui::Ui) {
        let categories: Vec<(String, String)> = self
            .engine
            .packs
            .categories()
            .into_iter()
            .map(|category| {
                (
                    category.id.clone(),
                    taskify_core::task::tr_or(
                        &category.title,
                        &self.settings.ui.locale,
                        &category.id,
                    ),
                )
            })
            .collect();
        let tags: Vec<(String, String)> = self
            .engine
            .packs
            .tags()
            .into_iter()
            .map(|tag| {
                (
                    tag.id.clone(),
                    taskify_core::task::tr_or(&tag.title, &self.settings.ui.locale, &tag.id),
                )
            })
            .collect();

        // Все поля фильтра одной высоты: иначе в горизонтальной строке они
        // выравниваются по центру и получается «лесенка».
        ui.horizontal_wrapped(|ui| {
            ui.spacing_mut().interact_size.y = 26.0;
            ui.spacing_mut().button_padding = egui::vec2(10.0, 3.0);
            ui.label(self.t("today.search"));
            ui.add_sized(
                [200.0, 26.0],
                egui::TextEdit::singleline(&mut self.filters.query)
                    .hint_text("…")
                    .margin(crate::search::SEARCH_FIELD_MARGIN),
            );

            let mut category_label = self
                .filters
                .category
                .as_ref()
                .and_then(|id| {
                    categories
                        .iter()
                        .find(|(cid, _)| cid == id)
                        .map(|(_, name)| name.clone())
                })
                .unwrap_or_else(|| self.t("common.all"));
            egui::ComboBox::from_id_salt("filter_category")
                .selected_text(category_label.clone())
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_label(self.filters.category.is_none(), self.t("common.all"))
                        .clicked()
                    {
                        self.filters.category = None;
                    }
                    for (id, name) in &categories {
                        if ui
                            .selectable_label(
                                self.filters.category.as_deref() == Some(id.as_str()),
                                name,
                            )
                            .clicked()
                        {
                            self.filters.category = Some(id.clone());
                            category_label = name.clone();
                        }
                    }
                });

            egui::ComboBox::from_id_salt("filter_tag")
                .selected_text(
                    self.filters
                        .tag
                        .as_ref()
                        .and_then(|id| {
                            tags.iter()
                                .find(|(tid, _)| tid == id)
                                .map(|(_, n)| n.clone())
                        })
                        .unwrap_or_else(|| self.t("common.all")),
                )
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_label(self.filters.tag.is_none(), self.t("common.all"))
                        .clicked()
                    {
                        self.filters.tag = None;
                    }
                    for (id, name) in &tags {
                        if ui
                            .selectable_label(
                                self.filters.tag.as_deref() == Some(id.as_str()),
                                name,
                            )
                            .clicked()
                        {
                            self.filters.tag = Some(id.clone());
                        }
                    }
                });

            egui::ComboBox::from_id_salt("filter_kind")
                .selected_text(
                    self.filters
                        .kind
                        .clone()
                        .unwrap_or_else(|| self.t("common.all")),
                )
                .show_ui(ui, |ui| {
                    if ui
                        .selectable_label(self.filters.kind.is_none(), self.t("common.all"))
                        .clicked()
                    {
                        self.filters.kind = None;
                    }
                    for kind in ["habit", "repeat", "once"] {
                        if ui
                            .selectable_label(
                                self.filters.kind.as_deref() == Some(kind),
                                self.t(&format!("today.{kind}")),
                            )
                            .clicked()
                        {
                            self.filters.kind = Some(kind.to_string());
                        }
                    }
                });

            if ui.button(self.t("common.refresh")).clicked() {
                self.engine.reload_packs();
            }
        });
    }

    /// Видно ли задание на текущем уровне его пака.
    fn task_level_visible(&self, task: &Task) -> bool {
        self.engine.task_level_visible(task)
    }

    fn task_matches(&self, task: &Task, date: chrono::NaiveDate) -> bool {
        // Задание вне уровневых рамок пака скрыто до открытия.
        if !self.task_level_visible(task) {
            return false;
        }
        // Ваншот после выполнения остаётся только во вкладке «Сделано».
        let done = self.engine.is_done_today(&task.id, date);
        if self.engine.is_spent_one_shot(task) && !done {
            return false;
        }
        // Задача цепочки не видна, пока предыдущая не выполнена.
        if !self.engine.is_unlocked(task) {
            return false;
        }
        let rejected = self.engine.is_rejected_today(&task.id, date);
        if !self.engine.is_due(task, date) && !done && !rejected {
            return false;
        }
        if let Some(category) = &self.filters.category {
            if task.category.as_deref() != Some(category.as_str()) {
                return false;
            }
        }
        if let Some(tag) = &self.filters.tag {
            if !task.tags.iter().any(|item| item == tag) {
                return false;
            }
        }
        if let Some(kind) = &self.filters.kind {
            if crate::engine::kind_key(task.kind) != kind {
                return false;
            }
        }
        let query = self.filters.query.trim().to_lowercase();
        if query.is_empty() {
            return true;
        }
        let title = task.title(&self.settings.ui.locale).to_lowercase();
        let description = task.description(&self.settings.ui.locale).to_lowercase();
        title.contains(&query)
            || description.contains(&query)
            || task.id.to_lowercase().contains(&query)
    }

    fn task_card(
        &mut self,
        ui: &mut egui::Ui,
        task: &Task,
        date: chrono::NaiveDate,
        actions: &mut Vec<Action>,
        started: bool,
        allow_start: bool,
    ) -> egui::Response {
        let locale = self.settings.ui.locale.clone();
        let done = self.engine.is_done_today(&task.id, date);
        let rejected = self
            .engine
            .store
            .state
            .task_ref(&task.id)
            .and_then(|state| state.last_rejected)
            == Some(date);
        let state = self
            .engine
            .store
            .state
            .task_ref(&task.id)
            .cloned()
            .unwrap_or_default();
        let available = self.engine.is_available(task);
        let window_label = task.window.map(|window| window.label());

        // Иконка задания — только картинка из assets/icons. Если нужной нет,
        // рисуем знак вопроса, но никогда не выводим символ текстом.
        let icon_key = task
            .icon
            .as_ref()
            .and_then(|icon| {
                self.engine
                    .packs
                    .packs
                    .values()
                    .find_map(|pack| pack.icons.get(icon))
            })
            .map(|icon| icon.image.clone())
            .filter(|image| !image.is_empty());

        let title = task.title(&locale);
        let category_name = task
            .category
            .as_ref()
            .and_then(|id| self.engine.packs.category(id))
            .map(|category| taskify_core::task::tr_or(&category.title, &locale, &category.id))
            .unwrap_or_default();
        let tag_names: Vec<String> = task
            .tags
            .iter()
            .filter_map(|id| self.engine.packs.tag(id))
            .map(|tag| taskify_core::task::tr_or(&tag.title, &locale, &tag.id))
            .collect();
        let hint_text = task
            .hints
            .requires
            .as_ref()
            .and_then(|map| taskify_core::task::tr(map, &locale));

        let accent = self.accent;
        let background = if started {
            mix(ui.visuals().extreme_bg_color, accent, 0.14)
        } else if done {
            self.palette().success_bg
        } else {
            ui.visuals().extreme_bg_color
        };

        let frame = egui::Frame::group(ui.style())
            .fill(background)
            .stroke(if started {
                egui::Stroke::new(2.0_f32, accent)
            } else {
                egui::Stroke::new(
                    1.0_f32,
                    if done {
                        ui.visuals().weak_text_color().gamma_multiply(0.6)
                    } else {
                        ui.visuals().widgets.noninteractive.bg_stroke.color
                    },
                )
            })
            .corner_radius(egui::CornerRadius::same(14))
            .inner_margin(egui::Margin::symmetric(16, 12));

        let card = frame.show(ui, |ui| {
            let width = ui.available_width();
            ui.set_width(width);
            // Нижняя граница карточки: кнопки прижаты ко дну, а не висят
            // посреди текста.
            let card_top = ui.cursor().top();
            ui.horizontal_top(|ui| {
                let key = icon_key.as_deref().unwrap_or("ui.question");
                if self
                    .icons
                    .image(ui, key, 22.0, None)
                    .map(|image| ui.add(image))
                    .is_none()
                {
                    self.icons.show(ui, "ui.question", 22.0);
                }
                ui.vertical(|ui| {
                    // Текст не должен раздвигать карточку: иначе она налезает
                    // на соседнюю колонку.
                    ui.set_max_width(ui.available_width());
                    ui.horizontal_wrapped(|ui| {
                        let mut text = egui::RichText::new(&title).size(16.0);
                        if done {
                            text = text.strikethrough().weak();
                        } else {
                            text = text.strong();
                        }
                        ui.add(egui::Label::new(text).wrap());
                        if done {
                            ui.label(
                                egui::RichText::new(self.t("today.done"))
                                    .small()
                                    .color(self.palette().success),
                            );
                        } else if rejected {
                            ui.label(
                                egui::RichText::new(self.t("today.rejected"))
                                    .small()
                                    .color(self.palette().danger),
                            );
                        }
                    });
                });
            });
            // Метаданные и описание идут от левого края карточки, а не от
            // текста заголовка: иначе они выглядят сдвинутыми вправо на
            // ширину иконки.
            let mut meta = Vec::new();
            if !category_name.is_empty() {
                meta.push(category_name.clone());
            }
            meta.extend(tag_names.clone());
            meta.push(kind_label(task.kind, self).to_string());
            if state.streak_current > 1 {
                meta.push(format!(
                    "{}: {}",
                    self.t("today.streak"),
                    state.streak_current
                ));
            }
            if !meta.is_empty() {
                let line = meta.join(" · ");
                // Label сам показывает полный текст, если строка не влезла.
                // Второй on_hover_text давал два тултипа подряд.
                ui.add(
                    egui::Label::new(
                        egui::RichText::new(line)
                            .small()
                            .color(ui.visuals().weak_text_color()),
                    )
                    .truncate(),
                );
            }
            let description = task.description(&locale);
            if !description.is_empty() && !done {
                // Описание переносится по словам: карточка растёт по высоте,
                // а не обрезает текст многоточием.
                ui.add(egui::Label::new(egui::RichText::new(description).small()).wrap());
            }
            if !task.subtasks.is_empty() && !done {
                let done_count = task
                    .subtasks
                    .iter()
                    .filter(|subtask| state.subtasks_done.contains(&subtask.id))
                    .count();
                let mut open = self.subtasks_open.contains(&task.id);
                let header = format!(
                    "{}: {}/{}",
                    self.t("today.subtasks"),
                    done_count,
                    task.subtasks.len()
                );
                let row = ui.horizontal(|ui| {
                    let (icon, _) = ui.allocate_exact_size(
                        egui::vec2(14.0, ui.spacing().interact_size.y),
                        egui::Sense::hover(),
                    );
                    let center = icon.center();
                    let color = ui.visuals().weak_text_color();
                    let points = if open {
                        vec![
                            center + egui::vec2(-4.0, -2.0),
                            center + egui::vec2(4.0, -2.0),
                            center + egui::vec2(0.0, 3.0),
                        ]
                    } else {
                        vec![
                            center + egui::vec2(-2.0, -4.0),
                            center + egui::vec2(-2.0, 4.0),
                            center + egui::vec2(3.0, 0.0),
                        ]
                    };
                    ui.painter().add(egui::Shape::convex_polygon(
                        points,
                        color,
                        egui::Stroke::NONE,
                    ));
                    ui.add(egui::Label::new(egui::RichText::new(header).small()));
                });
                let response = ui.interact(
                    row.response.rect,
                    ui.id().with(("subtasks", task.id.as_str())),
                    egui::Sense::click(),
                );
                if response.clicked() {
                    open = !open;
                    if open {
                        self.subtasks_open.insert(task.id.clone());
                    } else {
                        self.subtasks_open.remove(&task.id);
                    }
                    ui.ctx().request_repaint();
                }
                // Высоту раскрытия ведём сами: у CollapsingHeader первый
                // кадр берёт высоту-заглушку, из-за чего карточка прыгает.
                let anim_id = ui.make_persistent_id(("today_subtasks", task.id.as_str()));
                let openness = ui.ctx().animate_bool_with_time(anim_id, open, 0.18);
                if openness > 0.0 {
                    let row = ui.spacing().interact_size.y + ui.spacing().item_spacing.y;
                    let full_height = task.subtasks.len() as f32 * row;
                    let shown = full_height * openness;
                    let width = ui.available_width();
                    let (rect, _) =
                        ui.allocate_exact_size(egui::vec2(width, shown), egui::Sense::hover());
                    let body_rect = egui::Rect::from_min_size(
                        egui::pos2(rect.left() + 18.0, rect.top()),
                        egui::vec2((rect.width() - 18.0).max(1.0), full_height),
                    );
                    let mut body = ui.new_child(
                        egui::UiBuilder::new()
                            .max_rect(body_rect)
                            .layout(egui::Layout::top_down(egui::Align::LEFT)),
                    );
                    body.set_clip_rect(rect.intersect(ui.clip_rect()));
                    for subtask in &task.subtasks {
                        let checked = state.subtasks_done.contains(&subtask.id);
                        let mut is_checked = checked;
                        let label = subtask.title(&locale);
                        let response = body
                            .add_enabled(available, egui::Checkbox::new(&mut is_checked, label));
                        if response.changed() && !checked {
                            actions.push(Action::Subtask(task.id.clone(), subtask.id.clone()));
                        }
                    }
                }
            }
            if let Some(label) = &window_label {
                let color = if available {
                    ui.visuals().weak_text_color()
                } else {
                    self.palette().warning
                };
                let key = if available {
                    "today.window"
                } else {
                    "today.window_closed"
                };
                ui.label(
                    egui::RichText::new(format!("{}: {label}", self.t(key)))
                        .small()
                        .color(color),
                );
            }
            let reward = format!(
                "+{} {}  +{} {}",
                task.reward.xp,
                self.t("common.xp"),
                task.reward.coins,
                self.t("common.coins_short")
            );
            ui.label(
                egui::RichText::new(reward)
                    .strong()
                    .color(accent)
                    .size(14.0),
            );
            if !done {
                // Кнопки — отдельным рядом у нижней кромки карточки, поровну
                // на всю ширину: они не должны липнуть к тексту награды.
                // Свободное место добиваем отступом, чтобы ряд прижался к низу.
                let used = ui.cursor().top() - card_top;
                ui.add_space((CARD_HEIGHT - used - BUTTON_ROW).max(0.0));
                {
                    let mut buttons: Vec<(
                        Option<Action>,
                        &'static str,
                        Option<egui::Color32>,
                        String,
                    )> = vec![
                        (
                            Some(Action::Complete(task.id.clone())),
                            "ui.check",
                            Some(self.palette().success),
                            self.t("today.finish"),
                        ),
                        (
                            Some(Action::Reject(task.id.clone())),
                            "ui.cross",
                            None,
                            self.t("today.reject"),
                        ),
                    ];
                    if allow_start && !done && !rejected {
                        let (action, icon, tooltip) = if started {
                            (
                                Action::Stop(task.id.clone()),
                                "ui.stop",
                                self.t("today.at_work"),
                            )
                        } else {
                            (
                                Action::Start(task.id.clone()),
                                "ui.start",
                                self.t("today.take_work"),
                            )
                        };
                        buttons.insert(0, (Some(action), icon, Some(accent), tooltip));
                    }
                    if task.kind == Kind::Habit {
                        buttons.push((
                            Some(Action::Skip(task.id.clone())),
                            "ui.skip",
                            None,
                            self.t("today.skip"),
                        ));
                    }
                    if let Some(hint) = &hint_text {
                        buttons.push((
                            None,
                            "ui.question",
                            None,
                            format!("{}: {hint}", self.t("today.hint")),
                        ));
                    }
                    let count = buttons.len() as f32;
                    let gap = 6.0;
                    let width = ui.available_width();
                    let cell = ((width - gap * (count - 1.0)) / count).max(20.0);
                    let blocked_tip = format!(
                        "{}: {}",
                        self.t("today.window_closed"),
                        window_label.as_deref().unwrap_or_default()
                    );
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = gap;
                        for (action, icon, tint, tooltip) in buttons {
                            let clicked = self
                                .icons
                                .image(ui, icon, 14.0, tint)
                                .map(|image| {
                                    ui.add_sized(
                                        [cell, 24.0],
                                        egui::Button::image(image).frame(false),
                                    )
                                })
                                .map(|response| {
                                    response
                                        .on_hover_text(if available {
                                            tooltip
                                        } else {
                                            blocked_tip.clone()
                                        })
                                        .clicked()
                                })
                                .unwrap_or(false);
                            if clicked && available {
                                if let Some(action) = action {
                                    actions.push(action);
                                }
                            }
                        }
                    });
                }
            }
        });
        card.response
    }

    fn apply_action(&mut self, action: Action, _date: chrono::NaiveDate, ctx: &egui::Context) {
        match action {
            Action::Start(id) => {
                if let Err(error) = self.engine.start_task(&id) {
                    self.set_status(format!("{error:#}"));
                }
            }
            Action::Stop(id) => {
                if let Err(error) = self.engine.stop_task(&id) {
                    self.set_status(format!("{error:#}"));
                }
            }
            Action::Complete(id) => match self.engine.complete(&id) {
                Ok(outcome) => {
                    if let Some(path) = self.applied_asset(taskify_packs::TokenKind::Sound) {
                        crate::sound::play(&path);
                    }
                    let title = self
                        .engine
                        .task(&outcome.task_id)
                        .map(|task| task.title(&self.settings.ui.locale))
                        .unwrap_or_else(|| outcome.task_id.clone());
                    let mut text = format!(
                        "{}: {title}  +{} {}  +{} {}",
                        self.t("toast.completed"),
                        outcome.awarded.xp,
                        self.t("common.xp"),
                        outcome.awarded.coins,
                        self.t("common.coins_short")
                    );
                    if !outcome.milestones.is_empty() {
                        text.push_str(&format!("  ·  ×{}", outcome.milestones.len()));
                    }
                    if outcome.over_norm {
                        text.push_str(&format!("  ·  {}", self.t("today.over_norm")));
                    }
                    if !outcome.levels.is_empty() {
                        text.push_str(&format!(
                            "  ·  {} {}",
                            self.t("toast.level"),
                            outcome
                                .levels
                                .iter()
                                .map(|level| level.to_string())
                                .collect::<Vec<_>>()
                                .join(", ")
                        ));
                    }
                    if !outcome.combos.is_empty() {
                        text.push_str(&format!(
                            "  ·  {}: {}",
                            self.t("toast.combo"),
                            outcome.combos.len()
                        ));
                    }
                    if !outcome.achievements.is_empty() {
                        text.push_str(&format!(
                            "  ·  {}: {}",
                            self.t("toast.achievement"),
                            outcome.achievements.len()
                        ));
                    }
                    if let Some(phrase) = self
                        .notifications
                        .next(&self.i18n, &self.settings.ui.locale)
                    {
                        self.notifications.log(&phrase, &title);
                        text.push_str(&format!("\n{phrase}"));
                    }
                    self.toast(text, ctx);
                }
                Err(error) => self.set_status(format!("{error:#}")),
            },
            Action::Reject(id) => match self.engine.reject(&id, None) {
                Ok(()) => {
                    let text = self.t("today.rejected");
                    self.toast(text, ctx);
                }
                Err(error) => self.set_status(format!("{error:#}")),
            },
            Action::Skip(id) => match self.engine.skip_habit(&id) {
                Ok(()) => {
                    let text = self.t("today.skip");
                    self.toast(text, ctx);
                }
                Err(error) => self.set_status(format!("{error:#}")),
            },
            Action::Subtask(task_id, subtask_id) => {
                match self.engine.complete_subtask(&task_id, &subtask_id) {
                    Ok(reward) => {
                        let text = format!(
                            "{}: +{} {}",
                            self.t("today.done"),
                            reward.xp,
                            self.t("common.xp")
                        );
                        self.toast(text, ctx);
                    }
                    Err(error) => self.set_status(format!("{error:#}")),
                }
            }
        }
    }
}

pub fn kind_label(kind: Kind, app: &TaskifyApp) -> String {
    let key = match kind {
        Kind::Habit => "today.habit",
        Kind::Repeat => "today.repeat",
        Kind::Once => "today.once",
    };
    app.t(key)
}
