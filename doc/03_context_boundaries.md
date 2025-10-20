# ポートによるコンテキスト分離

## 境界づけられたコンテキスト

### 定義

特定のドメインモデルが適用される明確な境界。境界の内側では、用語やルールが一貫している。

**重要な原則：**
- 各コンテキストは独立したドメインモデルを持つ
- 同じ概念でもコンテキストごとに異なる表現をする
- 他のコンテキストの内部実装を知らない
- インターフェース（ポート）経由でのみ協調する

### なぜコンテキストを分離するのか

**1. 責務の分離**

各コンテキストは自分の責務に集中できる。

例：貸出管理コンテキストは「いつ誰に何を貸したか」を管理する責務を持つが、会員の詳細情報（住所、電話番号）を管理する責務は持たない。

**2. 変更の影響範囲を限定**

あるコンテキストの変更が他のコンテキストに波及しない。

例：会員情報の構造が変更されても、貸出管理には影響しない。

**3. チーム間の独立性**

異なるチームが異なるコンテキストを並行して開発できる。

**4. 技術選択の自由**

各コンテキストで異なる技術スタックを選択できる。

### 図書館システムのコンテキストマップ

```
【Core Domain】
┌──────────────────┐       ┌──────────────────┐
│  貸出管理         │       │  予約管理         │
│  Loan Management │◄─────►│  Reservation     │
└────────┬─────────┘       └────────┬─────────┘
         │ ポート経由                │
         ↓                          ↓
【Sub Domain】
┌──────────────┐  ┌──────────────┐  ┌──────────────┐
│  会員管理     │  │ カタログ管理  │  │  職員管理     │
└──────────────┘  └──────────────┘  └──────────────┘
```

**コンテキスト間の関係：**
- Core → Sub：ポート経由で依存
- Core ←→ Core：統合イベントで協調
- Sub → Core：依存しない

## コンテキスト境界の設計原則

### 原則1：他の集約はIDで参照する

**原則：**
集約は他の集約をオブジェクトで参照せず、IDで参照する。

**理由：**
- トランザクション境界を明確にする
- コンテキスト境界を守る
- 他のコンテキストの変更に影響されない

**例：貸出管理コンテキスト**

```rust
// ✅ 正しい：IDで参照
pub struct Loan {
    pub loan_id: LoanId,
    pub book_id: BookId,      // IDのみ
    pub member_id: MemberId,  // IDのみ
    pub loaned_at: DateTime<Utc>,
    // ...
}
```

```rust
// ❌ 間違い：オブジェクト参照
pub struct Loan {
    pub loan_id: LoanId,
    pub book: Book,           // 他コンテキストの集約
    pub member: Member,       // 他コンテキストの集約
}
```

**一般的な適用：**
- 予約管理でも同じ原則を適用
- すべてのコンテキストで一貫して適用

### 原則2：自分の責務のみを持つ

**原則：**
各コンテキストは自分の責務範囲の情報のみを持つ。

**例：貸出管理が知っていること**
- 誰が、いつ、どの書籍を借りているか
- 返却期限、延長回数
- 延滞状態

**例：貸出管理が知らないこと**
- 会員の名前、住所、電話番号（会員管理の責務）
- 書籍のタイトル、著者、ISBN（カタログ管理の責務）
- 職員の所属、権限（職員管理の責務）

**一般的な適用：**
各コンテキストで「何を知るべきか/知るべきでないか」を明確にする。

### 原則3：ポートで依存を抽象化する

**原則：**
他のコンテキストへの依存は、ポート（インターフェース）を通じて行う。

**理由：**
- 実装の詳細を隠蔽
- テスタビリティの向上
- 実装方法を後から変更可能

**ポートの定義例：**

```rust
/// このコンテキストが必要とする機能を定義
#[async_trait]
pub trait MemberService: Send + Sync {
    async fn exists(&self, member_id: MemberId) -> Result<bool>;
}
```

**実装は何でも可能：**
- インメモリ（開発初期、テスト）
- HTTP/REST（マイクロサービス）
- gRPC（高性能通信）
- 同一プロセス内の直接呼び出し

**一般的な適用：**
すべてのコンテキスト間の依存関係でポートを使用する。

## ポートの設計原則

### 設計原則1：必要最小限のインターフェース

**原則：**
ポートは利用側が必要とする最小限の機能のみを定義する。

**例：**

