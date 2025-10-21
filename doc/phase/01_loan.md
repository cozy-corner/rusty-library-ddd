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

## ユビキタス言語

### 集約

**貸出（Loan）**

1冊の書籍の1回の貸出。貸出管理コンテキストの中心概念。会員が書籍を借りた時点で開始し、返却した時点で終了する。

### 他コンテキストへの参照

**会員（Member）**

図書館の利用者。このコンテキストでは会員ID（MemberId）のみを保持する。詳細は会員管理コンテキストが管理する。

**書籍（Book）**

図書館の蔵書。このコンテキストでは書籍ID（BookId）のみを保持する。詳細はカタログ管理コンテキストが管理する。

**職員（Staff）**

図書館の職員。貸出・返却の受付を行う。このコンテキストでは職員ID（StaffId）のみを保持する。

### 値オブジェクト

**貸出ID（LoanId）**

貸出を一意に識別するID。

**延長回数（ExtensionCount）**

貸出を延長した回数。0または1のみ許可される。ビジネスルール「延長は1回まで」を型で表現する。

**貸出状態（LoanStatus）**

貸出の現在の状態：
- **貸出中（Active）**: 通常の貸出状態
- **延滞中（Overdue）**: 返却期限を過ぎている
- **返却済み（Returned）**: 返却が完了している

### 時間に関する用語

**貸出日（Loaned At）**

書籍を貸し出した日時。

**返却期限（Due Date）**

書籍を返却しなければならない日時。貸出日から2週間後に自動設定される。延長した場合は、さらに2週間延長される。

**返却日（Returned At）**

実際に書籍が返却された日時。

**貸出期間（Loan Period）**

書籍を借りることができる期間。標準は2週間。延長すると合計4週間になる。

### コマンド（意図）

**書籍を貸し出す（LoanBook）**

会員が書籍を借りる行為。職員が受付を行う。

**貸出を延長する（ExtendLoan）**

返却期限を延長する行為。延長は1回まで可能。返却期限が2週間延長される。

**書籍を返却する（ReturnBook）**

借りていた書籍を返す行為。職員が受付を行う。延滞していても返却は受け付ける。

### イベント（事実）

**書籍が貸し出された（BookLoaned）**

書籍の貸出が完了した事実。貸出ID、書籍ID、会員ID、貸出日、返却期限、職員IDを記録する。

**貸出が延長された（LoanExtended）**

貸出の返却期限が延長された事実。貸出ID、旧返却期限、新返却期限、延長回数を記録する。

**書籍が返却された（BookReturned）**

書籍の返却が完了した事実。貸出ID、書籍ID、会員ID、返却日、延滞していたかを記録する。

**貸出が延滞した（LoanBecameOverdue）**

返却期限を過ぎても返却されていない事実。バッチ処理で検出される。

### ビジネスルール関連の用語

**延滞（Overdue）**

返却期限を過ぎても書籍が返却されていない状態。延滞中の会員は新規の貸出ができない。

**延長（Extension）**

返却期限を延ばすこと。延長は1回まで可能。延長すると返却期限が2週間延長される。延滞中の貸出は延長できない。

**貸出上限（Loan Limit）**

1人の会員が同時に借りられる書籍の最大数。標準は5冊。

**貸出可能（Available for Loan）**

書籍が貸出できる状態。他の会員に貸出中でなく、予約による取り置きもされていない状態。

### コンテキスト境界

**このコンテキストでは：**
書籍と会員はIDでのみ参照し、詳細は知らない。これによりコンテキスト境界を守る。

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
## データベーススキーマ

### イベントストア

```sql
CREATE TABLE events (
    sequence_number BIGSERIAL PRIMARY KEY,
    aggregate_id UUID NOT NULL,
    aggregate_type VARCHAR(50) NOT NULL,
    event_type VARCHAR(100) NOT NULL,
    event_data JSONB NOT NULL,
    occurred_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_events_aggregate ON events(aggregate_id);
CREATE INDEX idx_events_type ON events(event_type);
CREATE INDEX idx_events_occurred_at ON events(occurred_at);
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
    status VARCHAR(20) NOT NULL,
    created_at TIMESTAMPTZ NOT NULL,
    updated_at TIMESTAMPTZ NOT NULL
);

CREATE INDEX idx_loans_member ON loans_view(member_id);
CREATE INDEX idx_loans_book ON loans_view(book_id);
CREATE INDEX idx_loans_status ON loans_view(status);
CREATE INDEX idx_loans_due_date ON loans_view(due_date) 
    WHERE returned_at IS NULL;
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
