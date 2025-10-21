# 関数型DDDの実装パターン

## ヘキサゴナルアーキテクチャ

### 定義

ヘキサゴナルアーキテクチャ（ポート&アダプター）は、ドメインロジックを中心に置き、外部依存を分離するアーキテクチャパターン。

**基本構造：**
```
┌─────────────────────────────────┐
│      Adapters（外側）            │
│  - REST API                     │
│  - Database                     │
│  - External Services            │
└────────────┬────────────────────┘
             │ ポート（インターフェース）
             ↓
┌─────────────────────────────────┐
│   Application Layer（中間）      │
│  - ユースケース                  │
│  - 副作用の実行                  │
└────────────┬────────────────────┘
             │ 純粋関数を呼ぶ
             ↓
┌─────────────────────────────────┐
│   Domain Layer（中心）           │
│  - 純粋関数                      │
│  - ビジネスロジック               │
└─────────────────────────────────┘
```

### なぜヘキサゴナルアーキテクチャなのか

**1. ドメインロジックの独立性**

ドメイン層は外部技術に依存しません。

```rust
// domain/loan.rs
// データベース、フレームワーク、外部APIに依存しない

pub fn loan_book(
    book_id: BookId,
    member_id: MemberId,
    loaned_at: DateTime<Utc>,
) -> Result<(Loan, BookLoaned), LoanError> {
    // 純粋なビジネスロジック
}
```

**2. テスタビリティ**

外部依存をモックに置き換えて、ドメインロジックを単体でテストできます。

**3. 技術の交換可能性**

アダプター層を変更するだけで、実装技術を変更できます。

```
PostgreSQL → MySQL
REST API → gRPC
HTTP Client → 同一プロセス呼び出し
```

### 各層の責務

**Domain Layer：**
- 純粋関数のみ
- ビジネスルールの実装
- 副作用なし

**Application Layer：**
- ユースケースの実行
- 副作用の制御（I/O）
- ドメイン層を呼び出す

**Adapters：**
- 外部システムとの接続
- ポートの実装
- 技術的な詳細

## 純粋関数によるドメインモデル

### 決定と実行の分離

関数型DDDの最重要パターン：**決定（Decision）**と**実行（Execution）**を分離する。

**決定（ドメイン層）：**
```rust
// domain/loan.rs

/// 純粋関数：何をすべきかを決定する
/// 
/// 副作用なし。計算のみ。
pub fn loan_book(
    book_id: BookId,
    member_id: MemberId,
    loaned_at: DateTime<Utc>,
    staff_id: StaffId,
) -> Result<(Loan, BookLoaned), LoanError> {
    // バリデーション
    // 計算
    // 新しい状態とイベントを生成
    
    let loan = Loan { /* ... */ };
    let event = BookLoaned { /* ... */ };
    
    Ok((loan, event))
}
```

**実行（アプリケーション層）：**
```rust
// application/loan_service.rs

/// 副作用を伴う実行
pub async fn execute_loan_book(
    cmd: LoanBookCommand,
    services: &Services,
) -> Result<LoanId> {
    // 副作用：外部への問い合わせ
    if !services.member_service.exists(cmd.member_id).await? {
        return Err(LoanError::MemberNotFound);
    }
    
    // 決定：純粋関数を呼ぶ
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

### なぜ分離するのか

**1. テストが容易**

純粋関数は入力と出力だけでテストできます。

```rust
#[test]
fn test_loan_book() {
    let result = loan_book(
        BookId::new(),
        MemberId::new(),
        Utc::now(),
        StaffId::new(),
    );
    
    assert!(result.is_ok());
    // データベース不要、外部APIコール不要
}
```

**2. 推論しやすい**

副作用がないため、関数の動作を理解しやすい。

**3. 並行処理で安全**

純粋関数は並行実行しても問題ありません。

**4. 再利用性が高い**

異なるコンテキストで再利用できます。

## イミュータビリティ

### すべてのデータはイミュータブル

関数型DDDでは、すべてのドメインオブジェクトを不変にします。

**値オブジェクト：**
```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoanId(Uuid);