```rust
// ✅ 良い例：必要最小限
#[async_trait]
pub trait MemberService: Send + Sync {
    async fn exists(&self, member_id: MemberId) -> Result<bool>;
}
```

```rust
// ❌ 悪い例：不要なメソッドが多い
#[async_trait]
pub trait MemberService: Send + Sync {
    async fn exists(&self, member_id: MemberId) -> Result<bool>;
    async fn get_name(&self, member_id: MemberId) -> Result<String>;
    async fn get_email(&self, member_id: MemberId) -> Result<String>;
    async fn get_address(&self, member_id: MemberId) -> Result<Address>;
    async fn update_points(&mut self, member_id: MemberId, points: u32) -> Result<()>;
    // 不要なメソッドが増える...
}
```

**理由：**
- インターフェースが小さいほど結合度が低い
- テストが容易
- 実装が簡単

### 設計原則2：利用側の視点で定義する

**原則：**
ポートは「提供側」ではなく「利用側」の視点で定義する。

**例：貸出管理が書籍情報を必要とする場合**

```rust
// ✅ 良い例：貸出管理の視点
#[async_trait]
pub trait BookService: Send + Sync {
    /// 貸出可能かを判定（貸出管理が知りたいこと）
    async fn is_available_for_loan(&self, book_id: BookId) -> Result<bool>;
}
```

```rust
// ❌ 悪い例：カタログ管理の視点
#[async_trait]
pub trait BookService: Send + Sync {
    /// 書籍の詳細情報を返す（カタログ管理が提供できること）
    async fn get_book_details(&self, book_id: BookId) -> Result<BookDetails>;
}

pub struct BookDetails {
    pub title: String,
    pub author: String,
    pub isbn: String,
    pub publisher: String,
    pub published_year: u16,
    pub genre: Vec<String>,
    // カタログ管理が持つすべての情報
}
```

**理由：**
- 利用側の責務が明確になる
- 不要な情報が含まれない
- 利用側のニーズに合致する

### 設計原則3：実装方法に依存しない

**原則：**
ポートは実装方法を規定しない。

**例：**

```rust
// ✅ 実装方法を問わない
#[async_trait]
pub trait MemberService: Send + Sync {
    async fn exists(&self, member_id: MemberId) -> Result<bool>;
}

// 様々な実装が可能：
// - InMemoryMemberService（Phase 1）
// - HttpMemberService（Phase 4）
// - GrpcMemberService（将来）
// - DirectMemberService（同一プロセス）
```

```rust
// ❌ 実装方法に依存
#[async_trait]
pub trait MemberService: Send + Sync {
    /// HTTPエンドポイントを指定
    async fn http_get_member(&self, url: &str) -> Result<Response>;
}
```

**理由：**
- 実装を後から変更できる
- テスト実装が容易
- 技術選択の自由度が高い

## コンテキスト境界違反のパターン

### アンチパターン1：他コンテキストの集約を直接操作

```rust
// ❌ 悪い例
impl LoanService {
    async fn loan_book(&self, cmd: LoanBookCommand) -> Result<()> {
        // 会員管理の集約を直接取得
        let mut member = self.member_repo.find_by_id(cmd.member_id)?;
        
        // 会員管理の責務を実行（境界違反）
        member.add_points(10);
        member.update_tier();
        
        self.member_repo.save(&member)?;
        // ...
    }
}
```

**問題点：**
- 会員管理のビジネスルールが貸出管理に漏れる
- Memberの変更が貸出管理に影響する
- トランザクション境界が曖昧

**正しいパターン：**

```rust
// ✅ 良い例
impl LoanService {
    async fn loan_book(&self, cmd: LoanBookCommand) -> Result<()> {
        // 最小限の依存（存在確認のみ）
        if !self.member_service.exists(cmd.member_id).await? {
            return Err(LoanError::MemberNotFound);
        }
        
        // 自分の責務に集中
        let (loan, event) = domain::loan_book(/*...*/)?;
        self.event_store.append(event).await?;
        
        Ok(())
    }
}
```

### アンチパターン2：他コンテキストのドメインモデルを定義

```rust
// ❌ 悪い例：貸出管理コンテキストで定義
pub struct Member {
    pub member_id: MemberId,
    pub name: String,
    pub email: String,
    pub phone: String,
    pub points: u32,
}

pub struct Loan {
    pub loan_id: LoanId,
    pub member: Member,  // 他コンテキストのモデル
    // ...
}
```

