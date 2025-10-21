# デフォルトレシピ（just実行時）
default:
    @just --list

# データベース起動
db-up:
    docker compose up -d

# データベース停止
db-down:
    docker compose down

# データベースリセット（データも削除）
db-reset:
    docker compose down -v
    docker compose up -d

# データベースログ確認
db-logs:
    docker compose logs -f postgres

# データベース接続確認
db-status:
    docker compose ps

# テスト実行
test:
    cargo test

# フォーマット
fmt:
    cargo fmt --all

# フォーマットチェック
fmt-check:
    cargo fmt --all -- --check

# Lint
clippy:
    cargo clippy --all-targets --all-features -- -D warnings

# すべてのチェック（pre-commitと同じ）
check:
    cargo fmt --all -- --check
    cargo clippy --all-targets --all-features -- -D warnings
    cargo test

# アプリケーション実行
run:
    cargo run

# アプリケーションビルド
build:
    cargo build

# リリースビルド
build-release:
    cargo build --release

# クリーン
clean:
    cargo clean

# 環境セットアップ
setup:
    cp .env.example .env
    docker compose up -d
    @echo "✓ Setup complete! Edit .env if needed."

# 開発環境起動（データベース起動 + アプリケーション実行）
dev:
    just db-up
    cargo run

# ====================
# 開発サイクル（PR単位）
# ====================

# タスクブランチ作成（例: just task-start phase1 value-objects）
task-start phase task_name:
    git checkout -b feature/{{phase}}/{{task_name}}
    @echo "✓ Branch 'feature/{{phase}}/{{task_name}}' created"
    @echo "Next: Implement your changes, then run 'just task-check'"

# タスク完了前のチェック（format, lint, test）
task-check:
    @echo "Running all checks..."
    just check
    @echo "✓ All checks passed!"
    @echo "Next: Commit your changes"

# PR作成準備（リモートへpush）
task-push:
    git push -u origin HEAD
    @echo "✓ Pushed to remote"
    @echo "Next: Create PR with 'just task-pr' or manually on GitHub"

# PR作成（GitHub CLI使用）
task-pr title:
    gh pr create --title "{{title}}" --fill
    @echo "✓ PR created!"

# タスク完了（mainにマージ後、ブランチ削除）
task-done:
    git checkout main
    git pull
    git branch -d @{-1}
    @echo "✓ Task completed and branch deleted"
