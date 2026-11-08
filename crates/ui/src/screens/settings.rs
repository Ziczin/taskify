//! Экран «Настройки»: игровой процесс, приложение и инструменты разработчика.
//!
//! Внутри — вертикальные подвкладки, как в браузере: слева список разделов,
//! справа содержимое выбранного. Всё применяется сразу, кнопок сохранения нет.

use eframe::egui;

use crate::app::{SettingsTab, TaskifyApp};

const LOCALE_NAMES: [(&str, &str); 8] = [
    ("ru", "Русский"),
    ("en", "English"),
    ("ja", "日本語"),
    ("uk", "Українська"),
    ("pl", "Polski"),
    ("es", "Español"),
    ("zh", "中文"),
    ("de", "Deutsch"),
];

/// Технические действия: живут на вкладке «Для разработчика».
enum TechAction {
    Backup,
    Verify,
    Rebuild,
    OpenLog,
    Reload,
    About,
}

impl TaskifyApp {
    pub(crate) fn screen_settings(&mut self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.heading(self.t("nav.settings"));
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                ui.label(
                    egui::RichText::new(format!(
                        "{} {}",
                        self.t("set.version"),
                        env!("CARGO_PKG_VERSION")
                    ))
                    .small()
                    .weak(),
                );
            });
        });
        ui.add_space(8.0);

        let tabs: Vec<(SettingsTab, String)> = SettingsTab::ALL
            .iter()
            .map(|tab| (*tab, self.t(tab.key())))
            .collect();
        let available = ui.available_height().max(200.0);
        ui.horizontal_top(|ui| {
            // Вертикальные вкладки.
            ui.allocate_ui_with_layout(
                egui::vec2(196.0, available),
                egui::Layout::top_down(egui::Align::LEFT),
                |ui| {
                    ui.set_width(196.0);
                    for (tab, label) in &tabs {
                        let selected = self.settings_tab == *tab;
                        if crate::app::centered_button(
                            ui,
                            egui::vec2(188.0, 34.0),
                            egui::Button::new(label).selected(selected),
                        )
                        .clicked()
                        {
                            self.settings_tab = *tab;
                            self.start_tab_transition(ui.ctx());
                        }
                    }
                },
            );

            // Разделительная линия между вкладками и содержимым.
            let (rect, _) =
                ui.allocate_exact_size(egui::vec2(1.0, available), egui::Sense::hover());
            ui.painter().vline(
                rect.center().x,
                rect.top()..=rect.bottom(),
                ui.visuals().widgets.noninteractive.bg_stroke,
            );

            ui.vertical(|ui| {
                ui.add_space(2.0);
                // Смена подвкладки двигает только содержимое справа.
                let progress = self.tab_progress(ui.ctx());
                Self::transition(ui, progress);
                // Содержимое настроек бывает выше окна: без прокрутки нижние
                // кнопки (в том числе удаление лайвспейса) были недоступны.
                let scroll_bottom =
                    std::env::var("TASKIFY_SETTINGS_SCROLL").as_deref() == Ok("bottom");
                egui::ScrollArea::vertical()
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        match self.settings_tab {
                            SettingsTab::Game => self.settings_game(ui),
                            SettingsTab::App => self.settings_app(ui),
                            SettingsTab::Dev => self.settings_dev(ui),
                        }
                        if scroll_bottom {
                            ui.scroll_to_cursor(Some(egui::Align::BOTTOM));
                        }
                    });
            });
        });
    }

    /// Игровой процесс: профиль, нормы, день и неделя.
    fn settings_game(&mut self, ui: &mut egui::Ui) {
        let mut game_changed = false;
        let mut interface_changed = false;
        let mut pick_avatar = false;
        let mut clear_avatar = false;

        egui::Grid::new("settings_game_grid")
            .num_columns(2)
            .spacing([16.0, 10.0])
            .show(ui, |ui| {
                ui.label(self.t("set.livespace_name"));
                let name_edit = ui.add(
                    egui::TextEdit::singleline(&mut self.livespace_rename).desired_width(220.0),
                );
                if name_edit.changed() && !self.livespace_rename.trim().is_empty() {
                    let name = self.livespace_rename.clone();
                    self.rename_livespace(&name, ui.ctx());
                }
                if name_edit.lost_focus() {
                    // Пустое имя не сохраняем: возвращаем последнее рабочее.
                    self.livespace_rename = self.livespace_display_name();
                }
                ui.end_row();

                ui.label(self.t("set.columns"));
                if ui
                    .add(egui::Slider::new(&mut self.settings.ui.columns, 1..=6))
                    .changed()
                {
                    interface_changed = true;
                }
                ui.end_row();

                ui.label(self.t("set.day_start"));
                if ui
                    .add(egui::Slider::new(&mut self.draft.day_start_hour, 0..=23))
                    .changed()
                {
                    game_changed = true;
                }
                ui.end_row();

                ui.label(self.t("set.week_start"));
                if ui
                    .checkbox(&mut self.draft.week_starts_monday, "")
                    .changed()
                {
                    game_changed = true;
                }
                ui.end_row();

                ui.label(self.t("set.daily_norm"));
                if ui
                    .add(egui::DragValue::new(&mut self.draft.daily_norm).range(0..=100))
                    .changed()
                {
                    game_changed = true;
                }
                ui.end_row();

                ui.label(self.t("set.weekly_goal"));
                if ui
                    .add(egui::DragValue::new(&mut self.draft.weekly_xp_goal).range(0..=100_000))
                    .changed()
                {
                    game_changed = true;
                }
                ui.end_row();

                ui.label(self.t("set.undo"));
                if ui
                    .add(egui::DragValue::new(&mut self.draft.undo_seconds).range(0..=60))
                    .changed()
                {
                    game_changed = true;
                }
                ui.end_row();
            });

        ui.add_space(10.0);
        ui.separator();
        ui.label(egui::RichText::new(self.t("set.avatar")).strong());
        ui.horizontal(|ui| {
            let avatar_id = self.avatar.as_ref().map(|texture| texture.id());
            match avatar_id {
                Some(id) => {
                    ui.add(
                        egui::Image::new(egui::load::SizedTexture::new(id, egui::vec2(56.0, 56.0)))
                            .corner_radius(28),
                    );
                }
                None => {
                    let (rect, _) =
                        ui.allocate_exact_size(egui::vec2(56.0, 56.0), egui::Sense::hover());
                    ui.painter()
                        .circle_stroke(rect.center(), 27.0, ui.visuals().window_stroke());
                    ui.painter().text(
                        rect.center(),
                        egui::Align2::CENTER_CENTER,
                        "?",
                        egui::FontId::proportional(22.0),
                        ui.visuals().weak_text_color(),
                    );
                }
            }
            ui.add_space(8.0);
            ui.vertical(|ui| {
                if ui.button(self.t("set.avatar_pick")).clicked() {
                    pick_avatar = true;
                }
                if avatar_id.is_some() && ui.button(self.t("set.avatar_clear")).clicked() {
                    clear_avatar = true;
                }
                ui.label(
                    egui::RichText::new(self.t("set.avatar_hint"))
                        .small()
                        .weak(),
                );
            });
        });
        ui.add_space(10.0);
        ui.label(egui::RichText::new(self.t("set.instant")).small().weak());

        if pick_avatar {
            if let Some(path) = rfd::FileDialog::new()
                .add_filter("Изображения", &["png", "jpg", "jpeg", "webp"])
                .pick_file()
            {
                match self.set_avatar(&path, ui.ctx()) {
                    Ok(()) => {
                        let text = self.t("set.avatar_ok");
                        self.toast(text, ui.ctx());
                    }
                    Err(error) => self.set_status(format!("{error:#}")),
                }
            }
        }
        if clear_avatar {
            self.clear_avatar(ui.ctx());
        }

        if game_changed {
            self.apply_game_settings(ui.ctx());
        }
        if interface_changed {
            self.apply_interface_settings(ui.ctx(), false);
        }

        ui.add_space(14.0);
        ui.separator();
        if ui.button(self.t("set.defaults")).clicked() {
            self.draft = taskify_store::GameSettings::default();
            self.apply_game_settings(ui.ctx());
        }

        // Опасная зона: удаление текущего аккаунта-лайвспейса.
        ui.add_space(18.0);
        ui.separator();
        let danger = self.palette().danger;
        let on_danger = self.palette().on_danger;
        ui.label(
            egui::RichText::new(self.t("set.delete_zone"))
                .strong()
                .color(danger),
        );
        ui.label(
            egui::RichText::new(self.t("set.delete_hint"))
                .small()
                .weak(),
        );
        let delete_button = egui::Button::new(
            egui::RichText::new(self.t("set.delete_livespace"))
                .color(on_danger)
                .strong(),
        )
        .fill(danger);
        if crate::app::centered_button(ui, egui::vec2(240.0, 32.0), delete_button).clicked() {
            self.delete_stage = 1;
            self.delete_confirm_name.clear();
            self.delete_error.clear();
        }
    }

    /// Приложение: язык, плотность и динамика интерфейса.
    fn settings_app(&mut self, ui: &mut egui::Ui) {
        let mut interface_changed = false;
        let locale_before = self.settings.ui.locale.clone();
        let font_before = self.settings.ui.font_scale;

        // --- лайвспейсы: список, переключение и создание -------------------
        let mut livespaces = crate::bootstrap::livespaces(&self.settings);
        // Внешний лайвспейс тоже должен быть виден в списке.
        if let Some(path) = crate::bootstrap::livespace_path(&self.settings) {
            if let Some(slug) = path
                .file_name()
                .map(|name| name.to_string_lossy().to_string())
            {
                if !livespaces.iter().any(|(found, _)| found == &slug) {
                    livespaces.insert(0, (slug.clone(), slug));
                }
            }
        }
        let current_name = livespaces
            .iter()
            .find(|(slug, _)| slug == &self.settings.livespace)
            .map(|(_, name)| name.clone())
            .unwrap_or_else(|| self.settings.livespace.clone());
        let livespace_label = self.t("set.livespace");
        let new_hint = self.t("set.livespace_new");
        let create_label = self.t("set.livespace_create");
        let mut switch_to: Option<String> = None;
        let mut create_clicked = false;
        let mut pick_livespace_path = false;
        let mut reset_livespace_path = false;
        let mut pick_packs_path = false;
        let mut reset_packs_path = false;
        ui.horizontal(|ui| {
            ui.label(&livespace_label);
            egui::ComboBox::from_id_salt("livespace")
                .selected_text(current_name)
                .show_ui(ui, |ui| {
                    for (slug, name) in &livespaces {
                        let selected = slug == &self.settings.livespace;
                        if ui.selectable_label(selected, name).clicked() && !selected {
                            switch_to = Some(slug.clone());
                        }
                    }
                });
        });
        ui.horizontal(|ui| {
            ui.add(
                egui::TextEdit::singleline(&mut self.livespace_name)
                    .desired_width(180.0)
                    .hint_text(&new_hint),
            );
            if ui.button(&create_label).clicked() {
                create_clicked = true;
            }
        });
        ui.add_space(8.0);
        ui.separator();
        ui.add_space(4.0);

        // --- пути: личная папка лайвспейса и общая папка паков -------------
        let livespace_root = self.engine.store.livespace.root().to_path_buf();
        let standard_root = self
            .settings
            .livespaces_dir()
            .join(&self.settings.livespace);
        let root_abs =
            std::path::absolute(&livespace_root).unwrap_or_else(|_| livespace_root.clone());
        let standard_abs = std::path::absolute(&standard_root).unwrap_or(standard_root.clone());
        let personal_path = root_abs != standard_abs;
        let packs_dir = crate::bootstrap::packs_dir(&self.settings);
        let default_packs = crate::bootstrap::data_root(&self.settings).join("packs");
        let packs_abs = std::path::absolute(&packs_dir).unwrap_or_else(|_| packs_dir.clone());
        let default_packs_abs =
            std::path::absolute(&default_packs).unwrap_or_else(|_| default_packs.clone());
        let personal_packs = packs_abs != default_packs_abs;

        ui.label(egui::RichText::new(self.t("set.paths")).strong());
        ui.label(egui::RichText::new(self.t("set.paths_hint")).small().weak());
        egui::Grid::new("settings_paths_grid")
            .num_columns(2)
            .spacing([16.0, 10.0])
            .show(ui, |ui| {
                ui.label(self.t("set.livespace_dir"));
                ui.vertical(|ui| {
                    let width = ui.available_width().clamp(140.0, 380.0);
                    ui.add_sized(
                        [width, 18.0],
                        egui::Label::new(
                            egui::RichText::new(livespace_root.display().to_string()).monospace(),
                        )
                        .truncate(),
                    )
                    .on_hover_text(livespace_root.display().to_string());
                    ui.horizontal(|ui| {
                        if ui.button(self.t("set.choose_folder")).clicked() {
                            pick_livespace_path = true;
                        }
                        if ui
                            .add_enabled(personal_path, egui::Button::new(self.t("set.reset_path")))
                            .clicked()
                        {
                            reset_livespace_path = true;
                        }
                    });
                });
                ui.end_row();

                ui.label(self.t("set.packs_dir"));
                ui.vertical(|ui| {
                    let width = ui.available_width().clamp(140.0, 380.0);
                    ui.add_sized(
                        [width, 18.0],
                        egui::Label::new(
                            egui::RichText::new(packs_dir.display().to_string()).monospace(),
                        )
                        .truncate(),
                    )
                    .on_hover_text(packs_dir.display().to_string());
                    ui.horizontal(|ui| {
                        if ui.button(self.t("set.choose_folder")).clicked() {
                            pick_packs_path = true;
                        }
                        if ui
                            .add_enabled(
                                personal_packs,
                                egui::Button::new(self.t("set.reset_path")),
                            )
                            .clicked()
                        {
                            reset_packs_path = true;
                        }
                    });
                });
                ui.end_row();
            });

        ui.add_space(6.0);
        ui.separator();
        ui.add_space(4.0);

        egui::Grid::new("settings_app_grid")
            .num_columns(2)
            .spacing([16.0, 10.0])
            .show(ui, |ui| {
                ui.label(self.t("set.language"));
                egui::ComboBox::from_id_salt("locale")
                    .selected_text(
                        LOCALE_NAMES
                            .iter()
                            .find(|(code, _)| *code == self.settings.ui.locale)
                            .map(|(_, name)| *name)
                            .unwrap_or("English"),
                    )
                    .show_ui(ui, |ui| {
                        for (code, name) in LOCALE_NAMES {
                            ui.selectable_value(
                                &mut self.settings.ui.locale,
                                code.to_string(),
                                name,
                            );
                        }
                    });
                ui.end_row();

                ui.label(self.t("set.market_columns"));
                if ui
                    .add(egui::Slider::new(
                        &mut self.settings.ui.market_columns,
                        2..=8,
                    ))
                    .changed()
                {
                    interface_changed = true;
                }
                ui.end_row();

                ui.label(self.t("set.anim_speed"));
                if ui
                    .add(egui::Slider::new(&mut self.settings.ui.anim_speed, 0.5..=2.0).suffix("×"))
                    .changed()
                {
                    interface_changed = true;
                }
                ui.end_row();

                ui.label(self.t("set.font_scale"));
                if ui
                    .add(egui::Slider::new(&mut self.settings.ui.font_scale, 0.8..=1.3).suffix("×"))
                    .changed()
                {
                    interface_changed = true;
                }
                ui.end_row();

                ui.label(self.t("set.notify_new_tasks"));
                if ui
                    .checkbox(&mut self.settings.ui.notify_new_tasks, "")
                    .changed()
                {
                    interface_changed = true;
                }
                ui.end_row();

                ui.label("");
                ui.label(
                    egui::RichText::new(self.t("set.notify_new_tasks_hint"))
                        .small()
                        .weak(),
                );
                ui.end_row();
            });

        if self.settings.ui.locale != locale_before
            || (self.settings.ui.font_scale - font_before).abs() > 0.001
        {
            interface_changed = true;
        }

        ui.add_space(10.0);
        ui.label(egui::RichText::new(self.t("set.instant")).small().weak());

        if interface_changed {
            let appearance = self.settings.ui.locale != locale_before
                || (self.settings.ui.font_scale - font_before).abs() > 0.001;
            self.apply_interface_settings(ui.ctx(), appearance);
        }

        if create_clicked {
            match crate::bootstrap::create_livespace(&self.settings, &self.livespace_name) {
                Ok(slug) => {
                    self.livespace_name.clear();
                    switch_to = Some(slug);
                }
                Err(error) => self.set_status(format!("{error:#}")),
            }
        }
        if let Some(slug) = switch_to {
            match self.switch_livespace(&slug, ui.ctx()) {
                Ok(()) => {
                    let display = crate::bootstrap::livespaces(&self.settings)
                        .into_iter()
                        .find(|(found, _)| found == &slug)
                        .map(|(_, name)| name)
                        .unwrap_or_else(|| slug.clone());
                    let text = format!("{}: {display}", self.t("set.livespace_switched"));
                    self.toast(text, ui.ctx());
                }
                Err(error) => self.set_status(format!("{error:#}")),
            }
        }

        if pick_livespace_path {
            if let Some(path) = rfd::FileDialog::new()
                .set_title(self.t("set.choose_folder"))
                .set_directory(
                    self.engine
                        .store
                        .livespace
                        .root()
                        .parent()
                        .unwrap_or_else(|| std::path::Path::new(".")),
                )
                .pick_folder()
            {
                match self.relocate_current_livespace(&path, ui.ctx()) {
                    Ok(warning) => {
                        let text = format!("{}: {}", self.t("set.relocate_ok"), path.display());
                        self.toast(text, ui.ctx());
                        if let Some(warning) = warning {
                            self.set_status(warning);
                        }
                    }
                    Err(error) => self.set_status(format!("{error:#}")),
                }
            }
        }
        if reset_livespace_path {
            let destination = self
                .settings
                .livespaces_dir()
                .join(&self.settings.livespace);
            match self.relocate_current_livespace(&destination, ui.ctx()) {
                Ok(warning) => {
                    let text = format!("{}: {}", self.t("set.relocate_ok"), destination.display());
                    self.toast(text, ui.ctx());
                    if let Some(warning) = warning {
                        self.set_status(warning);
                    }
                }
                Err(error) => self.set_status(format!("{error:#}")),
            }
        }
        if pick_packs_path || reset_packs_path {
            let previous = self.settings.paths.packs_dir.clone();
            if reset_packs_path {
                self.settings.paths.packs_dir = None;
            } else if let Some(path) = rfd::FileDialog::new()
                .set_title(self.t("set.choose_folder"))
                .set_directory(crate::bootstrap::packs_dir(&self.settings))
                .pick_folder()
            {
                self.settings.paths.packs_dir = Some(path);
            }
            if pick_packs_path && self.settings.paths.packs_dir == previous {
                // Диалог закрыли без выбора.
            } else {
                if let Err(error) = crate::bootstrap::install_builtin_packs(&self.settings) {
                    self.settings.paths.packs_dir = previous;
                    self.settings.save();
                    self.set_status(format!("{error:#}"));
                } else {
                    let path = crate::bootstrap::packs_dir(&self.settings);
                    self.engine.set_packs_dir(path.clone());
                    self.settings.save();
                    let text = format!("{}: {}", self.t("set.packs_changed"), path.display());
                    self.toast(text, ui.ctx());
                }
            }
        }

        ui.add_space(14.0);
        ui.separator();
        if ui.button(self.t("set.defaults")).clicked() {
            self.settings.ui.market_columns = crate::settings::default_market_columns();
            self.settings.ui.board_columns = crate::settings::default_board_columns();
            self.settings.ui.anim_speed = crate::settings::default_anim_speed();
            self.settings.ui.font_scale = crate::settings::default_font_scale();
            self.settings.ui.notify_new_tasks = false;
            self.apply_interface_settings(ui.ctx(), true);
        }
    }

    /// Для разработчика: пути, обслуживание и сведения о программе.
    fn settings_dev(&mut self, ui: &mut egui::Ui) {
        let mut action: Option<TechAction> = None;

        ui.label(egui::RichText::new(self.t("set.technical")).strong());
        ui.label(egui::RichText::new(self.t("set.dev_hint")).small().weak());
        ui.add_space(4.0);
        ui.horizontal_wrapped(|ui| {
            if ui.button(self.t("set.backup")).clicked() {
                action = Some(TechAction::Backup);
            }
            if ui.button(self.t("set.verify")).clicked() {
                action = Some(TechAction::Verify);
            }
            if ui.button(self.t("set.rebuild")).clicked() {
                action = Some(TechAction::Rebuild);
            }
            if ui.button(self.t("set.open_log")).clicked() {
                action = Some(TechAction::OpenLog);
            }
            if ui.button(self.t("nav.refresh")).clicked() {
                action = Some(TechAction::Reload);
            }
            if ui.button(self.t("nav.about")).clicked() {
                action = Some(TechAction::About);
            }
        });

        match action {
            Some(TechAction::Backup) => match self.engine.backup_now() {
                Ok(path) => {
                    let text = format!("{}: {}", self.t("status.backup_ok"), path.display());
                    self.toast(text, ui.ctx());
                }
                Err(error) => self.set_status(format!("{error:#}")),
            },
            Some(TechAction::Verify) => match self.engine.store.verify() {
                Ok(problems) if problems.is_empty() => {
                    let text = self.t("status.verify_ok");
                    self.toast(text, ui.ctx());
                }
                Ok(problems) => self.set_status(problems.join("; ")),
                Err(error) => self.set_status(format!("{error:#}")),
            },
            Some(TechAction::Rebuild) => match self.engine.store.rebuild() {
                Ok(events) => {
                    let text = format!("{} ({events})", self.t("status.rebuild_ok"));
                    self.toast(text, ui.ctx());
                }
                Err(error) => self.set_status(format!("{error:#}")),
            },
            Some(TechAction::OpenLog) => match crate::diag::open_log() {
                Ok(path) => {
                    let text = format!("{}: {}", self.t("set.log_opened"), path.display());
                    self.toast(text, ui.ctx());
                }
                Err(error) => self.set_status(error),
            },
            Some(TechAction::Reload) => {
                self.engine.reload_packs();
                let text = self.t("status.packs_reloaded");
                self.toast(text, ui.ctx());
            }
            Some(TechAction::About) => self.about_open = true,
            None => {}
        }
    }

    /// Игровые правила: сохраняем сразу, без кнопки.
    fn apply_game_settings(&mut self, ctx: &egui::Context) {
        if let Err(error) = self.engine.set_settings(self.draft.clone()) {
            self.set_status(format!("{error:#}"));
            return;
        }
        if let Err(error) = self
            .engine
            .record_settings_change("day_start_hour", &self.draft.day_start_hour.to_string())
        {
            eprintln!("settings event: {error:#}");
        }
        ctx.request_repaint();
    }

    /// Интерфейс: имя и колонки сохраняем сразу; внешний вид перекрашиваем,
    /// только когда поменялись язык или масштаб шрифта (иначе каждый символ
    /// имени перезагружал бы обои).
    fn apply_interface_settings(&mut self, ctx: &egui::Context, appearance: bool) {
        if appearance {
            self.i18n.set_locale(&self.settings.ui.locale);
            self.apply_look(ctx);
            self.refresh_accent();
        }
        self.settings.save();
        ctx.request_repaint();
    }
}
