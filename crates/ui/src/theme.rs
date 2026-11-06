//! Мягкая палитра интерфейса: базовая светлая и тёмная темы.
//!
//! Тема приходит из пака (`cosmetics/<id>.toml`, `type = "theme"`), а если её
//! нет — берётся встроенная пара. Скругления задаются здесь же, чтобы линии
//! во всём окне были одинаково плавными.

use eframe::egui;

/// Минимальный кегль интерфейса.
pub const MIN_FONT_SIZE: f32 = 12.0;

/// Кегль всегда целый, чётный и не меньше 12.
pub fn even_font_size(size: f32) -> f32 {
    if !size.is_finite() {
        return MIN_FONT_SIZE;
    }
    ((size / 2.0).round() * 2.0).max(MIN_FONT_SIZE)
}

/// Набор цветов темы.
#[derive(Debug, Clone, Copy)]
pub struct Palette {
    pub dark: bool,
    pub bg: egui::Color32,
    pub panel: egui::Color32,
    pub card: egui::Color32,
    pub field: egui::Color32,
    pub text: egui::Color32,
    pub muted: egui::Color32,
    pub accent: egui::Color32,
    pub success: egui::Color32,
    pub success_bg: egui::Color32,
    pub danger: egui::Color32,
    pub on_danger: egui::Color32,
    pub warning: egui::Color32,
    pub coin: egui::Color32,
    pub border: egui::Color32,
    pub shadow: egui::Color32,
}

/// Смешивает два цвета: `t = 0` — первый, `t = 1` — второй.
fn mix(a: egui::Color32, b: egui::Color32, t: f32) -> egui::Color32 {
    let t = t.clamp(0.0, 1.0);
    let lerp = |x: u8, y: u8| (x as f32 * (1.0 - t) + y as f32 * t).round() as u8;
    egui::Color32::from_rgb(lerp(a.r(), b.r()), lerp(a.g(), b.g()), lerp(a.b(), b.b()))
}

fn rgb(hex: &str) -> egui::Color32 {
    let text = hex.trim().trim_start_matches('#');
    if text.len() != 6 {
        return egui::Color32::GRAY;
    }
    let parse = |range: std::ops::Range<usize>| u8::from_str_radix(&text[range], 16).unwrap_or(128);
    egui::Color32::from_rgb(parse(0..2), parse(2..4), parse(4..6))
}

/// Consolas сидит в строке выше её середины: буквы кажутся прижатыми к верху
/// кнопок и полей. Сдвигаем глифы вниз на долю кегля; остальные шрифты
/// отцентрированы своими метриками.
pub fn font_tweak(family: &str) -> egui::FontTweak {
    let y_offset_factor = if family.eq_ignore_ascii_case("Consolas") {
        0.15
    } else {
        0.0
    };
    egui::FontTweak {
        y_offset_factor,
        ..Default::default()
    }
}

/// Подключает шрифт из системы, если он есть: остальные остаются как были.
pub fn apply_font(ctx: &egui::Context, family: &str) {
    // «Без засечек» — это встроенный Noto Sans. Возврат к нему должен
    // пересобрать шрифты, иначе после Consolas/Georgia останется старый.
    if !family.eq_ignore_ascii_case("Consolas") && !family.eq_ignore_ascii_case("Georgia") {
        crate::app::install_fonts(ctx);
        return;
    }
    let candidates: &[&str] = match family {
        "Georgia" => &["georgia.ttf", "georgiab.ttf"],
        "Consolas" => &["consola.ttf", "consolab.ttf"],
        _ => &[],
    };
    let root = std::env::var("SystemRoot").unwrap_or_else(|_| "C:\\Windows".to_string());
    let Some(bytes) = candidates
        .iter()
        .find_map(|name| std::fs::read(std::path::Path::new(&root).join("Fonts").join(name)).ok())
    else {
        return;
    };
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "app.font".to_owned(),
        std::sync::Arc::new(egui::FontData::from_owned(bytes).tweak(font_tweak(family))),
    );
    for family_name in [egui::FontFamily::Proportional, egui::FontFamily::Monospace] {
        fonts
            .families
            .entry(family_name)
            .or_default()
            .insert(0, "app.font".to_owned());
    }
    ctx.set_fonts(fonts);
}

