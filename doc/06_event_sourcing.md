# fold/reduceによる状態復元

## イベントソーシングとは

### 定義

イベントソーシングは、アプリケーションの状態を**イベントの履歴**として保存するパターン。

**従来のCRUD：**
```
現在の状態のみを保存
┌─────────────┐
│ Loan Table  │
├─────────────┤
│ loan_id     │
│ status      │ ← 現在の値のみ
│ due_date    │
└─────────────┘
```

**イベントソーシング：**
```
イベントの履歴を保存
┌──────────────────────┐
│ Events               │
├──────────────────────┤
│ BookLoaned           │ ← イベント1
│ LoanExtended         │ ← イベント2
│ BookReturned         │ ← イベント3
└──────────────────────┘
    ↓ リプレイ
現在の状態を復元
```

### 基本的な考え方

**状態 = イベントの集積**

現在の状態は、過去のすべてのイベントを順番に適用した結果です。

```
空の状態
  ↓ BookLoaned イベントを適用
貸出中の状態
  ↓ LoanExtended イベントを適用
延長済みの状態
  ↓ BookReturned イベントを適用
返却済みの状態
```

### 関数型プログラミングとの関係

イベントソーシングは関数型プログラミングのfold/reduce操作そのものです。

```rust
fn replay_events(events: Vec<DomainEvent>) -> Loan {
    events.into_iter().fold(
        Loan::empty(),              // 初期状態
        |state, event| {            // 累積関数
            apply_event(state, event)
        }
    )
}
```

## なぜイベントソーシングを使うのか

### 1. 完全な監査証跡

すべての変更が記録されます。

```
いつ、誰が、何をしたか、なぜそうなったか
↓
すべてイベントとして記録
```

**例：貸出の履歴**
```
2025-01-10: BookLoaned（田中職員が貸出）
2025-01-20: LoanExtended（利用者本人が延長）
2025-02-05: BookReturned（田中職員が返却受付）
```

### 2. 時系列の分析

過去の任意の時点の状態を再現できます。

```
「1月15日時点で延滞していた貸出は？」
↓
1月15日までのイベントをリプレイ
```

### 3. ドメインイベント中心の設計

イベントストーミングで洗い出したドメインイベントをそのまま実装に使えます。

```
Big Picture の黄色付箋
↓
そのままイベントとして実装
```

### 4. ビジネスの出来事が明確

現在の状態だけでなく、「どうしてそうなったか」が分かります。

```
CRUD: status = "returned"（返却済み）
↓ なぜ？いつ？

イベントソーシング:
BookLoaned → LoanExtended → BookReturned
↓ すべて記録されている
```

### 5. 関数型プログラミングとの相性

イベント = イミュータブルなデータ
状態復元 = fold/reduce

関数型プログラミングの原則と完全に一致します。

## イベントストアの設計原則

### 原則1：イベントは追記のみ

イベントは過去の事実なので、変更・削除できません。

```rust
// ✅ 追記のみ
event_store.append(event).await?;

// ❌ 更新・削除は禁止
// event_store.update(event)?;
// event_store.delete(event_id)?;
```

### 原則2：イベントは順序を持つ

イベントは発生順に保存されます。

```
sequence_number: 1 → BookLoaned
sequence_number: 2 → LoanExtended
sequence_number: 3 → BookReturned
```

### 原則3：集約ごとに分離

各集約のイベントは独立して管理されます。

```
Loan A: [BookLoaned, LoanExtended, BookReturned]
Loan B: [BookLoaned, BookReturned]
Loan C: [BookLoaned, LoanExtended]
```

### イベントストアの基本操作

**append（追記）：**
```rust
pub trait EventStore {
    async fn append(
        &self,
        aggregate_id: AggregateId,
        events: Vec<DomainEvent>,
    ) -> Result<()>;
}
```

**load（読み込み）：**
```rust
pub trait EventStore {
    async fn load(
        &self,
        aggregate_id: AggregateId,
    ) -> Result<Vec<DomainEvent>>;
}
```

**stream（ストリーム）：**
```rust
pub trait EventStore {
    fn stream_all(&self) -> BoxStream<'static, Result<DomainEvent>>;
}
```

## 状態復元のパターン

### fold/reduce による復元

イベントの履歴から現在の状態を復元します。

