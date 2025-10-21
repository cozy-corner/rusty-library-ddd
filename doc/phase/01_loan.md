# Phase 1: 貸出管理コンテキスト - 設計ドキュメント

## Phase 1の目的

### 学習目標

**関数型DDDの基礎：**
- 純粋関数でドメインロジックを表現する
- イミュータブルなデータ構造
- 決定と実行の分離
- 副作用の境界での管理

**DDDの戦術的設計：**
- 集約の設計判断
- 値オブジェクトの活用
- ドメインイベント駆動
- コンテキスト境界を守る実装

**イベントソーシング：**
- イベントストアの実装
- fold/reduceによる状態復元
- CQRS（コマンドとクエリの分離）

**ヘキサゴナルアーキテクチャ：**
- ポート&アダプターの実装
- 依存性の逆転

---

## スコープ

### 実装する範囲

**機能：**
- 書籍を貸し出す
- 貸出を延長する
- 書籍を返却する
- 延滞を検出する（バッチ処理）

**技術実装：**
- Loan集約
- イベントソーシング（PostgreSQL）
- CQRS（Read Model）
- REST API

**他コンテキストへの依存：**
- MemberService（ポート定義 + モック実装）
- BookService（ポート定義 + モック実装）
- NotificationService（ポート定義 + モック実装）

### 実装しない範囲

- 予約管理
- 会員管理の実装
- カタログ管理の実装
- コンテキスト間の統合
- フロントエンド

---

## ドメインモデル

### Loan集約

**責務：**
1冊の書籍の1回の貸出を管理する。

**管理する情報：**
- 誰が（member_id）
- いつ（loaned_at）
- どの本を（book_id）
- いつまでに（due_date）
- 返却したか（returned_at）
- 延長したか（extension_count）
- 延滞しているか（status）

**管理しない情報：**
- 会員の詳細（名前、住所など）
- 書籍の詳細（タイトル、著者など）
- 職員の詳細（所属、権限など）

**設計の理由：**
他の集約の詳細はIDで参照する。コンテキスト境界を守る。

### 値オブジェクト

**LoanId:**
貸出の識別子。

**BookId, MemberId, StaffId:**
他コンテキストへの参照。詳細は知らない。

**ExtensionCount:**
延長回数（0または1）。ビジネスルール「延長は1回まで」を型で強制。

**LoanStatus:**
貸出状態（Active, Overdue, Returned）。状態遷移を型で管理。

### コマンド

**LoanBook:**
書籍を貸し出す。

**ExtendLoan:**
貸出を延長する。

**ReturnBook:**
書籍を返却する。

### イベント

**BookLoaned:**
書籍が貸出された。

**LoanExtended:**
貸出が延長された。

**BookReturned:**
書籍が返却された。

**LoanBecameOverdue:**
貸出が延滞した。

---

## ビジネスルール

### 貸出時の制約

- 会員が存在すること
- 書籍が貸出可能であること
- 会員に延滞中の貸出がないこと
- 会員の貸出数が5冊未満であること

### 延長のルール

- 貸出が存在すること
- 返却済みでないこと
- 延滞していないこと
- 延長回数が1未満であること
- 延長期間：現在の返却期限 + 2週間

### 返却のルール

- 貸出が存在すること
- 返却済みでないこと
- 延滞していても返却は受け付ける

### 延滞検出のルール

- 返却期限を過ぎている
- まだ返却されていない
- まだ延滞マークされていない

---

## 主要な設計判断

### 1. 純粋関数によるドメインロジック

**ドメイン層の関数：**
- `loan_book()` - 書籍を貸し出す
- `extend_loan()` - 貸出を延長する
- `return_book()` - 書籍を返却する
- `mark_overdue()` - 延滞をマークする

**特徴：**
- 副作用なし
- 入力：現在の状態、パラメータ
- 出力：新しい状態、イベント
- テストが容易

**決定と実行の分離：**
- ドメイン層：何をすべきか決定（純粋関数）
- アプリケーション層：副作用を実行（I/O）

### 2. イミュータビリティ

**すべてのドメインオブジェクトは不変：**
- 値オブジェクト
- 集約
- イベント

**操作は新しいインスタンスを返す：**
元の状態は変更せず、新しい状態を生成。

### 3. 型でビジネスルールを表現

**ExtensionCount：**
「延長は1回まで」を型で強制。不正な値（2以上）を作れない。