impl Palette {
    /// Встроенная светлая тема: тёплый фон, белые карточки, приглушённый акцент.
    pub fn light() -> Self {
        Self {
            dark: false,
            bg: rgb("#f3f5f8"),
            panel: rgb("#fbfcfd"),
            card: rgb("#ffffff"),
            field: rgb("#eef2f7"),
            text: rgb("#2c313a"),
            muted: rgb("#7c8492"),
            accent: rgb("#6d8fc4"),
            success: rgb("#5f9e74"),
            success_bg: rgb("#e8f2ea"),
            danger: rgb("#c07a6c"),
            on_danger: rgb("#ffffff"),
            warning: rgb("#9a7435"),
            coin: rgb("#b08a54"),
            border: rgb("#dfe4ec"),
            shadow: rgb("#1a1f28"),
        }
    }

    /// Встроенная тёмная тема: без чёрного, с холодным серым.
    pub fn dark() -> Self {
        Self {
            dark: true,
            bg: rgb("#1b1f26"),
            panel: rgb("#21262f"),
            card: rgb("#272d38"),
            field: rgb("#2a303b"),
            text: rgb("#e7ebf1"),
            muted: rgb("#98a1b0"),
            accent: rgb("#8fb0e0"),
            success: rgb("#84bd97"),
            success_bg: rgb("#26372d"),
            danger: rgb("#d79a8d"),
            on_danger: rgb("#ffffff"),
            warning: rgb("#d3b171"),
            coin: rgb("#d4ae72"),
            border: rgb("#39414e"),
            shadow: rgb("#05070a"),
        }
    }

    /// Палитра из описания в паке.
    pub fn from_theme(value: &taskify_packs::ThemePalette, fallback_dark: bool) -> Self {
        let base = if fallback_dark {
            Self::dark()
        } else {
            Self::light()
        };
        Self {
            dark: fallback_dark,
            bg: value.bg.as_deref().map(rgb).unwrap_or(base.bg),
            panel: value.panel.as_deref().map(rgb).unwrap_or(base.panel),
            card: value.card.as_deref().map(rgb).unwrap_or(base.card),
            field: value.field.as_deref().map(rgb).unwrap_or(base.field),
            text: value.text.as_deref().map(rgb).unwrap_or(base.text),
            muted: value.muted.as_deref().map(rgb).unwrap_or(base.muted),
            accent: value.accent.as_deref().map(rgb).unwrap_or(base.accent),
            success: value.success.as_deref().map(rgb).unwrap_or(base.success),
            success_bg: value
                .success_bg
                .as_deref()
                .map(rgb)
                .unwrap_or(base.success_bg),
            danger: value.danger.as_deref().map(rgb).unwrap_or(base.danger),
            on_danger: value
                .on_danger
                .as_deref()
                .map(rgb)
                .unwrap_or(base.on_danger),
            warning: value.warning.as_deref().map(rgb).unwrap_or(base.warning),
            coin: value.coin.as_deref().map(rgb).unwrap_or(base.coin),
            border: value.border.as_deref().map(rgb).unwrap_or(base.border),
            shadow: value.shadow.as_deref().map(rgb).unwrap_or(base.shadow),
        }
    }