#[derive(Debug, Clone, Copy)]
pub struct BookId(Uuid);
```

**集約：**
```rust
pub struct Loan {
    pub loan_id: LoanId,
    pub book_id: BookId,
    pub due_date: DateTime<Utc>,
    // すべてイミュータブル
}
```

**操作は新しいインスタンスを返す：**
```rust
pub fn extend_loan(
    loan: &Loan,
    extended_at: DateTime<Utc>,
) -> Result<(Loan, LoanExtended), LoanError> {
    // 新しいインスタンスを生成
    let new_loan = Loan {
        due_date: calculate_new_due_date(loan.due_date),
        extension_count: loan.extension_count + 1,
        updated_at: extended_at,
        ..loan.clone()
    };
    
    let event = LoanExtended { /* ... */ };
    
    Ok((new_loan, event))
}
```

### イミュータビリティの利点

**1. 並行処理で安全**

データ競合が発生しません。

**2. 予測可能**

値が変わらないため、動作を予測しやすい。

**3. 履歴管理が容易**

イベントソーシングと相性が良い。

**4. デバッグが容易**

状態の変化を追跡しやすい。

## 型駆動設計

### 型でビジネスルールを表現

不正な状態を型システムで排除します。

**例：延長回数の制約**

ビジネスルール：「延長は1回まで」

```rust
pub struct ExtensionCount(u8);

impl ExtensionCount {
    pub fn new() -> Self {
        Self(0)
    }
    
    pub fn increment(self) -> Result<Self, ExtensionError> {
        if self.0 >= 1 {
            return Err(ExtensionError::LimitExceeded);
        }
        Ok(Self(self.0 + 1))
    }
    
    pub fn can_extend(&self) -> bool {
        self.0 < 1
    }
}
```

**利点：**
- 不正な値（2以上）を作れない
- コンパイル時にエラーを検出
- ビジネスルールがコードに明示される

### 状態を型で表現

状態遷移を型システムで制御できます。

```rust
pub enum LoanStatus {
    Active,
    Overdue,
    Returned,
}

impl LoanStatus {
    pub fn can_extend(&self) -> bool {
        matches!(self, LoanStatus::Active)
    }
    
    pub fn is_returned(&self) -> bool {
        matches!(self, LoanStatus::Returned)
    }
}
```

**使用例：**
```rust
pub fn extend_loan(loan: &Loan) -> Result<(Loan, LoanExtended), LoanError> {
    if !loan.status.can_extend() {
        return Err(LoanError::CannotExtend);
    }
    // ...
}
```

### Option型とResult型

**Option型で存在しない可能性を表現：**
```rust
pub struct Loan {
    pub returned_at: Option<DateTime<Utc>>,  // 返却されていない場合はNone
}
```

**Result型でエラーを表現：**
```rust
pub fn loan_book(/*...*/) -> Result<(Loan, BookLoaned), LoanError> {
    // 成功またはエラー
}
```

## Railway Oriented Programming

### Result型による制御フロー

エラーハンドリングをResult型で表現します。

```rust
pub fn loan_book(/*...*/) -> Result<(Loan, BookLoaned), LoanError> {
    // 各検証は?演算子で連鎖
    validate_loan_period(loaned_at)?;
    validate_book_id(book_id)?;
    validate_member_id(member_id)?;
    
    // すべて成功した場合のみここに到達
    let loan = create_loan(/*...*/);
    let event = create_event(/*...*/);
    
    Ok((loan, event))
}
```

### エラーの合成

複数のエラーパターンを統一的に扱います。

```rust
#[derive(Debug, thiserror::Error)]
pub enum LoanError {
    #[error("Member not found")]
    MemberNotFound,
    
    #[error("Book not available")]
    BookNotAvailable,
    
    #[error("Member has overdue loan")]
    MemberHasOverdueLoan,
    
    #[error("Loan limit exceeded")]
    LoanLimitExceeded,
}
```

**利点：**
- 明示的なエラーハンドリング
- 型安全
- コンパイラが検証

## 副作用の境界

### 副作用の分離パターン

副作用（I/O）はドメイン層に持ち込みません。

**ドメイン層（副作用なし）：**
```rust
// domain/loan.rs