```rust
/// イベント列から状態を復元
pub fn replay_events(events: Vec<DomainEvent>) -> Loan {
    events.into_iter().fold(
        Loan::empty(),
        apply_event
    )
}

/// 1つのイベントを状態に適用
fn apply_event(loan: Loan, event: DomainEvent) -> Loan {
    match event {
        DomainEvent::BookLoaned(e) => {
            Loan {
                loan_id: e.loan_id,
                book_id: e.book_id,
                member_id: e.member_id,
                loaned_at: e.loaned_at,
                due_date: e.due_date,
                status: LoanStatus::Active,
                extension_count: ExtensionCount::new(),
                returned_at: None,
                // ...
            }
        }
        DomainEvent::LoanExtended(e) => {
            Loan {
                due_date: e.new_due_date,
                extension_count: loan.extension_count.increment().unwrap(),
                updated_at: e.extended_at,
                ..loan
            }
        }
        DomainEvent::BookReturned(e) => {
            Loan {
                returned_at: Some(e.returned_at),
                status: LoanStatus::Returned,
                updated_at: e.returned_at,
                ..loan
            }
        }
    }
}
```

### なぜfold/reduceなのか

**関数型プログラミングの基本パターン：**
```
初期値 + 操作の列 → 最終結果

空の状態 + イベントの列 → 現在の状態
```

**特徴：**
- 純粋関数
- イミュータブル
- 副作用なし
- テストが容易

### 復元の流れ

```
1. イベントストアからイベント列を取得
   ↓
[BookLoaned, LoanExtended, BookReturned]

2. fold で順番に適用
   ↓
empty → apply(BookLoaned) → apply(LoanExtended) → apply(BookReturned)

3. 最終状態
   ↓
Loan（返却済み）
```

## CQRSパターン

### コマンドとクエリの分離

**Command（書き込み）：**
```
コマンド実行
  ↓
イベント生成
  ↓
イベントストアに追記
```

**Query（読み込み）：**
```
Read Modelから読み込み
  ↓
最適化されたビュー
```

### なぜ分離するのか

**1. 読み書きの最適化**

書き込みと読み込みで異なる最適化ができます。

**2. スケーラビリティ**

読み込みと書き込みを独立してスケールできます。

**3. 複雑なクエリに対応**

Read Modelで結合・集計・非正規化が可能です。

### Read Model の設計

**コマンド側（書き込み）：**
```rust
// イベントストアに保存
pub struct Loan {
    loan_id: LoanId,
    book_id: BookId,      // IDのみ
    member_id: MemberId,  // IDのみ
    // ...
}
```

**クエリ側（読み込み）：**
```rust
// Read Model（非正規化）
pub struct LoanView {
    pub loan_id: LoanId,
    pub book_id: BookId,
    pub book_title: String,     // 非正規化
    pub member_id: MemberId,
    pub member_name: String,    // 非正規化
    pub loaned_at: DateTime<Utc>,
    pub due_date: DateTime<Utc>,
    pub status: String,
}
```

**重要：**
- コマンド側は境界を守る（IDのみ）
- クエリ側は表示最適化のため結合可能
- それぞれ独立したモデル

## プロジェクション

### 定義

プロジェクションは、イベントストリームからRead Modelを更新する処理。

```
イベントストリーム
  ↓
プロジェクター
  ↓
Read Model更新
```

### プロジェクターのパターン

```rust
/// イベントを受け取りRead Modelを更新
pub async fn project_loan_event(
    event: &DomainEvent,
    read_model: &dyn LoanReadModel,
) -> Result<()> {
    match event {
        DomainEvent::BookLoaned(e) => {
            let view = LoanView {
                loan_id: e.loan_id,
                book_id: e.book_id,
                member_id: e.member_id,
                loaned_at: e.loaned_at,
                due_date: e.due_date,
                status: "active".to_string(),
                // ...
            };
            read_model.insert(view).await?;
        }
        DomainEvent::LoanExtended(e) => {
            read_model.update_due_date(
                e.loan_id,
                e.new_due_date,
            ).await?;
        }
        DomainEvent::BookReturned(e) => {
            read_model.update_status(
                e.loan_id,
                "returned",
                e.returned_at,
            ).await?;
        }
    }
    Ok(())
}
```

### プロジェクションの実行タイミング

**同期的：**
```
イベント保存後、即座にRead Model更新
```

**非同期的：**
```
イベントストリームを購読し、バックグラウンドで更新
```