    /// Применяет палитру и общие скругления к контексту.
    pub fn apply(&self, ctx: &egui::Context) {
        let mut visuals = if self.dark {
            egui::Visuals::dark()
        } else {
            egui::Visuals::light()
        };
        visuals.panel_fill = self.panel;
        visuals.window_fill = self.panel;
        // Поле ввода — отдельный элемент темы, а не производная карточки.
        visuals.extreme_bg_color = self.field;
        visuals.faint_bg_color = self.bg;
        visuals.override_text_color = Some(self.text);
        visuals.hyperlink_color = self.accent;
        visuals.selection.bg_fill = self.accent.gamma_multiply(0.35);
        visuals.selection.stroke = egui::Stroke::new(1.0, self.text);
        visuals.window_corner_radius = egui::CornerRadius::same(16);
        visuals.window_stroke = egui::Stroke::new(1.0, self.border);
        visuals.error_fg_color = self.danger;
        visuals.warn_fg_color = self.warning;
        let shadow_alpha = if self.dark { 140 } else { 55 };
        let shadow = egui::Color32::from_rgba_unmultiplied(
            self.shadow.r(),
            self.shadow.g(),
            self.shadow.b(),
            shadow_alpha,
        );
        visuals.window_shadow.color = shadow;
        visuals.popup_shadow.color = shadow;

        let control = egui::Stroke::new(1.0, self.accent.gamma_multiply(0.55));
        let widgets = &mut visuals.widgets;
        widgets.noninteractive.bg_fill = self.panel;
        widgets.noninteractive.weak_bg_fill = self.panel;
        widgets.noninteractive.bg_stroke = egui::Stroke::new(1.0, self.border);
        // Неактивные элементы читаются как отключённые, а не как обычный текст.
        widgets.noninteractive.fg_stroke = egui::Stroke::new(1.0, self.muted);
        widgets.noninteractive.corner_radius = egui::CornerRadius::same(14);

        widgets.inactive.bg_fill = mix(self.card, self.accent, 0.10);
        widgets.inactive.weak_bg_fill = mix(self.card, self.accent, 0.08);
        widgets.inactive.bg_stroke = control;
        widgets.inactive.fg_stroke = egui::Stroke::new(1.0, self.text);
        widgets.inactive.corner_radius = egui::CornerRadius::same(12);

        widgets.hovered.bg_fill = mix(self.card, self.accent, 0.20);
        widgets.hovered.weak_bg_fill = mix(self.card, self.accent, 0.17);
        widgets.hovered.bg_stroke = egui::Stroke::new(1.0, self.accent);
        widgets.hovered.fg_stroke = egui::Stroke::new(1.5, self.text);
        widgets.hovered.corner_radius = egui::CornerRadius::same(12);
        // Кнопка не должна вылезать за своё место при наведении: у края
        // центральной панели этот лишний пиксель обрезался клипом.
        widgets.hovered.expansion = 0.0;

        widgets.active.bg_fill = mix(self.card, self.accent, 0.34);
        widgets.active.weak_bg_fill = mix(self.card, self.accent, 0.30);
        widgets.active.bg_stroke = egui::Stroke::new(1.0, self.accent);
        widgets.active.fg_stroke = egui::Stroke::new(1.5, self.text);
        widgets.active.corner_radius = egui::CornerRadius::same(12);
        widgets.active.expansion = 0.0;

        widgets.open.bg_fill = mix(self.card, self.accent, 0.24);
        widgets.open.weak_bg_fill = mix(self.card, self.accent, 0.20);
        widgets.open.bg_stroke = egui::Stroke::new(1.0, self.accent);
        widgets.open.fg_stroke = egui::Stroke::new(1.0, self.text);
        widgets.open.corner_radius = egui::CornerRadius::same(12);
        widgets.open.expansion = 0.0;

        let mut style = (*ctx.style()).clone();
        style.visuals = visuals;
        // Базовая шкала: только целые чётные кегли, минимум 12.
        let mut text_styles = egui::Style::default().text_styles;
        for (style, size) in [
            (egui::TextStyle::Small, 12.0),
            (egui::TextStyle::Body, 14.0),
            (egui::TextStyle::Button, 14.0),
            (egui::TextStyle::Heading, 20.0),
            (egui::TextStyle::Monospace, 14.0),
        ] {
            if let Some(font) = text_styles.get_mut(&style) {
                font.size = even_font_size(size);
            }
        }
        style.text_styles = text_styles;
        style.spacing.item_spacing = egui::vec2(10.0, 10.0);
        style.spacing.button_padding = egui::vec2(14.0, 8.0);
        style.spacing.interact_size.y = 30.0;
        style.spacing.menu_margin = egui::Margin::same(10);
        ctx.set_style(style);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hex_parses_and_falls_back() {
        assert_eq!(rgb("#ffffff"), egui::Color32::WHITE);
        assert_eq!(rgb("000000"), egui::Color32::BLACK);
        assert_eq!(rgb("oops"), egui::Color32::GRAY);
    }

    #[test]
    fn controls_take_pack_accent() {
        let ctx = egui::Context::default();
        let palette = Palette::light();
        palette.apply(&ctx);
        let visuals = ctx.style().visuals.clone();
        assert_ne!(visuals.widgets.inactive.bg_fill, palette.card);
        assert_ne!(visuals.widgets.hovered.bg_fill, palette.card);
        assert_eq!(visuals.extreme_bg_color, palette.field);
        assert_eq!(visuals.panel_fill, palette.panel);
        assert_eq!(visuals.window_stroke.color, palette.border);
        assert_eq!(visuals.error_fg_color, palette.danger);
        assert_eq!(visuals.warn_fg_color, palette.warning);
        assert_eq!(
            visuals.widgets.noninteractive.fg_stroke.color,
            palette.muted
        );
        assert_eq!(visuals.override_text_color, Some(palette.text));
        assert_eq!(visuals.hyperlink_color, palette.accent);
        // Наведение не должно рисовать за пределами кнопки: этот пиксель
        // обрезался у края центральной панели.
        assert_eq!(visuals.widgets.hovered.expansion, 0.0);
        assert_eq!(visuals.widgets.active.expansion, 0.0);
        assert_eq!(visuals.widgets.open.expansion, 0.0);
    }

    /// Все цвета, которые тема обязана задать. Порядок не важен, набор — да.
    const TOKENS: [&str; 15] = [
        "bg",
        "panel",
        "card",
        "field",
        "text",
        "muted",
        "accent",
        "success",
        "success_bg",
        "danger",
        "on_danger",
        "warning",
        "coin",
        "border",
        "shadow",
    ];

    #[test]
    fn official_theme_palettes_are_complete() {
        let themes = [(
            "greeting",
            include_str!("../../../assets/packs/greeting/cosmetics/theme.toml"),
        )];
        let mut expected = TOKENS.to_vec();
        expected.sort_unstable();
        for (pack, raw) in themes {
            let value: toml::Value = toml::from_str(raw).expect("theme toml");
            let variants = value
                .get("variants")
                .and_then(toml::Value::as_array)
                .unwrap_or_else(|| panic!("{pack}: нет variants"));
            assert!(!variants.is_empty(), "{pack}: нет вариантов темы");
            for variant in variants {
                let id = variant
                    .get("id")
                    .and_then(toml::Value::as_str)
                    .unwrap_or("?");
                let palette = variant
                    .get("palette")
                    .and_then(toml::Value::as_table)
                    .unwrap_or_else(|| panic!("{pack}/{id}: нет palette"));
                let mut keys: Vec<String> = palette.keys().cloned().collect();
                keys.sort_unstable();
                assert_eq!(keys, expected, "{pack}/{id}: набор цветов темы неполный");
            }
        }
    }

    #[test]
    fn built_in_palettes_are_soft() {
        let light = Palette::light();
        assert!(!light.dark);
        assert!(light.bg.r() > 200, "светлый фон должен быть мягким");
        let dark = Palette::dark();
        assert!(dark.dark);
        assert!(dark.bg.r() < 60, "тёмный фон не должен быть чёрным");
    }

    #[test]
    fn applied_text_styles_are_even_and_at_least_twelve() {
        let ctx = egui::Context::default();
        Palette::light().apply(&ctx);
        let styles = ctx.style().text_styles.clone();
        for (name, font) in styles {
            assert!(
                font.size >= MIN_FONT_SIZE,
                "{name:?}: {size} меньше минимума",
                size = font.size
            );
            assert_eq!(
                font.size % 2.0,
                0.0,
                "{name:?}: нечётный кегль {}",
                font.size
            );
        }
    }

    #[test]
    fn font_sizes_are_even_and_at_least_twelve() {
        for raw in [0.0, 9.0, 10.5, 11.0, 13.0, 14.5, 15.0, 16.8, 19.6, 28.0] {
            let size = even_font_size(raw);
            assert!(size >= MIN_FONT_SIZE, "{raw} -> {size}: меньше минимума");
            assert_eq!(size % 2.0, 0.0, "{raw} -> {size}: нечётный кегль");
        }
        assert_eq!(even_font_size(9.0), 12.0);
        assert_eq!(even_font_size(14.5), 14.0);
        assert_eq!(even_font_size(15.0), 16.0);
    }
}
