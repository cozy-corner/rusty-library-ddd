# 関数型DDDにおけるサービスパターン

## 概要

関数型DDDでアプリケーション層のサービスを実装する際、**インスタンスベース（OOP）** か **関数ベース（FP）** のどちらを選択すべきかという設計判断について解説します。

## 2つのパターン

### パターンA: インスタンスベース（OOPスタイル）

```rust
/// 構造体にメソッドを持たせるパターン
pub struct LoanService {
    event_store: Arc<dyn EventStore>,
    loan_read_model: Arc<dyn LoanReadModel>,
    member_service: Arc<dyn MemberService>,
    book_service: Arc<dyn BookService>,
}

impl LoanService {
    pub fn new(
        event_store: Arc<dyn EventStore>,
        loan_read_model: Arc<dyn LoanReadModel>,
        member_service: Arc<dyn MemberService>,
        book_service: Arc<dyn BookService>,
    ) -> Self {
        Self {
            event_store,
            loan_read_model,
            member_service,
            book_service,
        }
    }

    pub async fn loan_book(&self, cmd: LoanBook) -> Result<LoanId> {
        // self経由で依存にアクセス
        self.event_store.append(...).await?;
        // ...
    }
}
```

**使用例:**
```rust
let service = LoanService::new(event_store, read_model, member_svc, book_svc);
let loan_id = service.loan_book(cmd).await?;
```

### パターンB: 関数ベース（FPスタイル）

```rust
/// 依存関係をデータ構造として定義
#[derive(Clone)]
pub struct ServiceDependencies {
    pub event_store: Arc<dyn EventStore>,
    pub loan_read_model: Arc<dyn LoanReadModel>,
    pub member_service: Arc<dyn MemberService>,
    pub book_service: Arc<dyn BookService>,
}

/// 純粋な関数（すべての依存が引数として明示的）
pub async fn loan_book(
    deps: &ServiceDependencies,
    cmd: LoanBook,
) -> Result<LoanId> {
    // deps経由で依存にアクセス
    deps.event_store.append(...).await?;
    // ...
}
```

**使用例:**
```rust
let deps = ServiceDependencies { event_store, loan_read_model, member_service, book_service };
let loan_id = loan_book(&deps, cmd).await?;
```

## 比較表

| 観点 | インスタンスベース（OOP） | 関数ベース（FP） |
|------|-------------------------|-----------------|
| **依存の明示性** | ❌ `self`を通じた暗黙的アクセス | ✅ 引数として完全に明示的 |
| **データと振る舞いの分離** | ❌ カプセル化（OOPの原則） | ✅ 完全に分離（FPの原則） |
| **関数合成** | ⚠️ メソッド呼び出しチェーン | ✅ 関数を変数として扱える |
| **テストの明確さ** | ⚠️ インスタンス生成が必要 | ✅ 依存を直接渡すだけ |
| **Rustでの一般性** | ✅ Rustコミュニティで主流 | ⚠️ やや非標準的 |
| **他言語との類似性** | ✅ Java/C#/Kotlinに近い | ❌ 他の主流言語と異なる |
| **関数型DDD文献との整合性** | ❌ OOP寄り | ✅ F#/Haskellパターンと一致 |

## 関数型DDDの観点からの判断

### 関数型プログラミングの核心原則

1. **純粋関数** - 副作用なし
2. **イミュータビリティ** - 不変データ
3. **明示的な依存** - 暗黙的な状態を避ける
4. **データと振る舞いの分離** - OOPのカプセル化ではなく
5. **関数合成** - 小さな関数を組み合わせる

### パターンAの問題点（関数型の観点）

```rust
impl LoanService {
    pub async fn loan_book(&self, cmd: LoanBook) -> Result<LoanId> {
        self.event_store  // ← self = 暗黙的な依存
    }
}
```

**問題:**
- ❌ `self`が暗黙的な状態に見える（関数型原則に反する）
- ❌ OOPのカプセル化パターン（データと振る舞いを一緒にする）
- ❌ 関数シグネチャからすべての依存が見えない

### パターンBの利点（関数型の観点）

```rust
pub async fn loan_book(
    deps: &ServiceDependencies,  // ← すべての依存が明示的
    cmd: LoanBook,
) -> Result<LoanId>
```

**利点:**
- ✅ すべての依存が引数として明示的
- ✅ データ（ServiceDependencies）と振る舞い（関数）が分離
- ✅ 関数シグネチャを見るだけで完全に理解できる
- ✅ 関数を値として扱える（高階関数に渡せる）

## 関数型DDD文献からの引用

### "Domain Modeling Made Functional" (Scott Wlaschin)

> "In functional programming, we prefer to pass dependencies explicitly rather than hiding them in objects. This makes the flow of data explicit and makes testing easier."

（関数型プログラミングでは、依存関係をオブジェクトに隠すのではなく、明示的に渡すことを好む。これによりデータフローが明確になり、テストも容易になる。）

### "Functional and Reactive Domain Modeling" (Debasish Ghosh)

> "Services are just functions that take dependencies as parameters. There's no need for objects or classes."

（サービスは依存関係をパラメータとして受け取る単なる関数である。オブジェクトやクラスは不要。）

## F#での標準的なパターン

