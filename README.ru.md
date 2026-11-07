# Taskify

[![Build](https://github.com/Ziczin/taskify/actions/workflows/build.yml/badge.svg)](https://github.com/Ziczin/taskify/actions/workflows/build.yml)

[English](README.en.md) · **Русский** · [日本語](README.ja.md) · [Українська](README.uk.md) · [Polski](README.pl.md) · [Español](README.es.md) · [中文](README.zh.md) · [Deutsch](README.de.md)

Taskify — локальное приложение для геймификации личных дел на компьютере. Веди привычки, повторяющиеся и разовые задания, чек-листы; получай опыт и монеты; открывай достижения, темы, косметику и стикеры. Всё хранится на компьютере в журнале событий JSONL: без аккаунта и без сети.

![Экран «Сегодня»](docs/presentation/today.png)

## Возможности

- Четыре вида заданий: привычки, повторы, разовые задания и чек-листы с подпунктами.
- Пять мягких паков: «Приветствие», «Уборка», «Полезные привычки», «Вдохновение» и «Социализация» — по одному действию на шаг и сюжетные цепочки.
- Уровни без потолка, дневной и недельный прогресс, серии, комбо и достижения.
- Монеты, магазин косметики, темы, акценты, фоны, шрифты, звуки и перетаскиваемые стикеры.
- Паки контента в TOML: задания, категории, теги, иконки, палитры, комбо, достижения, косметика и наборы.
- Несколько лайвспейсов: у каждого своя история, покупки и доска стикеров.
- Восемь языков интерфейса: русский, английский, японский, украинский, польский, испанский, китайский и немецкий.
- Только локальное хранение: журнал событий, пересобираемые кеши и автоматические бэкапы.
- Портативный режим через `TASKIFY_HOME`.

## Требования

- Windows 10 или новее, macOS 11 или новее либо Linux с GTK3, X11/Wayland и OpenGL.
- Rust 1.85 или новее.
- Инструменты сборки C++, если их ещё нет в установленном Rust.

## Сборка и запуск

GitHub Actions собирает и проверяет каждый push в `main` на Windows, Linux и macOS. Готовую сборку можно скачать из артефакта последней сборки или из [Releases](https://github.com/Ziczin/taskify/releases) для тегов `v*`.

```powershell
cargo build --release
.\tools\package.ps1          # Windows: dist\taskify-<сборка>-windows-x64.zip
.\target\release\taskify.exe
```

```bash
cargo build --release
./tools/package.sh           # Linux: tar.gz; macOS: Taskify.app в ZIP
./target/release/taskify
```

В Windows-ZIP и Linux-архиве исполняемый файл и папка `assets` должны лежать рядом. В macOS-приложении `assets` уже лежит в `Contents/Resources`. Тот же файл предоставляет консольные команды:

```bash
target/release/taskify version
target/release/taskify packs
target/release/taskify stats default
target/release/taskify validate history default
```

macOS-сборка подписана ad-hoc, но не нотаризована и собрана для Apple Silicon (`arm64`): при первом запуске может понадобиться правый клик → «Открыть».

При первом запуске открывается экран настройки, создаётся лайвспейс и устанавливаются пять встроенных паков. Включён только «Приветствие»; остальные можно включать по одному.

## Данные

По умолчанию данные лежат в каталоге данных платформы:

- Windows: `%APPDATA%\taskify`
- macOS: `~/Library/Application Support/taskify`
- Linux: `$XDG_DATA_HOME/taskify` или `~/.local/share/taskify`

Внутри:

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

Пак — папка с `pack.toml` и подпапками для заданий, комбо, достижений, косметики, наборов, определений и медиа. Id получают префикс пака (`greeting/hello`). Встроенные паки лежат в `assets/packs` и устанавливаются при первом запуске.

## Проверки

```powershell
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
.\target\release\taskify.exe validate all
```

Сейчас в наборе **183 теста**.

## Участие

Issues и pull requests приветствуются. Перед pull request прогони проверки выше и переводи все пользовательские строки на каждый язык из `crates/ui/src/i18n.rs`.

## Лицензия

Taskify распространяется по лицензии MIT. См. [LICENSE](LICENSE). Встроенные шрифты Noto — под SIL Open Font License, см. `assets/fonts/LICENSE-OFL.txt`.
