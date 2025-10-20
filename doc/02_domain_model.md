# 関数型で表現するドメインモデル

## このドキュメントについて

Phase 1-2で実装する具体的なドメインモデル（Loan、Reservation）を例に、
関数型DDDでのドメインモデリングの原則を説明します。

## ドメインモデルの基本概念

### 関数型DDDにおけるドメインモデル

**ドメインモデルとは：**
ビジネスの概念、ルール、振る舞いを表現したもの。

**関数型DDDでの表現方法：**
- データ構造（struct）= ビジネスの概念
- 純粋関数 = ビジネスの振る舞い
- 型 = ビジネスルールの強制

**重要な原則：**
1. すべてイミュータブル
2. ドメイン層に副作用を持ち込まない
3. 状態を変更せず、新しい状態を返す
4. 型で不正な状態を排除する

## 貸出管理コンテキストのドメインモデル

### ユビキタス言語

このコンテキストで使用する言葉：

| 用語 | 意味 | 英語 |
|------|------|------|
| 貸出 | 書籍を利用者に貸している状態 | Loan |
| 延長 | 返却期限を延ばす行為 | Extension |
| 延滞 | 返却期限を過ぎた状態 | Overdue |
| 返却期限 | 書籍を返すべき日 | Due Date |
| 貸出期間 | 貸出から返却までの期間 | Loan Period |

**コード内での使用：**
```rust
// ✅ ユビキタス言語を使う
fn extend_loan(loan: &Loan) -> Result<(Loan, LoanExtended)>

// ❌ 技術用語や曖昧な言葉
fn update_record(data: &Data) -> Result<Data>
```

### 集約：Loan（貸出）

**定義：**
1冊の書籍の1回の貸出。

**責務：**
- 貸出期間の管理
- 延長回数の管理
- 返却期限の計算
- 延滞判定

**データ構造：**
```rust
pub struct Loan {
    // 識別子
    pub loan_id: LoanId,
    
    // 他の集約への参照（IDのみ）
    pub book_id: BookId,
    pub member_id: MemberId,
    
    // 貸出管理の責務
    pub loaned_at: DateTime<Utc>,
    pub due_date: DateTime<Utc>,
    pub returned_at: Option<DateTime<Utc>>,
    pub extension_count: ExtensionCount,
    pub status: LoanStatus,
    
    // 監査情報
    pub created_by: StaffId,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

**状態遷移：**
```
[作成]
  ↓
Active（貸出中）
  ↓
[期限超過]
  ↓
Overdue（延滞中）
  ↓
[返却]
  ↓
Returned（返却済み）
```

**不変条件（Invariant）：**
- 延長回数は0または1のみ
- 返却済みのLoanは変更不可
- 返却期限は貸出日より未来
- 延滞中は延長不可

### 値オブジェクト

**LoanId（貸出ID）：**
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LoanId(Uuid);
```

貸出管理コンテキストの集約ID。

**BookId（書籍ID）：**
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct BookId(Uuid);
```

カタログ管理コンテキストへの参照。詳細は知らない。

**MemberId（会員ID）：**
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct MemberId(Uuid);
```

会員管理コンテキストへの参照。詳細は知らない。

**StaffId（職員ID）：**
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct StaffId(Uuid);
```

職員管理コンテキストへの参照。

**ExtensionCount（延長回数）：**
```rust
pub struct ExtensionCount(u8);
```

ビジネスルール「延長は1回まで」を型で強制。

**LoanStatus（貸出状態）：**
```rust
pub enum LoanStatus {
    Active,    // 貸出中
    Overdue,   // 延滞中
    Returned,  // 返却済み
}
```

### ビジネスルール

**貸出時の制約：**
- 延滞中の利用者は新規貸出不可
- 1人あたり5冊まで貸出可能
- 貸出期間：2週間

**延長のルール：**
- 延長可能回数：1回のみ
- 延長時：現在の返却期限 + 2週間
- 延滞中は延長不可

**返却のルール：**
- 延滞していても返却は受け付ける
- 延滞料金なし（公立図書館）

### 純粋関数による振る舞い

**貸出する：**
```rust
pub fn loan_book(
    book_id: BookId,
    member_id: MemberId,
    loaned_at: DateTime<Utc>,
    staff_id: StaffId,
) -> Result<(Loan, BookLoaned), LoanError>
```

**延長する：**
```rust
pub fn extend_loan(
    loan: &Loan,
    extended_at: DateTime<Utc>,
) -> Result<(Loan, LoanExtended), LoanError>
```

**返却する：**
```rust
pub fn return_book(
    loan: &Loan,
    returned_at: DateTime<Utc>,
) -> Result<(Loan, BookReturned), LoanError>
```

**延滞判定：**
```rust
pub fn is_overdue(loan: &Loan, now: DateTime<Utc>) -> bool
```

すべて純粋関数。副作用なし。

## 予約管理コンテキストのドメインモデル

### ユビキタス言語

| 用語 | 意味 | 英語 |
|------|------|------|
| 予約 | 貸出中の書籍を予約する | Reservation |
| 予約確定 | 書籍が返却され、予約者用に確保された状態 | Confirmed |
| 予約履行 | 予約者が書籍を受け取った | Fulfilled |
| 受取期限 | 予約確定後、受け取りに来るべき期限 | Pickup Deadline |
| 予約キュー | 同じ書籍への複数の予約 | Reservation Queue |

### 集約：Reservation（予約）

**定義：**
1人の利用者による1冊の書籍への予約。

**責務：**
- 予約状態の管理
- 受取期限の計算
- 予約の確定・履行判定

**データ構造：**
```rust
pub struct Reservation {
    // 識別子
    pub reservation_id: ReservationId,
    
