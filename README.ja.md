# Taskify

[English](README.en.md) · [Русский](README.ru.md) · **日本語** · [Українська](README.uk.md) · [Polski](README.pl.md) · [Español](README.es.md) · [中文](README.zh.md) · [Deutsch](README.de.md)

Taskify は Windows 向けのローカルな生活ゲーミフィケーションアプリです。習慣、繰り返しタスク、単発タスク、チェックリストを管理し、XP とコインを獲得できます。実績、テーマ、コスメ、ステッカーを解放しましょう。データはすべて JSONL のイベントジャーナルとして PC 内に保存され、アカウントもネットワークも不要です。

![「今日」画面](docs/presentation/today.png)

## 機能

- 4 種類のタスク：習慣、繰り返し、単発、サブタスク付きチェックリスト。
- 上限のないレベル、日次・週次の進捗、連続記録、コンボ、実績。
- コイン、コスメショップ、テーマ、アクセント、背景、フォント、サウンド、ドラッグできるステッカー。
- TOML 形式のコンテンツパック：タスク、カテゴリ、タグ、アイコン、パレット、コンボ、実績、コスメ、バンドル。
- 複数のライブスペース：それぞれに履歴、購入、ステッカーボードがあります。
- 8 言語の UI：ロシア語、英語、日本語、ウクライナ語、ポーランド語、スペイン語、中国語、ドイツ語。
- ローカル保存のみ：イベントジャーナル、再構築できるキャッシュ、自動バックアップ。
- `TASKIFY_HOME` によるポータブルモード。

## 必要な環境

- Windows 10 以降。
- Rust 1.85 以降。
- Rust の環境に C++ ビルドツールがない場合は別途インストールしてください。

## ビルドと実行

```powershell
cargo build --release
.\target\release\taskify.exe
```

同じ実行ファイルでコマンドラインも使えます。

```powershell
.\target\release\taskify.exe version
.\target\release\taskify.exe packs
.\target\release\taskify.exe stats default
.\target\release\taskify.exe validate history default
```

初回起動時に `default` ライブスペースが作成され、組み込みパックがインストールされます。

## データ

既定では `%APPDATA%\taskify` に保存されます。

- `livespaces\<名前>` — ジャーナル `data/log/YYYY-MM.jsonl`、キャッシュ `state.json` と `aggregates.json`、バックアップ。
- `packs\<id>` — インストール済みのコンテンツパック。
- `settings.toml` — ウィンドウとアプリの設定。

`TASKIFY_HOME` を設定すると、データディレクトリ全体を別の場所に移せます（ポータブルモード）。ジャーナルが正本で、キャッシュは削除してもイベントから再構築できます。

## コマンドライン

```text
taskify version
taskify packs [<data-root>]
taskify validate pack <パックのパス>
taskify validate all [<data-root>]
taskify validate history <ライブスペース>
taskify stats <ライブスペース>
taskify log <ライブスペース> [--limit N]
taskify rebuild <ライブスペース>
taskify backup <ライブスペース>
taskify import <アーカイブ.zip> <ライブスペース> [<data-root>]
taskify export <ライブスペース> <アーカイブ.zip>
```

## プロジェクト構成

| クレート | 役割 |
| --- | --- |
| `crates/core` | タスク、報酬、レベル曲線、メトリクス、日付境界。 |
| `crates/store` | イベント、ジャーナル、キャッシュ、リプレイ、バックアップ、ライブスペース。 |
| `crates/packs` | TOML パックの読み込み、レジストリ、バリデータ、zip インストール。 |
| `crates/ui` | egui/eframe のウィンドウ、画面、ゲームルール、ローカライズ、CLI。 |


## パック

パックは `pack.toml` と、タスク、コンボ、実績、コスメ、バンドル、定義、メディア用のサブフォルダを持つフォルダです。ID にはパックのプレフィックスが付きます（`greeting/hello`）。組み込みパックは `assets/packs` にあり、初回起動時にインストールされます。

## チェック

```powershell
cargo fmt --all --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
.\target\release\taskify.exe validate all
```

現在のテストは **183 件**です。

## コントリビュート

Issue と Pull Request を歓迎します。Pull Request の前に上記のチェックを実行し、`crates/ui/src/i18n.rs` にあるすべての言語へ表示テキストを翻訳してください。

## ライセンス

Taskify は MIT ライセンスで公開されています。[LICENSE](LICENSE) を参照してください。同梱の Noto フォントは SIL Open Font License です。`assets/fonts/LICENSE-OFL.txt` を参照してください。
