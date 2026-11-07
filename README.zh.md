# Taskify

[English](README.en.md) · [Русский](README.ru.md) · [日本語](README.ja.md) · [Українська](README.uk.md) · [Polski](README.pl.md) · [Español](README.es.md) · **中文** · [Deutsch](README.de.md)

Taskify 是一款面向桌面的本地个人事务游戏化应用。管理习惯、重复任务、一次性任务和清单；获得 XP 和金币；解锁成就、主题、外观和贴纸。所有数据都以 JSONL 事件日志的形式保存在你的电脑上，无需账号，也无需联网。

![“今天”界面](docs/presentation/today.png)

## 功能

- 四种任务：习惯、重复任务、一次性任务，以及带子任务的清单。
- 无上限等级、每日与每周进度、连续记录、组合和成就。
- 金币、外观商店、主题、强调色、背景、字体、声音和可拖动的贴纸。
- TOML 内容包：任务、分类、标签、图标、调色板、组合、成就、外观和套装。
- 多个生活空间：每个都有自己的历史、购买记录和贴纸板。
- 八种界面语言：俄语、英语、日语、乌克兰语、波兰语、西班牙语、中文和德语。
- 仅本地存储：事件日志、可重建的缓存和自动备份。
- 通过 `TASKIFY_HOME` 使用便携模式。

## 要求

- Windows 10 或更高版本、macOS 11 或更高版本，或带 GTK3、X11/Wayland 和 OpenGL 的 Linux。
- Rust 1.85 或更高版本。
- 如果 Rust 环境尚未包含 C++ 构建工具，请另行安装。

## 构建与运行

```powershell
cargo build --release
.\tools\package.ps1          # Windows
.\target\release\taskify.exe
```

```bash
cargo build --release
./tools/package.sh           # Linux: tar.gz；macOS: Taskify.app 压缩包
./target/release/taskify
```

同一个可执行文件也提供命令行工具：

```bash
target/release/taskify version
target/release/taskify packs
target/release/taskify stats default
target/release/taskify validate history default
```

macOS 版本仅做 ad-hoc 签名，未经公证：首次启动若被 Gatekeeper 拦截，请右键 →“打开”。

首次启动时会创建 `default` 生活空间并安装内置内容包。

## 数据

默认全部保存在 `%APPDATA%\taskify`：

- `livespaces\<名称>` — 日志 `data/log/YYYY-MM.jsonl`、缓存 `state.json` 和 `aggregates.json`、备份。
- `packs\<id>` — 已安装的内容包。
- `settings.toml` — 窗口和应用设置。

设置 `TASKIFY_HOME` 可以把整个数据目录移到其他位置（便携模式）。日志是唯一事实来源：缓存可以删除，并从事件重建。

## 命令行

```text
taskify version
taskify packs [<data-root>]
taskify validate pack <内容包路径>
taskify validate all [<data-root>]
taskify validate history <生活空间>
taskify stats <生活空间>
taskify log <生活空间> [--limit N]
taskify rebuild <生活空间>
taskify backup <生活空间>
taskify import <压缩包.zip> <生活空间> [<data-root>]
taskify export <生活空间> <压缩包.zip>
```

## 项目结构

| Crate | 职责 |
| --- | --- |
| `crates/core` | 任务、奖励、等级曲线、指标、日期边界。 |
| `crates/store` | 事件、日志、缓存、重放、备份、生活空间。 |
| `crates/packs` | TOML 内容包加载、注册表、校验器、zip 安装。 |
| `crates/ui` | egui/eframe 窗口、界面、游戏规则、本地化、CLI。 |


## 内容包

内容包是一个包含 `pack.toml` 的文件夹，下面有任务、组合、成就、外观、套装、定义和媒体等子文件夹。ID 会带上内容包前缀（`greeting/hello`）。内置内容包位于 `assets/packs`，首次启动时安装。

## 检查

```powershell
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
.\target\release\taskify.exe validate all
```

当前共有 **183 个测试**。

## 贡献

欢迎提交 Issue 和 Pull Request。提交前请运行上面的检查，并把所有界面文本翻译到 `crates/ui/src/i18n.rs` 中列出的每种语言。

## 许可证

Taskify 使用 MIT 许可证，详见 [LICENSE](LICENSE)。内置 Noto 字体使用 SIL Open Font License，详见 `assets/fonts/LICENSE-OFL.txt`。