### 複数のRead Model

目的に応じて複数のRead Modelを作成できます。

```
イベントストリーム
  ↓
  ├→ Projector A → Loans View（一覧表示用）
  ├→ Projector B → Overdue View（延滞一覧用）
  └→ Projector C → Statistics View（統計用）
```

## イベントハンドラー

### ポリシーの実装

イベントに反応して自動的に処理を実行します。

**例：延滞検出ポリシー**

```rust
/// ポリシー：毎日延滞をチェック
pub async fn detect_overdue_loans(
    loan_read_model: &dyn LoanReadModel,
    event_store: &dyn EventStore,
) -> Result<()> {
    let now = Utc::now();
    
    // Read Modelから延滞候補を取得
    let overdue_loans = loan_read_model
        .find_overdue_candidates(now)
        .await?;
    
    for loan_view in overdue_loans {
        // イベントストアから完全な履歴を取得
        let events = event_store.load(loan_view.loan_id.into()).await?;
        let loan = replay_events(events);
        
        // 延滞判定
        if is_overdue(&loan, now) && !already_marked_overdue(&loan) {
            let event = LoanBecameOverdue {
                loan_id: loan.loan_id,
                book_id: loan.book_id,
                member_id: loan.member_id,
                due_date: loan.due_date,
                detected_at: now,
            };
            
            event_store.append(
                loan.loan_id.into(),
                vec![DomainEvent::LoanBecameOverdue(event)]
            ).await?;
        }
    }
    
    Ok(())
}
```

### イベント駆動の統合

イベントを使ってコンテキスト間で協調します。

**例：返却イベントでの予約確定**

```
貸出管理コンテキスト
  ↓ BookReturned イベント発行
予約管理コンテキスト
  ↓ イベントハンドラーが受信
ReservationConfirmed イベント発行
```

## イベントのバージョニング

### なぜバージョニングが必要か

イベントの構造が変わることがあります。

```
V1: BookLoaned { loan_id, book_id, member_id }
  ↓ 監査要件で追加
V2: BookLoaned { loan_id, book_id, member_id, staff_id }
```

### バージョニングの戦略

**戦略1：アップキャスト**

古いイベントを読み込み時に新しい形式に変換。

```rust
fn upcast_event(event: EventV1) -> EventV2 {
    EventV2 {
        loan_id: event.loan_id,
        book_id: event.book_id,
        member_id: event.member_id,
        staff_id: StaffId::unknown(), // デフォルト値
    }
}
```

**戦略2：複数バージョンの共存**

```rust
pub enum BookLoanedEvent {
    V1(BookLoanedV1),
    V2(BookLoanedV2),
}

fn apply_event(loan: Loan, event: BookLoanedEvent) -> Loan {
    match event {
        BookLoanedEvent::V1(e) => apply_v1(loan, e),
        BookLoanedEvent::V2(e) => apply_v2(loan, e),
    }
}
```

**戦略3：イベント変換**

過去のイベントを新しい形式に一括変換。

### バージョニングの原則

**1. 後方互換性を保つ**

古いイベントも読み込めるようにします。

**2. デフォルト値を定義**

新しいフィールドにはデフォルト値を設定。

**3. 段階的移行**

一度にすべてを変更しない。

## イベントソーシングの課題と対策

### 課題1：イベント数の増加

集約のイベントが増えると、復元に時間がかかります。

**対策：スナップショット**

定期的に現在の状態を保存します。

```
Snapshot（100イベント目の状態）
  ↓
101〜150のイベントだけをリプレイ
```

```rust
pub async fn load_aggregate(
    aggregate_id: AggregateId,
    event_store: &dyn EventStore,
    snapshot_store: &dyn SnapshotStore,
) -> Result<Loan> {
    // スナップショットを取得
    if let Some(snapshot) = snapshot_store.load(aggregate_id).await? {
        // スナップショット以降のイベントのみをリプレイ
        let events = event_store
            .load_after(aggregate_id, snapshot.version)
            .await?;
        Ok(replay_events_from(snapshot.state, events))
    } else {
        // すべてのイベントをリプレイ
        let events = event_store.load(aggregate_id).await?;
        Ok(replay_events(events))
    }
}
```

### 課題2：イベントの修正

過去のイベントは変更できません。

**対策：補正イベント**

誤りを修正する新しいイベントを発行します。