**LoanStatus：**
状態遷移を型で制御。返却済みの貸出は延長できない。

### 4. コンテキスト境界を守る

**ポート経由で依存：**
- MemberService（trait）
- BookService（trait）
- NotificationService（trait）

**Phase 1のモック実装：**
- 固定値を返す
- データベースを持たない
- 他コンテキストの代わり

**設計の理由：**
Phase 4でHTTP実装に切り替え可能。最初からポート設計することで、後で実装を変更できる。

### 5. イベントソーシング

**イベントストア：**
すべてのドメインイベントをPostgreSQLに保存。

**状態復元：**
イベント列をfold/reduceして現在の状態を復元。

**CQRS：**
- コマンド側：イベントストアに書き込み
- クエリ側：Read Model（loans_viewテーブル）から読み込み

### 6. ヘキサゴナルアーキテクチャ

**レイヤー構成：**
```
domain/          純粋関数、ビジネスロジック
  ↑
application/     ユースケース、副作用の実行
  ↑
ports/           トレイト定義
  ↑
adapters/        実装（PostgreSQL, モック, REST API）
```

**依存の方向：**
外側が内側に依存。ドメイン層は何にも依存しない。

---

## 技術スタック

### 言語：Rust

**選定理由：**
- イミュータビリティがデフォルト
- 強力な型システム
- Result型によるエラーハンドリング
- トレイトシステム（ポート設計に最適）
- 関数型プログラミングとの相性

### データベース：PostgreSQL

**用途：**
- イベントストア（eventsテーブル）
- Read Model（loans_viewテーブル）

**選定理由：**
- イベントソーシングに必要
- JSONBでイベントデータを保存
- トランザクション管理

### Webフレームワーク：Axum

**用途：**
- REST API

**選定理由：**
- Tokio上で動作
- 型安全
- シンプル

### その他：**
- async/await（非同期処理）
- serde（シリアライゼーション）
- chrono（日時処理）
- uuid（ID生成）
- sqlx（データベースアクセス）

---

## プロジェクト構造

```
loan_management/
├── src/
│   ├── domain/
│   │   ├── loan.rs              # Loan集約、純粋関数
│   │   ├── value_objects.rs     # 値オブジェクト
│   │   ├── commands.rs          # コマンド定義
│   │   ├── events.rs            # イベント定義
│   │   └── errors.rs            # ドメインエラー
│   │
│   ├── ports/
│   │   ├── event_store.rs       # trait EventStore
│   │   ├── loan_read_model.rs   # trait LoanReadModel
│   │   ├── member_service.rs    # trait MemberService
│   │   ├── book_service.rs      # trait BookService
│   │   └── notification_service.rs
│   │
│   ├── adapters/
│   │   ├── mock/                # モック実装
│   │   │   ├── member_service.rs
│   │   │   ├── book_service.rs
│   │   │   └── notification_service.rs
│   │   ├── postgres/            # PostgreSQL実装
│   │   │   ├── event_store.rs
│   │   │   └── loan_read_model.rs
│   │   └── api/                 # REST API
│   │       └── handlers.rs
│   │
│   ├── application/
│   │   └── loan_service.rs      # ユースケース
│   │
│   └── main.rs
│
├── Cargo.toml
└── migrations/                  # データベースマイグレーション
    ├── 001_create_events.sql
    └── 002_create_loans_view.sql
```

---

## データベーススキーマ

### イベントストア

```sql
CREATE TABLE events (
    sequence_number BIGSERIAL PRIMARY KEY,
    aggregate_id UUID NOT NULL,
    aggregate_type VARCHAR(50) NOT NULL,
    event_type VARCHAR(100) NOT NULL,
    event_data JSONB NOT NULL,
    occurred_at TIMESTAMPTZ NOT NULL
);
```

**設計のポイント：**
- sequence_number：イベントの順序を保証
- aggregate_id：集約ごとにイベントをグループ化
- event_data：JSONBでイベントの完全な情報を保存
- 追記のみ（UPDATE/DELETEしない）

### Read Model

```sql
CREATE TABLE loans_view (
    loan_id UUID PRIMARY KEY,
    book_id UUID NOT NULL,
    member_id UUID NOT NULL,
    loaned_at TIMESTAMPTZ NOT NULL,
    due_date TIMESTAMPTZ NOT NULL,
    returned_at TIMESTAMPTZ,
    extension_count SMALLINT NOT NULL,
    status VARCHAR(20) NOT NULL
);
```

