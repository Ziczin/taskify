# Taskify

**English** · [Русский](README.ru.md) · [日本語](README.ja.md) · [Українська](README.uk.md) · [Polski](README.pl.md) · [Español](README.es.md) · [中文](README.zh.md) · [Deutsch](README.de.md)

Taskify is a local-first life gamification app for Windows. Keep habits, repeating tasks, one-off tasks, and checklists; earn XP and coins; unlock achievements, themes, cosmetics, and stickers. Everything is stored on your computer as an append-only JSONL journal — no account and no network required.

![Taskify today screen](docs/presentation/today.png)

## Features

- Four task kinds: habits, repeating tasks, one-off tasks, and checklists with subtasks.
- Unlimited levels, daily and weekly progress, streaks, combos, and achievements.
- Coins, a cosmetic shop, themes, accents, backgrounds, fonts, sounds, and draggable stickers.
- Content packs in TOML: tasks, categories, tags, icons, palettes, combos, achievements, cosmetics, and bundles.
- Multiple livespaces: each one has its own history, purchases, and sticker board.
- Eight UI languages: Russian, English, Japanese, Ukrainian, Polish, Spanish, Chinese, and German.
- Local-only storage with an event journal, rebuildable caches, and automatic backups.
- Portable mode through `TASKIFY_HOME`.

## Requirements

- Windows 10 or newer.
- Rust 1.85 or newer.
- A C++ build toolchain if your Rust installation does not already have one.

## Build and run

```powershell
cargo build --release
.\target\release\taskify.exe
```

The same executable also provides command-line commands:

```powershell
.\target\release\taskify.exe version
.\target\release\taskify.exe packs
.\target\release\taskify.exe stats default
.\target\release\taskify.exe validate history default
```

The first launch creates the `default` livespace and installs the built-in packs.

## Data

By default, everything lives in `%APPDATA%\taskify`:

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

A pack is a folder with `pack.toml` and subfolders for tasks, combos, achievements, cosmetics, bundles, definitions, and media. IDs are namespaced by the pack (`starter/water`). Built-in packs live in `assets/packs` and are installed on first launch.

## Checks

```powershell
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
.\target\release\taskify.exe validate all
```

The current test suite has **155 tests**.

## Contributing

Issues and pull requests are welcome. Run the checks above before opening a pull request, and keep user-visible text localized in every language listed in `crates/ui/src/i18n.rs`.

## License

Taskify is released under the MIT License. See [LICENSE](LICENSE). Bundled Noto fonts are distributed under the SIL Open Font License; see `assets/fonts/LICENSE-OFL.txt`.
