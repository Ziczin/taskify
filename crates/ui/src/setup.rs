//! Первый запуск: пока лайвспейса нет, предлагаем создать его или открыть
//! существующий. Путь выбирает сам игрок — внутри данных программы или где
//! угодно на диске (например, в папке синхронизации).

use std::path::{Path, PathBuf};

use eframe::egui;

use crate::bootstrap;
use crate::i18n::I18n;
use crate::settings::Settings;

pub struct SetupApp {
    settings: Settings,
    i18n: I18n,
    name: String,
    folder: Option<PathBuf>,
    error: Option<String>,
    ready: bool,
    frames: usize,
}

impl SetupApp {
    pub fn new(settings: Settings) -> Self {
        let i18n = I18n::load(&settings.ui.locale);
        Self {
            settings,
            i18n,
            name: String::new(),
            folder: None,
            error: None,
            ready: false,
            frames: 0,
        }
    }

    /// Готовые настройки после создания или открытия лайвспейса.
    pub fn ready_settings(&self) -> Option<Settings> {
        self.ready.then(|| self.settings.clone())
    }

    fn t(&self, key: &str) -> String {
        self.i18n.t(key)
    }

    fn create(&mut self) {
        let Some(parent) = self.folder.clone() else {
            self.error = Some(self.t("setup.no_folder"));
            return;
        };
        let name = self.name.trim();
        if name.is_empty() {
            self.error = Some(self.t("setup.no_name"));
            return;
        }
        match bootstrap::create_livespace_at(&parent, name) {
            Ok(path) => {
                let slug = path
                    .file_name()
                    .map(|name| name.to_string_lossy().to_string())
                    .unwrap_or_else(|| "default".to_string());
                self.settings
                    .set_personal_livespace_dir(&slug, Some(path.clone()));
                self.settings.livespace_path = Some(path.display().to_string());
                self.settings.livespace = slug;
                self.settings.save();
                self.ready = true;
            }
            Err(error) => self.error = Some(format!("{error:#}")),
        }
    }

    fn open_existing(&mut self) {
        let Some(path) = rfd::FileDialog::new()
            .set_title(self.t("setup.open"))
            .pick_folder()
        else {
            return;
        };
        match bootstrap::adopt_livespace(&path) {
            Ok(path) => {
                let slug = path
                    .file_name()
                    .map(|name| name.to_string_lossy().to_string())
                    .unwrap_or_else(|| "default".to_string());
                self.settings
                    .set_personal_livespace_dir(&slug, Some(path.clone()));
                self.settings.livespace_path = Some(path.display().to_string());
                self.settings.livespace = slug;
                self.settings.save();
                self.ready = true;
            }
            Err(error) => self.error = Some(format!("{error:#}")),
        }
    }

    pub fn ui(&mut self, ctx: &egui::Context) {
        self.frames += 1;
        if self.frames == 1 {
            crate::app::install_fonts(ctx);
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.add_space(48.0);
            ui.vertical_centered(|ui| {
                ui.heading(egui::RichText::new(self.t("setup.title")).size(28.0));
                ui.add_space(6.0);
                ui.label(
                    egui::RichText::new(self.t("setup.subtitle"))
                        .size(16.0)
                        .weak(),
                );
            });
            ui.add_space(28.0);

            let width = ui.available_width().min(560.0);
            ui.vertical_centered(|ui| {
                ui.allocate_ui_with_layout(
                    egui::vec2(width, 0.0),
                    egui::Layout::top_down(egui::Align::LEFT),
                    |ui| {
                        egui::Frame::group(ui.style())
                            .inner_margin(egui::Margin::same(16))
                            .show(ui, |ui| {
                                ui.label(
                                    egui::RichText::new(self.t("setup.create_title")).strong(),
                                );
                                ui.add_space(6.0);
                                ui.label(self.t("setup.name"));
                                let hint = self.t("setup.name_hint");
                                ui.add(
                                    egui::TextEdit::singleline(&mut self.name)
                                        .hint_text(hint)
                                        .desired_width(f32::INFINITY),
                                );
                                ui.add_space(8.0);
                                ui.label(self.t("setup.folder"));
                                ui.horizontal(|ui| {
                                    let shown = self
                                        .folder
                                        .as_ref()
                                        .map(|path| path.display().to_string())
                                        .unwrap_or_else(|| self.t("setup.not_chosen"));
                                    ui.label(egui::RichText::new(shown).monospace());
                                    if ui.button(self.t("setup.choose")).clicked() {
                                        if let Some(path) = rfd::FileDialog::new()
                                            .set_title(self.t("setup.choose"))
                                            .pick_folder()
                                        {
                                            self.folder = Some(path);
                                        }
                                    }
                                });
                                ui.add_space(12.0);
                                if ui
                                    .add_sized(
                                        [220.0, 34.0],
                                        egui::Button::new(self.t("setup.create")),
                                    )
                                    .clicked()
                                {
                                    self.create();
                                }
                            });

                        ui.add_space(14.0);
                        ui.horizontal(|ui| {
                            if ui
                                .add_sized([260.0, 32.0], egui::Button::new(self.t("setup.open")))
                                .clicked()
                            {
                                self.open_existing();
                            }
                            ui.label(
                                egui::RichText::new(self.t("setup.open_hint"))
                                    .small()
                                    .weak(),
                            );
                        });

                        if let Some(error) = &self.error {
                            ui.add_space(10.0);
                            ui.colored_label(crate::theme::Palette::light().danger, error);
                        }
                    },
                );
            });
        });

        // Скриншот для проверки экрана.
        if let Some(path) = std::env::var_os("TASKIFY_SCREENSHOT").map(PathBuf::from) {
            if self.frames == 3 {
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(Default::default()));
            }
            if let Some(image) = ctx.input(|input| {
                input.events.iter().find_map(|event| match event {
                    egui::Event::Screenshot { image, .. } => Some(image.clone()),
                    _ => None,
                })
            }) {
                save_png(&image, &path);
                ctx.send_viewport_cmd(egui::ViewportCommand::Close);
            }
        }
    }
}

fn save_png(image: &egui::ColorImage, path: &Path) {
    let width = image.width() as u32;
    let height = image.height() as u32;
    if let Some(buffer) = image::RgbaImage::from_raw(width, height, image.as_raw().to_vec()) {
        let _ = buffer.save(path);
    }
}
