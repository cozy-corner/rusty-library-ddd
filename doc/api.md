# API ドキュメント

Rusty Library DDD の REST API リファレンス。

## ベースURL

```
http://localhost:3000
```

デフォルトポートは3000です。環境変数`PORT`で変更可能です。

## エンドポイント一覧

| メソッド | パス | 説明 |
|---------|------|------|
| POST | /loans | 貸出を作成 |
| POST | /loans/:id/extend | 貸出を延長 |
| POST | /loans/:id/return | 本を返却 |
| GET | /loans/:id | 貸出の詳細を取得 |
| GET | /loans | 貸出の一覧を取得（フィルタリング可能） |

---

## 1. 貸出を作成

会員に本を貸し出します。

### リクエスト

```http
POST /loans
Content-Type: application/json

{
  "book_id": "550e8400-e29b-41d4-a716-446655440000",
  "member_id": "650e8400-e29b-41d4-a716-446655440000",
  "staff_id": "750e8400-e29b-41d4-a716-446655440000"
}
```

**パラメータ:**

| フィールド | 型 | 必須 | 説明 |
|-----------|-----|------|------|
| book_id | UUID | ✓ | 貸し出す本のID |
| member_id | UUID | ✓ | 借りる会員のID |
| staff_id | UUID | ✓ | 貸出処理を行う職員のID |

**ビジネスルール:**
- 会員が存在すること
- 本が貸出可能であること
- 会員が延滞中の本を持っていないこと
- 会員の貸出数が上限（5冊）未満であること

### レスポンス

**成功 (201 Created):**

```json
{
  "loan_id": "750e8400-e29b-41d4-a716-446655440000",
  "book_id": "550e8400-e29b-41d4-a716-446655440000",
  "member_id": "650e8400-e29b-41d4-a716-446655440000",
  "loaned_at": "2025-01-15T10:30:00Z",
  "due_date": "2025-01-29T10:30:00Z"
}
```

**エラーレスポンス:**

| ステータス | 説明 |
|-----------|------|
| 404 Not Found | 会員が見つからない |
| 409 Conflict | 本が貸出不可、または会員が延滞中 |
| 400 Bad Request | 貸出上限超過、または不正なリクエスト |

### curlコマンド例

```bash
curl -X POST http://localhost:3000/loans \
  -H "Content-Type: application/json" \
  -d '{
    "book_id": "550e8400-e29b-41d4-a716-446655440000",
    "member_id": "650e8400-e29b-41d4-a716-446655440000",
    "staff_id": "750e8400-e29b-41d4-a716-446655440000"
  }'
```

---

## 2. 貸出を延長

貸出期間を14日間延長します（1回のみ可能）。

### リクエスト

```http
POST /loans/:id/extend
Content-Type: application/json
```

**パスパラメータ:**

| パラメータ | 型 | 説明 |
|-----------|-----|------|
| id | UUID | 延長する貸出のID |

**ビジネスルール:**
- 貸出が存在すること
- 貸出がActive状態であること
- 延長回数が0回であること（未延長）

### レスポンス

**成功 (200 OK):**

```json
{
  "loan_id": "750e8400-e29b-41d4-a716-446655440000",
  "new_due_date": "2025-02-12T10:30:00Z",
  "extended_at": "2025-01-25T14:20:00Z"
}
```

**エラーレスポンス:**

| ステータス | 説明 |
|-----------|------|
| 404 Not Found | 貸出が見つからない |
| 400 Bad Request | 既に延長済み、または延長不可能な状態 |

### curlコマンド例

```bash
curl -X POST http://localhost:3000/loans/750e8400-e29b-41d4-a716-446655440000/extend \
  -H "Content-Type: application/json"
```

---

## 3. 本を返却

貸し出された本を返却します。

### リクエスト

```http
POST /loans/:id/return
Content-Type: application/json
```

**パスパラメータ:**

| パラメータ | 型 | 説明 |
|-----------|-----|------|
| id | UUID | 返却する貸出のID |

**ビジネスルール:**
- 貸出が存在すること
- 貸出がActive または Overdue 状態であること

### レスポンス

**成功 (200 OK):**

```json
{
  "loan_id": "750e8400-e29b-41d4-a716-446655440000",
  "returned_at": "2025-01-28T16:45:00Z"
}
```

**エラーレスポンス:**

| ステータス | 説明 |
|-----------|------|
| 404 Not Found | 貸出が見つからない |
| 400 Bad Request | 既に返却済み |

### curlコマンド例

```bash
curl -X POST http://localhost:3000/loans/750e8400-e29b-41d4-a716-446655440000/return \
  -H "Content-Type: application/json"
```

---

## 4. 貸出の詳細を取得

指定された貸出の詳細情報を取得します。

### リクエスト

```http
GET /loans/:id
```

**パスパラメータ:**

| パラメータ | 型 | 説明 |
|-----------|-----|------|
| id | UUID | 取得する貸出のID |

### レスポンス

**成功 (200 OK):**

```json
{
  "loan_id": "750e8400-e29b-41d4-a716-446655440000",
  "book_id": "550e8400-e29b-41d4-a716-446655440000",
  "member_id": "650e8400-e29b-41d4-a716-446655440000",
  "loaned_at": "2025-01-15T10:30:00Z",
  "due_date": "2025-01-29T10:30:00Z",
  "returned_at": null,
  "extension_count": 0,
  "status": "active",
  "created_at": "2025-01-15T10:30:00Z",
  "updated_at": "2025-01-15T10:30:00Z"
}
```

**フィールド説明:**

