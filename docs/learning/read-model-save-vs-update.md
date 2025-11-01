# Read Modelの更新: saveとupdateの判断基準

## 背景

Task 5のPRでCodeRabbitから指摘を受けた問題：

```
extend_loanでextension_countとupdated_atがRead Modelに保存されていない
```

この問題をきっかけに、Read Modelの更新方式について深く議論し、以下の結論に至った。

## 問題の本質

当初の実装では、Read Modelのポート定義に以下の3つのメソッドがあった：

```rust
async fn insert(&self, loan_view: LoanView) -> Result<()>;
async fn update_status(&self, loan_id: LoanId, status: LoanStatus, returned_at: Option<DateTime<Utc>>) -> Result<()>;
async fn update_due_date(&self, loan_id: LoanId, new_due_date: DateTime<Utc>) -> Result<()>;
```

`extend_loan`では`update_due_date()`を呼んでいたため、`due_date`しか更新されず、`extension_count`と`updated_at`が更新されなかった。

## 検討した選択肢

### Option A: update_due_dateを拡張
```rust
async fn update_due_date(&self, loan_id: LoanId, new_due_date: DateTime<Utc>, extension_count: u8) -> Result<()>;
```

**問題点**: 依然として部分更新であり、`updated_at`の更新忘れのリスクが残る

### Option B: 新しい更新メソッドを追加
```rust
async fn update_loan_extension(&self, loan_id: LoanId, new_due_date: DateTime<Utc>, extension_count: u8) -> Result<()>;
```

**問題点**: メソッドが増え続け、更新ケースごとに新メソッドが必要になる

### Option C: イベントオブジェクトを渡す
```rust
async fn apply_event(&self, event: LoanExtended) -> Result<()>;
```

**問題点**: イベントから必要なフィールドを抽出する責務がポート実装側に移る

### Option D: 完全な状態を保存 (採用)
```rust
async fn save(&self, loan_view: LoanView) -> Result<()>;
```

**利点**:
- Read Modelは常に集約の完全な状態を反映
- 更新漏れがコンパイルエラーで検出される（構造体リテラルは全フィールド必須）
- シンプルで一貫性がある

## イベントソーシングの原則からの判断

### イベントソーシングにおけるRead Modelの位置づけ

```
EventStore (Write)  ──→  Read Model (Read)
  ↓                         ↓
Events (Source of Truth)  Projection (Current State)
```

**重要な原則**:
1. **EventStoreが唯一の真実の源泉**: すべてのイベントが記録される
2. **Read Modelは投影（Projection）**: イベントから復元された現在の状態のキャッシュ
3. **完全な状態の再現性**: イベントを再生すれば、いつでも完全な状態を復元できる

### 部分更新がイベントソーシングに合わない理由

**質問**: 「部分更新というのはイベントソーシングの考え方にあっていますか？」

**回答**: 合っていない。理由：

1. **Read Modelは集約の完全な状態のスナップショット**
   - イベントから`fold/reduce`で集約を復元
   - その完全な状態をRead Modelに保存
   - 部分更新は「変更差分を適用する」RDBの発想

2. **イベント再生時の動作と一貫性を保つべき**
   - イベント再生: 集約を最初から再構築（完全な状態）
   - Read Model更新: 同じ原則に従うべき（完全な状態）

3. **CQRSの本質**
   - Write側（EventStore）: イベント追記のみ
   - Read側（Read Model）: 最新状態の上書き（upsert）
   - "部分更新"という概念は、Write/Readの分離が不完全な証拠

## insertとupdateとsaveの違い

### 用語の混乱

**質問**: 「upsertというのもおかしいですよね。再構築可能ならinsertだけになるはずではないですか？」

**回答**: 用語の整理が必要。

### SQL層での意味
- **INSERT**: 新規レコード追加（存在すればエラー）
- **UPDATE**: 既存レコード更新（存在しなければエラー）
- **UPSERT**: INSERT ON CONFLICT DO UPDATE（存在すれば更新、なければ挿入）

### 概念層での意味
- **insert**: 「新しいもの」を追加する意図
- **update**: 「既存のもの」を変更する意図
- **save**: 「現在の状態」を保存する意図（新規か既存かは問わない）

### なぜ`save`を選んだか

1. **アプリケーション層の意図を正確に表現**
   ```rust
   // アプリケーション層の意図
   let loan_view = build_loan_view(&loan);  // 集約から完全な状態を構築
   loan_read_model.save(loan_view);         // その状態を保存
   ```

   - 「新規か既存か」はアプリケーション層の関心事ではない
   - 「現在の状態を保存したい」が関心事

2. **実装の柔軟性**
   - SQL: INSERT ON CONFLICT DO UPDATEで実装
   - HashMap: `insert()`で実装（既存の値を上書き）
   - 実装詳細をポート定義から隠蔽

