# Taskify

[English](README.en.md) · [Русский](README.ru.md) · [日本語](README.ja.md) · **Українська** · [Polski](README.pl.md) · [Español](README.es.md) · [中文](README.zh.md) · [Deutsch](README.de.md)

Taskify — локальний застосунок для гейміфікації особистих справ на комп'ютері. Веде звички, повторювані й одноразові завдання, чек-листи; дає досвід і монети; відкриває досягнення, теми, косметику та стікери. Усе зберігається на комп'ютері в журналі подій JSONL: без акаунта й без мережі.

![Екран «Сьогодні»](docs/presentation/today.png)

## Можливості

- Чотири види завдань: звички, повторювані, одноразові та чек-листи з підпунктами.
- Рівні без верхньої межі, денний і тижневий прогрес, серії, комбо й досягнення.
- Монети, магазин косметики, теми, акценти, фони, шрифти, звуки та стікери, які можна перетягувати.
- Паки контенту в TOML: завдання, категорії, теги, іконки, палітри, комбо, досягнення, косметика й набори.
- Кілька лайвспейсів: кожен має власну історію, покупки й дошку стікерів.
- Вісім мов інтерфейсу: російська, англійська, японська, українська, польська, іспанська, китайська й німецька.
- Лише локальне зберігання: журнал подій, кеші, які можна перебудувати, і автоматичні резервні копії.
- Портативний режим через `TASKIFY_HOME`.

## Вимоги

- Windows 10 або новіша, macOS 11 або новіший чи Linux з GTK3, X11/Wayland і OpenGL.
- Rust 1.85 або новіший.
- Інструменти збирання C++, якщо їх ще немає в установленому Rust.

## Збирання й запуск

```powershell
cargo build --release
.\tools\package.ps1          # Windows
.\target\release\taskify.exe
```

```bash
cargo build --release
./tools/package.sh           # Linux: tar.gz; macOS: Taskify.app у ZIP
./target/release/taskify
```

Той самий файл надає команди термінала:

```bash
target/release/taskify version
target/release/taskify packs
target/release/taskify stats default
target/release/taskify validate history default
```

macOS-збірка підписана ad-hoc, але не нотаризована: під час першого запуску може знадобитися правий клік → «Відкрити».

Під час першого запуску створюється лайвспейс `default` і встановлюються вбудовані паки.

## Дані

За замовчуванням усе лежить у `%APPDATA%\taskify`:

- `livespaces\<ім'я>` — журнал `data/log/YYYY-MM.jsonl`, кеші `state.json` та `aggregates.json`, резервні копії.
- `packs\<id>` — встановлені паки контенту.
- `settings.toml` — налаштування вікна й застосунку.

Змінна `TASKIFY_HOME` переносить увесь каталог даних в інше місце (портативний режим). Джерело істини — журнал: кеші можна видалити й перебудувати з подій.

## Командний рядок

```text
taskify version
taskify packs [<data-root>]
taskify validate pack <шлях-до-паку>
taskify validate all [<data-root>]
taskify validate history <лайвспейс>
taskify stats <лайвспейс>
taskify log <лайвспейс> [--limit N]
taskify rebuild <лайвспейс>
taskify backup <лайвспейс>
taskify import <архів.zip> <лайвспейс> [<data-root>]
taskify export <лайвспейс> <архів.zip>
```

## Структура проєкту

| Крейт | Відповідальність |
| --- | --- |
| `crates/core` | Завдання, нагороди, крива рівнів, метрики, межі дня. |
| `crates/store` | Події, журнал, кеші, перерахунок, резервні копії, лайвспейси. |
| `crates/packs` | Завантаження паків TOML, реєстри, валідатор, встановлення zip. |
| `crates/ui` | Вікно egui/eframe, екрани, правила гри, локалізація, CLI. |


## Паки

Пак — це тека з `pack.toml` і підтеками для завдань, комбо, досягнень, косметики, наборів, визначень і медіа. Id отримують префікс пака (`greeting/hello`). Вбудовані паки лежать у `assets/packs` і встановлюються під час першого запуску.

## Перевірки

```powershell
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
.\target\release\taskify.exe validate all
```

Зараз у наборі **189 тести**.

## Участь

Issues і pull requests вітаються. Перед pull request запусти перевірки вище й переклади всі користувацькі рядки на кожну мову з `crates/ui/src/i18n.rs`.

## Ліцензія

Taskify поширюється за ліцензією MIT. Див. [LICENSE](LICENSE). Вбудовані шрифти Noto — під SIL Open Font License, див. `assets/fonts/LICENSE-OFL.txt`.
