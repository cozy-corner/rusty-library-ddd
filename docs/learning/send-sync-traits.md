# Send + Sync トレイト境界の意味

## 質問

```rust
pub trait NotificationService: Send + Sync {
    // ...
}
```

この`Send + Sync`は何を意味するのか？

## 結論

**`Send + Sync`は、このtraitを実装する型がスレッド間で安全に使えることを保証する。**

- **Send**: 所有権を別のスレッドに移動できる
- **Sync**: 複数のスレッドから参照（&T）で安全にアクセスできる

## Rustの並行処理安全性

Rustはコンパイル時に並行処理の安全性を保証する。その仕組みが`Send`と`Sync`というマーカートレイト。

### Sendトレイト

**定義:**
```rust
pub unsafe auto trait Send { }
```

**意味:**
型`T`が`Send`を実装している = `T`の所有権を別のスレッドに移動しても安全

**例：**
```rust
use std::thread;

let data = vec![1, 2, 3];  // Vec<i32>はSend

// 所有権を別スレッドに移動
thread::spawn(move || {
    println!("{:?}", data);  // OK: Vec<i32>はSend
});
```

**Sendを実装していない型の例：**
```rust
use std::rc::Rc;

let data = Rc::new(5);  // Rc<i32>はSendではない

// コンパイルエラー！
thread::spawn(move || {
    println!("{}", data);  // NG: Rc<i32>はSendではない
});
// error[E0277]: `Rc<i32>` cannot be sent between threads safely
```

### Syncトレイト

**定義:**
```rust
pub unsafe auto trait Sync { }
```

**意味:**
型`T`が`Sync`を実装している = `&T`（不変参照）を複数のスレッドから同時にアクセスしても安全

**言い換え:**
`T: Sync` ⇔ `&T: Send`（Tへの参照を別スレッドに送っても安全）

**例：**
```rust
use std::sync::Arc;
use std::thread;

let data = Arc::new(vec![1, 2, 3]);  // Arc<Vec<i32>>はSync

let data1 = Arc::clone(&data);
let data2 = Arc::clone(&data);

// 複数のスレッドから同時にアクセス可能
thread::spawn(move || {
    println!("{:?}", data1);  // OK
});

thread::spawn(move || {
    println!("{:?}", data2);  // OK
});
```

**Syncを実装していない型の例：**
```rust
use std::cell::Cell;

let data = Cell::new(5);  // Cell<i32>はSyncではない

// もし複数スレッドから&Cell<i32>にアクセスできたら、
// データ競合が起きる可能性がある（Rustはこれを防ぐ）
```

## なぜportに`Send + Sync`が必要か？

### 理由1：非同期ランタイム（Tokio）の要件

このプロジェクトは`async/await`を使用している。

```rust
#[async_trait]
pub trait NotificationService: Send + Sync {
    async fn send_overdue_notification(...) -> Result<()>;
}
```

**Tokioランタイムの動作：**
- 非同期タスクは複数のスレッドプールで実行される
- タスクが途中で別のスレッドに移動する可能性がある
- そのため、タスク内で使う型は`Send`が必要

```rust
// application/loan_service.rs
pub async fn execute_loan_book(
    cmd: LoanBookCommand,
    services: &Services,  // ServicesはSend + Syncが必要
) -> Result<LoanId> {
    // この関数は途中で別のスレッドに移動する可能性がある
    services.member_service.exists(cmd.member_id).await?;
    // ↑ awaitポイントでスレッドが切り替わる可能性
}
```

### 理由2：Arc（アトミック参照カウント）との併用

サービスは通常`Arc`でラップして複数箇所で共有する。

```rust
use std::sync::Arc;

// サービスの構築
let notification_service: Arc<dyn NotificationService> =
    Arc::new(EmailNotificationService::new());
```

**Arcの要件：**
```rust
impl<T: ?Sized + Sync + Send> Send for Arc<T> { }
impl<T: ?Sized + Sync + Send> Sync for Arc<T> { }
```

`Arc<T>`が`Send + Sync`であるためには、`T`も`Send + Sync`でなければならない。

### 理由3：複数のタスクから同時アクセス

```rust
// main.rs
let services = Services {
    notification_service: Arc::new(EmailNotificationService::new()),
    // ...
};

// 複数の非同期タスクから同時にアクセス
tokio::spawn(async move {
    services.notification_service.send_overdue_notification(...).await;
});

tokio::spawn(async move {
    services.notification_service.send_extension_confirmation(...).await;
});
```

## 実装例での確認

### Sync + Sendな実装

```rust
// adapters/email/notification_service.rs

pub struct EmailNotificationService {
    smtp_client: Arc<SmtpClient>,  // Arc内部でスレッド安全
}

// EmailNotificationServiceは自動的にSend + Syncになる
// （SmtpClientがSend + Syncなら）

impl NotificationService for EmailNotificationService {
    async fn send_overdue_notification(
        &self,
        member_id: MemberId,
        book_title: &str,
        due_date: DateTime<Utc>,
    ) -> Result<()> {
        // 実装
        Ok(())
    }
}
```

### Sync + Sendでない実装（コンパイルエラー）

