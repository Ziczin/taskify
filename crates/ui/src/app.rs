//! Окно, навигация и общее состояние интерфейса.

use std::path::PathBuf;

use chrono::Datelike;
use eframe::egui;

use taskify_store::GameSettings;

use crate::engine::Engine;
use crate::i18n::I18n;
use crate::settings::Settings;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Section {
    Today,
    Statistics,
    Packs,
    Achievements,
    Market,
    Stock,
    Settings,
}

impl Section {
    pub const ALL: [Section; 7] = [
        Section::Today,
        Section::Statistics,
        Section::Packs,
        Section::Achievements,
        Section::Market,
        Section::Stock,
        Section::Settings,
    ];

    pub fn key(self) -> &'static str {
        match self {
            Section::Today => "nav.today",
            Section::Statistics => "nav.stats",
            Section::Packs => "nav.packs",
            Section::Achievements => "nav.achievements",
            Section::Market => "nav.market",
            Section::Stock => "nav.stock",
            Section::Settings => "nav.settings",
        }
    }

    /// Файл иконки раздела в `assets/icons`.
    pub fn icon_key(self) -> &'static str {
        match self {
            Section::Today => "nav.today",
            // Календарь недели остаётся узнаваемым значком статистики.
            Section::Statistics => "nav.week",
            Section::Packs => "nav.packs",
            Section::Achievements => "nav.achievements",
            Section::Market => "nav.market",
            Section::Stock => "nav.stock",
            Section::Settings => "nav.settings",
        }
    }
}

/// Вкладки внутри «Статистики».
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum StatsTab {
    Week,
    Month,
    History,
}

impl StatsTab {
    pub const ALL: [StatsTab; 3] = [StatsTab::Week, StatsTab::Month, StatsTab::History];

    pub fn key(self) -> &'static str {
        match self {
            StatsTab::Week => "stats.week",
            StatsTab::Month => "stats.month",
            StatsTab::History => "stats.history",
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Filters {
    pub query: String,
    pub category: Option<String>,
    pub tag: Option<String>,
    pub kind: Option<String>,
    /// Показывать только взятые в работу (фильтр внутри «Доступных»).
    pub started_only: bool,
}

/// Вкладка склада: косметика, темы, стикеры и профили вида.
///
/// «Темы» — это готовые наборы оформления и базовые темы; отдельной сущности
/// «набор» в интерфейсе больше нет.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum StockTab {
    #[default]
    Themes,
    Cosmetics,
    Stickers,
    Profiles,
}

impl StockTab {
    pub fn key(self) -> &'static str {
        match self {
            StockTab::Themes => "stock.tab_themes",
            StockTab::Cosmetics => "stock.tab_cosmetics",
            StockTab::Stickers => "stock.tab_stickers",
            StockTab::Profiles => "stock.tab_profiles",
        }
    }
}

/// Вкладка экрана «Настройки»: игровые параметры и параметры программы.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum SettingsTab {
    #[default]
    Game,
    App,
    Dev,
}

impl SettingsTab {
    pub const ALL: [SettingsTab; 3] = [SettingsTab::Game, SettingsTab::App, SettingsTab::Dev];

    pub fn key(self) -> &'static str {
        match self {
            SettingsTab::Game => "set.tab_game",
            SettingsTab::App => "set.tab_app",
            SettingsTab::Dev => "set.tab_dev",
        }
    }
}

/// Вкладки внутри пака.
#[derive(Clone, Copy, PartialEq, Eq, Debug, Default)]
pub enum PackTab {
    #[default]
    Tasks,
    Combos,
    Achievements,
    Categories,
    Tags,
    Types,
    Icons,
    Palette,
    Tokens,
    Bundles,
    Stickers,
    Metrics,
    Info,
}

impl PackTab {
    pub const ALL: [PackTab; 13] = [
        PackTab::Tasks,
        PackTab::Combos,
        PackTab::Achievements,
        PackTab::Categories,
        PackTab::Tags,
        PackTab::Types,
        PackTab::Icons,
        PackTab::Palette,
        PackTab::Tokens,
        PackTab::Bundles,
        PackTab::Stickers,
        PackTab::Metrics,
        PackTab::Info,
    ];

    pub fn key(self) -> &'static str {
        match self {
            PackTab::Tasks => "packs.tab_tasks",
            PackTab::Combos => "packs.tab_combos",
            PackTab::Achievements => "packs.tab_achievements",
            PackTab::Categories => "packs.tab_categories",
            PackTab::Tags => "packs.tab_tags",
            PackTab::Types => "packs.tab_types",
            PackTab::Icons => "packs.tab_icons",
            PackTab::Palette => "packs.tab_palette",
            PackTab::Tokens => "packs.tab_tokens",
            PackTab::Bundles => "packs.tab_bundles",
            PackTab::Stickers => "packs.tab_stickers",
            PackTab::Metrics => "packs.tab_metrics",
            PackTab::Info => "packs.tab_info",
        }
    }

    /// Количество единиц контента этого вида в паке.
    pub fn count(self, pack: &taskify_packs::Pack) -> usize {
        match self {
            PackTab::Tasks => pack.tasks.len(),
            PackTab::Combos => pack.combos.len(),
            PackTab::Achievements => pack.achievements.len(),
            PackTab::Categories => pack.categories.len(),
            PackTab::Tags => pack.tags.len(),
            PackTab::Types => pack.types.len(),
            PackTab::Icons => pack.icons.len(),
            PackTab::Palette => pack.palette.len(),
            PackTab::Tokens => pack.tokens.len(),
            PackTab::Bundles => pack.bundles.len(),
            PackTab::Stickers => pack.stickers.len(),
            PackTab::Metrics => pack.metrics_declared.len(),
            PackTab::Info => 0,
        }
    }

    /// Вкладки, которые имеют контент в этом паке; «О паке» есть всегда.
    pub fn for_pack(pack: &taskify_packs::Pack) -> Vec<PackTab> {
        Self::ALL
            .iter()
            .copied()
            .filter(|tab| *tab == PackTab::Info || tab.count(pack) > 0)
            .collect()
    }
}

/// Какие ачивки показывать: все, полученные или ещё закрытые.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum AchState {
    #[default]
    All,
    Unlocked,
    Locked,
}

impl AchState {
    pub const ALL: [AchState; 3] = [AchState::All, AchState::Unlocked, AchState::Locked];

    pub fn key(self) -> &'static str {
        match self {
            AchState::All => "ach.f_all",
            AchState::Unlocked => "ach.f_unlocked",
            AchState::Locked => "ach.f_locked",
        }
    }
}

/// Сортировка ачивок.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum AchSort {
    #[default]
    Name,
    Percent,
}

impl AchSort {
    pub const ALL: [AchSort; 2] = [AchSort::Name, AchSort::Percent];

    pub fn key(self) -> &'static str {
        match self {
            AchSort::Name => "ach.sort_name",
            AchSort::Percent => "ach.sort_percent",
        }
    }
}

/// Какие паки показывать в списке.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum PacksFilter {
    #[default]
    All,
    Enabled,
    Disabled,
}

impl PacksFilter {
    pub const ALL: [PacksFilter; 3] = [
        PacksFilter::All,
        PacksFilter::Enabled,
        PacksFilter::Disabled,
    ];

    pub fn key(self) -> &'static str {
        match self {
            PacksFilter::All => "packs.f_all",
            PacksFilter::Enabled => "packs.f_enabled",
            PacksFilter::Disabled => "packs.f_disabled",
        }
    }
}

/// Что показывать на витрине маркета.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum MarketKind {
    #[default]
    All,
    Themes,
    Cosmetics,
    Stickers,
}

impl MarketKind {
    pub const ALL: [MarketKind; 4] = [
        MarketKind::All,
        MarketKind::Themes,
        MarketKind::Cosmetics,
        MarketKind::Stickers,
    ];

    pub fn key(self) -> &'static str {
        match self {
            MarketKind::All => "market.kind_all",
            MarketKind::Themes => "market.kind_themes",
            MarketKind::Cosmetics => "market.kind_cosmetics",
            MarketKind::Stickers => "market.kind_stickers",
        }
    }
}

/// Вкладка экрана «Сегодня»: доступные, завершённые, отклонённые.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum TodayTab {
    #[default]
    Available,
    Done,
    Rejected,
}

#[derive(Debug, Clone, Default)]
pub struct HistoryState {
    pub loaded: bool,
    pub events: Vec<taskify_store::Event>,
    pub query: String,
    pub kind: String,
    pub from: String,
    pub to: String,
    pub limit: usize,
    /// Выбранная подсказка в выпадающем списке поиска.
    pub suggest: usize,
}

pub struct Toast {
    pub text: String,
    pub until: f64,
    /// Когда тост появился: по нему считается анимация всплытия.
    pub shown_at: f64,
}

/// Что именно тянет игрок за ручку стикера.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum StickerTransformMode {
    Scale,
    Rotate,
}

/// Начало жеста ресайза или вращения. Значения фиксируются на первом кадре,
/// чтобы стикер не «уползал» от накопленного за кадр дельта-смещения.
#[derive(Debug, Clone)]
struct StickerTransformDrag {
    slot: String,
    mode: StickerTransformMode,
    start_pointer: egui::Pos2,
    center: egui::Pos2,
    base_scale: f32,
    base_rotation: f32,
}

/// Ключ локализации для дня недели.
/// Цвет варианта косметики: либо сам `value` — `#rrggbb`, либо имя в палитре пака.
pub(crate) fn variant_color(
    packs: &taskify_packs::PackSet,
    token_id: &str,
    variant: &taskify_packs::Variant,
) -> Option<egui::Color32> {
    if let Some(color) = hex_color(&variant.value) {
        return Some(color);
    }
    let pack_id = token_id
        .split_once('/')
        .map(|(pack, _)| pack)
        .unwrap_or_default();
    packs
        .packs
        .get(pack_id)
        .and_then(|pack| pack.palette.get(&variant.value))
        .and_then(|color| hex_color(&color.light))
}

/// `#rrggbb` → цвет egui; нужен и маркету, и складу для образцов цвета.
pub(crate) fn hex_color(hex: &str) -> Option<egui::Color32> {
    let hex = hex.trim().trim_start_matches('#');
    if hex.len() != 6 {
        return None;
    }
    let value = u32::from_str_radix(hex, 16).ok()?;
    Some(egui::Color32::from_rgb(
        (value >> 16) as u8,
        (value >> 8) as u8,
        value as u8,
    ))
}

fn day_key(day: chrono::Weekday) -> &'static str {
    use chrono::Weekday::{Fri, Mon, Sat, Sun, Thu, Tue, Wed};
    match day {
        Mon => "day.mon",
        Tue => "day.tue",
        Wed => "day.wed",
        Thu => "day.thu",
        Fri => "day.fri",
        Sat => "day.sat",
        Sun => "day.sun",
    }
}

pub struct TaskifyApp {
    pub engine: Engine,
    pub i18n: I18n,
    pub settings: Settings,
    pub section: Section,
    pub about_open: bool,
    pub toast: Option<Toast>,
    pub filters: Filters,
    pub today_tab: TodayTab,
    /// Текущие обои: имя файла и загруженная текстура.
    pub wallpaper: Option<(String, egui::TextureHandle)>,
    /// Имя нового профиля оформления (поле ввода на складе).
    pub look_profile_name: String,
    /// Тема, которую пользователь подтверждает перезаписать.
    pub overwrite_look_target: Option<String>,
    /// Профиль, который авто-режим применил последним.
    pub auto_look_name: Option<String>,
    /// Начало анимации перехода между разделами.
    pub section_anim_start: Option<f32>,
    /// Начало анимации содержимого вкладки (карточки, списки).
    pub tab_anim_start: Option<f32>,
    /// Имя нового лайвспейса в поле настройки.
    pub livespace_name: String,
    /// Текущее имя лайвспейса в поле настроек.
    pub livespace_rename: String,
    /// Фильтр списка паков: все, включённые или отключённые.
    pub packs_filter: PacksFilter,
    /// Поиск по пакам: имя, id и описание.
    pub packs_query: String,
    /// Открытый пак: id, если смотрим содержимое пака.
    pub pack_detail: Option<String>,
    /// Вкладка внутри открытого пака.
    pub pack_tab: PackTab,
    /// Фильтр ачивок по паку.
    pub ach_pack: Option<String>,
    /// Поиск по ачивкам.
    pub ach_query: String,
    /// Выбранная подсказка поиска ачивок.
    pub ach_suggest: usize,
    /// Фильтр ачивок по получению.
    pub ach_state: AchState,
    /// Сортировка ачивок.
    pub ach_sort: AchSort,
    /// Аватар этого лайвспейса.
    pub avatar: Option<egui::TextureHandle>,
    /// Фильтр витрины по типу товара.
    pub market_kind: MarketKind,
    /// Фильтр витрины по паку (`None` — все паки).
    pub market_pack: Option<String>,
    /// Подвкладка внутри статистики.
    pub stats_tab: StatsTab,
    /// Сдвиг выбранной недели относительно текущей.
    pub stats_week_shift: i32,
    /// Сдвиг выбранного месяца относительно текущего.
    pub stats_month_shift: i32,
    /// Подвкладка внутри настроек.
    pub settings_tab: SettingsTab,
    /// Вкладка склада.
    pub stock_tab: StockTab,
    /// Подвкладка косметики: тема, акцент, фон, обои, шрифт или звук.
    pub stock_cosmetic_group: String,
    /// Открыт диалог создания лайвспейса.
    pub livespace_dialog: bool,
    /// Поставить курсор в поле названия при открытии диалога.
    pub livespace_focus: bool,
    /// Ошибка создания лайвспейса.
    pub livespace_error: String,
    /// Для снимков: держать меню лайвспейсов открытым.
    pub livespace_menu_force: bool,
    /// Выбранный лайвспейс: переключим после отрисовки кадра.
    pub livespace_pending: Option<String>,
    /// Этап подтверждения удаления лайвспейса: 0 — нет, 1 и 2 — окна.
    pub delete_stage: u8,
    /// Название, которое нужно ввести на втором подтверждении.
    pub delete_confirm_name: String,
    /// Ошибка удаления.
    pub delete_error: String,
    /// Пак, для которого открыт диалог сброса прогресса.
    pub pack_reset_pending: Option<String>,
    /// Сбрасывать ли ачивки выбранного пака вместе с заданиями.
    pub pack_reset_achievements: bool,
    pub history: HistoryState,
    pub status: String,
    pub draft: GameSettings,
    pub startup_error: Option<String>,
    pub accent: egui::Color32,
    /// Картинки кнопок и иконок: эмодзи есть не во всех шрифтах.
    pub icons: crate::icons::Icons,
    pub notifications: crate::notifications::Notifications,
    /// Последняя проверка новых заданий для системного уведомления.
    last_new_tasks_check: Option<std::time::Instant>,
    pub screenshot: Option<PathBuf>,
    pub screenshot_requested: bool,
    /// Один раз сфокусировать поиск (для скриншотов подсказок).
    pub focus_search: bool,
    /// Скорость инерционной прокрутки колесом, точки в секунду.
    pub scroll_velocity: egui::Vec2,
    /// Раскрытые карточки подзадач на экране «Сегодня».
    pub subtasks_open: std::collections::BTreeSet<String>,
    /// Отладка раскладки: TASKIFY_LAYOUT_DEBUG печатает прямоугольники карточек.
    pub layout_debug: bool,
    /// Текущие позиции карточек: по ним анимируется перебалансировка сетки.
    pub card_anim: std::collections::HashMap<String, egui::Vec2>,
    /// Измеренные высоты карточек: по ним колонки укладываются вплотную.
    pub card_heights: std::collections::HashMap<String, f32>,
    /// Активный жест изменения размера или поворота стикера.
    sticker_transform: Option<StickerTransformDrag>,
}

