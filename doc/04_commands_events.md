# イミュータブルなコマンドとイベント

## コマンド（Command）

### 定義

コマンドは、システムに対する**意図**または**指示**を表現するデータ構造。

**特徴：**
- 命令形で命名（〜する）
- 失敗する可能性がある
- 副作用を引き起こす意図を表す
- イミュータブル

### コマンドの役割

**ユーザーの意図を捉える：**
コマンドは「何をしたいか」を明確に表現します。

```
ユーザー：「この本を借りたい」
    ↓
コマンド：LoanBook（書籍を貸し出す）
    ↓
システム：検証して処理
```

**ビジネスロールを検証する場所：**
コマンドを受け取った時点で、ビジネスルールを検証します。

**副作用の引き金：**
コマンドの実行により、状態が変化し、イベントが発行されます。

### 関数型DDDでのコマンド設計

**原則1：イミュータブル**

コマンドは作成後変更できません。

```rust
// ✅ すべてのフィールドがイミュータブル
pub struct LoanBook {
    pub book_id: BookId,
    pub member_id: MemberId,
    pub loaned_at: DateTime<Utc>,
    pub staff_id: StaffId,
}
```

**原則2：検証可能な構造**

コマンドには必要な情報がすべて含まれます。

```rust
pub struct ExtendLoan {
    pub loan_id: LoanId,
    pub extended_at: DateTime<Utc>,
    pub requested_by: UserId,  // 誰が要求したか
}
```

**原則3：ビジネスの言葉で表現**

技術用語ではなく、ドメインの言葉を使用します。

```rust
// ✅ ビジネスの言葉
pub struct LoanBook { /* ... */ }
pub struct ExtendLoan { /* ... */ }
pub struct ReturnBook { /* ... */ }

// ❌ 技術用語
pub struct CreateLoanRecord { /* ... */ }
pub struct UpdateLoanData { /* ... */ }
pub struct DeleteLoanEntry { /* ... */ }
```

### コマンドの命名規則

**動詞 + 名詞（命令形）：**
- LoanBook（書籍を貸し出す）
- ExtendLoan（貸出を延長する）
- ReturnBook（書籍を返却する）
- ReserveBook（書籍を予約する）
- CancelReservation（予約をキャンセルする）

**命名のポイント：**
- ユーザーの意図が明確
- ドメインエキスパートが理解できる
- ユビキタス言語と一致

## イベント（Domain Event）

### 定義

イベントは、ドメイン内で**起きた重要な出来事**を表現するデータ構造。

**特徴：**
- 過去形で命名（〜された）
- 必ず発生した事実
- イミュータブル
- 時刻情報を含む
- 完全な情報を持つ

### イベントの役割

**ビジネスの出来事を記録：**
イベントは「何が起きたか」を記録します。

```
コマンド実行
    ↓
ビジネスルール検証
    ↓
イベント発行：BookLoaned（書籍が貸出された）
    ↓
他のシステムに通知可能
```

**イベントソーシングの基礎：**
イベントの履歴から現在の状態を復元できます。

**コンテキスト間の統合：**
イベントを使って他のコンテキストに通知します。

### 関数型DDDでのイベント設計

**原則1：完全にイミュータブル**

イベントは過去の事実なので、絶対に変更できません。

```rust
// ✅ すべてイミュータブル
pub struct BookLoaned {
    pub loan_id: LoanId,
    pub book_id: BookId,
    pub member_id: MemberId,
    pub loaned_at: DateTime<Utc>,
    pub due_date: DateTime<Utc>,
    pub loaned_by: StaffId,
}
```

**原則2：完全な情報を含む**

イベント単体で意味が分かるようにします。

```rust
// ✅ 完全な情報
pub struct LoanExtended {
    pub loan_id: LoanId,
    pub old_due_date: DateTime<Utc>,  // 変更前
    pub new_due_date: DateTime<Utc>,  // 変更後
    pub extended_at: DateTime<Utc>,
    pub extended_by: UserId,
}

// ❌ 不完全な情報
pub struct LoanExtended {
    pub loan_id: LoanId,
    pub new_due_date: DateTime<Utc>,  // 前の状態が分からない
}
```

**原則3：ビジネスの言葉で表現**

```rust
// ✅ ビジネスの言葉
pub struct BookLoaned { /* ... */ }
pub struct LoanExtended { /* ... */ }
pub struct BookReturned { /* ... */ }

// ❌ 技術用語
pub struct LoanRecordCreated { /* ... */ }
pub struct LoanDataUpdated { /* ... */ }
pub struct LoanStatusChanged { /* ... */ }
```