**設計のポイント：**
- クエリ最適化されたビュー
- 非正規化（IDのみ）
- インデックスで高速検索

---

## API設計

### エンドポイント

**貸出を作成：**
```
POST /loans
```

**貸出を延長：**
```
POST /loans/{loan_id}/extend
```

**貸出を返却：**
```
POST /loans/{loan_id}/return
```

**貸出一覧を取得：**
```
GET /loans?member_id={uuid}&status={active|overdue|returned}
```

**貸出詳細を取得：**
```
GET /loans/{loan_id}
```

---

## 実装の優先順位

### ステップ1：ドメイン層

1. 値オブジェクトを定義
2. Loan集約を定義
3. 純粋関数を実装
4. コマンド、イベントを定義
5. fold/reduceによる状態復元を実装

**確認ポイント：**
- すべて純粋関数か
- イミュータブルか
- 型でビジネスルールを表現しているか

### ステップ2：ポート定義

1. EventStoreトレイトを定義
2. LoanReadModelトレイトを定義
3. MemberServiceトレイトを定義
4. BookServiceトレイトを定義
5. NotificationServiceトレイトを定義

**確認ポイント：**
- 必要最小限のインターフェースか
- 利用側の視点で定義されているか
- 実装方法に依存していないか

### ステップ3：アダプター（モック）

1. MemberServiceのモック実装
2. BookServiceのモック実装
3. NotificationServiceのモック実装

**実装方針：**
- 固定値を返す
- データを保存しない

### ステップ4：アダプター（PostgreSQL）

1. EventStoreの実装
2. LoanReadModelの実装
3. プロジェクター（イベント → Read Model更新）

### ステップ5：アプリケーション層

1. LoanServiceの実装
2. 各ユースケースの実装
3. 延滞検出バッチの実装

**実装方針：**
- 副作用の実行
- ドメイン層の純粋関数を呼ぶ
- エラーハンドリング

### ステップ6：API層

1. Axumでのルーター設定
2. ハンドラーの実装
3. リクエスト/レスポンス型の定義

### ステップ7：統合

1. main.rsで依存性注入
2. データベースマイグレーション
3. 動作確認

---

## テスト戦略

### ドメイン層のテスト

**テストすべきこと：**
- 正常系の動作
- ビジネスルール違反のエラー
- 境界値
- 状態復元（fold/reduce）

**特徴：**
- 純粋関数なので単体テストが容易
- データベース不要
- 高速実行

### アプリケーション層のテスト

**テストすべきこと：**
- ユースケースの正常系
- 外部サービスのエラーハンドリング
- イベント保存とRead Model更新

**実装方針：**
- モックを使った統合テスト

### API層のテスト

**テストすべきこと：**
- HTTPリクエストの処理
- レスポンス形式
- エラーレスポンス

---

## 動作確認の方法

### curl での確認

**貸出を作成：**
```bash
curl -X POST http://localhost:3000/loans \
  -H "Content-Type: application/json" \
  -d '{
    "book_id": "...",
    "member_id": "...",
    "staff_id": "..."
  }'
```

**貸出を延長：**
```bash
curl -X POST http://localhost:3000/loans/{loan_id}/extend
```

**貸出を返却：**
```bash
curl -X POST http://localhost:3000/loans/{loan_id}/return
```

**貸出一覧を取得：**
```bash
curl http://localhost:3000/loans?member_id={uuid}
```

### Postman での確認

コレクションを作成して、各エンドポイントをテスト。

---

## 完成の定義

Phase 1が完成したと言える基準：

**機能：**
- [ ] 貸出、延長、返却ができる
- [ ] 延滞検出が動作する
- [ ] Read Modelが正しく更新される

**設計：**
- [ ] ドメイン層が純粋関数のみ
- [ ] ポート経由で他コンテキストに依存
- [ ] イベントソーシングが動作
- [ ] CQRSが実装されている

**テスト：**
- [ ] ドメイン層の単体テストが通る
- [ ] アプリケーション層の統合テストが通る

**学習：**
- [ ] 関数型DDDのパターンを理解した
- [ ] コンテキスト境界の重要性を体感した
- [ ] イベントソーシングの実装ができた

---

## Phase 1完了後

Phase 1が完了したら：

1. コードレビュー（自己レビュー）
2. 学んだことを記録
3. Phase 2の準備