    // 他の集約への参照
    pub book_id: BookId,
    pub member_id: MemberId,
    
    // 予約管理の責務
    pub reserved_at: DateTime<Utc>,
    pub confirmed_at: Option<DateTime<Utc>>,
    pub pickup_deadline: Option<DateTime<Utc>>,
    pub fulfilled_at: Option<DateTime<Utc>>,
    pub status: ReservationStatus,
    
    // 監査情報
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}
```

**状態遷移：**
```
[予約]
  ↓
Pending（予約中）
  ↓
[書籍返却]
  ↓
Confirmed（確定済み）
  ↓
[利用者来館]     [期限超過]
  ↓               ↓
Fulfilled       Expired
（履行済み）     （期限切れ）
```

**不変条件：**
- 確定前は受取期限なし
- 履行済みまたは期限切れは変更不可
- 受取期限は確定日より未来

### 値オブジェクト

**ReservationId（予約ID）：**
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct ReservationId(Uuid);
```

**ReservationStatus（予約状態）：**
```rust
pub enum ReservationStatus {
    Pending,    // 予約中（書籍待ち）
    Confirmed,  // 確定済み（受取待ち）
    Fulfilled,  // 履行済み
    Expired,    // 期限切れ
    Cancelled,  // キャンセル済み
}
```

### ビジネスルール

**予約時の制約：**
- 貸出中の書籍のみ予約可能
- 延滞中の利用者は予約不可
- 同じ書籍を複数予約不可

**確定のルール：**
- 書籍が返却されたら自動的に確定
- 受取期限：確定から7日間

**履行のルール：**
- 予約者が来館し、書籍を受け取る
- 同時に貸出が開始される

**期限切れのルール：**
- 受取期限を過ぎたら自動的に期限切れ
- 次の予約者がいれば確定される

### 純粋関数による振る舞い

**予約する：**
```rust
pub fn reserve_book(
    book_id: BookId,
    member_id: MemberId,
    reserved_at: DateTime<Utc>,
) -> Result<(Reservation, BookReserved), ReservationError>
```

**確定する：**
```rust
pub fn confirm_reservation(
    reservation: &Reservation,
    confirmed_at: DateTime<Utc>,
) -> Result<(Reservation, ReservationConfirmed), ReservationError>
```

**履行する：**
```rust
pub fn fulfill_reservation(
    reservation: &Reservation,
    fulfilled_at: DateTime<Utc>,
) -> Result<(Reservation, ReservationFulfilled), ReservationError>
```

**キャンセルする：**
```rust
pub fn cancel_reservation(
    reservation: &Reservation,
    cancelled_at: DateTime<Utc>,
) -> Result<(Reservation, ReservationCancelled), ReservationError>
```

## 型でビジネスルールを表現

### ExtensionCount - 延長回数

**ビジネスルール：**
延長は1回まで。

**型での表現：**
```rust
pub struct ExtensionCount(u8);

impl ExtensionCount {
    /// 新規作成（0回）
    pub fn new() -> Self {
        Self(0)
    }
    
    /// 延長回数を増やす
    pub fn increment(self) -> Result<Self, ExtensionError> {
        if self.0 >= 1 {
            return Err(ExtensionError::LimitExceeded);
        }
        Ok(Self(self.0 + 1))
    }
    
    /// 現在の回数
    pub fn value(&self) -> u8 {
        self.0
    }
    
    /// 延長可能か
    pub fn can_extend(&self) -> bool {
        self.0 < 1
    }
}
```

**利点：**
- 不正な値（2以上）を設定できない
- コンパイル時にエラーを検出
- ビジネスルールがコードに明示される

### LoanStatus - 貸出状態

**ビジネスルール：**
返却済みのLoanは変更不可。

**型での表現：**
```rust
pub enum LoanStatus {
    Active,
    Overdue,
    Returned,
}

impl LoanStatus {
    pub fn is_returned(&self) -> bool {
        matches!(self, LoanStatus::Returned)
    }
    
    pub fn can_extend(&self) -> bool {
        matches!(self, LoanStatus::Active)
    }
}
```