```rust
use std::rc::Rc;

pub struct BadNotificationService {
    client: Rc<Client>,  // Rc<T>はSend/Syncではない
}

impl NotificationService for BadNotificationService {
    // コンパイルエラー！
    // error[E0277]: `Rc<Client>` cannot be sent between threads safely
}
```

## 自動実装（auto trait）

`Send`と`Sync`は**auto trait**（自動実装されるトレイト）。

**ルール：**
- 型のすべてのフィールドが`Send`なら、その型は自動的に`Send`
- 型のすべてのフィールドが`Sync`なら、その型は自動的に`Sync`

**例：**
```rust
struct MyService {
    counter: Arc<AtomicU64>,     // Send + Sync
    config: String,               // Send + Sync
}

// MyServiceは自動的にSend + Sync
// （すべてのフィールドがSend + Syncだから）
```

## 代表的な型のSend/Sync実装状況

| 型 | Send | Sync | 備考 |
|---|------|------|------|
| `i32`, `String`, `Vec<T>` | ✅ | ✅ | 基本的な型は両方実装 |
| `Arc<T>` | ✅ | ✅ | スレッド安全な共有（Tも必要） |
| `Rc<T>` | ❌ | ❌ | シングルスレッド専用 |
| `Cell<T>`, `RefCell<T>` | ✅ | ❌ | 内部可変性（Syncではない） |
| `Mutex<T>` | ✅ | ✅ | スレッド安全なロック |
| `MutexGuard<T>` | ❌ | ✅* | ロックは移動できない |

*`MutexGuard`は特殊（詳細は省略）

## このプロジェクトでの使用例

### ポート定義

```rust
// ports/notification_service.rs
#[async_trait]
pub trait NotificationService: Send + Sync {
    async fn send_overdue_notification(...) -> Result<()>;
}
```

**意味：**
- このtraitを実装する型は`Send + Sync`でなければならない
- 複数のスレッド/タスクから安全に使える
- `Arc`でラップして共有できる

### サービス構造体

```rust
// application/services.rs
pub struct Services {
    pub event_store: Arc<dyn EventStore>,
    pub loan_read_model: Arc<dyn LoanReadModel>,
    pub member_service: Arc<dyn MemberService>,
    pub book_service: Arc<dyn BookService>,
    pub notification_service: Arc<dyn NotificationService>,
}

// Servicesも自動的にSend + Sync
// （すべてのフィールドがArc<dyn Trait>で、TraitがSend + Syncだから）
```

### 非同期関数での使用

```rust
// application/loan_service.rs
pub async fn execute_loan_book(
    cmd: LoanBookCommand,
    services: &Services,  // Send + Syncが必要
) -> Result<LoanId> {
    // awaitポイントで別スレッドに移動する可能性
    if !services.member_service.exists(cmd.member_id).await? {
        return Err(LoanError::MemberNotFound);
    }

    // ドメイン層を呼ぶ
    let (loan, event) = domain::loan_book(...)?;

    // 永続化（awaitでスレッド切り替えの可能性）
    services.event_store.append(loan.loan_id, vec![event]).await?;

    Ok(loan.loan_id)
}
```

## async_traitマクロとの関係

```rust
#[async_trait]
pub trait NotificationService: Send + Sync {
    async fn send_overdue_notification(...) -> Result<()>;
}
```

`async_trait`マクロは内部的に以下のように展開する：

```rust
// 展開後のイメージ
pub trait NotificationService: Send + Sync {
    fn send_overdue_notification(
        &self,
        // ...
    ) -> Pin<Box<dyn Future<Output = Result<()>> + Send + '_>>;
    //                                              ^^^^
    //                                              FutureもSendが必要
}
```

**なぜ`Send`が必要か：**
- 非同期関数の戻り値は`Future`
- `Future`がスレッド間を移動する可能性がある
- そのため`Future: Send`が必要
- `Future: Send`であるためには、traitも`Send`が必要

## まとめ

### `Send + Sync`の意味

```rust
pub trait NotificationService: Send + Sync {
    // ...
}
```

**この宣言は以下を保証する：**

1. **Send**:
   - このtraitの実装をスレッド間で移動できる
   - `Arc`でラップして共有できる
   - 非同期タスクで安全に使える

2. **Sync**:
   - このtraitの実装への参照（`&`）を複数スレッドから同時にアクセスできる
   - `Arc<dyn Trait>`が`Send + Sync`になる
   - 並行アクセスが安全

3. **なぜ必要か：**
   - Tokioの非同期ランタイムの要件
   - `Arc`での共有の要件
   - 複数タスクからの同時アクセス

4. **自動実装：**
   - フィールドが全て`Send + Sync`なら自動的に実装される
   - `Rc`, `Cell`, `RefCell`など一部の型は実装していない

### ベストプラクティス

**非同期traitには常に`Send + Sync`を付ける：**
```rust
#[async_trait]
pub trait MyService: Send + Sync {
    async fn do_something(&self) -> Result<()>;
}
```

**理由：**
- Tokioで使う場合は必須
- `Arc`で共有する場合は必須
- 後から追加するのは困難（破壊的変更）

このプロジェクトのすべてのport定義に`Send + Sync`が付いているのは、非同期ランタイム（Tokio）で安全に使うための必須要件です。