/// Открывает профиль, повторяя попытку: антивирус и синхронизация иногда
/// держат файлы занятыми первые доли секунды после запуска.
fn open_profile(settings: &Settings) -> anyhow::Result<Engine> {
    let mut last: Option<anyhow::Error> = None;
    for attempt in 0..4 {
        match crate::bootstrap::prepare(settings) {
            Ok(engine) => return Ok(engine),
            Err(error) => {
                crate::diag::write(&format!("попытка {} не удалась: {error:#}", attempt + 1));
                last = Some(error);
                if attempt < 3 {
                    let pause = 150u64 << attempt;
                    std::thread::sleep(std::time::Duration::from_millis(pause));
                }
            }
        }
    }
    Err(last.unwrap_or_else(|| anyhow::anyhow!("профиль не открылся")))
}

/// Куда откатываться, если основной профиль недоступен.
///
/// Первым идёт постоянный каталог данных платформы: у него обычные права и
/// его не чистит система. Затем — уникальная папка в `%TEMP%`, чтобы два
/// запущенных экземпляра не мешали друг другу, и последняя — общая.
fn fallback_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    dirs.push(crate::settings::Settings::dir().join("fallback"));
    #[cfg(windows)]
    if let Some(local) = std::env::var_os("LOCALAPPDATA") {
        dirs.push(PathBuf::from(local).join("taskify").join("fallback"));
    }
    // Портативный вариант рядом с exe: помогает, когда профиль и temp
    // заблокированы синхронизацией или политикой.
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            dirs.push(dir.join("portable-data"));
        }
    }
    dirs.push(std::env::temp_dir().join(format!("taskify-fallback-{}", std::process::id())));
    dirs.push(std::env::temp_dir().join("taskify-fallback"));
    dirs
}

fn open_fallback_profile() -> Result<Engine, String> {
    let mut failures = Vec::new();
    for dir in fallback_dirs() {
        if let Err(error) = crate::diag::probe(&dir) {
            crate::diag::write(&format!(
                "запасной путь {} не прошёл проверку: {error}",
                dir.display()
            ));
            failures.push(format!("{}: {error}", dir.display()));
            continue;
        }
        match Engine::open(&dir) {
            Ok(engine) => {
                crate::diag::write(&format!("открыт запасной профиль {}", dir.display()));
                return Ok(engine);
            }
            Err(error) => {
                crate::diag::write(&format!(
                    "запасной профиль {} не открылся: {error:#}",
                    dir.display()
                ));
                failures.push(format!("{}: {error:#}", dir.display()));
            }
        }
    }
    Err(failures.join("\n"))
}

/// Подсказка, если профиль недоступен из-за прав: чаще всего приложение
/// запустили из песочницы (например, из окна чата), где запись запрещена.
fn permission_hint(text: &str) -> &'static str {
    if text.contains("os error 5") || text.contains("Отказано в доступе") {
        "\n\nПохоже, приложение запущено с ограниченными правами: запись в папку\
         \nданных запрещена. Закройте его и откройте taskify.exe двойным кликом\
         \nв Проводнике — тогда данные сохранятся в обычном профиле."
    } else {
        ""
    }
}

fn describe_startup_failure(error: &anyhow::Error, fallback_error: &str) -> String {
    let log = crate::diag::log_path()
        .map(|path| path.display().to_string())
        .unwrap_or_else(|| "не удалось создать".to_string());
    format!(
        "Taskify не смог открыть профиль.\n\n\
         Основной профиль:\n{error:#}\n\n\
         Запасные пути:\n{fallback_error}\n\n\
         Подробности: {log}\n\
         Проверьте права на папку данных и свободное место.{}",
        permission_hint(fallback_error)
    )
}

/// Раскладка стикеров плиткой: `(столбцы, ряды, диаметр кружка)`.
///
/// Перебирает 1–4 столбца и берёт вариант с самым крупным кружком, который
/// помещается и по ширине, и по высоте панели.
/// Ширины трёх колонок: две панели по краям и центр, который забирает
/// ровно остаток. Панели ужимаются на совсем узком окне, чтобы не вылезти.
fn rail_columns(available: f32, preferred_rail: f32, gap: f32) -> (f32, f32, f32) {
    let available = available.max(0.0);
    let gap = gap.max(0.0);
    let rail = preferred_rail
        .max(0.0)
        .min(((available - gap * 2.0) / 2.0).max(0.0));
    let center = (available - rail * 2.0 - gap * 2.0).max(0.0);
    (rail, center, rail)
}

/// Абсолютные прямоугольники реек и центра. Рейки всегда прижаты к краям
/// окна, поэтому ширина содержимого в центре не может сдвинуть правую панель.
fn shell_columns(
    area: egui::Rect,
    preferred_rail: f32,
    gap: f32,
    side_inset: f32,
) -> (egui::Rect, egui::Rect, egui::Rect) {
    // Боковые панели отступают от краёв окна на тот же зазор, что и снизу.
    let side_inset = side_inset.max(0.0);
    let area = egui::Rect::from_min_size(
        egui::pos2(area.left() + side_inset, area.top()),
        egui::vec2((area.width() - side_inset * 2.0).max(0.0), area.height()),
    );
    let (rail, _, _) = rail_columns(area.width(), preferred_rail, gap);
    let left = egui::Rect::from_min_size(area.min, egui::vec2(rail, area.height()));
    let right = egui::Rect::from_min_size(
        egui::pos2(area.right() - rail, area.top()),
        egui::vec2(rail, area.height()),
    );
    let center = egui::Rect::from_min_max(
        egui::pos2(left.right() + gap, area.top()),
        egui::pos2(right.left() - gap, area.bottom()),
    );
    (left, center, right)
}

/// Альфа канала, умноженная на проценты: 0 — прозрачно, 100 — как было.
fn scale_alpha(alpha: u8, percent: u8) -> u8 {
    ((alpha as u16 * percent.min(100) as u16 + 50) / 100) as u8
}

/// Альфа подложки панели из процентов: 0 — прозрачно, 100 — плотно.
fn panel_alpha(opacity: u8) -> u8 {
    scale_alpha(255, opacity)
}

/// Узлы сетки по одной оси. Края тоже узлы: стикер может свисать за
/// границу зоны, а его невидимая часть потом обрезается.
fn grid_nodes(start: f32, end: f32, step: f32) -> Vec<f32> {
    let step = step.max(1.0);
    let mut nodes = vec![start];
    let mut value = start + step;
    while value < end {
        nodes.push(value);
        value += step;
    }
    if end > start {
        nodes.push(end);
    }
    nodes
}

fn nearest_node(nodes: &[f32], value: f32) -> Option<f32> {
    nodes
        .iter()
        .copied()
        .min_by(|a, b| (a - value).abs().total_cmp(&(b - value).abs()))
}

fn clamp_sticker_center(panel: egui::Rect, pos: egui::Pos2) -> egui::Pos2 {
    // Центр не выходит за панель, но сам стикер может свисать за край
    // и обрезаться — поэтому здесь нет отступа на радиус картинки.
    egui::pos2(
        pos.x.clamp(panel.left(), panel.right()),
        pos.y.clamp(panel.top(), panel.bottom()),
    )
}

/// Магнит: ближайший узел по каждой оси. Если узлов нет, работает обычный clamp.
fn snap_to_grid(panel: egui::Rect, step: f32, pos: egui::Pos2) -> egui::Pos2 {
    let xs = grid_nodes(panel.left(), panel.right(), step);
    let ys = grid_nodes(panel.top(), panel.bottom(), step);
    match (nearest_node(&xs, pos.x), nearest_node(&ys, pos.y)) {
        (Some(x), Some(y)) => egui::pos2(x, y),
        _ => clamp_sticker_center(panel, pos),
    }
}

/// Куда рисовать стикер в текущем кадре.
///
/// `drag` задан только когда игрок держит стикер: сетка включается лишь в этот
/// момент. В покое позиция не зависит от сетки, её шага и включённости.
fn resolve_sticker_center(
    panel: egui::Rect,
    rest: egui::Pos2,
    drag: Option<egui::Pos2>,
    grid: Option<f32>,
) -> egui::Pos2 {
    match drag {
        Some(pointer) => match grid {
            Some(step) => snap_to_grid(panel, step, pointer),
            None => clamp_sticker_center(panel, pointer),
        },
        None => clamp_sticker_center(panel, rest),
    }
}

/// Редактирование доски стикеров доступно только на складе, на вкладке
/// «Стикеры». На остальных экранах стикеры — только оформление.
fn sticker_edit_mode(section: Section, tab: StockTab) -> bool {
    section == Section::Stock && tab == StockTab::Stickers
}

/// Поворот точки вокруг центра против часовой стрелки в экранных координатах.
fn rotate_around(center: egui::Pos2, point: egui::Pos2, angle: f32) -> egui::Pos2 {
    center + egui::emath::Rot2::from_angle(angle) * (point - center)
}

/// Позиции двух ручек: угол — размер, верхний кружок — поворот.
/// Ручки живут снаружи картинки, поэтому не мешают перетаскиванию. При этом
/// они всегда остаются внутри панели: иначе у верхнего края поворотную ручку
/// просто не было бы видно.
fn sticker_handle_positions(
    panel: egui::Rect,
    center: egui::Pos2,
    size: egui::Vec2,
    rotation: f32,
) -> (egui::Pos2, egui::Pos2) {
    let rect = egui::Rect::from_center_size(center, size);
    let clamp = |pos: egui::Pos2| {
        egui::pos2(
            pos.x.clamp(panel.left() + 12.0, panel.right() - 12.0),
            pos.y.clamp(panel.top() + 12.0, panel.bottom() - 12.0),
        )
    };
    let resize = rotate_around(center, rect.right_bottom() + egui::vec2(8.0, 8.0), rotation);
    let rotate = rotate_around(center, rect.center_top() + egui::vec2(0.0, -20.0), rotation);
    (clamp(resize), clamp(rotate))
}

/// Радиус ареола: по большей стороне картинки плюс четыре пикселя.
/// Диагональ здесь не подходит — у широких стикеров она даёт огромный круг.
fn sticker_aura_radius(size: egui::Vec2) -> f32 {
    size.x.max(size.y) / 2.0 + 4.0
}

/// Новый размер при перетаскивании угла: отношение расстояний до центра.
fn sticker_scale_from_drag(
    base_scale: f32,
    center: egui::Pos2,
    start_pointer: egui::Pos2,
    pointer: egui::Pos2,
) -> f32 {
    let start_distance = (start_pointer - center).length().max(8.0);
    let distance = (pointer - center).length();
    taskify_store::state::clamp_sticker_scale(base_scale * distance / start_distance)
}

/// Новый поворот при перетаскивании верхней ручки.
fn sticker_rotation_from_drag(
    base_rotation: f32,
    center: egui::Pos2,
    start_pointer: egui::Pos2,
    pointer: egui::Pos2,
) -> f32 {
    let start_angle = (start_pointer - center).angle();
    let angle = (pointer - center).angle();
    taskify_store::state::normalize_sticker_rotation(base_rotation + angle - start_angle)
}

fn sticker_grid(width: f32, height: f32, count: usize) -> (usize, usize, f32) {
    let count = count.max(1);
    let gap = 10.0;
    let pad = 14.0;
    let mut best = (1usize, count, 0.0f32);
    for cols in 1..=4.min(count) {
        let rows = count.div_ceil(cols);
        let by_width = (width - pad * 2.0 - gap * (cols.saturating_sub(1) as f32)) / cols as f32;
        let by_height = (height - pad * 2.0 - gap * (rows.saturating_sub(1) as f32)) / rows as f32;
        let diameter = by_width.min(by_height).min(64.0);
        if diameter > best.2 + 0.01 {
            best = (cols, rows, diameter);
        }
    }
    // Совсем маленькая панель: пусть лучше чуть вылезет, чем исчезнет.
    best.2 = best.2.max(24.0);
    best
}

impl TaskifyApp {
    /// Строит приложение по уже готовому контексту: нужно, чтобы перейти из
    /// экрана первого запуска в основной интерфейс без перезапуска окна.
    pub fn build(ctx: &egui::Context, settings: Settings) -> Self {
        install_fonts(ctx);

        let started = std::time::Instant::now();
        let (engine, startup_error) = match open_profile(&settings) {
            Ok(engine) => (engine, None),
            Err(error) => {
                let root = crate::bootstrap::data_root(&settings);
                crate::diag::investigate(&root);
                match open_fallback_profile() {
                    Ok(engine) => {
                        let text = format!("{error:#}");
                        crate::diag::write(&format!("работаем на запасном профиле: {text}"));
                        let hint = permission_hint(&text);
                        (engine, Some(format!("{text}{hint}")))
                    }
                    Err(fallback_error) => {
                        // Окна ещё нет: пишем след и выходим, без паники.
                        crate::diag::write(&format!("профиль не открылся: {error:#}"));
                        crate::diag::dump_acl(&root);
                        crate::console::message_box(&describe_startup_failure(
                            &error,
                            &fallback_error,
                        ));
                        std::process::exit(1);
                    }
                }
            }
        };
        let i18n = I18n::load(&settings.ui.locale);
        let draft = engine.settings().clone();
        let notifications = crate::notifications::Notifications::new(
            engine.store.livespace.notifications_history(),
        );
        // Отладочный режим: TASKIFY_SCREENSHOT=<путь> сохраняет кадр и закрывает окно.
        let screenshot = std::env::var_os("TASKIFY_SCREENSHOT").map(PathBuf::from);
        let settings_tab = match std::env::var("TASKIFY_SETTINGS_TAB").as_deref() {
            Ok("app") => SettingsTab::App,
            Ok("dev") => SettingsTab::Dev,
            _ => SettingsTab::Game,
        };
        let mut stats_tab = match std::env::var("TASKIFY_STATS_TAB").as_deref() {
            Ok("month") => StatsTab::Month,
            Ok("history") => StatsTab::History,
            _ => StatsTab::Week,
        };
        let section = match std::env::var("TASKIFY_SECTION").as_deref() {
            Ok("week") => {
                stats_tab = StatsTab::Week;
                Section::Statistics
            }
            Ok("month") => {
                stats_tab = StatsTab::Month;
                Section::Statistics
            }
            Ok("history") => {
                stats_tab = StatsTab::History;
                Section::Statistics
            }
            Ok("stats") | Ok("statistics") => Section::Statistics,
            Ok("packs") => Section::Packs,
            Ok("achievements") => Section::Achievements,
            Ok("market") => Section::Market,
            Ok("stock") => Section::Stock,
            Ok("settings") => Section::Settings,
            _ => Section::Today,
        };

        if settings.window.maximized {
            ctx.send_viewport_cmd(egui::ViewportCommand::Maximized(true));
        }
        if settings.window.fullscreen {
            ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(true));
        }