**関数での使用：**
```rust
pub fn extend_loan(loan: &Loan, /*...*/) -> Result<(Loan, LoanExtended)> {
    // 型で状態を判定
    if loan.status.is_returned() {
        return Err(LoanError::AlreadyReturned);
    }
    
    if !loan.status.can_extend() {
        return Err(LoanError::CannotExtend);
    }
    
    // ...
}
```

## 集約の境界とID参照

### 原則：他の集約はIDで参照する

**正しい設計：**
```rust
pub struct Loan {
    pub loan_id: LoanId,
    pub book_id: BookId,      // IDのみ
    pub member_id: MemberId,  // IDのみ
    // ...
}
```

**間違った設計：**
```rust
// ❌ 他の集約をオブジェクト参照
pub struct Loan {
    pub loan_id: LoanId,
    pub book: Book,           // オブジェクト参照
    pub member: Member,       // オブジェクト参照
}
```

**理由：**
1. コンテキスト境界を守る
2. トランザクション境界を明確にする
3. 他のコンテキストの変更に影響されない
4. テスト容易性

### どうしても他コンテキストの情報が必要な場合

**一時的にポート経由で取得する：**
```rust
// アプリケーション層で一時的に取得
pub async fn send_overdue_notification(
    loan: &Loan,
    book_service: &dyn BookService,
    notification_service: &dyn NotificationService,
) -> Result<()> {
    // 一時的に取得（保持しない）
    let book_title = book_service.get_book_title(loan.book_id).await?;
    
    let message = format!("書籍「{}」が延滞しています", book_title);
    notification_service.send(loan.member_id, &message).await?;
    
    Ok(())
}
```

**ポイント：**
- ローカル変数として使用
- ドメイン層には持ち込まない
- ポート経由でのみアクセス

## ドメインモデルの設計判断

### なぜMemberやBookを作らないのか

**このコンテキストで作るもの：**
- LoanId, BookId, MemberId（値オブジェクト）
- Loan集約

**このコンテキストで作らないもの：**
- Member struct（会員の詳細）
- Book struct（書籍の詳細）

**理由：**
1. **責務の分離**
   - 貸出管理は「いつ誰に何を貸したか」を管理
   - 会員の詳細情報（住所、電話番号）は管理しない

2. **コンテキスト境界**
   - MemberやBookは別のコンテキストの関心事
   - 境界を越えると結合度が上がる

3. **変更の影響範囲**
   - Memberの構造変更が貸出管理に影響しない
   - 疎結合を保つ

## 純粋関数によるドメインモデル

### 決定（Decision）と実行（Execution）の分離

**決定：純粋関数（ドメイン層）**
```rust
// domain/loan.rs

/// 純粋関数：書籍を貸し出す
/// 
/// 副作用なし。ビジネスロジックのみ。
pub fn loan_book(
    book_id: BookId,
    member_id: MemberId,
    loaned_at: DateTime<Utc>,
    staff_id: StaffId,
) -> Result<(Loan, BookLoaned), LoanError> {
    // ビジネスルールの検証
    // 計算
    // 新しい状態とイベントを生成
    
    let loan = Loan { /* ... */ };
    let event = BookLoaned { /* ... */ };
    
    Ok((loan, event))
}
```

**実行：副作用を扱う（アプリケーション層）**
```rust
// application/loan_service.rs

pub async fn execute_loan_book(
    cmd: LoanBookCommand,
    services: &Services,
) -> Result<LoanId> {
    // 副作用：外部への問い合わせ
    services.member_service.exists(cmd.member_id).await?;
    services.book_service.is_available(cmd.book_id).await?;
    
    // 純粋関数を呼ぶ（決定）
    let (loan, event) = domain::loan_book(
        cmd.book_id,
        cmd.member_id,
        cmd.loaned_at,
        cmd.staff_id,
    )?;
    
    // 副作用：永続化
    services.event_store.append(event).await?;
    
    Ok(loan.loan_id)
}
```

### イミュータビリティ

**すべての操作は新しいインスタンスを返す：**
```rust
pub fn extend_loan(
    loan: &Loan,
    extended_at: DateTime<Utc>,
) -> Result<(Loan, LoanExtended), LoanError> {
    // バリデーション
    if loan.status.is_returned() {
        return Err(LoanError::AlreadyReturned);
    }
    
    // 新しいインスタンスを生成
    let new_loan = Loan {
        due_date: calculate_new_due_date(loan.due_date),
        extension_count: loan.extension_count.increment()?,
        updated_at: extended_at,
        ..loan.clone()  // 他のフィールドはコピー
    };
    
    let event = LoanExtended { /* ... */ };
    
    Ok((new_loan, event))
}
```

**元の状態は変更されない：**
```rust
let loan = /* ... */;
let (new_loan, event) = extend_loan(&loan, Utc::now())?;

// loan は変更されていない
// new_loan が新しい状態
```
