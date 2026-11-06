# Taskify

[English](README.en.md) · **Русский** · [日本語](README.ja.md) · [Українська](README.uk.md) · [Polski](README.pl.md) · [Español](README.es.md) · [中文](README.zh.md) · [Deutsch](README.de.md)

Taskify — локальное приложение для геймификации личных дел на Windows. Веди привычки, повторяющиеся и разовые задания, чек-листы; получай опыт и монеты; открывай достижения, темы, косметику и стикеры. Всё хранится на компьютере в журнале событий JSONL: без аккаунта и без сети.

![Экран «Сегодня»](docs/presentation/today.png)

## Возможности

- Четыре вида заданий: привычки, повторы, разовые задания и чек-листы с подпунктами.
- Уровни без потолка, дневной и недельный прогресс, серии, комбо и достижения.
- Монеты, магазин косметики, темы, акценты, фоны, шрифты, звуки и перетаскиваемые стикеры.
- Паки контента в TOML: задания, категории, теги, иконки, палитры, комбо, достижения, косметика и наборы.
- Несколько лайвспейсов: у каждого своя история, покупки и доска стикеров.
- Восемь языков интерфейса: русский, английский, японский, украинский, польский, испанский, китайский и немецкий.
- Только локальное хранение: журнал событий, пересобираемые кеши и автоматические бэкапы.
- Портативный режим через `TASKIFY_HOME`.

## Требования

- Windows 10 или новее.
- Rust 1.85 или новее.
- Инструменты сборки C++, если их ещё нет в установленном Rust.

## Сборка и запуск

```powershell
cargo build --release
.\target\release\taskify.exe
```

Тот же файл предоставляет консольные команды:

```powershell
.\target\release\taskify.exe version
.\target\release\taskify.exe packs
.\target\release\taskify.exe stats default
.\target\release\taskify.exe validate history default
```

При первом запуске создаётся лайвспейс `default` и устанавливаются встроенные паки.

## Данные

По умолчанию всё лежит в `%APPDATA%\taskify`:

- `livespaces\<имя>` — журнал `data/log/YYYY-MM.jsonl`, кеши `state.json` и `aggregates.json`, бэкапы.
- `packs\<id>` — установленные паки контента.
- `settings.toml` — настройки окна и приложения.

Переменная `TASKIFY_HOME` переносит весь каталог данных в другое место (портативный режим). Источник правды — журнал: кеши можно удалить и пересобрать из событий.

## Командная строка

```text
taskify version
taskify packs [<data-root>]
taskify validate pack <путь-к-паку>
taskify validate all [<data-root>]
taskify validate history <лайвспейс>
taskify stats <лайвспейс>
taskify log <лайвспейс> [--limit N]
taskify rebuild <лайвспейс>
taskify backup <лайвспейс>
taskify import <архив.zip> <лайвспейс> [<data-root>]
taskify export <лайвспейс> <архив.zip>
```

## Структура проекта

| Крейт | Ответственность |
| --- | --- |
| `crates/core` | Задания, награды, кривая уровней, метрики, границы дня. |
| `crates/store` | События, журнал, кеши, пересчёт, бэкапы, лайвспейсы. |
| `crates/packs` | Загрузка паков TOML, реестры, валидатор, установка zip. |
| `crates/ui` | Окно egui/eframe, экраны, правила игры, локализация, CLI. |


## Паки

Пак — папка с `pack.toml` и подпапками для заданий, комбо, достижений, косметики, наборов, определений и медиа. Id получают префикс пака (`starter/water`). Встроенные паки лежат в `assets/packs` и устанавливаются при первом запуске.

## Проверки

```powershell
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
.\target\release\taskify.exe validate all
```

Сейчас в наборе **155 тестов**.

## Участие

Issues и pull requests приветствуются. Перед pull request прогони проверки выше и переводи все пользовательские строки на каждый язык из `crates/ui/src/i18n.rs`.

## Лицензия

Taskify распространяется по лицензии MIT. См. [LICENSE](LICENSE). Встроенные шрифты Noto — под SIL Open Font License, см. `assets/fonts/LICENSE-OFL.txt`.