        if let Some(error) = &startup_error {
            eprintln!("taskify: {error}");
        } else {
            eprintln!(
                "taskify: профиль открыт за {} мс",
                started.elapsed().as_millis()
            );
        }

        let mut icons = crate::icons::Icons::load(ctx);
        icons.load_sticker_images(ctx, &engine.packs);
        let mut app = Self {
            engine,
            i18n,
            settings,
            section,
            about_open: false,
            toast: None,
            filters: Filters::default(),
            today_tab: match std::env::var("TASKIFY_TODAY_TAB").as_deref() {
                Ok("done") => TodayTab::Done,
                Ok("rejected") => TodayTab::Rejected,
                _ => TodayTab::Available,
            },
            wallpaper: None,
            look_profile_name: String::new(),
            overwrite_look_target: std::env::var("TASKIFY_OVERWRITE_LOOK")
                .ok()
                .filter(|name| !name.is_empty()),
            auto_look_name: None,
            section_anim_start: None,
            tab_anim_start: None,
            livespace_name: String::new(),
            livespace_rename: String::new(),
            packs_filter: match std::env::var("TASKIFY_PACKS_FILTER").as_deref() {
                Ok("enabled") => PacksFilter::Enabled,
                Ok("disabled") => PacksFilter::Disabled,
                _ => PacksFilter::All,
            },
            packs_query: std::env::var("TASKIFY_PACKS_QUERY").unwrap_or_default(),
            pack_detail: std::env::var("TASKIFY_PACK")
                .ok()
                .filter(|id| !id.is_empty()),
            pack_tab: match std::env::var("TASKIFY_PACK_TAB").as_deref() {
                Ok("combos") => PackTab::Combos,
                Ok("ach") | Ok("achievements") => PackTab::Achievements,
                Ok("categories") => PackTab::Categories,
                Ok("tags") => PackTab::Tags,
                Ok("types") => PackTab::Types,
                Ok("icons") => PackTab::Icons,
                Ok("palette") => PackTab::Palette,
                Ok("tokens") | Ok("cosmetics") => PackTab::Tokens,
                Ok("bundles") | Ok("themes") => PackTab::Bundles,
                Ok("stickers") => PackTab::Stickers,
                Ok("metrics") => PackTab::Metrics,
                Ok("info") => PackTab::Info,
                _ => PackTab::Tasks,
            },
            ach_pack: std::env::var("TASKIFY_ACH_PACK")
                .ok()
                .filter(|id| !id.is_empty()),
            ach_query: std::env::var("TASKIFY_ACH_QUERY").unwrap_or_default(),
            ach_suggest: 0,
            ach_state: match std::env::var("TASKIFY_ACH_STATE").as_deref() {
                Ok("unlocked") => AchState::Unlocked,
                Ok("locked") => AchState::Locked,
                _ => AchState::All,
            },
            ach_sort: match std::env::var("TASKIFY_ACH_SORT").as_deref() {
                Ok("percent") => AchSort::Percent,
                _ => AchSort::Name,
            },
            avatar: None,
            market_kind: match std::env::var("TASKIFY_MARKET_KIND").as_deref() {
                Ok("themes") => MarketKind::Themes,
                Ok("cosmetics") => MarketKind::Cosmetics,
                Ok("stickers") => MarketKind::Stickers,
                _ => MarketKind::All,
            },
            market_pack: std::env::var("TASKIFY_MARKET_PACK")
                .ok()
                .filter(|id| !id.is_empty()),
            stats_tab,
            stats_week_shift: 0,
            stats_month_shift: 0,
            settings_tab,
            stock_tab: match std::env::var("TASKIFY_STOCK_TAB").as_deref() {
                Ok("themes") | Ok("bundles") => StockTab::Themes,
                Ok("cosmetics") => StockTab::Cosmetics,
                Ok("stickers") => StockTab::Stickers,
                Ok("profiles") => StockTab::Profiles,
                _ => StockTab::Themes,
            },
            stock_cosmetic_group: std::env::var("TASKIFY_COSMETIC_GROUP")
                .unwrap_or_else(|_| "theme".to_string()),
            livespace_dialog: std::env::var("TASKIFY_LIVESPACE_DIALOG").as_deref() == Ok("1"),
            livespace_focus: std::env::var("TASKIFY_LIVESPACE_DIALOG").as_deref() == Ok("1"),
            livespace_error: String::new(),
            livespace_menu_force: std::env::var("TASKIFY_LIVESPACE_MENU").as_deref() == Ok("1"),
            livespace_pending: None,
            delete_stage: std::env::var("TASKIFY_DELETE_STAGE")
                .ok()
                .and_then(|value| value.parse().ok())
                .unwrap_or(0),
            delete_confirm_name: std::env::var("TASKIFY_DELETE_NAME").unwrap_or_default(),
            delete_error: String::new(),
            pack_reset_pending: None,
            pack_reset_achievements: false,
            history: HistoryState {
                query: std::env::var("TASKIFY_HIST_QUERY").unwrap_or_default(),
                limit: 300,
                ..HistoryState::default()
            },
            status: String::new(),
            draft,
            startup_error,
            accent: crate::theme::Palette::light().accent,
            icons,
            notifications,
            last_new_tasks_check: None,
            screenshot,
            screenshot_requested: false,
            focus_search: std::env::var_os("TASKIFY_FOCUS_SEARCH").is_some(),
            scroll_velocity: egui::Vec2::ZERO,
            subtasks_open: std::collections::BTreeSet::new(),
            layout_debug: std::env::var_os("TASKIFY_LAYOUT_DEBUG").is_some(),
            card_anim: std::collections::HashMap::new(),
            card_heights: std::collections::HashMap::new(),
            sticker_transform: None,
        };
        app.apply_look(ctx);
        app.reload_avatar(ctx);
        app.livespace_rename = app.livespace_display_name();
        app
    }

    pub fn t(&self, key: &str) -> String {
        self.i18n.t(key)
    }

    /// Подпись базы темы: «светлая» или «тёмная».
    pub(crate) fn base_label(&self, base: &str) -> String {
        if base == "dark" {
            self.t("appearance.base_dark")
        } else {
            self.t("appearance.base_light")
        }
    }

    pub fn toast(&mut self, text: impl Into<String>, ctx: &egui::Context) {
        let now = ctx.input(|i| i.time);
        self.toast = Some(Toast {
            text: text.into(),
            until: now + 4.0,
            shown_at: now,
        });
    }

    pub fn set_status(&mut self, text: impl Into<String>) {
        self.status = text.into();
    }

    fn toggle_fullscreen(&mut self, ctx: &egui::Context) {
        self.settings.window.fullscreen = !self.settings.window.fullscreen;
        ctx.send_viewport_cmd(egui::ViewportCommand::Fullscreen(
            self.settings.window.fullscreen,
        ));
    }

    fn capture_geometry(&mut self, ctx: &egui::Context) {
        let (outer_rect, maximized, fullscreen) = ctx.input(|i| {
            let viewport = i.viewport();
            (viewport.outer_rect, viewport.maximized, viewport.fullscreen)
        });
        if let Some(rect) = outer_rect {
            self.settings.window.width = rect.width();
            self.settings.window.height = rect.height();
            self.settings.window.x = Some(rect.min.x);
            self.settings.window.y = Some(rect.min.y);
        }
        self.settings.window.maximized = maximized.unwrap_or(false);
        self.settings.window.fullscreen = fullscreen.unwrap_or(false);
    }

    fn header(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("header")
            .exact_height(100.0)
            .show(ctx, |ui| {
                ui.add_space(6.0);
                // Первая строка: аватар лайвспейса и его имя слева, часы справа.
                ui.horizontal(|ui| {
                    let livespace = self.livespace_display_name();
                    // Аватар — это и есть отображение лайвспейса: по клику
                    // открывается список аккаунтов.
                    let avatar = if let Some(id) = self.avatar.as_ref().map(|texture| texture.id())
                    {
                        ui.add(
                            egui::Button::image(
                                egui::Image::new(egui::load::SizedTexture::new(
                                    id,
                                    egui::vec2(30.0, 30.0),
                                ))
                                .corner_radius(15),
                            )
                            .frame(false),
                        )
                    } else {
                        // Без картинки — кружок с первой буквой имени.
                        let (rect, response) =
                            ui.allocate_exact_size(egui::vec2(30.0, 30.0), egui::Sense::click());
                        ui.painter().circle_filled(
                            rect.center(),
                            15.0,
                            self.accent.gamma_multiply(0.22),
                        );
                        let letter = livespace
                            .chars()
                            .next()
                            .map(|ch| ch.to_uppercase().to_string())
                            .unwrap_or_else(|| "?".to_string());
                        ui.painter().text(
                            rect.center(),
                            egui::Align2::CENTER_CENTER,
                            letter,
                            egui::FontId::proportional(16.0),
                            self.palette().text,
                        );
                        response
                    };
                    let avatar = avatar.on_hover_text(self.t("livespace.tip"));
                    self.livespace_menu(&avatar, false);
                    ui.add_space(8.0);
                    let name = ui
                        .add(
                            egui::Button::new(egui::RichText::new(&livespace).size(18.0).strong())
                                .frame(false),
                        )
                        .on_hover_text(self.t("livespace.tip"));
                    self.livespace_menu(&name, true);
                    ui.label(egui::RichText::new(self.rank_title()).small().weak());
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let now = chrono::Local::now();
                        ui.label(
                            egui::RichText::new(now.format("%H:%M:%S").to_string())
                                .size(18.0)
                                .strong(),
                        );
                    });
                });
                ui.add_space(6.0);
                // Вторая строка: уровень и монеты слева, дни недели справа.
                ui.horizontal(|ui| {
                    let level = self.engine.store.state.level;
                    ui.label(
                        egui::RichText::new(format!("Lv {level}"))
                            .size(16.0)
                            .strong()
                            .color(self.accent),
                    );
                    let (current, needed) = self.level_progress();
                    let fraction = if needed > 0 {
                        current as f32 / needed as f32
                    } else {
                        1.0
                    };
                    let left = (needed - current).max(0);
                    ui.add(
                        egui::ProgressBar::new(fraction.clamp(0.0, 1.0))
                            .desired_width(180.0)
                            .text(format!("{current} / {needed}")),
                    )
                    .on_hover_text(format!(
                        "{}: {left} {}",
                        self.t("header.left"),
                        self.t("common.xp")
                    ));

                    ui.add_space(6.0);
                    self.icons.show(ui, "ui.coin", 18.0);
                    ui.label(
                        egui::RichText::new(self.engine.store.state.coins.to_string())
                            .size(16.0)
                            .strong()
                            .color(self.palette().coin),
                    )
                    .on_hover_text(self.t("header.coins"));

                    // Дни растягиваются на всю оставшуюся ширину строки.
                    let rect = ui.available_rect_before_wrap();
                    if rect.width() >= 7.0 * 70.0 {
                        let bar_height = 42.0;
                        let top = rect.top() + ((rect.height() - bar_height) * 0.5).max(0.0);
                        let bar = egui::Rect::from_min_max(
                            egui::pos2(rect.left() + 10.0, top),
                            egui::pos2(rect.right(), top + bar_height),
                        );
                        let mut child = ui.new_child(
                            egui::UiBuilder::new()
                                .max_rect(bar)
                                .layout(egui::Layout::left_to_right(egui::Align::TOP)),
                        );
                        self.week_strip(&mut child);
                        ui.allocate_rect(bar, egui::Sense::hover());
                    }
                });
            });
        // Часы идут даже без действий пользователя.
        ctx.request_repaint_after(std::time::Duration::from_secs(1));
    }

    fn navbar(&mut self, ctx: &egui::Context) {
        egui::TopBottomPanel::top("navbar")
            .exact_height(56.0)
            .show(ctx, |ui| {
                // Кнопки центрируются по высоте панели: одинаковый отступ
                // сверху и снизу, без «висящей» полосы над вкладками.
                let button_height = 34.0;
                let top_gap = ((ui.available_height() - button_height) * 0.5).max(0.0);
                ui.add_space(top_gap);
                let spacing = ui.spacing().item_spacing.x;
                let buttons = Section::ALL.len();
                let width = ((ui.available_width() - spacing * (buttons as f32 - 1.0))
                    / buttons as f32)
                    .max(68.0);
                if self.layout_debug {
                    eprintln!(
                        "navbar gap {:.1} height {:.1}",
                        top_gap,
                        ui.available_height()
                    );
                }
                ui.horizontal(|ui| {
                    for section in Section::ALL {
                        let selected = self.section == section;
                        let text = self.t(section.key());
                        let button = match self.icons.image(ui, section.icon_key(), 16.0, None) {
                            Some(image) => egui::Button::image_and_text(image, text),
                            None => egui::Button::new(text),
                        };
                        if ui
                            .add_sized([width, button_height], button.selected(selected))
                            .clicked()
                        {
                            self.switch_section(section, ctx);
                            self.status.clear();
                        }
                    }
                });
            });
    }

    /// Панель стикеров: полупрозрачная подложка без подписи, на ней лежат
    /// стикеры как оформление и награды.
    ///
    /// Панель видна на всех экранах, поэтому стикер работает как награда-
    /// украшение. Один и тот же стикер может висеть и слева, и справа.
    fn sticker_rail(&mut self, ui: &mut egui::Ui, left: bool) {
        let editing = sticker_edit_mode(self.section, self.stock_tab);
        let side = if left { "left" } else { "right" };
        let size = egui::vec2(ui.available_width(), ui.available_height());
        let (panel, _) = ui.allocate_exact_size(size, egui::Sense::hover());
        let visuals = ui.visuals().widgets.noninteractive;
        // Панель полупрозрачная: обои должны читаться как фон и под стикерами.
        // Плотность подложки настраивается на складе, по умолчанию 59 %.
        let fill = ui.visuals().faint_bg_color;
        let opacity = self.settings.ui.sticker_panel_opacity;
        let border = visuals.bg_stroke;
        let border_color = border.color;
        let border = egui::Stroke::new(
            border.width,
            egui::Color32::from_rgba_unmultiplied(
                border_color.r(),
                border_color.g(),
                border_color.b(),
                scale_alpha(border_color.a(), opacity),
            ),
        );
        let fill = egui::Color32::from_rgba_unmultiplied(
            fill.r(),
            fill.g(),
            fill.b(),
            panel_alpha(opacity),
        );
        ui.painter().rect(
            panel,
            egui::CornerRadius::same(12),
            fill,
            border,
            egui::StrokeKind::Inside,
        );
        let grid_step = self.settings.ui.sticker_grid_size.clamp(8.0, 256.0);
        if editing && self.settings.ui.sticker_grid {
            // Сетка с магнитом: линии и узлы — одно и то же. Центр стикера
            // притягивается к ближайшему узлу, где он целиком влезает.
            let weak = ui.visuals().weak_text_color();
            let color = egui::Color32::from_rgba_unmultiplied(weak.r(), weak.g(), weak.b(), 45);
            let node_color =
                egui::Color32::from_rgba_unmultiplied(weak.r(), weak.g(), weak.b(), 90);
            let stroke = egui::Stroke::new(1.0_f32, color);
            let painter = ui.painter().with_clip_rect(panel.shrink(1.0));
            let mut x = panel.left() + grid_step;
            while x < panel.right() {
                painter.vline(x, panel.top()..=panel.bottom(), stroke);
                x += grid_step;
            }
            let mut y = panel.top() + grid_step;
            while y < panel.bottom() {
                painter.hline(panel.left()..=panel.right(), y, stroke);
                y += grid_step;
            }
            for x in grid_nodes(panel.left(), panel.right(), grid_step) {
                for y in grid_nodes(panel.top(), panel.bottom(), grid_step) {
                    painter.circle_filled(egui::pos2(x, y), 1.5, node_color);
                }
            }
        }
        let grid_enabled = editing && self.settings.ui.sticker_grid;

        // Копируем позиции: дальше понадобится &mut self для перетаскивания.
        let mut slots: Vec<(String, bool, f32, f32, f32, f32)> = self
            .engine
            .store
            .state
            .stickers
            .iter()
            .filter(|(_, placement)| placement.side == side)
            .map(|(key, placement)| {
                (
                    key.clone(),
                    placement.moved,
                    placement.x,
                    placement.y,
                    placement.scale,
                    placement.rotation,
                )
            })
            .collect();
        slots.sort_by(|a, b| a.3.total_cmp(&b.3).then_with(|| a.0.cmp(&b.0)));
        if slots.is_empty() {
            return;
        }

        // Стикеры рисуются только внутри своей панели: всё, что свисает
        // за край, обрезается, но координата центра может быть на краю.
        ui.set_clip_rect(ui.clip_rect().intersect(panel));
        let locale = self.settings.ui.locale.clone();
        let accent = self.accent;
        let text_color = ui.visuals().text_color();
        let gap = 10.0;
        let pad = 14.0;
        // Нетронутые стикеры лежат плиткой; перетащенные — там, куда их
        // положили. Так старые записи сразу выглядят сеткой, а не столбцом.
        let auto_count = slots.iter().filter(|slot| !slot.1).count();
        let (cols, rows, auto_diameter) =
            sticker_grid(panel.width(), panel.height(), auto_count.max(1));
        let diameter = if auto_count == 0 { 56.0 } else { auto_diameter };
        let cols = cols.max(1);
        let grid_w = cols as f32 * diameter + gap * (cols.saturating_sub(1) as f32);
        let grid_h = rows as f32 * diameter + gap * (rows.saturating_sub(1) as f32);
        let x0 = panel.center().x - grid_w / 2.0;
        let y0 = panel.top() + ((panel.height() - grid_h) / 2.0).max(pad);
        let grid = grid_enabled.then_some(grid_step);
        let drag_hint = self.t("stickers.drag");
        let mut auto_index = 0usize;
        let mut removed: Option<String> = None;
        let mut moved_target: Option<(String, f32, f32)> = None;
        let mut transform_target: Option<(String, f32, f32)> = None;

        for (key, moved, saved_x, saved_y, saved_scale, saved_rotation) in &slots {
            let Some((_, id)) = key.split_once(':') else {
                continue;
            };
            let Some(sticker) = crate::stickers::find(&self.engine.packs, id) else {
                continue;
            };
            // Пока игрок тянет ручку, показываем живой размер/поворот, но в
            // журнал пишем только на отпускании — иначе один жест создаст
            // десятки событий.
            let gesture = self
                .sticker_transform
                .as_ref()
                .filter(|gesture| &gesture.slot == key)
                .cloned();
            let pointer = ui
                .ctx()
                .pointer_interact_pos()
                .or_else(|| ui.ctx().pointer_latest_pos());
            let mut live_scale = *saved_scale;
            let mut live_rotation = *saved_rotation;
            if let (Some(gesture), Some(pointer)) = (&gesture, pointer) {
                match gesture.mode {
                    StickerTransformMode::Scale => {
                        live_scale = sticker_scale_from_drag(
                            gesture.base_scale,
                            gesture.center,
                            gesture.start_pointer,
                            pointer,
                        );
                    }
                    StickerTransformMode::Rotate => {
                        live_rotation = sticker_rotation_from_drag(
                            gesture.base_rotation,
                            gesture.center,
                            gesture.start_pointer,
                            pointer,
                        );
                    }
                }
            }
            // Своя PNG-картинка сохраняет пропорции: гусь может быть широким,
            // кактус — высоким, а не обязан вписываться в круг.
            let max_side = diameter * sticker.display_scale() * live_scale;
            let art_size = sticker
                .image_path()
                .and_then(|path| self.icons.file_fit(path, max_side))
                .unwrap_or_else(|| egui::vec2(max_side, max_side));
            let rest = if *moved {
                egui::pos2(
                    panel.left() + saved_x * panel.width(),
                    panel.top() + saved_y * panel.height(),
                )
            } else {
                let col = auto_index % cols;
                let row = auto_index / cols;
                auto_index += 1;
                egui::pos2(
                    x0 + col as f32 * (diameter + gap) + diameter / 2.0,
                    y0 + row as f32 * (diameter + gap) + diameter / 2.0,
                )
            };
            let mut center = resolve_sticker_center(panel, rest, None, grid);
            let mut rect = egui::Rect::from_center_size(center, art_size);
            let hit = rect.intersect(panel);
            let (resize_hint, rotate_hint) =
                sticker_handle_positions(panel, center, rect.size(), live_rotation);
            let pointer_on_handle = pointer.is_some_and(|pointer| {
                egui::Rect::from_center_size(resize_hint, egui::vec2(22.0, 22.0)).contains(pointer)
                    || egui::Rect::from_center_size(rotate_hint, egui::vec2(22.0, 22.0))
                        .contains(pointer)
            });
            let sense = if editing && !pointer_on_handle {
                egui::Sense::click_and_drag()
            } else if editing {
                egui::Sense::click()
            } else {
                egui::Sense::hover()
            };
            let response = ui.interact(hit, ui.id().with(("sticker_rail", side, id)), sense);
            let released = editing && response.drag_stopped();
            if editing {
                if response.dragged() {
                    if let Some(pointer) = ui.ctx().pointer_interact_pos() {
                        center = resolve_sticker_center(panel, center, Some(pointer), grid);
                        rect = egui::Rect::from_center_size(center, art_size);
                    }
                }
                if released {
                    // В кадре отпускания response.dragged() уже false. Нельзя
                    // брать rest: иначе стикер мигнёт на старой точке, прежде
                    // чем сохранится новая. Сразу ставим его под указатель.
                    let pointer = ui
                        .ctx()
                        .pointer_latest_pos()
                        .or_else(|| ui.ctx().pointer_interact_pos())
                        .unwrap_or(center);
                    center = resolve_sticker_center(panel, center, Some(pointer), grid);
                    rect = egui::Rect::from_center_size(center, art_size);
                    let x = ((center.x - panel.left()) / panel.width()).clamp(0.0, 1.0);
                    let y = ((center.y - panel.top()) / panel.height()).clamp(0.0, 1.0);
                    moved_target = Some((id.to_string(), x, y));
                }
                if response.dragged() || released {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::Grabbing);
                } else if response.hovered() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::Grab);
                }
                // Ареол — признак редактируемой доски; вне склада его нет.
                // Считаем по большей стороне, а не по диагонали: у широкого
                // гуся диагональ раздувала круг почти вдвое.
                let fill = if response.dragged() || released || response.hovered() {
                    accent.gamma_multiply(0.55)
                } else {
                    accent.gamma_multiply(0.35)
                };
                ui.painter()
                    .circle_filled(center, sticker_aura_radius(rect.size()), fill);
            }
            let painted_file = sticker
                .image_path()
                .is_some_and(|path| self.icons.paint_file_rotated(ui, path, rect, live_rotation));
            if !painted_file {
                self.icons.paint_rotated(
                    ui,
                    sticker.icon_key(),
                    rect.shrink(art_size.x.min(art_size.y) * 0.22),
                    text_color,
                    live_rotation,
                );
            }
            let title = sticker.title(&locale);
            let hint = if editing {
                format!("{title}\n{drag_hint}\n{}", self.t("stickers.transform"))
            } else {
                title
            };
            response.clone().on_hover_text(hint);
            if editing
                && (response.hovered()
                    || response.dragged()
                    || released
                    || gesture.is_some()
                    || pointer.is_some_and(|pointer| {
                        egui::Rect::from_center_size(resize_hint, egui::vec2(22.0, 22.0))
                            .contains(pointer)
                            || egui::Rect::from_center_size(rotate_hint, egui::vec2(22.0, 22.0))
                                .contains(pointer)
                    }))
            {
                let (resize_pos, rotate_pos) =
                    sticker_handle_positions(panel, center, rect.size(), live_rotation);
                let top = rotate_around(center, rect.center_top(), live_rotation);
                let stroke = egui::Stroke::new(1.5_f32, accent);
                let corners = [
                    rect.left_top(),
                    rect.right_top(),
                    rect.right_bottom(),
                    rect.left_bottom(),
                ]
                .map(|point| rotate_around(center, point, live_rotation));
                ui.painter()
                    .add(egui::Shape::closed_line(corners.to_vec(), stroke));
                ui.painter().line_segment([top, rotate_pos], stroke);

                let handle_fill = ui.visuals().extreme_bg_color;
                ui.painter().circle_filled(rotate_pos, 7.0, handle_fill);
                ui.painter().circle_stroke(rotate_pos, 7.0, stroke);
                let resize_rect = egui::Rect::from_center_size(resize_pos, egui::vec2(12.0, 12.0));
                ui.painter().rect_filled(resize_rect, 3, handle_fill);
                ui.painter()
                    .rect_stroke(resize_rect, 3, stroke, egui::StrokeKind::Inside);

                let resize_response = ui.interact(
                    egui::Rect::from_center_size(resize_pos, egui::vec2(22.0, 22.0)),
                    ui.id().with(("sticker_resize", side, id)),
                    egui::Sense::drag(),
                );
                let rotate_response = ui.interact(
                    egui::Rect::from_center_size(rotate_pos, egui::vec2(22.0, 22.0)),
                    ui.id().with(("sticker_rotate", side, id)),
                    egui::Sense::drag(),
                );

                if resize_response.drag_started() {
                    self.sticker_transform = Some(StickerTransformDrag {
                        slot: key.clone(),
                        mode: StickerTransformMode::Scale,
                        start_pointer: ui.ctx().pointer_interact_pos().unwrap_or(resize_pos),
                        center,
                        base_scale: *saved_scale,
                        base_rotation: *saved_rotation,
                    });
                }
                if rotate_response.drag_started() {
                    self.sticker_transform = Some(StickerTransformDrag {
                        slot: key.clone(),
                        mode: StickerTransformMode::Rotate,
                        start_pointer: ui.ctx().pointer_interact_pos().unwrap_or(rotate_pos),
                        center,
                        base_scale: *saved_scale,
                        base_rotation: *saved_rotation,
                    });
                }
                if resize_response.hovered() || resize_response.dragged() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::ResizeNwSe);
                }
                if rotate_response.hovered() || rotate_response.dragged() {
                    ui.ctx().set_cursor_icon(egui::CursorIcon::Crosshair);
                }
                if resize_response.drag_stopped() {
                    if let Some(gesture) = self.sticker_transform.take() {
                        let pointer = ui
                            .ctx()
                            .pointer_latest_pos()
                            .or_else(|| ui.ctx().pointer_interact_pos())
                            .unwrap_or(resize_pos);
                        let scale = sticker_scale_from_drag(
                            gesture.base_scale,
                            gesture.center,
                            gesture.start_pointer,
                            pointer,
                        );
                        transform_target = Some((id.to_string(), scale, gesture.base_rotation));
                    }
                }
                if rotate_response.drag_stopped() {
                    if let Some(gesture) = self.sticker_transform.take() {
                        let pointer = ui
                            .ctx()
                            .pointer_latest_pos()
                            .or_else(|| ui.ctx().pointer_interact_pos())
                            .unwrap_or(rotate_pos);
                        let rotation = sticker_rotation_from_drag(
                            gesture.base_rotation,
                            gesture.center,
                            gesture.start_pointer,
                            pointer,
                        );
                        transform_target = Some((id.to_string(), gesture.base_scale, rotation));
                    }
                }
            }
            if editing {
                response.context_menu(|ui| {
                    if ui.button(self.t("stickers.reset")).clicked() {
                        transform_target = Some((id.to_string(), 1.0, 0.0));
                        ui.close();
                    }
                    if ui.button(self.t("stickers.remove")).clicked() {
                        removed = Some(id.to_string());
                        ui.close();
                    }
                });
            }
        }

        if let Some(id) = removed {
            if let Err(error) = self.engine.remove_sticker(&id, side) {
                self.set_status(format!("{error:#}"));
            }
        }
        if let Some((id, x, y)) = moved_target {
            if let Err(error) = self.engine.move_sticker(&id, side, x, y) {
                self.set_status(format!("{error:#}"));
            }
        }
        if let Some((id, scale, rotation)) = transform_target {
            if let Err(error) = self.engine.transform_sticker(&id, side, scale, rotation) {
                self.set_status(format!("{error:#}"));
            }
        }
    }

    /// Полоса дней недели: ширина равна центральной панели, высота — по месту.
    /// Полоса дней недели под центром: семь карточек с итогами дня.
    /// Дни недели таблицей 2×2: день и дата, опыт и число заданий.
    ///
    /// Ячейки рисуются вручную в прямоугольниках фиксированного размера —
    /// иначе текст распирает рамку и полоса уезжает за правый край шапки.
    fn week_strip(&mut self, ui: &mut egui::Ui) {
        let today = self.engine.today();
        let settings = self.engine.settings().clone();
        let rules =
            taskify_core::time::DayRules::new(settings.day_start_hour, settings.week_starts_monday);
        let start = rules.week_start(today);
        let accent = self.accent;
        let border = ui.visuals().widgets.noninteractive.bg_stroke.color;
        let fill = ui.visuals().extreme_bg_color;
        let text_color = ui.visuals().text_color();
        let weak = ui.visuals().weak_text_color();

        let cell_height = 42.0;
        let gap = 4.0;
        let cell_width = ((ui.available_width() - gap * 6.0) / 7.0).max(56.0);
        let font = crate::theme::MIN_FONT_SIZE;
        ui.spacing_mut().item_spacing.x = gap;

        ui.horizontal(|ui| {
            for offset in 0..7u64 {
                let day = start + chrono::Days::new(offset);
                let is_today = day == today;
                let agg = self
                    .engine
                    .store
                    .aggregates
                    .by_day
                    .get(&day)
                    .cloned()
                    .unwrap_or_default();
                let (rect, response) = ui
                    .allocate_exact_size(egui::vec2(cell_width, cell_height), egui::Sense::hover());
                let stroke = if is_today {
                    egui::Stroke::new(1.5_f32, accent)
                } else {
                    egui::Stroke::new(1.0_f32, border)
                };
                ui.painter().rect(
                    rect,
                    egui::CornerRadius::same(8),
                    fill,
                    stroke,
                    egui::StrokeKind::Inside,
                );

                // Все значения — одной строкой по центру ячейки.
                let weekday = self.t(day_key(day.weekday()));
                let date = day.format("%d.%m").to_string();
                let mut text = format!("{weekday} {date}");
                if cell_width >= 84.0 {
                    text.push_str(&format!(" · {} · {}", agg.xp, agg.completed));
                }
                ui.painter().text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    text,
                    egui::FontId::proportional(font),
                    if is_today { accent } else { text_color },
                );
                if cell_width < 84.0 {
                    // Совсем узкая ячейка: цифры уходят в подсказку.
                    ui.painter().text(
                        rect.center_bottom() + egui::vec2(0.0, -4.0),
                        egui::Align2::CENTER_BOTTOM,
                        format!("{}·{}", agg.xp, agg.completed),
                        egui::FontId::proportional(12.0),
                        weak,
                    );
                }

                if self.layout_debug {
                    eprintln!(
                        "week cell {:.1} {:.1} {:.1} {:.1}",
                        rect.min.x,
                        rect.min.y,
                        rect.width(),
                        rect.height()
                    );
                }
                response.on_hover_text(format!(
                    "{}: {} · {}: {}",
                    self.t("header.tip_xp"),
                    agg.xp,
                    self.t("header.tip_done"),
                    agg.completed
                ));
            }
        });
    }

    /// Один шаг инерционной прокрутки: импульс колеса и затухание скорости.
    fn scroll_step(velocity: &mut egui::Vec2, impulse: Option<egui::Vec2>, dt: f32) -> egui::Vec2 {
        const TAU: f32 = 0.11;
        const MAX_SPEED: f32 = 6000.0;
        if let Some(raw) = impulse {
            *velocity += raw / TAU;
            *velocity = velocity.clamp(egui::Vec2::splat(-MAX_SPEED), egui::Vec2::splat(MAX_SPEED));
        }
        if *velocity == egui::Vec2::ZERO {
            return egui::Vec2::ZERO;
        }
        // Точный интеграл экспоненциального затухания: сумма шагов
        // совпадает с импульсом колеса, без перелёта на высоком FPS.
        let decay = (-dt / TAU).exp();
        let step = *velocity * TAU * (1.0 - decay);
        *velocity *= decay;
        if velocity.length() < 0.5 {
            *velocity = egui::Vec2::ZERO;
        }
        step
    }

    /// Сглаживает колесо в инерцию: одно событие не прыгает на весь шаг,
    /// а разгоняет прокрутку, которая плавно затухает между событиями.
    fn drive_scroll(&mut self, ctx: &egui::Context) {
        let (raw, dt, discrete) = ctx.input(|input| {
            let discrete = input.events.iter().any(|event| {
                matches!(
                    event,
                    egui::Event::MouseWheel {
                        unit: egui::MouseWheelUnit::Line | egui::MouseWheelUnit::Page,
                        ..
                    }
                )
            });
            (input.raw_scroll_delta, input.stable_dt.min(0.05), discrete)
        });
        let impulse = (raw != egui::Vec2::ZERO && discrete).then_some(raw);
        if impulse.is_none() && self.scroll_velocity == egui::Vec2::ZERO {
            return;
        }
        let step = Self::scroll_step(&mut self.scroll_velocity, impulse, dt);
        ctx.input_mut(|input| input.smooth_scroll_delta = step);
        if self.scroll_velocity != egui::Vec2::ZERO {
            ctx.request_repaint();
        }
    }

    fn body(&mut self, ctx: &egui::Context) {
        if let Some((_, texture)) = &self.wallpaper {
            // Обои — именно фон под контентом: рисуем их непрозрачно в той
            // области, что осталась под шапкой, и обрезаем по центру, а не
            // растягиваем. Раньше картинка была полупрозрачной и уходила под
            // верхние панели, из-за чего казалась слоем поверх интерфейса.
            let rect = ctx.available_rect();
            let tex = texture.size_vec2();
            let scale = (rect.width() / tex.x).max(rect.height() / tex.y);
            let size = tex * scale;
            let uv_w = (rect.width() / size.x).clamp(0.0, 1.0);
            let uv_h = (rect.height() / size.y).clamp(0.0, 1.0);
            let u0 = (1.0 - uv_w) / 2.0;
            let v0 = (1.0 - uv_h) / 2.0;
            ctx.layer_painter(egui::LayerId::background()).image(
                texture.id(),
                rect,
                egui::Rect::from_min_max(egui::pos2(u0, v0), egui::pos2(u0 + uv_w, v0 + uv_h)),
                egui::Color32::WHITE,
            );
        }
        let mut frame = egui::Frame::central_panel(&ctx.style());
        // Боковые панели со стикерами прибиты к краям окна, поэтому боковых
        // отступов у центральной панели нет. Верхний и нижний отступы
        // оставляем для обычного содержимого.
        frame.inner_margin = egui::Margin {
            left: 0,
            right: 0,
            top: 8,
            bottom: 8,
        };
        if self.wallpaper.is_some() {
            frame.fill = egui::Color32::TRANSPARENT;
        }
        let rail_inset = frame.inner_margin.bottom as f32;
        egui::CentralPanel::default().frame(frame).show(ctx, |ui| {
            if let Some(error) = self.startup_error.clone() {
                ui.colored_label(
                    self.palette().danger,
                    format!("Профиль не открылся: {error}"),
                );
                ui.separator();
            }
            let gap = 10.0;
            // Панели прибиты к краям окна, центральная колонка забирает ровно
            // остаток и растягивается вместе с окном. Рейки рисуем первыми и
            // в абсолютных прямоугольниках: ширина контента в центре больше
            // не может сдвинуть правую панель.
            let area = ui.available_rect_before_wrap();
            let (left_rect, center_rect, right_rect) = shell_columns(area, 190.0, gap, rail_inset);
            let content_height = (center_rect.height() - 8.0).max(120.0);

            ui.scope_builder(
                egui::UiBuilder::new()
                    .max_rect(left_rect)
                    .layout(egui::Layout::top_down(egui::Align::LEFT)),
                |ui| self.sticker_rail(ui, true),
            );
            ui.scope_builder(
                egui::UiBuilder::new()
                    .max_rect(right_rect)
                    .layout(egui::Layout::top_down(egui::Align::LEFT)),
                |ui| self.sticker_rail(ui, false),
            );
            ui.scope_builder(
                egui::UiBuilder::new()
                    .max_rect(center_rect)
                    .layout(egui::Layout::top_down(egui::Align::LEFT)),
                |ui| {
                    ui.set_clip_rect(center_rect.intersect(ui.clip_rect()));
                    ui.allocate_ui_with_layout(
                        egui::vec2(center_rect.width(), content_height),
                        egui::Layout::top_down(egui::Align::LEFT),
                        |ui| {
                            let progress = self.section_progress(ui.ctx());
                            if let Section::Today = self.section {
                                Self::transition(ui, progress);
                                self.screen_today(ui);
                                return;
                            }
                            egui::ScrollArea::vertical().show(ui, |ui| {
                                // Полоса прокрутки живёт в отступе справа: карточки
                                // до неё не доходят, поэтому она ничего не перекрывает
                                // и не сдвигает при появлении.
                                let gutter = 14.0;
                                let content_width = (ui.available_width() - gutter).max(160.0);
                                ui.set_max_width(content_width);
                                Self::transition(ui, progress);
                                match self.section {
                                    Section::Today => {}
                                    Section::Statistics => self.screen_stats(ui),
                                    Section::Packs => self.screen_packs(ui),
                                    Section::Achievements => self.screen_achievements(ui),
                                    Section::Market => self.screen_market(ui),
                                    Section::Stock => self.screen_stock(ui),
                                    Section::Settings => self.screen_settings(ui),
                                }
                            });
                        },
                    );
                },
            );
        });
    }

    /// Название профиля оформления для интерфейса: встроенные локализуются.
    pub fn look_profile_title(&self, name: &str) -> String {
        match name {
            crate::settings::BASE_PROFILE_LIGHT => self.t("stock.base_light"),
            crate::settings::BASE_PROFILE_DARK => self.t("stock.base_dark"),
            _ => name.to_string(),
        }
    }

    /// Название текущего лайвспейса для интерфейса.
    pub fn livespace_display_name(&self) -> String {
        self.engine.store.livespace.name()
    }

    /// Сохраняет новое имя текущего лайвспейса.
    pub fn rename_livespace(&mut self, name: &str, ctx: &egui::Context) {
        let name = name.trim();
        if name.is_empty() {
            return;
        }
        if name == self.livespace_display_name() {
            return;
        }
        match self.engine.store.livespace.rename(name) {
            Ok(()) => ctx.request_repaint(),
            Err(error) => {
                self.set_status(format!("не удалось переименовать лайвспейс: {error}"));
                self.livespace_rename = self.livespace_display_name();
            }
        }
    }

    fn rank_title(&self) -> String {
        let level = self.engine.store.state.level;
        let key = if level >= 200 {
            "rank.legend"
        } else if level >= 50 {
            "rank.master"
        } else if level >= 15 {
            "rank.practitioner"
        } else if level >= 5 {
            "rank.apprentice"
        } else {
            "rank.novice"
        };
        self.t(key)
    }

    pub fn level_progress(&self) -> (i64, i64) {
        let state = &self.engine.store.state;
        let level = state.level.max(1);
        let next = taskify_core::level::xp_to_reach(level.saturating_add(1));
        let base = taskify_core::level::xp_to_reach(level);
        let current = (state.xp_total - base as i64).max(0);
        let needed = (next as i64 - base as i64).max(1);
        (current.min(needed), needed)
    }

    pub fn toast_bar(&mut self, ctx: &egui::Context, bottom_offset: f32) {
        let now = ctx.input(|i| i.time);
        let mut expired = false;
        if let Some(toast) = &self.toast {
            if now > toast.until {
                expired = true;
            }
        }
        if expired {
            self.toast = None;
        }
        let undo_left = self.engine.undo_seconds_left();
        if self.toast.is_none() && undo_left.is_none() && self.status.is_empty() {
            return;
        }
        // Тост всплывает снизу и проявляется; перед исчезновением гаснет.
        let alpha = self
            .toast
            .as_ref()
            .map(|toast| {
                let ease = |t: f32| t * t * (3.0 - 2.0 * t);
                let appearing = ease(((now - toast.shown_at) / 0.16).clamp(0.0, 1.0) as f32);
                let leaving = ease(((toast.until - now) / 0.22).clamp(0.0, 1.0) as f32);
                appearing.min(leaving)
            })
            .unwrap_or(1.0);
        if alpha < 0.999 {
            ctx.request_repaint_after(std::time::Duration::from_millis(16));
        }
        let slide = (1.0 - alpha) * 22.0;
        egui::Area::new(egui::Id::new("toast_area"))
            .anchor(
                egui::Align2::CENTER_BOTTOM,
                egui::vec2(0.0, -bottom_offset + slide),
            )
            .show(ctx, |ui| {
                egui::Frame::popup(ui.style())
                    .corner_radius(egui::CornerRadius::same(14))
                    .show(ui, |ui| {
                        ui.multiply_opacity(alpha);
                        ui.horizontal(|ui| {
                            if let Some(toast) = &self.toast {
                                ui.label(egui::RichText::new(&toast.text).strong());
                            }
                            if !self.status.is_empty() {
                                ui.label(&self.status);
                            }
                            if let Some(left) = undo_left {
                                let label = format!("{} ({:.0})", self.t("today.undo"), left);
                                if ui.button(label).clicked() {
                                    match self.engine.undo_last() {
                                        Ok(true) => {
                                            let text = self.t("toast.undone");
                                            self.toast(text, ctx);
                                        }
                                        Ok(false) => self.set_status("окно отмены истекло"),
                                        Err(error) => self.set_status(format!("{error:#}")),
                                    }
                                }
                            }
                        });
                    });
            });
    }

    fn about(&mut self, ctx: &egui::Context) {
        if !self.about_open {
            return;
        }
        let mut open = self.about_open;
        let mut close_clicked = false;
        egui::Window::new(self.t("about.title"))
            .collapsible(false)
            .resizable(false)
            .open(&mut open)
            .show(ctx, |ui| {
                ui.label(format!(
                    "{} {}",
                    self.t("app.title"),
                    env!("CARGO_PKG_VERSION_MAJOR")
                ));
                ui.label(self.t("about.text"));
                ui.separator();
                ui.label(format!(
                    "{}: {}",
                    self.t("set.livespace"),
                    self.engine.store.livespace.root().display()
                ));
                ui.separator();
                if ui.button(self.t("common.close")).clicked() {
                    close_clicked = true;
                }
            });
        self.about_open = open && !close_clicked;
    }

    /// Текущая палитра: тема из пака, иначе встроенная светлая или тёмная пара.
    pub fn palette(&self) -> crate::theme::Palette {
        let dark = self.settings.ui.theme == "dark";
        let mut palette = if dark {
            crate::theme::Palette::dark()
        } else {
            crate::theme::Palette::light()
        };

        if let Some(token_id) = self.settings.extra.get("appearance.theme_token") {
            if let Some(token) = self.engine.packs.token_any(token_id) {
                if let Some(variant_id) = self.settings.extra.get(token_id) {
                    if let Some(variant) = token.variants.iter().find(|v| &v.id == variant_id) {
                        if let Some(theme) = &variant.palette {
                            let variant_dark = dark || variant.value == "dark";
                            palette = crate::theme::Palette::from_theme(theme, variant_dark);
                        }
                    }
                }
            }
        }

        if let Some(hex) = self.settings.extra.get("accent") {
            if let Some(color) = parse_hex(hex) {
                palette.accent = color;
            }
        }
        palette
    }

    /// Применяет палитру и мягкие скругления к окну.
    pub fn apply_look(&mut self, ctx: &egui::Context) {
        let palette = self.palette();
        palette.apply(ctx);
        self.accent = palette.accent;
        // Масштаб шрифтов: тема только что выставила базовые размеры.
        let scale = self.settings.ui.font_scale.clamp(0.75, 1.4);
        if (scale - 1.0).abs() > 0.001 {
            ctx.style_mut(|style| {
                for font in style.text_styles.values_mut() {
                    font.size = crate::theme::even_font_size(font.size * scale);
                }
            });
        }
        if let Some((_, family)) = self.applied_value(taskify_packs::TokenKind::Font) {
            crate::theme::apply_font(ctx, &family);
        }
        self.sync_wallpaper(ctx);
    }

    /// Значение применённого варианта нужного типа: (токен, значение).
    pub fn applied_value(&self, kind: taskify_packs::TokenKind) -> Option<(String, String)> {
        for token in self.engine.packs.tokens_any() {
            if token.kind != kind {
                continue;
            }
            let Some(variant_id) = self.settings.extra.get(&token.id) else {
                continue;
            };
            if let Some(variant) = token.variants.iter().find(|item| &item.id == variant_id) {
                return Some((token.id.clone(), variant.value.clone()));
            }
        }
        None
    }

    /// Переключает профиль оформления, если наступило его время.
    fn tick_auto_look(&mut self, ctx: &egui::Context) {
        if !self.settings.auto_look {
            self.auto_look_name = None;
            return;
        }
        // Не затираем временную тему по расписанию: пользователь ещё не решил,
        // сохранить её или отменить.
        if self.settings.look_draft {
            return;
        }
        use chrono::Timelike;
        let now = chrono::Local::now().time();
        let minutes = now.hour() * 60 + now.minute();
        let wanted = crate::settings::active_profile(&self.settings.look_profiles, minutes);
        if wanted == self.auto_look_name {
            return;
        }
        let Some(name) = wanted else {
            return;
        };
        if !self.settings.apply_saved_look(&name) {
            return;
        }
        self.apply_look(ctx);
        self.settings.save();
        self.auto_look_name = Some(name.clone());
        let text = format!(
            "{}: {}",
            self.t("stock.profile_auto_applied"),
            self.look_profile_title(&name)
        );
        self.toast(text, ctx);
    }

    /// Анимация только содержимого вкладки: заголовок и вкладки не дёргаются.
    pub(crate) fn start_tab_transition(&mut self, ctx: &egui::Context) {
        self.tab_anim_start = Some(ctx.input(|input| input.time) as f32);
    }

    /// Прогресс анимации содержимого вкладки: 0 — переключили, 1 — конец.
    pub(crate) fn tab_progress(&mut self, ctx: &egui::Context) -> f32 {
        let Some(start) = self.tab_anim_start else {
            return 1.0;
        };
        let now = ctx.input(|input| input.time) as f32;
        let speed = self.settings.ui.anim_speed.clamp(0.5, 3.0);
        let raw = ((now - start) / (0.18 / speed)).clamp(0.0, 1.0);
        if raw >= 1.0 {
            self.tab_anim_start = None;
            return 1.0;
        }
        ctx.request_repaint();
        crate::app::TaskifyApp::ease_out_cubic(raw)
    }

    /// Ищет аватар в корне лайвспейса: `avatar.png` и родственные.
    fn avatar_file(root: &std::path::Path) -> Option<std::path::PathBuf> {
        ["avatar.png", "avatar.jpg", "avatar.jpeg", "avatar.webp"]
            .iter()
            .map(|name| root.join(name))
            .find(|path| path.exists())
    }

    /// Перечитывает аватар текущего лайвспейса.
    pub(crate) fn reload_avatar(&mut self, ctx: &egui::Context) {
        self.avatar = None;
        let root = self.engine.store.livespace.root().to_path_buf();
        let Some(path) = Self::avatar_file(&root) else {
            return;
        };
        let Ok(bytes) = std::fs::read(&path) else {
            return;
        };
        let Ok(image) = image::load_from_memory(&bytes) else {
            return;
        };
        let rgba = image.to_rgba8();
        let size = [rgba.width() as usize, rgba.height() as usize];
        let color = egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw());
        self.avatar = Some(ctx.load_texture("avatar", color, egui::TextureOptions::LINEAR));
    }

    /// Ставит аватар из выбранного файла: копия живёт в лайвспейсе.
    pub(crate) fn set_avatar(
        &mut self,
        path: &std::path::Path,
        ctx: &egui::Context,
    ) -> anyhow::Result<()> {
        let bytes = std::fs::read(path)?;
        image::load_from_memory(&bytes)
            .map_err(|error| anyhow::anyhow!("это не картинка: {error}"))?;
        let root = self.engine.store.livespace.root().to_path_buf();
        for name in ["avatar.png", "avatar.jpg", "avatar.jpeg", "avatar.webp"] {
            let old = root.join(name);
            if old.exists() {
                let _ = std::fs::remove_file(old);
            }
        }
        let extension = path
            .extension()
            .and_then(|ext| ext.to_str())
            .unwrap_or("png")
            .to_lowercase();
        let extension = match extension.as_str() {
            "jpg" | "jpeg" => "jpg",
            "webp" => "webp",
            _ => "png",
        };
        std::fs::write(root.join(format!("avatar.{extension}")), &bytes)?;
        self.reload_avatar(ctx);
        ctx.request_repaint();
        Ok(())
    }

    /// Убирает аватар из лайвспейса.
    pub(crate) fn clear_avatar(&mut self, ctx: &egui::Context) {
        let root = self.engine.store.livespace.root().to_path_buf();
        for name in ["avatar.png", "avatar.jpg", "avatar.jpeg", "avatar.webp"] {
            let path = root.join(name);
            if path.exists() {
                let _ = std::fs::remove_file(path);
            }
        }
        self.reload_avatar(ctx);
        ctx.request_repaint();
    }

    /// Переключает лайвспейс: переоткрывает движок и сбрасывает кеши.
    /// Переносит текущий лайвспейс в другую папку.
    ///
    /// Сначала копируем и открываем копию, и только потом удаляем старую
    /// папку: сбой копирования или открытия не должен стоить данных.
    pub(crate) fn relocate_current_livespace(
        &mut self,
        destination: &std::path::Path,
        ctx: &egui::Context,
    ) -> anyhow::Result<Option<String>> {
        let slug = self.settings.livespace.clone();
        let from = self.engine.store.livespace.root().to_path_buf();
        crate::bootstrap::copy_livespace_to(&from, destination)?;
        let previous_map = self.settings.personal_livespace_dir(&slug).cloned();
        let previous_legacy = self.settings.livespace_path.clone();
        let standard = self.settings.livespaces_dir().join(&slug);
        let destination_abs =
            std::path::absolute(destination).unwrap_or_else(|_| destination.to_path_buf());
        let standard_abs = std::path::absolute(&standard).unwrap_or(standard);
        if destination_abs == standard_abs {
            self.settings.set_personal_livespace_dir(&slug, None);
            self.settings.livespace_path = None;
        } else {
            self.settings
                .set_personal_livespace_dir(&slug, Some(destination_abs));
            self.settings.livespace_path = Some(destination.display().to_string());
        }
        self.settings.save();
        if let Err(error) = self.switch_livespace(&slug, ctx) {
            self.settings
                .set_personal_livespace_dir(&slug, previous_map);
            self.settings.livespace_path = previous_legacy;
            self.settings.save();
            return Err(error);
        }
        let old_from = from.canonicalize().unwrap_or(from);
        let remove_error = std::fs::remove_dir_all(&old_from)
            .err()
            .map(|error| format!("старая папка не удалена: {error}"));
        Ok(remove_error)
    }

    pub(crate) fn switch_livespace(
        &mut self,
        slug: &str,
        ctx: &egui::Context,
    ) -> anyhow::Result<()> {
        let root = crate::bootstrap::data_root(&self.settings);
        let packs = crate::bootstrap::packs_dir(&self.settings);
        let personal = self
            .settings
            .personal_livespace_dir(slug)
            .filter(|path| path.exists())
            .cloned();
        let livespace = personal
            .clone()
            .unwrap_or_else(|| self.settings.livespaces_dir().join(slug));
        let engine = Engine::open_at_with_packs(&livespace, &root, &packs)?;
        self.engine = engine;
        self.draft = self.engine.store.settings.clone();
        self.settings.livespace = slug.to_string();
        self.settings.livespace_path = personal.map(|path| path.display().to_string());
        self.settings.save();
        self.livespace_rename = self.livespace_display_name();
        self.card_heights.clear();
        self.card_anim.clear();
        self.subtasks_open.clear();
        self.section_anim_start = None;
        self.tab_anim_start = None;
        self.status.clear();
        self.apply_look(ctx);
        self.reload_avatar(ctx);
        ctx.request_repaint();
        Ok(())
    }

    /// Переключает раздел и запускает плавный переход.
    fn switch_section(&mut self, section: Section, ctx: &egui::Context) {
        if self.section == section {
            return;
        }
        self.section = section;
        if section != Section::Packs {
            self.pack_detail = None;
            self.pack_reset_pending = None;
            self.pack_reset_achievements = false;
        }
        self.section_anim_start = Some(ctx.input(|input| input.time) as f32);
        self.status.clear();
    }

    /// Прогресс перехода: 0 — только открыли, 1 — анимация закончилась.
    fn section_progress(&mut self, ctx: &egui::Context) -> f32 {
        let Some(start) = self.section_anim_start else {
            return 1.0;
        };
        let now = ctx.input(|input| input.time) as f32;
        let speed = self.settings.ui.anim_speed.clamp(0.5, 3.0);
        let raw = ((now - start) / (0.22 / speed)).clamp(0.0, 1.0);
        if raw >= 1.0 {
            self.section_anim_start = None;
            return 1.0;
        }
        ctx.request_repaint();
        Self::ease_out_cubic(raw)
    }

    /// Плавное замедление к концу перехода.
    fn ease_out_cubic(raw: f32) -> f32 {
        let left = 1.0 - raw.clamp(0.0, 1.0);
        1.0 - left * left * left
    }

    /// Мягкое появление: контент чуть ниже и прозрачнее в начале.
    pub(crate) fn transition(ui: &mut egui::Ui, progress: f32) {
        if progress >= 1.0 {
            return;
        }
        ui.add_space((1.0 - progress) * 18.0);
        ui.multiply_opacity(0.15 + 0.85 * progress);
    }

    /// Путь к применённому файлу нужного типа (обои или звук).
    pub fn applied_asset(&self, kind: taskify_packs::TokenKind) -> Option<std::path::PathBuf> {
        let (token, value) = self.applied_value(kind)?;
        let variant = self
            .engine
            .packs
            .token_any(&token)?
            .variants
            .iter()
            .find(|item| item.value == value)?
            .id
            .clone();
        self.engine.variant_asset(&token, &variant)
    }

    /// Перезагружает обои, если выбранная картинка поменялась.
    /// Меню аккаунта-лайвспейса: список и создание нового.
    fn livespace_menu(&mut self, response: &egui::Response, allow_force: bool) {
        let mut chosen: Option<String> = None;
        let mut want_create = false;
        let list = crate::bootstrap::livespaces(&self.settings);
        let popup = egui::Popup::menu(response);
        let popup = if allow_force && self.livespace_menu_force {
            popup.open(true)
        } else {
            popup
        };
        popup.show(|ui| {
            ui.set_min_width(240.0);
            ui.label(
                egui::RichText::new(self.t("livespace.title"))
                    .small()
                    .weak(),
            );
            ui.label(
                egui::RichText::new(format!(
                    "{}: {}",
                    self.t("livespace.current"),
                    self.livespace_display_name()
                ))
                .small()
                .strong(),
            );
            ui.separator();
            for (slug, label) in &list {
                let current = slug == &self.settings.livespace;
                let text = if current {
                    format!("✓ {label}")
                } else {
                    label.clone()
                };
                if ui.selectable_label(current, text).clicked() {
                    chosen = Some(slug.clone());
                    ui.close();
                }
            }
            ui.separator();
            if ui.button(self.t("livespace.create")).clicked() {
                want_create = true;
                ui.close();
            }
        });
        if chosen.is_some() {
            self.livespace_pending = chosen;
        }
        if want_create {
            self.livespace_dialog = true;
            self.livespace_focus = true;
            self.livespace_error.clear();
            self.livespace_name.clear();
        }
    }

    /// Удаление текущего лайвспейса: два независимых подтверждения.
    fn delete_livespace_dialog(&mut self, ctx: &egui::Context) {
        if self.delete_stage == 0 {
            return;
        }
        let name = self.livespace_display_name();
        let danger = self.palette().danger;
        let on_danger = self.palette().on_danger;
        let mut open = true;
        let mut cancel = false;
        let mut next_stage = self.delete_stage;
        let mut delete = false;
        let stage = self.delete_stage;
        let title = if stage == 1 {
            self.t("set.delete_title")
        } else {
            self.t("set.delete_final")
        };
        egui::Window::new(title)
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.set_width(380.0);
                ui.label(
                    egui::RichText::new(format!("{}: {name}", self.t("livespace.current")))
                        .strong(),
                );
                ui.label(
                    egui::RichText::new(self.t("set.delete_warning"))
                        .small()
                        .weak(),
                );
                ui.add_space(6.0);
                if stage == 1 {
                    ui.horizontal(|ui| {
                        let next = egui::Button::new(
                            egui::RichText::new(self.t("set.delete_continue"))
                                .color(on_danger)
                                .strong(),
                        )
                        .fill(danger);
                        if ui.add(next).clicked() {
                            next_stage = 2;
                        }
                        if ui.button(self.t("common.cancel")).clicked() {
                            cancel = true;
                        }
                    });
                } else {
                    ui.label(self.t("set.delete_type_hint"));
                    let response = ui.add(
                        egui::TextEdit::singleline(&mut self.delete_confirm_name)
                            .desired_width(f32::INFINITY),
                    );
                    if !self.delete_error.is_empty() {
                        ui.colored_label(danger, self.delete_error.clone());
                    }
                    let can_delete = self.delete_confirm_name.trim().to_lowercase()
                        == name.trim().to_lowercase()
                        && !name.trim().is_empty();
                    ui.horizontal(|ui| {
                        let forever = egui::Button::new(
                            egui::RichText::new(self.t("set.delete_forever"))
                                .color(on_danger)
                                .strong(),
                        )
                        .fill(danger);
                        if ui.add_enabled(can_delete, forever).clicked() {
                            delete = true;
                        }
                        if ui.button(self.t("common.cancel")).clicked() {
                            cancel = true;
                        }
                    });
                    let _ = response;
                }
            });
        if delete {
            match self.delete_livespace_now(ctx) {
                Ok(()) => {
                    self.delete_stage = 0;
                    self.delete_confirm_name.clear();
                    self.delete_error.clear();
                    self.toast(self.t("set.deleted"), ctx);
                }
                Err(error) => {
                    // Аккаунт мог уже переключиться: повторное нажатие не
                    // должно удалить новый. Закрываем окно и показываем ошибку.
                    self.delete_stage = 0;
                    self.delete_confirm_name.clear();
                    self.set_status(format!("{}: {error:#}", self.t("set.delete_failed")));
                }
            }
            return;
        }
        if cancel || !open {
            self.delete_stage = 0;
            self.delete_confirm_name.clear();
            self.delete_error.clear();
            return;
        }
        if next_stage == 2 && stage == 1 {
            self.delete_confirm_name.clear();
            self.delete_error.clear();
        }
        self.delete_stage = next_stage;
    }

    /// Переключается на другой аккаунт и удаляет старый каталог.
    fn delete_livespace_now(&mut self, ctx: &egui::Context) -> anyhow::Result<()> {
        let root = self.engine.store.livespace.root().to_path_buf();
        if !root.join("livespace.toml").is_file() {
            anyhow::bail!("каталог лайвспейса не найден");
        }
        let root = root.canonicalize().unwrap_or(root);
        let data = crate::bootstrap::data_root(&self.settings);
        if root == data {
            anyhow::bail!("нельзя удалить корень данных");
        }
        let current = self.settings.livespace.clone();
        let others: Vec<String> = crate::bootstrap::livespaces(&self.settings)
            .into_iter()
            .map(|(slug, _)| slug)
            .filter(|slug| slug != &current)
            .collect();
        // Сначала открываем другой аккаунт, и только потом удаляем старый:
        // так приложение не останется с движком, указывающим в пустоту.
        if let Some(next) = others.first() {
            self.switch_livespace(next, ctx)?;
        } else {
            let name = self.t("set.new_account_name");
            let slug = crate::bootstrap::create_livespace(&self.settings, &name)?;
            self.switch_livespace(&slug, ctx)?;
        }
        self.settings.set_personal_livespace_dir(&current, None);
        self.settings.livespace_path = self
            .settings
            .personal_livespace_dir(&self.settings.livespace)
            .map(|path| path.display().to_string());
        self.settings.save();
        crate::bootstrap::delete_livespace_dir(&root)?;
        Ok(())
    }

    /// Диалог создания нового лайвспейса.
    fn livespace_dialog(&mut self, ctx: &egui::Context) {
        if !self.livespace_dialog {
            return;
        }
        let mut open = true;
        let mut create = false;
        let mut close = false;
        let danger = self.palette().danger;
        egui::Window::new(self.t("livespace.new_title"))
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.set_width(340.0);
                ui.label(self.t("livespace.name"));
                let hint = self.t("livespace.name_hint");
                let response = ui.add(
                    egui::TextEdit::singleline(&mut self.livespace_name)
                        .desired_width(f32::INFINITY)
                        .hint_text(hint),
                );
                if self.livespace_focus {
                    response.request_focus();
                    self.livespace_focus = false;
                }
                if response.lost_focus() && ui.input(|input| input.key_pressed(egui::Key::Enter)) {
                    create = true;
                }
                if !self.livespace_error.is_empty() {
                    ui.colored_label(danger, self.livespace_error.clone());
                }
                ui.horizontal(|ui| {
                    if ui.button(self.t("livespace.create_button")).clicked() {
                        create = true;
                    }
                    if ui.button(self.t("common.cancel")).clicked() {
                        close = true;
                    }
                });
            });
        if create {
            match crate::bootstrap::create_livespace(&self.settings, &self.livespace_name) {
                Ok(slug) => {
                    self.livespace_dialog = false;
                    self.livespace_error.clear();
                    match self.switch_livespace(&slug, ctx) {
                        Ok(()) => self.set_status(self.t("livespace.created")),
                        Err(error) => {
                            self.livespace_error = format!("{error:#}");
                            self.livespace_dialog = true;
                        }
                    }
                }
                Err(error) => self.livespace_error = format!("{error:#}"),
            }
        }
        if close || !open {
            self.livespace_dialog = false;
            self.livespace_error.clear();
        }
    }

    /// Подтверждение перезаписи пользовательской темы.
    fn overwrite_look_dialog(&mut self, ctx: &egui::Context) {
        let Some(name) = self.overwrite_look_target.clone() else {
            return;
        };
        let mut open = true;
        let mut confirm = false;
        let mut cancel = false;
        let title = self.t("stock.overwrite_title");
        let hint = self.t("stock.overwrite_hint");
        let confirm_label = self.t("stock.overwrite_confirm");
        let cancel_label = self.t("common.cancel");
        let target = self.look_profile_title(&name);
        let danger = self.palette().danger;
        let on_danger = self.palette().on_danger;
        egui::Window::new(title)
            .open(&mut open)
            .collapsible(false)
            .resizable(false)
            .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
            .show(ctx, |ui| {
                ui.set_width(390.0);
                ui.label(hint);
                ui.label(egui::RichText::new(target).strong());
                ui.add_space(8.0);
                ui.horizontal(|ui| {
                    if ui.button(cancel_label).clicked() {
                        cancel = true;
                    }
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        let button =
                            egui::Button::new(egui::RichText::new(confirm_label).color(on_danger))
                                .fill(danger);
                        if ui.add(button).clicked() {
                            confirm = true;
                        }
                    });
                });
            });
        if confirm {
            self.overwrite_look_profile(&name, ctx);
            self.overwrite_look_target = None;
        } else if cancel || !open {
            self.overwrite_look_target = None;
        }
    }

    pub fn sync_wallpaper(&mut self, ctx: &egui::Context) {
        let wanted = self
            .applied_value(taskify_packs::TokenKind::Image)
            .map(|(_, value)| value);
        if self.wallpaper.as_ref().map(|(name, _)| name.clone()) == wanted {
            return;
        }
        self.wallpaper = None;
        let Some(file) = wanted else {
            return;
        };
        let Some(path) = self.applied_asset(taskify_packs::TokenKind::Image) else {
            eprintln!("wallpaper asset not found for {file}");
            return;
        };
        let Ok(bytes) = std::fs::read(&path) else {
            return;
        };
        let Ok(image) = image::load_from_memory(&bytes) else {
            return;
        };
        let rgba = image.to_rgba8();
        let size = [rgba.width() as usize, rgba.height() as usize];
        let color = egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw());
        let texture = ctx.load_texture(
            format!("wallpaper:{file}"),
            color,
            egui::TextureOptions::LINEAR,
        );
        self.wallpaper = Some((file, texture));
    }

    /// Обновление акцента после смены косметики.
    pub fn refresh_accent(&mut self) {
        self.accent = self.palette().accent;
    }

    /// Отладочный снимок окна: `TASKIFY_SCREENSHOT=<путь>`.
    fn handle_screenshot(&mut self, ctx: &egui::Context) {
        let Some(path) = self.screenshot.clone() else {
            return;
        };
        if !self.screenshot_requested {
            if ctx.input(|input| input.time) > 0.6 {
                ctx.send_viewport_cmd(egui::ViewportCommand::Screenshot(egui::UserData::default()));
                self.screenshot_requested = true;
            }
            return;
        }
        let image = ctx.input(|input| {
            input.events.iter().find_map(|event| match event {
                egui::Event::Screenshot { image, .. } => Some(image.clone()),
                _ => None,
            })
        });
        let Some(image) = image else {
            return;
        };
        if let Some(parent) = path.parent() {
            let _ = std::fs::create_dir_all(parent);
        }
        let width = image.width() as u32;
        let height = image.height() as u32;
        let mut bytes = Vec::with_capacity((width * height * 4) as usize);
        for pixel in &image.pixels {
            bytes.extend_from_slice(&pixel.to_array());
        }
        if let Some(buffer) = image::RgbaImage::from_raw(width, height, bytes) {
            let _ = buffer.save(&path);
        }
        ctx.send_viewport_cmd(egui::ViewportCommand::Close);
    }

    /// Проверяет новые задания и, если пользователь включил уведомления,
    /// показывает настоящее системное уведомление. Не чаще раза в 15 секунд.
    fn check_new_task_notifications(&mut self, ctx: &egui::Context) {
        if !self.settings.ui.notify_new_tasks {
            return;
        }
        // Проверяем и в простое: задания могут открыться по уровню или времени.
        ctx.request_repaint_after(std::time::Duration::from_secs(15));
        let now = std::time::Instant::now();
        if let Some(last) = self.last_new_tasks_check {
            if now.duration_since(last) < std::time::Duration::from_secs(15) {
                return;
            }
        }
        self.last_new_tasks_check = Some(now);

        let date = self.engine.today();
        let tasks = self.engine.unnotified_new_tasks(date);
        if tasks.is_empty() {
            return;
        }
        let titles: Vec<String> = tasks
            .iter()
            .take(3)
            .filter_map(|id| self.engine.display_task(id, date))
            .map(|task| task.title(&self.settings.ui.locale))
            .collect();
        let title = self.t("system.new_tasks_title");
        let body = if tasks.len() == 1 {
            self.t("system.new_tasks_one").replace(
                "{title}",
                titles.first().map(String::as_str).unwrap_or_default(),
            )
        } else {
            self.t("system.new_tasks_many")
                .replace("{count}", &tasks.len().to_string())
                .replace("{titles}", &titles.join(", "))
        };
        match crate::system_notify::notify(&title, &body) {
            Ok(()) => {
                if let Err(error) = self.engine.mark_tasks_notified(&tasks) {
                    crate::diag::write(&format!("не удалось записать уведомление: {error:#}"));
                }
                ctx.request_repaint();
            }
            Err(error) => crate::diag::write(&format!("системное уведомление: {error:#}")),
        }
    }
}