| フィールド | 型 | 説明 |
|-----------|-----|------|
| loan_id | UUID | 貸出ID |
| book_id | UUID | 本のID |
| member_id | UUID | 会員のID |
| loaned_at | DateTime | 貸出日時 |
| due_date | DateTime | 返却期限 |
| returned_at | DateTime? | 返却日時（未返却の場合はnull） |
| extension_count | integer | 延長回数（0または1） |
| status | string | 貸出状態（"active", "overdue", "returned"） |
| created_at | DateTime | レコード作成日時 |
| updated_at | DateTime | レコード更新日時 |

**エラーレスポンス:**

| ステータス | 説明 |
|-----------|------|
| 404 Not Found | 貸出が見つからない |

### curlコマンド例

```bash
curl http://localhost:3000/loans/750e8400-e29b-41d4-a716-446655440000
```

---

## 5. 貸出の一覧を取得

貸出の一覧を取得します。クエリパラメータでフィルタリングが可能です。

### リクエスト

```http
GET /loans?member_id={member_id}&status={status}
```

**クエリパラメータ:**

| パラメータ | 型 | 必須 | 説明 |
|-----------|-----|------|------|
| member_id | UUID | - | 指定した会員の貸出のみ取得 |
| status | string | - | 指定した状態の貸出のみ取得（"active", "overdue", "returned"） |

パラメータは組み合わせ可能です。パラメータを省略した場合、すべての貸出を取得します。

### レスポンス

**成功 (200 OK):**

```json
[
  {
    "loan_id": "750e8400-e29b-41d4-a716-446655440000",
    "book_id": "550e8400-e29b-41d4-a716-446655440000",
    "member_id": "650e8400-e29b-41d4-a716-446655440000",
    "loaned_at": "2025-01-15T10:30:00Z",
    "due_date": "2025-01-29T10:30:00Z",
    "returned_at": null,
    "extension_count": 0,
    "status": "active",
    "created_at": "2025-01-15T10:30:00Z",
    "updated_at": "2025-01-15T10:30:00Z"
  },
  {
    "loan_id": "850e8400-e29b-41d4-a716-446655440000",
    "book_id": "950e8400-e29b-41d4-a716-446655440000",
    "member_id": "650e8400-e29b-41d4-a716-446655440000",
    "loaned_at": "2025-01-10T14:20:00Z",
    "due_date": "2025-01-24T14:20:00Z",
    "returned_at": "2025-01-23T09:15:00Z",
    "extension_count": 0,
    "status": "returned",
    "created_at": "2025-01-10T14:20:00Z",
    "updated_at": "2025-01-23T09:15:00Z"
  }
]
```

貸出が存在しない場合は空の配列`[]`を返します。

### curlコマンド例

**すべての貸出を取得:**

```bash
curl http://localhost:3000/loans
```

**特定の会員の貸出を取得:**

```bash
curl http://localhost:3000/loans?member_id=650e8400-e29b-41d4-a716-446655440000
```

**Active状態の貸出のみ取得:**

```bash
curl http://localhost:3000/loans?status=active
```

**特定の会員のActive状態の貸出のみ取得:**

```bash
curl "http://localhost:3000/loans?member_id=650e8400-e29b-41d4-a716-446655440000&status=active"
```

---

## エラーレスポンス形式

すべてのエラーレスポンスは以下の形式で返されます:

```json
{
  "error": "エラーの種類",
  "message": "エラーの詳細メッセージ"
}
```

### 共通のHTTPステータスコード

| ステータスコード | 説明 |
|----------------|------|
| 200 OK | リクエストが成功 |
| 201 Created | リソースの作成に成功 |
| 400 Bad Request | リクエストが不正（バリデーションエラー、ビジネスルール違反など） |
| 404 Not Found | リソースが見つからない |
| 409 Conflict | リソースの状態が競合（本が貸出不可、会員が延滞中など） |
| 500 Internal Server Error | サーバー内部エラー |

---

## 使用例: 完全な貸出フロー

以下は、貸出から返却までの完全なフローの例です。

### 1. 本を借りる

```bash
LOAN_RESPONSE=$(curl -s -X POST http://localhost:3000/loans \
  -H "Content-Type: application/json" \
  -d '{
    "book_id": "550e8400-e29b-41d4-a716-446655440000",
    "member_id": "650e8400-e29b-41d4-a716-446655440000"
  }')

LOAN_ID=$(echo $LOAN_RESPONSE | jq -r '.loan_id')
echo "Created loan: $LOAN_ID"
```

### 2. 貸出状態を確認

```bash
curl http://localhost:3000/loans/$LOAN_ID
```

### 3. 延長する

```bash
curl -X POST http://localhost:3000/loans/$LOAN_ID/extend \
  -H "Content-Type: application/json"
```

### 4. 返却する

```bash
curl -X POST http://localhost:3000/loans/$LOAN_ID/return \
  -H "Content-Type: application/json"
```

### 5. 返却済みの状態を確認

```bash
curl http://localhost:3000/loans/$LOAN_ID
```

---

## 備考

### イベントソーシング

このAPIはイベントソーシングアーキテクチャを採用しています。すべてのコマンド操作（POST）はイベントとして永続化され、Read Model（クエリ用のビュー）に反映されます。

### CQRS

読み取り操作（GET）と書き込み操作（POST）は分離されており、それぞれ最適化されています。

### 非同期処理

現在の実装ではすべての操作が同期的に処理されますが、将来的には非同期イベント処理やバッチ処理（延滞検出など）が追加される可能性があります。