```fsharp
// F#（関数型言語）での標準パターン

// 依存関係をレコードで定義（データ構造）
type ServiceDependencies = {
    EventStore: EventStore
    ReadModel: LoanReadModel
    MemberService: MemberService
    BookService: BookService
}

// 純粋な関数
let loanBook
    (deps: ServiceDependencies)  // すべての依存が明示的
    (cmd: LoanBook)
    : Async<Result<LoanId, LoanError>> =
    async {
        let! memberExists = deps.MemberService.exists cmd.memberId
        // ...
    }
```

## Haskellでの標準的なパターン

```haskell
-- Haskell（純粋関数型言語）での標準パターン

-- 依存関係をデータ型で定義
data ServiceDeps = ServiceDeps
  { eventStore :: EventStore
  , readModel :: LoanReadModel
  , memberService :: MemberService
  , bookService :: BookService
  }

-- 純粋な関数
loanBook :: ServiceDeps -> LoanBook -> IO (Either LoanError LoanId)
loanBook deps cmd = do
  memberExists <- exists (memberService deps) (memberId cmd)
  -- ...
```

## Rustの他のパターンとの比較

### axum（Webフレームワーク）での一般的なパターン

```rust
// axumではインスタンスベースが主流
pub struct AppState {
    loan_service: Arc<LoanService>,
}

async fn loan_book_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<LoanBookRequest>,
) -> Result<Json<LoanResponse>, ApiError> {
    let loan_id = state.loan_service.loan_book(cmd).await?;
    // ...
}
```

**ただし、関数ベースでも同様に可能:**
```rust
pub struct AppState {
    deps: ServiceDependencies,
}

async fn loan_book_handler(
    State(state): State<Arc<AppState>>,
    Json(req): Json<LoanBookRequest>,
) -> Result<Json<LoanResponse>, ApiError> {
    let loan_id = loan_book(&state.deps, cmd).await?;
    // ...
}
```

## 判断基準

### 関数ベース（パターンB）を選ぶべき場合

✅ **プロジェクトのテーマが「関数型DDD」**
- 関数型プログラミングの原則を学ぶことが目的
- F#/Haskellの関数型DDDパターンを参考にしている
- 関数型の明示性を重視

✅ **教育・学習目的**
- 関数型の原則を明確に示したい
- データと振る舞いの分離を強調したい

✅ **関数合成が重要**
- 関数をパイプラインで組み合わせたい
- 高階関数を多用する

### インスタンスベース（パターンA）を選ぶべき場合

✅ **Rustコミュニティの標準に従いたい**
- axum、actix-webなどの主流フレームワークと整合
- Rustエコシステムとの親和性

✅ **他の主流言語からの移行者が多い**
- Java/C#/Kotlinから来たチームメンバー
- OOPに慣れている開発者

✅ **実用性を最優先**
- 学習コストを抑えたい
- Rustのベストプラクティスに従いたい

## このプロジェクトでの決定

### 選択：**関数ベース（パターンB）**

**理由:**

1. **プロジェクトのテーマが「関数型DDD」**
   - 明示的に関数型プログラミングの原則を学ぶことが目的
   - ドキュメント（CLAUDE.md）にも関数型DDDが明記されている

2. **関数型DDD文献との整合性**
   - Scott WlaschinやDebasish Ghoshのパターンと一致
   - F#/Haskellの標準パターンに準拠

3. **明示性の重視**
   - すべての依存が関数シグネチャから見える
   - データフローが完全に明確

4. **教育的価値**
   - 関数型の原則（データと振る舞いの分離）が明確
   - OOPとの違いが一目瞭然

### 実装例

```rust
// src/application/loan/loan_service.rs

/// サービスの依存関係（データ構造のみ）
#[derive(Clone)]
pub struct ServiceDependencies {
    pub event_store: Arc<dyn EventStore>,
    pub loan_read_model: Arc<dyn LoanReadModel>,
    pub member_service: Arc<dyn MemberService>,
    pub book_service: Arc<dyn BookService>,
}

/// 書籍を貸し出す（純粋な関数）
///
/// すべての依存が引数として明示的に渡される（関数型の原則）。
pub async fn loan_book(
    deps: &ServiceDependencies,
    cmd: LoanBook,
) -> Result<LoanId> {
    // 実装...
}

/// 貸出を延長する（純粋な関数）
pub async fn extend_loan(
    deps: &ServiceDependencies,
    cmd: ExtendLoan,
) -> Result<()> {
    // 実装...
}

/// 書籍を返却する（純粋な関数）
pub async fn return_book(
    deps: &ServiceDependencies,
    cmd: ReturnBook,
) -> Result<()> {
    // 実装...
}
```

## まとめ

### 関数型DDDの観点

**結論: 関数ベース（パターンB）が適切**

- ✅ 関数型の原則に完全準拠
- ✅ F#/Haskellパターンと一致
- ✅ 明示性が最大化
- ✅ データと振る舞いの分離

### 実用的な観点

**インスタンスベース（パターンA）もRustでは妥当**

- ✅ Rustコミュニティの標準
- ✅ axum/actix-webとの親和性
- ✅ Java/C#からの移行が容易

### 最終推奨

**プロジェクトのテーマと目的に応じて選択:**

| プロジェクトの性質 | 推奨パターン |
|------------------|-------------|
| 関数型DDDの学習 | **関数ベース（B）** |
| 実用的なRustアプリ | インスタンスベース（A） |
| F#/Haskell移植 | **関数ベース（B）** |
| Java/C#からの移行 | インスタンスベース（A） |

このプロジェクトは**関数型DDDをテーマ**としているため、**関数ベース（パターンB）**を採用します。