3. **意味的な中立性**
   - `insert`: 「新規」を暗示 → 誤解を招く
   - `update`: 「変更」を暗示 → 誤解を招く
   - `save`: 「保存」を表現 → 中立的で正確

## 実装の詳細

### build_loan_view()ヘルパー関数

```rust
pub(super) fn build_loan_view(loan: &domain::loan::Loan) -> LoanView {
    match loan {
        domain::loan::Loan::Active(active) => LoanView {
            loan_id: active.loan_id,
            book_id: active.book_id,
            member_id: active.member_id,
            loaned_at: active.loaned_at,
            due_date: active.due_date,
            returned_at: None,
            extension_count: active.extension_count.value(),  // ← 必須
            status: LoanStatus::Active,
            created_at: active.created_at,
            updated_at: active.updated_at,                    // ← 必須
        },
        // Overdue, Returnedも同様
    }
}
```

**重要**: Rustの構造体リテラルは全フィールドの指定が必須
- フィールドを忘れるとコンパイルエラー
- 型システムが更新漏れを防ぐ

### 使用箇所

```rust
// loan_book
let loan_view = build_loan_view(&domain::loan::Loan::Active(active_loan));
deps.loan_read_model.save(loan_view).await?;

// extend_loan
let (updated_loan, event) = domain::loan::extend_loan(active_loan, cmd.extended_at)?;
let loan_view = build_loan_view(&domain::loan::Loan::Active(updated_loan));
deps.loan_read_model.save(loan_view).await?;  // ← extension_countとupdated_atも保存される

// return_book
let (returned_loan, event) = domain::loan::return_book(loan, cmd.returned_at)?;
let loan_view = build_loan_view(&domain::loan::Loan::Returned(returned_loan));
deps.loan_read_model.save(loan_view).await?;

// detect_overdue_loans
let updated_loan = domain::loan::apply_event(
    Some(domain::loan::Loan::Active(active)),
    &DomainEvent::LoanBecameOverdue(event),
);
let loan_view = build_loan_view(&updated_loan);
deps.loan_read_model.save(loan_view).await?;
```

すべて同じパターン：
1. イベントを適用して集約を更新
2. 集約から完全な状態（LoanView）を構築
3. Read Modelに保存

## ポート設計の原則（再確認）

### 質問: 「ポートは使う側が定義するのか、それとも実装側の要求を反映すべきか？」

### 回答: 両方を考慮する

1. **アプリケーション層の意図を表現**（主）
   - ポートはアプリケーション層のインターフェース
   - 「何をしたいか」を表現

2. **実装可能性を考慮**（従）
   - 実装できない要求は意味がない
   - Read Modelの場合、完全な状態があれば実装可能

3. **今回のケース**
   - アプリケーション層: 「集約の現在状態を保存したい」
   - 実装層: 「完全な状態があればINSERT/UPDATEできる」
   - 結論: `save(LoanView)`が双方の要求を満たす

## まとめ

### 判断基準

**Read Modelの更新メソッドを設計する際の判断基準**:

1. **イベントソーシングの原則に従う**
   - Read Modelは集約の完全な状態の投影
   - 部分更新ではなく、完全な状態の保存

2. **アプリケーション層の意図を表現**
   - 「現在の状態を保存する」→ `save()`
   - 新規/既存の区別は実装詳細

3. **型システムで安全性を確保**
   - 構造体リテラルで全フィールド強制
   - 更新漏れをコンパイル時に検出

4. **シンプルさと一貫性**
   - 単一のメソッド（save）
   - すべての操作で同じパターン

### saveを使うべきケース

- **Event Sourcing + CQRS**のRead Model更新
- 集約の**完全な状態**をキャッシュする場合
- 新規/既存の区別が**アプリケーションの関心事でない**場合

### updateを使うべきケース（参考）

- 従来のCRUDアプリケーション
- 特定フィールドのみの**差分更新が意図**である場合
- データベースが唯一の真実の源泉である場合

### 今回の学び

1. **用語の正確な理解が重要**: insert/update/saveの意味の違い
2. **アーキテクチャパターンの原則に従う**: Event Sourcingでは完全な状態の保存
3. **ポート設計は双方の視点が必要**: 使う側と実装側の両方を考慮
4. **型システムを活用**: Rustの構造体リテラルで安全性を確保

## 参照

- Task 5 PR: CodeRabbitレビューコメントへの対応
- `src/ports/loan_read_model.rs`: ポート定義
- `src/application/loan/loan_service.rs`: build_loan_viewヘルパーと使用例
- `docs/learning/port-design-philosophy.md`: ポート設計の一般原則