### イベントの命名規則

**名詞 + 過去形動詞：**
- BookLoaned（書籍が貸出された）
- LoanExtended（貸出が延長された）
- BookReturned（書籍が返却された）
- BookReserved（書籍が予約された）
- ReservationConfirmed（予約が確定された）

**命名のポイント：**
- 過去形（既に起きたこと）
- 受動態が自然なことが多い
- ビジネスの出来事を表現

## コマンドとイベントの関係

### 基本的な流れ

```
コマンド（意図）
    ↓
検証・ビジネスルール適用
    ↓
イベント（結果）
```

**1対1の関係：**
1つのコマンドから1つのイベントが発行されることが多い。

```rust
LoanBook コマンド
    ↓
BookLoaned イベント
```

**1対多の関係：**
1つのコマンドから複数のイベントが発行されることもある。

```rust
ReturnBook コマンド
    ↓
BookReturned イベント
    + LoanBecameOverdue イベント（延滞していた場合）
```

### 関数型DDDでの表現

```rust
// 純粋関数：コマンド → (状態, イベント)
pub fn loan_book(
    book_id: BookId,
    member_id: MemberId,
    loaned_at: DateTime<Utc>,
    staff_id: StaffId,
) -> Result<(Loan, BookLoaned), LoanError> {
    // バリデーション
    // ビジネスルール適用
    
    let loan = Loan { /* ... */ };
    let event = BookLoaned { /* ... */ };
    
    Ok((loan, event))
}
```

**重要：**
- コマンドは入力
- 状態とイベントは出力
- 純粋関数（副作用なし）

## コマンドとイベントの設計判断

### なぜイミュータブルなのか

**コマンド：**
- 意図は変わらない
- 並行処理で安全
- 監査証跡として残せる

**イベント：**
- 過去は変更できない
- イベントソーシングの基礎
- 関数型プログラミングの原則

### なぜ完全な情報を含むのか

**イベントの自己完結性：**
他のイベントやデータソースを参照せずに、イベント単体で意味が分かる。

```rust
// ✅ 完全な情報
pub struct BookReturned {
    pub loan_id: LoanId,
    pub book_id: BookId,
    pub member_id: MemberId,
    pub returned_at: DateTime<Utc>,
    pub was_overdue: bool,        // 延滞していたか
    pub overdue_days: Option<u32>, // 延滞日数
}

// ❌ 不完全（他の情報が必要）
pub struct BookReturned {
    pub loan_id: LoanId,
    pub returned_at: DateTime<Utc>,
    // book_idもmember_idもない → 誰が何を返却したか不明
}
```

### なぜ過去形で命名するのか

**事実の記録：**
イベントは「既に起きた出来事」を表現します。

```
BookLoaned = 書籍が（既に）貸出された
LoanExtended = 貸出が（既に）延長された
```

**コマンドとの対比：**
```
LoanBook（命令）→ BookLoaned（結果）
ExtendLoan（命令）→ LoanExtended（結果）
ReturnBook（命令）→ BookReturned（結果）
```

## イベントストーミングとの対応

### Big Pictureからの成果物

イベントストーミングで洗い出したイベントを、そのまま実装に使用します。

**黄色の付箋（ドメインイベント）：**
```
付箋：「書籍が貸出された」
  ↓
コード：BookLoaned イベント
```

**青色の付箋（コマンド）：**
```
付箋：「書籍を貸し出す」
  ↓
コード：LoanBook コマンド
```

**対応の利点：**
- ビジネスとコードの一致
- ドメインエキスパートとの対話が容易
- ユビキタス言語の一貫性

## 例：Phase 1 の貸出管理

以下はPhase 1で実装する貸出管理の例です。これらは原則を適用した具体例として示します。

### コマンド例

**書籍を貸し出す：**
```rust
pub struct LoanBook {
    pub book_id: BookId,
    pub member_id: MemberId,
    pub loaned_at: DateTime<Utc>,
    pub staff_id: StaffId,
}
```

**貸出を延長する：**
```rust
pub struct ExtendLoan {
    pub loan_id: LoanId,
    pub extended_at: DateTime<Utc>,
    pub requested_by: UserId,
}
```

**書籍を返却する：**
```rust
pub struct ReturnBook {
    pub loan_id: LoanId,
    pub returned_at: DateTime<Utc>,
    pub staff_id: StaffId,
}
```

### イベント例