**問題点：**
- コンテキスト境界が曖昧
- 責務が混在
- 変更の影響が広がる

**正しいパターン：**

```rust
// ✅ 良い例：IDのみ参照
pub struct Loan {
    pub loan_id: LoanId,
    pub member_id: MemberId,  // IDのみ
    // ...
}

// Memberは会員管理コンテキストで定義される
```

### アンチパターン3：直接リポジトリを使用

```rust
// ❌ 悪い例
pub struct LoanService {
    loan_repo: Arc<dyn LoanRepository>,
    member_repo: Arc<dyn MemberRepository>,  // 直接依存
    book_repo: Arc<dyn BookRepository>,      // 直接依存
}
```

**問題点：**
- リポジトリは実装の詳細
- 他コンテキストの永続化層に依存
- テストが困難

**正しいパターン：**

```rust
// ✅ 良い例：ポート経由
pub struct LoanService {
    loan_repo: Arc<dyn LoanRepository>,
    member_service: Arc<dyn MemberService>,  // ポート
    book_service: Arc<dyn BookService>,      // ポート
}
```

## どうしても他コンテキストの情報が必要な場合

### 一時的な取得パターン

**シナリオ：**
延滞通知に書籍のタイトルを含めたい。

**解決方法：**

```rust
// application/loan_service.rs

pub async fn send_overdue_notification(
    &self,
    loan: &Loan,
) -> Result<()> {
    // ポート経由で一時的に取得
    let book_title = self.book_service
        .get_book_title(loan.book_id)
        .await?;
    
    // メッセージを作成
    let message = format!(
        "書籍「{}」が延滞しています。至急ご返却ください。",
        book_title
    );
    
    // 通知を送信
    self.notification_service
        .send_overdue_notification(loan.member_id, &message)
        .await?;
    
    Ok(())
}
```

**重要：**
- ローカル変数として使用（保持しない）
- ドメイン層には持ち込まない
- 必要な時だけ取得

### Read Modelでの結合

**シナリオ：**
画面表示のために貸出と書籍情報を一緒に表示したい。

**解決方法：**

CQRS のRead Modelで結合する。

```rust
// クエリ側（Read Model）
pub struct LoanView {
    pub loan_id: LoanId,
    pub book_id: BookId,
    pub book_title: String,      // 非正規化
    pub member_id: MemberId,
    pub member_name: String,     // 非正規化
    pub due_date: DateTime<Utc>,
    // ...
}
```

**重要：**
- コマンド側（書き込み）では境界を守る
- クエリ側（読み込み）で結合
- ドメインモデルには影響しない

## 実装例：Phase 1の貸出管理

Phase 1では、ポート設計の原則を最初から適用します。

### ポート定義

```rust
// ports/member_service.rs
#[async_trait]
pub trait MemberService: Send + Sync {
    async fn exists(&self, member_id: MemberId) -> Result<bool>;
}

// ports/book_service.rs
#[async_trait]
pub trait BookService: Send + Sync {
    async fn is_available_for_loan(&self, book_id: BookId) -> Result<bool>;
    async fn get_book_title(&self, book_id: BookId) -> Result<String>;
}
```

### インメモリ実装（例示のため）

開発初期やテストでは、シンプルな実装を使用します。

```rust
// adapters/in_memory/member_service.rs
pub struct MemberService;

#[async_trait]
impl crate::ports::MemberService for MemberService {
    async fn exists(&self, _member_id: MemberId) -> Result<bool> {
        Ok(true)  // シンプルな実装
    }
}
```

### 実装の切り替え

ポートのおかげで、実装を簡単に切り替えられます。

```rust
// 開発初期
let member_service = Arc::new(in_memory::MemberService);

// 後で切り替え
let member_service = Arc::new(http::MemberService::new(url));
```

## まとめ

### コンテキスト境界設計の原則

1. **各コンテキストは独立している**
   - 独自のドメインモデル
   - 独自のユビキタス言語
   - 自分の責務のみを持つ

2. **他の集約はIDで参照**
   - オブジェクト参照しない
   - トランザクション境界を明確にする

3. **ポートで依存を抽象化**
   - 必要最小限のインターフェース
   - 利用側の視点で定義
   - 実装方法に依存しない

4. **境界違反を避ける**
   - 他コンテキストの集約を直接操作しない
   - 他コンテキストのドメインモデルを定義しない
   - リポジトリではなくポートを使う

これらの原則は、すべてのPhaseで一貫して適用されます。
