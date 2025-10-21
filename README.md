# Rusty Library DDD

公立図書館管理システムを題材に、関数型ドメイン駆動設計（Functional DDD）を実践的に学ぶプロジェクト。

## 環境要件

- **Rust**: edition 2024
- **PostgreSQL**: 17（Dockerで実行）
- **Docker & Docker Compose**
- **just**: タスクランナー

## セットアップ

### 1. justのインストール

```bash
cargo install just
```

### 2. 環境セットアップ

```bash
just setup
```

このコマンドで以下が実行されます：
- `.env`ファイルの作成（`.env.example`からコピー）
- PostgreSQL（Docker）の起動

## 開発

### よく使うコマンド

```bash
# 利用可能なコマンド一覧
just

# データベース起動
just db-up

# データベース停止
just db-down

# データベースリセット（データも削除）
just db-reset

# テスト実行
just test

# すべてのチェック（format, lint, test）
just check

# アプリケーション実行
just run

# 開発環境起動（DB起動 + アプリ実行）
just dev
```

### その他のコマンド

```bash
# フォーマット
just fmt

# Lint
just clippy

# ビルド
just build

# データベースログ確認
just db-logs

# データベース接続確認
just db-status

# クリーン
just clean
```

## プロジェクト構成

詳細は `doc/` ディレクトリのドキュメントを参照してください。

- [プロジェクト概要](doc/01_overview.md)
- [Phase 1: 貸出管理](doc/phase/01_loan.md)
- [開発ガイド](claude.md)

## 品質管理

このプロジェクトでは、コミット前に自動的に以下がチェックされます（cargo-husky）：
- コードフォーマット
- 静的解析（clippy）
- テスト

エラーがある場合、コミットは拒否されます。
