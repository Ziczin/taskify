//! Подсказки к поиску: общий вид списка и отбор вариантов.
//!
//! Поиск ведёт себя как поисковая строка: пока поле в фокусе и запрос не пуст,
//! под ним показывается несколько совпадений. Стрелки выбирают строку, Enter
//! подставляет её, клик тоже работает.

use eframe::egui;
use std::collections::BTreeSet;

/// Отступ текста внутри поисковой строки. Сверху чуть больше, чем снизу:
/// так строка и плейсхолдер сидят по центру поля, а не липнут к верхнему краю.
pub(crate) const SEARCH_FIELD_MARGIN: egui::Margin = egui::Margin {
    left: 10,
    right: 10,
    top: 5,
    bottom: 3,
};

/// Вариант подсказки.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Suggestion {
    /// Что попадёт в строку поиска.
    pub value: String,
    /// Основная строка подсказки.
    pub label: String,
    /// Вторая строка: дата, тип события или пак.
    pub meta: String,
}

/// Оставляет совпадения по подстроке, начиная с совпадений в начале.
pub(crate) fn suggestions(
    query: &str,
    items: impl IntoIterator<Item = Suggestion>,
) -> Vec<Suggestion> {
    let query = query.trim().to_lowercase();
    if query.is_empty() {
        return Vec::new();
    }
    let mut seen = BTreeSet::new();
    let mut found: Vec<(bool, Suggestion)> = Vec::new();
    for item in items {
        if !seen.insert(item.value.clone()) {
            continue;
        }
        let haystack = item.label.to_lowercase();
        if !haystack.contains(&query) {
            continue;
        }
        // `false` — совпадение с начала, такие варианты выше.
        found.push((!haystack.starts_with(&query), item));
    }
    found.sort_by(|(a_prefix, a), (b_prefix, b)| {
        a_prefix
            .cmp(b_prefix)
            .then_with(|| a.label.to_lowercase().cmp(&b.label.to_lowercase()))
    });
    found.into_iter().take(6).map(|(_, item)| item).collect()
}

/// Рисует подсказки под полем и возвращает выбранное значение.
///
/// Показывается только у поля в фокусе. `selected` хранится между кадрами,
/// чтобы стрелки не сбрасывались.
pub(crate) fn show_suggestions(
    ctx: &egui::Context,
    field: &egui::Response,
    suggestions: &[Suggestion],
    selected: &mut usize,
    id_salt: &str,
) -> Option<String> {
    if !field.has_focus() || suggestions.is_empty() {
        return None;
    }
    if *selected >= suggestions.len() {
        *selected = 0;
    }
    let (down, up, enter, escape) = ctx.input(|input| {
        (
            input.key_pressed(egui::Key::ArrowDown),
            input.key_pressed(egui::Key::ArrowUp),
            input.key_pressed(egui::Key::Enter),
            input.key_pressed(egui::Key::Escape),
        )
    });
    if escape {
        field.surrender_focus();
        return None;
    }
    if down {
        *selected = (*selected + 1) % suggestions.len();
    } else if up {
        *selected = (*selected + suggestions.len() - 1) % suggestions.len();
    }
    let mut picked = if enter {
        Some(suggestions[*selected].value.clone())
    } else {
        None
    };

    let width = field.rect.width().max(370.0);
    egui::Area::new(egui::Id::new(id_salt))
        .order(egui::Order::Foreground)
        .fixed_pos(field.rect.left_bottom() + egui::vec2(0.0, 4.0))
        .constrain(true)
        .show(ctx, |ui| {
            egui::Frame::popup(ui.style())
                .corner_radius(egui::CornerRadius::same(10))
                .inner_margin(egui::Margin::symmetric(6, 4))
                .show(ui, |ui| {
                    ui.set_width(width);
                    for (index, item) in suggestions.iter().enumerate() {
                        let text = if item.meta.is_empty() {
                            item.label.clone()
                        } else {
                            format!("{}   ·   {}", item.label, item.meta)
                        };
                        let row = egui::Button::selectable(
                            index == *selected,
                            egui::RichText::new(text).size(14.0),
                        )
                        .frame(false)
                        .truncate();
                        if ui.add_sized([width - 12.0, 26.0], row).clicked() {
                            picked = Some(item.value.clone());
                        }
                    }
                });
        });
    picked
}

#[cfg(test)]
mod tests {
    use super::*;

    fn suggestion(label: &str) -> Suggestion {
        Suggestion {
            value: label.to_string(),
            label: label.to_string(),
            meta: String::new(),
        }
    }

    #[test]
    fn prefix_matches_go_before_substring_matches() {
        let source = vec![
            suggestion("Ночная прогулка"),
            suggestion("Прогулка"),
            suggestion("Утренняя прогулка"),
        ];
        let found = suggestions("прогулка", source);
        assert_eq!(found[0].label, "Прогулка");
        assert_eq!(found.len(), 3);
    }

    #[test]
    fn duplicates_and_empty_queries_are_ignored() {
        let source = vec![suggestion("Первый шаг"), suggestion("Первый шаг")];
        assert!(suggestions("", source.clone()).is_empty());
        assert_eq!(suggestions("первый", source).len(), 1);
    }

    #[test]
    fn only_six_suggestions_are_shown() {
        let source = (0..10).map(|index| suggestion(&format!("Задача {index}")));
        assert_eq!(suggestions("задача", source).len(), 6);
    }
}
