# Portの設計哲学：ビジネスロジックを含むべきか？

## 質問

以下のようなport定義は、ビジネスロジックが含まれているように見えるが、一般的なのか？

```rust
pub trait BookService: Send + Sync {
    /// 書籍が貸出可能か確認する
    ///
    /// ビジネスルール: 貸出不可の書籍は貸し出せない。
    async fn is_available_for_loan(&self, book_id: BookId) -> Result<bool>;
}
```

## 結論

**このようなビジネスロジックを含むportは一般的であり、推奨される。**

DDDとヘキサゴナルアーキテクチャでは、portは**利用側の視点**で定義すべきという原則がある。

## 2つのアプローチの比較

### アプローチ1：ビジネスロジックを含む（推奨）✅

```rust
// 貸出管理コンテキストの視点で定義
pub trait BookService: Send + Sync {
    /// 貸出可能かを判定（貸出管理が知りたいこと）
    async fn is_available_for_loan(&self, book_id: BookId) -> Result<bool>;
}
```

**メリット：**
- ✅ 利用側の意図が明確
- ✅ 結合度が低い（貸出管理はカタログの内部構造を知らない）
- ✅ 実装の柔軟性が高い
- ✅ Tell, Don't Ask原則に従う

**デメリット：**
- ⚠️ ビジネスロジックがport（境界）に「要求」として現れる
- ⚠️ カタログコンテキストのビジネスルールを貸出側のportで定義している（ように見える）

### アプローチ2：汎用的なデータ取得（非推奨）❌

```rust
// カタログ管理コンテキストの視点で定義
pub trait BookService: Send + Sync {
    /// 書籍の詳細情報を返す
    async fn get_book_details(&self, book_id: BookId) -> Result<BookDetails>;
}

pub struct BookDetails {
    pub title: String,
    pub author: String,
    pub isbn: String,
    pub publisher: String,
    pub status: BookStatus,  // これだけ必要なのに...
    pub genre: Vec<String>,
}
```

**メリット：**
- ✅ ビジネスロジックがカタログ側に完全に留まる
- ✅ 汎用的で再利用しやすい

**デメリット：**
- ❌ 貸出管理がカタログの内部状態（BookDetails）を知ってしまう
- ❌ 結合度が高い
- ❌ 不要な情報が漏れる（Interface Segregation Principleに違反）
- ❌ Ask（聞く）パターンでビジネスロジックが利用側に漏れる

```rust
// ❌ 悪い例：ビジネスロジックが貸出管理側に漏れる
let details = book_service.get_book_details(book_id).await?;
if details.status == BookStatus::Available {
    // この判定は本来カタログ側の責務
}
```

## 設計原則との関係

### 1. Dependency Inversion Principle（依存性の逆転）

```
従来の依存方向：
貸出管理 → カタログ管理の具体的な実装

DIP適用後：
貸出管理 → BookService（貸出管理が定義） ← カタログ管理が実装
```

**重要：** portは**利用側（貸出管理）が定義**する。提供側（カタログ）が定義するのではない。

### 2. Interface Segregation Principle（インターフェース分離）

クライアントは自分が使わないメソッドに依存すべきではない。

```rust
// ✅ 良い例：必要最小限
pub trait BookService: Send + Sync {
    async fn is_available_for_loan(&self, book_id: BookId) -> Result<bool>;
    async fn get_book_title(&self, book_id: BookId) -> Result<String>;
}

// ❌ 悪い例：不要な情報が含まれる
pub trait BookService: Send + Sync {
    async fn get_all_details(&self, book_id: BookId) -> Result<BookDetails>;
}
```

### 3. Tell, Don't Ask（聞くな、命令しろ）

```rust
// ❌ Ask（聞く）：ビジネスロジックが利用側に漏れる
let status = book_service.get_status(book_id).await?;
if status == BookStatus::Available {
    // 貸出処理
}

// ✅ Tell（命令/質問）：ビジネスロジックは提供側
if book_service.is_available_for_loan(book_id).await? {
    // 貸出処理
}
```

## 重要な理解：ビジネスロジックの所在

**誤解：** ビジネスロジックがportに「含まれている」

**正確：** ビジネスロジックはportが「要求している」

実際のビジネスロジックは**adapter実装側（提供側）** にある：

```rust
// ===== ports/book_service.rs（定義のみ、実装なし）=====
pub trait BookService: Send + Sync {
    /// 貸出可能かを判定
    async fn is_available_for_loan(&self, book_id: BookId) -> Result<bool>;
}

// ===== adapters/catalog/book_service.rs（ビジネスロジックはここ）=====
pub struct CatalogBookService {
    repo: Arc<dyn BookRepository>,
}

impl BookService for CatalogBookService {
    async fn is_available_for_loan(&self, book_id: BookId) -> Result<bool> {
        let book = self.repo.find(book_id).await?;

        // ↓ ビジネスロジックはカタログコンテキストが実装
        Ok(book.status == Status::Available
            && !book.is_reserved
            && book.condition == Condition::Good)
    }
}

// ===== adapters/mock/book_service.rs（テスト用）=====
pub struct MockBookService;

impl BookService for MockBookService {
    async fn is_available_for_loan(&self, _book_id: BookId) -> Result<bool> {
        Ok(true)  // テストではシンプルに
    }
}
```

## レイヤーの責務

### Domain層（貸出管理コンテキスト）
- 純粋関数でビジネスロジック
- 外部依存なし

### Ports層（境界のインターフェース）
- 利用側の視点で定義
- **「何を知りたいか」を表現**（実装は含まない）

