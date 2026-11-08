# Taskify

[![Build](https://github.com/Ziczin/taskify/actions/workflows/build.yml/badge.svg)](https://github.com/Ziczin/taskify/actions/workflows/build.yml)

[English](README.en.md) · [Русский](README.ru.md) · [日本語](README.ja.md) · [Українська](README.uk.md) · [Polski](README.pl.md) · [Español](README.es.md) · [中文](README.zh.md) · [Deutsch](README.de.md)

Taskify is a local-first life gamification app for desktop. Keep habits, repeating tasks, one-off tasks, and checklists; earn XP and coins; unlock achievements, themes, cosmetics, and stickers. Everything is stored on your computer as an append-only JSONL journal — no account and no network required.

![Taskify today screen](docs/presentation/today.png)

## Features

- Four task kinds: habits, repeating tasks, one-shot tasks, and checklists with subtasks.
- Task chains: a task opens only after its predecessor has been completed at least once.
- Availability windows: a task can be limited to a time range such as `09:00–18:00`, including ranges that cross midnight.
- New or newly unlocked tasks are highlighted until you mark them as viewed.
- Pack progress can be reset without deleting history or purchases; completed one-shots stay completed.
- Six gentle packs: Getting Started, Greeting, Cleaning, Useful Habits, Inspiration, and Socializing, built from one-action steps and story chains.
- Every completed task pays at least 10 XP and 10 coins; “Getting Started” is a seven-step tour of the interface that is enabled on first launch.
- Unlimited levels, daily and weekly progress, streaks, combos, and achievements.
- Coins, a cosmetic shop, themes, accents, backgrounds, fonts, sounds, and draggable stickers.
- Content packs in TOML: tasks, categories, tags, icons, palettes, combos, achievements, cosmetics, and bundles.
- Multiple livespaces: each one has its own history, purchases, and sticker board.
- Eight UI languages: Russian, English, Japanese, Ukrainian, Polish, Spanish, Chinese, and German.
- Local-only storage with an event journal, rebuildable caches, and automatic backups.
- Portable mode through `TASKIFY_HOME`.

## Requirements

- Windows 10 or newer, macOS 11 or newer, or a Linux desktop with GTK3 and X11/Wayland/OpenGL libraries.
- Rust 1.85 or newer.
- A C++ build toolchain if your Rust installation does not already have one.

## Build and run

GitHub Actions builds and tests every push to `main` on Windows, Linux, and macOS. Download a portable build from the latest workflow artifact or from [Releases](https://github.com/Ziczin/taskify/releases) for `v*` tags.

```powershell
cargo build --release
.\tools\package.ps1          # Windows: dist\taskify-<build>-windows-x64.zip
.\target\release\taskify.exe
```

```bash
cargo build --release
./tools/package.sh           # Linux: tar.gz; macOS: zipped Taskify.app
./target/release/taskify
```

The Windows ZIP and Linux archive must keep the executable and the `assets` folder together. The macOS app already contains `assets` in `Contents/Resources`. The same executable also provides command-line commands:

```bash
target/release/taskify version
target/release/taskify packs
target/release/taskify stats default
target/release/taskify validate history default
```

The macOS build is ad-hoc signed, not notarized, and targets Apple Silicon (`arm64`): on first launch use right-click → Open if Gatekeeper blocks it.

The first launch opens the setup screen, creates a livespace, and installs the six built-in packs. “Getting Started” and “Greeting” are enabled initially; the other packs can be enabled one at a time.

## Data

By default, data lives in the platform data directory:

- Windows: `%APPDATA%\taskify`
- macOS: `~/Library/Application Support/taskify`
- Linux: `$XDG_DATA_HOME/taskify` or `~/.local/share/taskify`

Inside it:

- `livespaces\<name>` — journal `data/log/YYYY-MM.jsonl`, caches `state.json` and `aggregates.json`, backups.
- `packs\<id>` — installed content packs.
- `settings.toml` — window and application settings.

Set `TASKIFY_HOME` to keep the whole data directory somewhere else (portable mode). The journal is the source of truth: caches can be deleted and rebuilt from events.

## Command line

```text
taskify version
taskify packs [<data-root>]
taskify validate pack <pack-path>
taskify validate all [<data-root>]
taskify validate history <livespace>
taskify stats <livespace>
taskify log <livespace> [--limit N]
taskify rebuild <livespace>
taskify backup <livespace>
taskify import <archive.zip> <livespace> [<data-root>]
taskify export <livespace> <archive.zip>
```

## Project structure

| Crate | Responsibility |
| --- | --- |
| `crates/core` | Tasks, rewards, level curve, metrics, day boundaries. |
| `crates/store` | Events, journal, caches, replay, backups, livespaces. |
| `crates/packs` | TOML pack loading, registries, validator, zip installation. |
| `crates/ui` | egui/eframe window, screens, game rules, localization, CLI. |


## Packs

A pack is a folder with `pack.toml` and subfolders for tasks, combos, achievements, cosmetics, bundles, definitions, and media. IDs are namespaced by the pack (`greeting/hello`). Built-in packs live in `assets/packs` and are installed on first launch.

A task can be limited to a time of day with an optional window:

```toml
[window]
from = "09:00"
to = "18:00"
```

If `from` is later than `to`, the window crosses midnight (`22:00`–`06:00`). Without `[window]`, the task is available all day.

## Checks

```powershell
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
.\target\release\taskify.exe validate all
```

The current test suite has **189 tests**.

## Contributing

Issues and pull requests are welcome. Run the checks above before opening a pull request, and keep user-visible text localized in every language listed in `crates/ui/src/i18n.rs`.

## License

Taskify is released under the MIT License. See [LICENSE](LICENSE). Bundled Noto fonts are distributed under the SIL Open Font License; see `assets/fonts/LICENSE-OFL.txt`.
