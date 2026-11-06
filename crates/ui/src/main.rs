#![cfg_attr(windows, windows_subsystem = "windows")]

use eframe::egui;

mod app;
mod bootstrap;
mod cli;
mod diag;
mod engine;
mod i18n;
mod icon_data;
mod icons;
mod notifications;
mod screens;
mod search;
mod settings;
mod setup;
mod sound;
mod stickers;
mod theme;

#[cfg(test)]
mod engine_tests;
#[cfg(test)]
mod test_support;

/// Один бинарник: без аргументов открывается окно, с аргументами работают команды.
fn main() -> eframe::Result<()> {
    install_crash_handler();
    diag::init();
    let args: Vec<String> = std::env::args().skip(1).collect();
    if !args.is_empty() {
        console::attach_parent_console();
        if let Err(error) = cli::run(args) {
            eprintln!("ошибка: {error:#}");
            std::process::exit(1);
        }
        return Ok(());
    }

    let settings = settings::Settings::load();

    let mut viewport = egui::ViewportBuilder::default()
        .with_inner_size([settings.window.width, settings.window.height])
        .with_min_inner_size([960.0, 600.0])
        .with_title("Taskify");

    if let (Some(x), Some(y)) = (settings.window.x, settings.window.y) {
        viewport = viewport.with_position([x, y]);
    }

    let options = eframe::NativeOptions {
        viewport,
        ..Default::default()
    };

    eframe::run_native(
        "Taskify",
        options,
        Box::new(|cc| Ok(Box::new(RootApp::new(cc, settings)))),
    )
}

/// Окно приложения: либо экран первого запуска, либо основной интерфейс.
struct RootApp {
    mode: RootMode,
}

enum RootMode {
    Setup(Box<setup::SetupApp>),
    Main(Box<app::TaskifyApp>),
}

impl RootApp {
    fn new(cc: &eframe::CreationContext<'_>, settings: settings::Settings) -> Self {
        let mode = if bootstrap::has_livespace(&settings) {
            RootMode::Main(Box::new(app::TaskifyApp::build(&cc.egui_ctx, settings)))
        } else {
            RootMode::Setup(Box::new(setup::SetupApp::new(settings)))
        };
        Self { mode }
    }
}

impl eframe::App for RootApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        match &mut self.mode {
            RootMode::Setup(app) => {
                app.ui(ctx);
                if let Some(settings) = app.ready_settings() {
                    self.mode = RootMode::Main(Box::new(app::TaskifyApp::build(ctx, settings)));
                }
            }
            RootMode::Main(app) => app.update(ctx, frame),
        }
    }
}

/// Консоль нужна только командам: окно приложения запускается без неё.
///
/// Бинарник собирается как GUI-приложение, поэтому двойной клик не создаёт
/// чёрное окно. Если команду вызвали из терминала, стандартные потоки уже
/// наследуются; если консоли нет вовсе, подключаемся к родительской.
#[cfg(windows)]
mod console {
    use core::ffi::c_void;

    const ATTACH_PARENT_PROCESS: u32 = 0xFFFF_FFFF;
    const STD_OUTPUT_HANDLE: u32 = 0xFFFF_FFF5;
    const STD_ERROR_HANDLE: u32 = 0xFFFF_FFF4;
    const INVALID_HANDLE_VALUE: isize = -1;
    const GENERIC_WRITE: u32 = 0x4000_0000;
    const FILE_SHARE_READ: u32 = 0x0000_0001;
    const FILE_SHARE_WRITE: u32 = 0x0000_0002;
    const OPEN_EXISTING: u32 = 3;

    unsafe extern "system" {
        fn AttachConsole(process_id: u32) -> i32;
        fn GetStdHandle(which: u32) -> isize;
        fn SetStdHandle(which: u32, handle: isize) -> i32;
        fn CreateFileW(
            name: *const u16,
            access: u32,
            share: u32,
            security: *mut c_void,
            disposition: u32,
            flags: u32,
            template: isize,
        ) -> isize;
    }

    pub fn attach_parent_console() {
        unsafe {
            let current = GetStdHandle(STD_OUTPUT_HANDLE);
            if current != 0 && current != INVALID_HANDLE_VALUE {
                return;
            }
            if AttachConsole(ATTACH_PARENT_PROCESS) == 0 {
                return;
            }
            let name: Vec<u16> = "CONOUT$\0".encode_utf16().collect();
            let handle = CreateFileW(
                name.as_ptr(),
                GENERIC_WRITE,
                FILE_SHARE_READ | FILE_SHARE_WRITE,
                core::ptr::null_mut(),
                OPEN_EXISTING,
                0,
                0,
            );
            if handle == INVALID_HANDLE_VALUE {
                return;
            }
            SetStdHandle(STD_OUTPUT_HANDLE, handle);
            SetStdHandle(STD_ERROR_HANDLE, handle);
        }
    }

    const MB_OK: u32 = 0;
    const MB_ICONERROR: u32 = 0x10;

    unsafe extern "system" {
        fn MessageBoxW(window: isize, text: *const u16, caption: *const u16, kind: u32) -> i32;
    }

    pub fn message_box(text: &str) {
        let body: Vec<u16> = text.encode_utf16().chain(std::iter::once(0)).collect();
        let caption: Vec<u16> = "Taskify".encode_utf16().chain(std::iter::once(0)).collect();
        unsafe {
            MessageBoxW(0, body.as_ptr(), caption.as_ptr(), MB_OK | MB_ICONERROR);
        }
    }
}

#[cfg(not(windows))]
mod console {
    pub fn attach_parent_console() {}

    pub fn message_box(_text: &str) {}
}

/// Куда писать отчёт о падении: рядом с данными, чтобы файл было легко найти.
fn crash_path() -> std::path::PathBuf {
    let root = std::env::var_os("TASKIFY_HOME")
        .map(std::path::PathBuf::from)
        .or_else(|| {
            std::env::var_os("APPDATA").map(|base| std::path::PathBuf::from(base).join("taskify"))
        })
        .unwrap_or_else(std::env::temp_dir);
    root.join("crash.log")
}

/// Окно приложения не имеет консоли, поэтому панику показываем и пишем в файл.
fn install_crash_handler() {
    let previous = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |info| {
        let message = format!("{info}");
        let mut report = format!("=== {} ===\n{message}\n", timestamp());
        if let Some(location) = info.location() {
            report.push_str(&format!(
                "место: {}:{}:{}\n",
                location.file(),
                location.line(),
                location.column()
            ));
        }
        report.push_str(&format!(
            "backtrace:\n{}\n\n",
            std::backtrace::Backtrace::force_capture()
        ));

        let path = crash_path();
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        use std::io::Write;
        if let Ok(mut file) = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(&path)
        {
            let _ = file.write_all(report.as_bytes());
        }

        console::message_box(&format!(
            "Taskify упал.\n\n{message}\n\nПодробности: {}",
            path.display()
        ));
        previous(info);
    }));
}

fn timestamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default();
    format!("unix {seconds}")
}