**書籍が貸出された：**
```rust
pub struct BookLoaned {
    pub loan_id: LoanId,
    pub book_id: BookId,
    pub member_id: MemberId,
    pub loaned_at: DateTime<Utc>,
    pub due_date: DateTime<Utc>,
    pub loaned_by: StaffId,
}
```

**貸出が延長された：**
```rust
pub struct LoanExtended {
    pub loan_id: LoanId,
    pub old_due_date: DateTime<Utc>,
    pub new_due_date: DateTime<Utc>,
    pub extended_at: DateTime<Utc>,
    pub extension_count: u8,
}
```

**書籍が返却された：**
```rust
pub struct BookReturned {
    pub loan_id: LoanId,
    pub book_id: BookId,
    pub member_id: MemberId,
    pub returned_at: DateTime<Utc>,
    pub was_overdue: bool,
}
```

**貸出が延滞した：**
```rust
pub struct LoanBecameOverdue {
    pub loan_id: LoanId,
    pub book_id: BookId,
    pub member_id: MemberId,
    pub due_date: DateTime<Utc>,
    pub detected_at: DateTime<Utc>,
}
```

### コマンド → イベントの流れ

```rust
// 純粋関数による処理
pub fn loan_book(
    book_id: BookId,
    member_id: MemberId,
    loaned_at: DateTime<Utc>,
    staff_id: StaffId,
) -> Result<(Loan, BookLoaned), LoanError> {
    // 計算
    let due_date = loaned_at + Duration::days(14);
    
    let loan = Loan {
        loan_id: LoanId::new(),
        book_id,
        member_id,
        loaned_at,
        due_date,
        // ...
    };
    
    let event = BookLoaned {
        loan_id: loan.loan_id,
        book_id,
        member_id,
        loaned_at,
        due_date,
        loaned_by: staff_id,
    };
    
    Ok((loan, event))
}
```

この例は、原則を適用した実装を示すものです。

## エラーハンドリング

### コマンドは失敗する可能性がある

コマンドは意図を表すため、ビジネスルールにより拒否されることがあります。

```rust
pub enum LoanError {
    MemberNotFound,
    BookNotAvailable,
    MemberHasOverdueLoan,
    LoanLimitExceeded,
}

pub fn loan_book(/*...*/) -> Result<(Loan, BookLoaned), LoanError> {
    // バリデーション
    // 失敗する可能性がある
}
```

### イベントは必ず成功している

イベントは既に起きた事実なので、エラーはありません。

```rust
pub struct BookLoaned { /* ... */ }

// Resultではなく、直接イベント
// なぜなら既に起きた事実だから
```

### Result型でのフロー制御

関数型DDDでは、Result型でエラーを表現します。

```rust
pub fn loan_book(/*...*/) -> Result<(Loan, BookLoaned), LoanError> {
    validate_member()?;
    validate_book()?;
    check_overdue_loans()?;
    check_loan_limit()?;
    
    // すべて成功した場合のみここに到達
    Ok((loan, event))
}
```

これはRailway Oriented Programmingのパターンです。

## コマンド・イベントの型階層

### 統合イベント型

複数のイベントをまとめる型を定義できます。

```rust
pub enum DomainEvent {
    BookLoaned(BookLoaned),
    LoanExtended(LoanExtended),
    BookReturned(BookReturned),
    LoanBecameOverdue(LoanBecameOverdue),
}
```

**利点：**
- イベントストアで統一的に扱える
- パターンマッチで処理を分岐できる
- イベントハンドラーの実装が容易

### パターンマッチによる処理

```rust
fn apply_event(loan: Loan, event: &DomainEvent) -> Loan {
    match event {
        DomainEvent::BookLoaned(e) => {
            // BookLoanedイベントの処理
        }
        DomainEvent::LoanExtended(e) => {
            // LoanExtendedイベントの処理
        }
        // ...
    }
}
```

## まとめ

### コマンド設計の原則

1. **命令形で命名**
   - LoanBook, ExtendLoan, ReturnBook

2. **イミュータブル**
   - 作成後変更しない

3. **ビジネスの言葉**
   - ユビキタス言語を使用

4. **失敗する可能性**
   - Result型で表現

### イベント設計の原則

1. **過去形で命名**
   - BookLoaned, LoanExtended, BookReturned

2. **完全にイミュータブル**
   - 過去は変更できない

3. **完全な情報**
   - 単体で意味が分かる

4. **既に発生した事実**
   - 必ず成功している

### 関数型DDDでの扱い

- コマンドは入力
- イベントは出力
- 純粋関数で処理
- Result型でエラーハンドリング

これらの原則は、すべてのコンテキスト、すべてのPhaseで一貫して適用されます。