### Adapters層（インフラストラクチャ）
- portsの実装
- **実際のビジネスロジックを実装**
- カタログコンテキストのルールはここに

### Application層（ユースケース）
- portsを使って外部と協調
- domainの純粋関数を呼ぶ

```rust
// application/loan_service.rs
pub async fn execute_loan_book(
    cmd: LoanBookCommand,
    services: &Services,
) -> Result<LoanId> {
    // ポート経由で外部に問い合わせ（ビジネスロジックは提供側）
    if !services.book_service.is_available_for_loan(cmd.book_id).await? {
        return Err(LoanError::BookNotAvailable);
    }

    // ドメイン層の純粋関数を呼ぶ
    let (loan, event) = domain::loan_book(
        cmd.book_id,
        cmd.member_id,
        cmd.loaned_at,
        cmd.staff_id,
    )?;

    // 永続化
    services.event_store.append(loan.loan_id, vec![event]).await?;

    Ok(loan.loan_id)
}
```

## 実例：有名なフレームワーク/アーキテクチャ

### Spring Framework（Java）
```java
// 利用側の視点で定義
public interface PaymentGateway {
    boolean canProcessPayment(Payment payment);  // ビジネスロジックを要求
}
```

### Clean Architecture（Uncle Bob）
> "Ports are defined by the use cases, not by the adapters"
> （ポートはユースケース（利用側）が定義し、アダプターが定義するのではない）

### Hexagonal Architecture（Alistair Cockburn）
> "Ports represent what the application needs, not what the infrastructure provides"
> （ポートはアプリケーションが必要とすることを表現し、インフラが提供できることではない）

## 設計ドキュメントでの推奨

`doc/03_context_boundaries.md`（186-216行目）より：

**✅ 良い例：利用側の視点**
```rust
pub trait BookService: Send + Sync {
    /// 貸出可能かを判定（貸出管理が知りたいこと）
    async fn is_available_for_loan(&self, book_id: BookId) -> Result<bool>;
}
```

**❌ 悪い例：提供側の視点**
```rust
pub trait BookService: Send + Sync {
    /// 書籍の詳細情報を返す（カタログ管理が提供できること）
    async fn get_book_details(&self, book_id: BookId) -> Result<BookDetails>;
}
```

**理由：**
- 利用側の責務が明確になる
- 不要な情報が含まれない
- 利用側のニーズに合致する

## このプロジェクトでの適用

### MemberService
```rust
pub trait MemberService: Send + Sync {
    /// 会員が存在するか確認する
    async fn exists(&self, member_id: MemberId) -> Result<bool>;

    /// 会員が延滞中の貸出を持っているか確認する
    async fn has_overdue_loans(&self, member_id: MemberId) -> Result<bool>;
}
```

**ビジネスルール「延滞中の会員には貸出不可」を表現**
- ビジネスロジックはport定義に「含まれている」のではない
- 貸出管理が「何を知りたいか」を表現している
- 実際のロジック（延滞判定）は会員管理側のadapterが実装

### BookService
```rust
pub trait BookService: Send + Sync {
    /// 書籍が貸出可能か確認する
    async fn is_available_for_loan(&self, book_id: BookId) -> Result<bool>;

    /// 書籍タイトルを取得する
    async fn get_book_title(&self, book_id: BookId) -> Result<String>;
}
```

**ビジネスルール「書籍が貸出可能であること」を表現**
- 「在庫あり」「予約なし」「状態良好」などの判定はカタログ側
- 貸出管理は結果（bool）だけを受け取る

### NotificationService
```rust
pub trait NotificationService: Send + Sync {
    /// 延滞通知を会員に送信する
    async fn send_overdue_notification(
        &self,
        member_id: MemberId,
        book_title: &str,
        due_date: DateTime<Utc>,
    ) -> Result<()>;
}
```

**通知の「内容」ではなく「意図」を表現**
- メール/SMS/Slackのどれを使うかはadapterが決める
- 通知の文面もadapterが実装

## まとめ

### ポート設計の原則

1. **利用側の視点で定義する**
   - portは利用側（貸出管理）が「何を知りたいか」を表現
   - 提供側（カタログ）が「何を提供できるか」ではない

2. **必要最小限のインターフェース**
   - 必要な情報だけを要求
   - 不要な情報は含めない

3. **実装方法に依存しない**
   - PostgreSQL、HTTP、インメモリどれでも実装可能
   - ビジネスロジックの実装はadapterに委ねる

4. **Tell, Don't Ask**
   - 「状態を取得して判定」ではなく「判定結果を取得」
   - ビジネスロジックの漏洩を防ぐ

### ビジネスロジックの所在

| レイヤー | 役割 | ビジネスロジック |
|---------|------|----------------|
| Domain | 純粋関数でコアロジック | ✅ ある（貸出管理のルール） |
| Ports | インターフェース定義 | ❌ ない（要求のみ） |
| Adapters | portsの実装 | ✅ ある（カタログ/会員のルール） |
| Application | ユースケース実行 | △ 少しある（調整のみ） |

### 最終的な理解

**`is_available_for_loan()` のようなport定義は：**
- ✅ ビジネスロジックを「要求」している
- ✅ ビジネスロジックを「含んでいる」わけではない
- ✅ 利用側の視点で設計されている
- ✅ DDDとヘキサゴナルアーキテクチャの原則に準拠
- ✅ 一般的であり推奨される設計パターン

このアプローチにより：
- コンテキスト境界が明確
- 結合度が低い
- テスタビリティが高い
- 変更に強い

という利点が得られる。