impl eframe::App for TaskifyApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        if ctx.input(|i| i.key_pressed(egui::Key::F11)) {
            self.toggle_fullscreen(ctx);
        }

        if let Err(error) = self.engine.close_week_if_needed() {
            self.status = format!("{error:#}");
        }
        self.check_new_task_notifications(ctx);

        self.sync_wallpaper(ctx);
        self.tick_auto_look(ctx);
        self.drive_scroll(ctx);
        self.header(ctx);
        self.navbar(ctx);
        self.body(ctx);
        if let Some(slug) = self.livespace_pending.take() {
            match self.switch_livespace(&slug, ctx) {
                Ok(()) => {
                    let text = format!(
                        "{}: {}",
                        self.t("set.livespace_switched"),
                        self.livespace_display_name()
                    );
                    self.toast(text, ctx);
                }
                Err(error) => {
                    self.set_status(format!("{}: {error:#}", self.t("livespace.switch_failed")))
                }
            }
        }
        self.livespace_dialog(ctx);
        self.overwrite_look_dialog(ctx);
        self.delete_livespace_dialog(ctx);
        let offset = (ctx.screen_rect().height() * 0.20).clamp(90.0, 150.0);
        self.toast_bar(ctx, offset);
        self.about(ctx);
        self.handle_screenshot(ctx);
        self.capture_geometry(ctx);
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.settings.save();
    }
}