pub fn loan_book(/*...*/) -> Result<(Loan, BookLoaned), LoanError> {
    // 計算のみ
    // データベースアクセスなし
    // 外部APIコールなし
}

pub fn extend_loan(/*...*/) -> Result<(Loan, LoanExtended), LoanError> {
    // 計算のみ
}
```

**アプリケーション層（副作用あり）：**
```rust
// application/loan_service.rs

pub async fn execute_loan_book(/*...*/) -> Result<LoanId> {
    // 副作用：読み込み
    let exists = services.member_service.exists(member_id).await?;
    
    // 純粋関数：決定
    let (loan, event) = domain::loan_book(/*...*/)?;
    
    // 副作用：書き込み
    services.event_store.append(event).await?;
    
    Ok(loan.loan_id)
}
```

### なぜ分離するのか

**1. テスト容易性**

純粋関数は単体テストが簡単。

**2. 並行処理**

副作用がない部分は並行実行しやすい。

**3. 理解しやすさ**

計算ロジックとI/Oが明確に分離される。

## プロジェクト構造の原則

### レイヤー別の構成

```
src/
├── domain/           # ドメイン層（純粋関数）
│   ├── loan.rs
│   ├── commands.rs
│   ├── events.rs
│   └── errors.rs
│
├── ports/            # ポート（トレイト定義）
│   ├── event_store.rs
│   └── member_service.rs
│
├── adapters/         # アダプター（実装）
│   ├── in_memory/
│   ├── postgres/
│   └── http/
│
├── application/      # アプリケーション層
│   └── loan_service.rs
│
└── api/             # API層
    └── handlers.rs
```

### 依存の方向

```
API層
  ↓ 依存
Application層
  ↓ 依存
Domain層（依存なし）
```

**重要な原則：**
- ドメイン層は何にも依存しない
- 依存は内側（ドメイン）に向かう
- 外側が内側を知る、内側は外側を知らない

### モジュール命名の原則

**実装クラスはシンプルに：**
```rust
// ports/member_service.rs
pub trait MemberService { /* ... */ }

// adapters/in_memory/member_service.rs
pub struct MemberService;  // 同じ名前

// adapters/http/member_service.rs
pub struct MemberService;  // 同じ名前
```

**モジュールで区別：**
```rust
use adapters::in_memory::MemberService;
use adapters::http::MemberService;
```

## イベントからの状態復元

### fold/reduceパターン

イベントの履歴から現在の状態を復元します。

```rust
pub fn replay_events(events: Vec<DomainEvent>) -> Loan {
    events.into_iter().fold(Loan::empty(), |loan, event| {
        apply_event(loan, event)
    })
}

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
                // ...
            }
        }
        DomainEvent::LoanExtended(e) => {
            Loan {
                due_date: e.new_due_date,
                extension_count: loan.extension_count.increment().unwrap(),
                ..loan
            }
        }
        DomainEvent::BookReturned(e) => {
            Loan {
                returned_at: Some(e.returned_at),
                status: LoanStatus::Returned,
                ..loan
            }
        }
    }
}
```

**特徴：**
- 純粋関数
- イミュータブル
- 関数型プログラミングの基本パターン

### 状態復元の流れ

```
イベントストアから読み込み
    ↓
[Event1, Event2, Event3, ...]
    ↓
fold/reduce
    ↓
現在の状態（Loan）
```

## 関数合成とパイプライン

### 純粋関数の合成

小さな純粋関数を組み合わせて、大きな機能を実現します。

```rust
pub fn loan_book(/*...*/) -> Result<(Loan, BookLoaned), LoanError> {
    validate_inputs(book_id, member_id)?;
    
    let due_date = calculate_due_date(loaned_at);
    let loan = create_loan(loan_id, book_id, member_id, loaned_at, due_date);
    let event = create_loan_event(&loan, staff_id);
    
    Ok((loan, event))
}