```
誤ったイベント: BookLoaned（間違った本）
  ↓ 削除できない
補正イベント: LoanCorrected（正しい本に訂正）
```

### 課題3：複雑なクエリ

イベントストアから直接クエリするのは困難。

**対策：Read Model（CQRS）**

クエリ用に最適化されたビューを作成します。

```
イベントストア（正規化された履歴）
  ↓ プロジェクション
Read Model（非正規化されたビュー）
```

## イベントソーシングの実装例

以下は原則を示すための実装例です。

### コマンド実行の流れ

```rust
// 1. コマンド受信
pub async fn execute_loan_book(
    cmd: LoanBookCommand,
    services: &Services,
) -> Result<LoanId> {
    // 2. 外部チェック（副作用）
    services.member_service.exists(cmd.member_id).await?;
    services.book_service.is_available(cmd.book_id).await?;
    
    // 3. 既存のイベントをロード
    let events = services.event_store
        .load(cmd.member_id.into())
        .await?;
    
    // 4. 状態を復元（純粋関数）
    let existing_loans = find_active_loans(events);
    
    // 5. ビジネスルール検証
    if existing_loans.len() >= 5 {
        return Err(LoanError::LoanLimitExceeded);
    }
    
    // 6. 純粋関数でイベント生成
    let (loan, event) = domain::loan_book(
        cmd.book_id,
        cmd.member_id,
        cmd.loaned_at,
        cmd.staff_id,
    )?;
    
    // 7. イベント保存（副作用）
    services.event_store.append(
        loan.loan_id.into(),
        vec![DomainEvent::BookLoaned(event)]
    ).await?;
    
    // 8. Read Model更新（副作用）
    project_loan_event(&event, services.read_model).await?;
    
    Ok(loan.loan_id)
}
```

### イベントからの状態復元

```rust
/// すべてのイベントから状態を復元
pub fn replay_events(events: Vec<DomainEvent>) -> Loan {
    events.into_iter()
        .fold(Loan::empty(), apply_event)
}

/// 1つのイベントを適用
fn apply_event(mut loan: Loan, event: DomainEvent) -> Loan {
    match event {
        DomainEvent::BookLoaned(e) => {
            Loan {
                loan_id: e.loan_id,
                book_id: e.book_id,
                member_id: e.member_id,
                loaned_at: e.loaned_at,
                due_date: e.due_date,
                status: LoanStatus::Active,
                extension_count: ExtensionCount::new(),
                returned_at: None,
                created_at: e.loaned_at,
                updated_at: e.loaned_at,
            }
        }
        DomainEvent::LoanExtended(e) => {
            Loan {
                due_date: e.new_due_date,
                extension_count: loan.extension_count
                    .increment()
                    .expect("Extension count overflow"),
                updated_at: e.extended_at,
                ..loan
            }
        }
        DomainEvent::BookReturned(e) => {
            Loan {
                returned_at: Some(e.returned_at),
                status: LoanStatus::Returned,
                updated_at: e.returned_at,
                ..loan
            }
        }
        DomainEvent::LoanBecameOverdue(e) => {
            Loan {
                status: LoanStatus::Overdue,
                updated_at: e.detected_at,
                ..loan
            }
        }
    }
}
```

## まとめ

### イベントソーシングの原則

**1. イベントは追記のみ**
- 過去は変更できない
- イミュータブル

**2. 状態はイベントの集積**
- fold/reduceで復元
- 純粋関数

**3. CQRSと組み合わせる**
- コマンド側：イベントストア
- クエリ側：Read Model

**4. イベント駆動の統合**
- コンテキスト間の協調
- ポリシーの実装

### 関数型プログラミングとの相性

**イベント = イミュータブルなデータ**
```rust
pub struct BookLoaned {
    pub loan_id: LoanId,
    // すべて不変
}
```

**状態復元 = fold/reduce**
```rust
events.fold(initial_state, apply_event)
```

**純粋関数で処理**
```rust
fn apply_event(state: Loan, event: DomainEvent) -> Loan {
    // 副作用なし
}
```

### イベントソーシングの利点（再確認）

1. **完全な監査証跡**
2. **時系列の分析**
3. **ドメインイベント中心の設計**
4. **ビジネスの出来事が明確**
5. **関数型プログラミングとの相性**

これらの原則は、すべてのコンテキスト、すべてのPhaseで一貫して適用されます。