pub fn parse_hex(value: &str) -> Option<egui::Color32> {
    let text = value.trim_start_matches('#');
    if text.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&text[0..2], 16).ok()?;
    let g = u8::from_str_radix(&text[2..4], 16).ok()?;
    let b = u8::from_str_radix(&text[4..6], 16).ok()?;
    Some(egui::Color32::from_rgb(r, g, b))
}

pub fn install_fonts(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    let assets = crate::bootstrap::assets_root().join("fonts");
    let candidates = [
        assets.join("NotoSans-Regular.ttf"),
        std::path::PathBuf::from(r"C:\Windows\Fonts\segoeui.ttf"),
    ];
    for path in candidates {
        if let Ok(bytes) = std::fs::read(path) {
            fonts.font_data.insert(
                "taskify_ui".to_string(),
                std::sync::Arc::new(egui::FontData::from_owned(bytes)),
            );
            fonts
                .families
                .entry(egui::FontFamily::Proportional)
                .or_default()
                .insert(0, "taskify_ui".to_string());
            break;
        }
    }
    // CJK-шрифты подключаем отдельно: они тяжёлые.
    let cjk = [
        assets.join("NotoSansJP-Regular.otf"),
        std::path::PathBuf::from(r"C:\Windows\Fonts\msgothic.ttc"),
    ];
    for path in cjk {
        if let Ok(bytes) = std::fs::read(path) {
            fonts.font_data.insert(
                "taskify_cjk".to_string(),
                std::sync::Arc::new(egui::FontData::from_owned(bytes)),
            );
            fonts
                .families
                .entry(egui::FontFamily::Proportional)
                .or_default()
                .push("taskify_cjk".to_string());
            break;
        }
    }
    ctx.set_fonts(fonts);
}