fn validate_inputs(book_id: BookId, member_id: MemberId) -> Result<(), LoanError> {
    // バリデーション
}

fn calculate_due_date(loaned_at: DateTime<Utc>) -> DateTime<Utc> {
    loaned_at + Duration::days(14)
}

fn create_loan(/*...*/) -> Loan {
    // Loan生成
}

fn create_loan_event(loan: &Loan, staff_id: StaffId) -> BookLoaned {
    // イベント生成
}
```

**利点：**
- 各関数が単純
- テストが容易
- 再利用性が高い

### パターンマッチ

Rustの強力なパターンマッチを活用します。

```rust
fn apply_event(loan: Loan, event: &DomainEvent) -> Loan {
    match event {
        DomainEvent::BookLoaned(e) => handle_book_loaned(e),
        DomainEvent::LoanExtended(e) => handle_loan_extended(loan, e),
        DomainEvent::BookReturned(e) => handle_book_returned(loan, e),
    }
}
```

## CQRSパターン

### コマンドとクエリの分離

**コマンド側（書き込み）：**
- イベントストアに書き込む
- ビジネスルールを適用
- 純粋関数で処理

**クエリ側（読み込み）：**
- Read Modelから読み込む
- 最適化されたビュー
- 非正規化可能

### Read Modelの設計

```rust
// コマンド側のドメインモデル
pub struct Loan {
    loan_id: LoanId,
    book_id: BookId,      // IDのみ
    member_id: MemberId,  // IDのみ
    // ...
}

// クエリ側のRead Model（非正規化）
pub struct LoanView {
    pub loan_id: LoanId,
    pub book_id: BookId,
    pub book_title: String,     // 非正規化
    pub member_id: MemberId,
    pub member_name: String,    // 非正規化
    pub due_date: DateTime<Utc>,
    pub status: String,
}
```

**重要：**
- コマンド側はドメインの境界を守る
- クエリ側は表示最適化のため結合可能

## テスト戦略

### ドメイン層のテスト

純粋関数なので単純にテストできます。

```rust
#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_loan_book_success() {
        let result = loan_book(
            BookId::new(),
            MemberId::new(),
            Utc::now(),
            StaffId::new(),
        );
        
        assert!(result.is_ok());
        let (loan, event) = result.unwrap();
        assert_eq!(loan.status, LoanStatus::Active);
        assert_eq!(loan.extension_count.value(), 0);
    }
    
    #[test]
    fn test_extend_loan_at_limit() {
        let mut loan = create_test_loan();
        loan.extension_count = ExtensionCount::new().increment().unwrap();
        
        let result = extend_loan(&loan, Utc::now());
        
        assert!(matches!(result, Err(LoanError::ExtensionLimitExceeded)));
    }
}
```

**特徴：**
- データベース不要
- 外部API不要
- 高速実行

### アプリケーション層のテスト

モックを使用してテストします。

```rust
#[tokio::test]
async fn test_execute_loan_book() {
    // モックの準備
    let member_service = Arc::new(MockMemberService::new());
    let event_store = Arc::new(InMemoryEventStore::new());
    
    let service = LoanService::new(member_service, event_store);
    
    let cmd = LoanBookCommand { /* ... */ };
    let result = service.execute_loan_book(cmd).await;
    
    assert!(result.is_ok());
}
```

## まとめ

### 関数型DDDの実装パターン

**1. ヘキサゴナルアーキテクチャ**
- ドメインを中心に
- 副作用を外側に

**2. 純粋関数**
- 決定と実行の分離
- 副作用なし

**3. イミュータビリティ**
- すべて不変
- 新しいインスタンスを返す

**4. 型駆動設計**
- 型でビジネスルールを表現
- 不正な状態を排除

**5. Result型**
- Railway Oriented Programming
- 明示的なエラーハンドリング

**6. イベントソーシング**
- fold/reduceで状態復元
- イベント中心の設計

**7. CQRS**
- コマンドとクエリの分離
- 最適化されたRead Model

これらのパターンは、すべてのコンテキスト、すべてのPhaseで一貫して適用されます。