/// Кнопка заданного размера с текстом по центру и по горизонтали, и по
/// вертикали. В вертикальном списке `add_sized` прижимает подпись к верху.
pub(crate) fn centered_button(
    ui: &mut egui::Ui,
    size: egui::Vec2,
    button: egui::Button<'_>,
) -> egui::Response {
    ui.allocate_ui_with_layout(
        size,
        egui::Layout::left_to_right(egui::Align::Center),
        |ui| ui.add(button.min_size(size)),
    )
    .inner
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn avatar_file_prefers_png_over_webp() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/avatar-test")
            .join(std::process::id().to_string());
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(&root).unwrap();
        std::fs::write(root.join("avatar.webp"), b"webp").unwrap();
        assert_eq!(
            TaskifyApp::avatar_file(&root),
            Some(root.join("avatar.webp"))
        );
        std::fs::write(root.join("avatar.png"), b"png").unwrap();
        assert_eq!(
            TaskifyApp::avatar_file(&root),
            Some(root.join("avatar.png"))
        );
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn stickers_are_arranged_as_tiles() {
        let (cols, rows, diameter) = sticker_grid(190.0, 600.0, 12);
        assert!(cols >= 2, "стикеры должны идти плиткой, а не столбцом");
        assert!(cols * rows >= 12);
        assert!(diameter >= 30.0);
        let width = cols as f32 * diameter + 10.0 * (cols - 1) as f32;
        assert!(width <= 190.0, "плитка не должна вылезать за панель");
        let (one_col, one_row, one_diameter) = sticker_grid(190.0, 600.0, 1);
        assert_eq!((one_col, one_row), (1, 1));
        assert!(one_diameter <= 64.0);
    }

    #[test]
    fn sticker_centers_snap_to_grid_nodes() {
        let panel = egui::Rect::from_min_size(egui::pos2(10.0, 20.0), egui::vec2(190.0, 600.0));
        let step = 32.0;
        let snapped = snap_to_grid(panel, step, egui::pos2(51.0, 53.0));
        assert_eq!(snapped, egui::pos2(42.0, 52.0));
        assert!((snapped.x - panel.left()) % step == 0.0);
        assert!((snapped.y - panel.top()) % step == 0.0);
        let far = snap_to_grid(panel, step, egui::pos2(500.0, 500.0));
        assert_eq!(far.x, panel.right());
        assert!(
            far.x + 30.0 > panel.right(),
            "стикер должен свисать за край"
        );
        assert!((far.y - panel.top()) % step == 0.0);
        assert_eq!(panel_alpha(0), 0);
        assert_eq!(panel_alpha(59), 150);
        assert_eq!(panel_alpha(100), 255);
        assert_eq!(scale_alpha(128, 50), 64);
        assert_eq!(scale_alpha(255, 0), 0);
    }

    #[test]
    fn sticker_board_is_editable_only_on_stock_stickers_tab() {
        assert!(sticker_edit_mode(Section::Stock, StockTab::Stickers));
        assert!(!sticker_edit_mode(Section::Stock, StockTab::Themes));
        for section in Section::ALL {
            if section != Section::Stock {
                assert!(
                    !sticker_edit_mode(section, StockTab::Stickers),
                    "{section:?}: стикеры нельзя двигать вне склада"
                );
            }
        }
    }

    #[test]
    fn resting_sticker_ignores_grid_toggle_and_scale() {
        let panel = egui::Rect::from_min_size(egui::pos2(10.0, 20.0), egui::vec2(190.0, 600.0));
        let rest = egui::pos2(51.0, 53.0);
        let without = resolve_sticker_center(panel, rest, None, None);
        let small = resolve_sticker_center(panel, rest, None, Some(16.0));
        let large = resolve_sticker_center(panel, rest, None, Some(64.0));
        assert_eq!(without, rest);
        assert_eq!(small, rest, "сетка не должна двигать лежащий стикер");
        assert_eq!(
            large, rest,
            "масштаб сетки не должен двигать лежащий стикер"
        );
        let dragged = resolve_sticker_center(panel, egui::pos2(10.0, 10.0), Some(rest), Some(32.0));
        assert_eq!(dragged, egui::pos2(42.0, 52.0));
    }

    #[test]
    fn sticker_center_can_hang_over_the_panel_edge() {
        let panel = egui::Rect::from_min_size(egui::pos2(10.0, 20.0), egui::vec2(190.0, 600.0));
        let center = clamp_sticker_center(panel, egui::pos2(500.0, 700.0));
        assert_eq!(center.x, panel.right());
        assert_eq!(center.y, panel.bottom());
        let art = egui::Rect::from_center_size(center, egui::vec2(56.0, 56.0));
        assert!(art.right() > panel.right(), "стикер должен свисать");
        let visible = art.intersect(panel);
        assert!(visible.width() > 0.0 && visible.width() < art.width());
        assert_eq!(visible.right(), panel.right());
    }

    #[test]
    fn column_edges_fill_window_and_center_stretches() {
        let (left, center, right) = rail_columns(1280.0, 190.0, 10.0);
        assert_eq!((left, center, right), (190.0, 880.0, 190.0));
        assert!((left + center + right + 20.0 - 1280.0).abs() < 0.01);
        let (left, center, right) = rail_columns(1920.0, 190.0, 10.0);
        assert_eq!((left, center, right), (190.0, 1520.0, 190.0));
        let (left, center, right) = rail_columns(700.0, 190.0, 10.0);
        assert!((left + center + right + 20.0 - 700.0).abs() < 0.01);
        assert!(right <= 700.0 / 2.0);
        let (left, center, right) = rail_columns(300.0, 190.0, 10.0);
        assert!((left + center + right + 20.0 - 300.0).abs() < 0.01);
        assert!(center >= 0.0);
    }

    #[test]
    fn sticker_rails_are_inset_from_side_edges() {
        let area = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(1280.0, 800.0));
        let (left, center, right) = shell_columns(area, 190.0, 10.0, 8.0);
        assert_eq!(left.left(), area.left() + 8.0);
        assert_eq!(left.width(), 190.0);
        assert_eq!(right.right(), area.right() - 8.0);
        assert_eq!(right.width(), 190.0);
        assert_eq!(right.top(), area.top());
        assert_eq!(center.left(), left.right() + 10.0);
        assert_eq!(center.right(), right.left() - 10.0);

        let narrow = egui::Rect::from_min_size(egui::pos2(0.0, 0.0), egui::vec2(700.0, 800.0));
        let (_, _, narrow_right) = shell_columns(narrow, 190.0, 10.0, 8.0);
        assert_eq!(narrow_right.right(), narrow.right() - 8.0);
    }

    #[test]
    fn livespace_name_can_be_changed_from_settings() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/rename-flow-test")
            .join(std::process::id().to_string());
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("livespaces")).unwrap();
        let mut settings = Settings::default();
        settings.paths.livespaces_dir = Some(root.join("livespaces"));
        let slug = crate::bootstrap::create_livespace(&settings, "Старое имя").unwrap();
        settings.livespace = slug;
        let ctx = egui::Context::default();
        let mut app = TaskifyApp::build(&ctx, settings);
        app.rename_livespace("Новое имя", &ctx);
        assert_eq!(app.livespace_display_name(), "Новое имя");
        app.rename_livespace("   ", &ctx);
        assert_eq!(app.livespace_display_name(), "Новое имя");
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn deleting_livespace_switches_and_removes_directory() {
        let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../target/delete-flow-test")
            .join(std::process::id().to_string());
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("livespaces")).unwrap();
        let mut settings = Settings::default();
        settings.paths.livespaces_dir = Some(root.join("livespaces"));
        settings.save_path = Some(root.join("settings.toml"));
        let first = crate::bootstrap::create_livespace(&settings, "Первый").unwrap();
        let second = crate::bootstrap::create_livespace(&settings, "Второй").unwrap();
        settings.livespace = first.clone();
        let ctx = egui::Context::default();
        let mut app = TaskifyApp::build(&ctx, settings);
        assert_eq!(app.livespace_display_name(), "Первый");
        app.delete_livespace_now(&ctx).unwrap();
        assert!(!root.join("livespaces").join(&first).exists());
        assert_eq!(app.settings.livespace, second);
        assert!(app.engine.store.livespace.root().exists());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn transition_eases_monotonically() {
        assert_eq!(super::TaskifyApp::ease_out_cubic(0.0), 0.0);
        assert_eq!(super::TaskifyApp::ease_out_cubic(1.0), 1.0);
        let mut previous = 0.0;
        for step in 1..=10 {
            let value = super::TaskifyApp::ease_out_cubic(step as f32 / 10.0);
            assert!(value > previous, "переход должен идти только вперёд");
            previous = value;
        }
        assert!(
            super::TaskifyApp::ease_out_cubic(0.5) > 0.5,
            "в начале быстрее"
        );
    }

    #[test]
    fn sections_are_unique_and_complete() {
        let keys: BTreeSet<&str> = Section::ALL.iter().map(|s| s.key()).collect();
        assert_eq!(keys.len(), 7);
        assert!(keys.contains("nav.today"));
        assert!(keys.contains("nav.stats"));
        assert!(keys.contains("nav.settings"));
    }

    #[test]
    fn pack_tabs_follow_pack_content() {
        let mut pack = taskify_packs::Pack::default();
        assert_eq!(PackTab::for_pack(&pack), vec![PackTab::Info]);

        pack.categories
            .insert("demo/category".into(), Default::default());
        pack.tags.insert("demo/tag".into(), Default::default());
        pack.types.insert("demo/type".into(), Default::default());
        pack.icons.insert("demo/icon".into(), Default::default());
        pack.palette.insert("demo/color".into(), Default::default());
        pack.tokens.insert(
            "demo/token".into(),
            taskify_packs::Token {
                id: "demo/token".into(),
                kind: taskify_packs::TokenKind::Color,
                group: String::new(),
                title: Default::default(),
                variants: Vec::new(),
            },
        );
        pack.bundles.insert(
            "demo/theme".into(),
            taskify_packs::Bundle {
                id: "demo/theme".into(),
                price: 10,
                base: None,
                title: Default::default(),
                description: Default::default(),
                tokens: Vec::new(),
                stickers: Vec::new(),
            },
        );
        pack.stickers
            .insert("demo/sticker".into(), Default::default());
        pack.metrics_declared.push("demo/metric".into());

        let tabs = PackTab::for_pack(&pack);
        for expected in [
            PackTab::Categories,
            PackTab::Tags,
            PackTab::Types,
            PackTab::Icons,
            PackTab::Palette,
            PackTab::Tokens,
            PackTab::Bundles,
            PackTab::Stickers,
            PackTab::Metrics,
        ] {
            assert!(tabs.contains(&expected), "нет вкладки {expected:?}");
        }
        assert_eq!(tabs.last(), Some(&PackTab::Info));
        assert!(!tabs.contains(&PackTab::Tasks));
    }

    #[test]
    fn hex_colors_parse() {
        assert_eq!(
            parse_hex("#2f6fed"),
            Some(egui::Color32::from_rgb(0x2f, 0x6f, 0xed))
        );
        assert_eq!(parse_hex("oops"), None);
    }

    #[test]
    fn wheel_inertia_carries_the_full_step_between_frames() {
        let mut velocity = egui::Vec2::ZERO;
        let first =
            TaskifyApp::scroll_step(&mut velocity, Some(egui::vec2(0.0, -40.0)), 1.0 / 145.0);
        assert!(first.y < 0.0);
        assert!(first.y > -40.0);
        let mut total = first.y;
        for _ in 0..400 {
            total += TaskifyApp::scroll_step(&mut velocity, None, 1.0 / 145.0).y;
        }
        assert!((total + 40.0).abs() < 1.0, "wheel step lost: {total}");
        assert_eq!(velocity, egui::Vec2::ZERO);
    }

    #[test]
    fn wheel_inertia_keeps_horizontal_axis() {
        let mut velocity = egui::Vec2::ZERO;
        let step = TaskifyApp::scroll_step(&mut velocity, Some(egui::vec2(30.0, 0.0)), 1.0 / 60.0);
        assert!(step.x > 0.0);
        assert_eq!(step.y, 0.0);
    }

    #[test]
    fn sticker_aura_uses_the_longest_side_not_the_diagonal() {
        assert_eq!(sticker_aura_radius(egui::vec2(64.0, 64.0)), 36.0);
        assert_eq!(sticker_aura_radius(egui::vec2(64.0, 42.666_668)), 36.0);
        assert_eq!(sticker_aura_radius(egui::vec2(128.0, 85.333_336)), 68.0);
    }

    #[test]
    fn sticker_scale_follows_pointer_distance_without_the_old_cap() {
        let center = egui::pos2(100.0, 100.0);
        let start = egui::pos2(140.0, 100.0);
        assert!(
            (sticker_scale_from_drag(1.0, center, start, egui::pos2(180.0, 100.0)) - 2.0).abs()
                < 0.001
        );
        assert!(
            (sticker_scale_from_drag(1.0, center, start, egui::pos2(460.0, 100.0)) - 9.0).abs()
                < 0.001,
            "размер должен расти далеко за прежние 200 %"
        );
        assert!(
            (sticker_scale_from_drag(1.0, center, start, egui::pos2(101.0, 100.0))
                - taskify_store::state::STICKER_MIN_SCALE)
                .abs()
                < 0.001
        );
        assert_eq!(
            sticker_scale_from_drag(1.0, center, start, egui::pos2(2660.0, 100.0)),
            taskify_store::state::STICKER_MAX_SCALE
        );
    }

    #[test]
    fn sticker_rotation_wraps_around_and_handles_follow_it() {
        let center = egui::pos2(100.0, 100.0);
        let start = egui::pos2(100.0, 60.0);
        let rotation = sticker_rotation_from_drag(0.0, center, start, egui::pos2(140.0, 100.0));
        assert!((rotation - std::f32::consts::FRAC_PI_2).abs() < 0.001);

        let panel = egui::Rect::from_min_size(egui::Pos2::ZERO, egui::vec2(500.0, 500.0));
        let (resize, rotate) =
            sticker_handle_positions(panel, center, egui::vec2(40.0, 40.0), rotation);
        assert!((resize.x - 72.0).abs() < 0.001);
        assert!((resize.y - 128.0).abs() < 0.001);
        assert!((rotate.x - 140.0).abs() < 0.001);
        assert!((rotate.y - 100.0).abs() < 0.001);
    }

    #[test]
    fn fallback_dirs_are_distinct_and_end_with_temp_folder() {
        let dirs = fallback_dirs();
        assert!(dirs.len() >= 2, "нужен хотя бы один запасной путь");

        let mut unique = dirs.clone();
        unique.sort();
        unique.dedup();
        assert_eq!(unique.len(), dirs.len(), "пути не должны повторяться");

        assert!(
            dirs.iter().any(|dir| dir.ends_with("taskify-fallback")),
            "последний вариант — общая папка в temp: {dirs:?}"
        );
        assert!(
            dirs.iter()
                .any(|dir| dir.to_string_lossy().contains("taskify-fallback-")),
            "у процесса должен быть свой каталог: {dirs:?}"
        );
    }
}
