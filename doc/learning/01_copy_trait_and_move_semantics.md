# Copy トレイトとムーブセマンティクスの学び

**日付:** 2025-10-29
**コンテキスト:** Task 1.1b - 純粋関数の移行（CodeRabbitレビュー対応）

## 目次

1. [問題の発見](#問題の発見)
2. [Copy トレイトとは](#copy-トレイトとは)
3. [Use-After-Moveの問題](#use-after-moveの問題)
4. [ID型にCopyを実装する判断](#id型にcopyを実装する判断)
5. [ベストプラクティス](#ベストプラクティス)
6. [参考資料](#参考資料)

---

## 問題の発見

### CodeRabbitの指摘

Task 1.1bの実装で、以下のコードに対してCodeRabbitが指摘：

```rust
// extend_loan_v2() の元実装
pub fn extend_loan_v2(
    loan: ActiveLoan,
    extended_at: DateTime<Utc>,
) -> Result<(ActiveLoan, LoanExtended), ExtendLoanError> {
    // ...
    let old_due_date = loan.due_date;
    let new_due_date = loan.due_date + Duration::days(LOAN_PERIOD_DAYS);

    // 新しいActiveLoanを生成
    let new_loan = ActiveLoan {
        core: LoanCore {
            ..loan.core  // ← loan.core をムーブ
        },
    };

    let event = LoanExtended {
        loan_id: loan.loan_id,  // ← ムーブ後のアクセス！
        // ...
    };

    Ok((new_loan, event))
}
```

**指摘内容:**
> Fix use-after-move in extend_loan_v2 (move of loan.core then using loan.*)

### なぜコンパイルが通ったのか？

```rust
// src/domain/value_objects.rs:7
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LoanId(Uuid);
```

`LoanId`が`Copy`トレイトを実装しているため：
- `loan.core`をムーブした後でも
- `loan.loan_id`にアクセスできる（`Copy`されるため）

**これは偶然動いているだけ！**

---

## Copy トレイトとは

### 定義

```rust
pub trait Copy: Clone { }
```

`Copy`トレイトは、値を**ビット単位でコピー**できる型に実装される。

### Copy可能な型の条件

1. **固定サイズ**でスタックに収まる
2. **すべてのフィールド**が`Copy`
3. **Drop実装なし**（デストラクタ不要）

### Copyの動作

```rust
let x = 5;       // i32 は Copy
let y = x;       // x がコピーされる
println!("{}", x); // x はまだ使える！

let s1 = String::from("hello");  // String は Copy でない
let s2 = s1;     // s1 がムーブされる
// println!("{}", s1); // コンパイルエラー！
```

---

## Use-After-Moveの問題

### 問題のパターン

```rust
struct Data {
    id: u32,      // Copy
    name: String, // Copy でない
}

fn problematic(data: Data) {
    let new_data = Data {
        name: data.name,  // ← data.name をムーブ
        ..data            // エラー！data.nameは既にムーブ済み
    };

    // この時点で data.id は Copy なのでアクセス可能だが、
    // data.name は既にムーブされている
    println!("{}", data.id);   // OK（Copy）
    // println!("{}", data.name); // エラー（ムーブ済み）
}
```

### 今回のケース

```rust
// ActiveLoanの構造
pub struct ActiveLoan {
    pub core: LoanCore,  // core は Copy でない
}

impl std::ops::Deref for ActiveLoan {
    type Target = LoanCore;
    fn deref(&self) -> &Self::Target {
        &self.core
    }
}

// loan.loan_id は Deref により loan.core.loan_id へのアクセス
// LoanId は Copy なので、core をムーブした後もアクセス可能
```

**問題点:**
1. `loan.core`をムーブ
2. `loan.loan_id`（`loan.core.loan_id`）にアクセス
3. `LoanId`が`Copy`なのでコンパイルは通る
4. しかし、これは`Copy`への暗黙の依存

### 修正方法

**修正前:**
```rust
let new_loan = ActiveLoan {
    core: LoanCore {
        ..loan.core  // ムーブ
    },
};

let event = LoanExtended {
    loan_id: loan.loan_id,  // ムーブ後のアクセス
    // ...
};
```

**修正後:**
```rust
// 必要な値を先に取得
let loan_id = loan.loan_id;
let old_due_date = loan.due_date;

let new_loan = ActiveLoan {
    core: LoanCore {
        ..loan.core  // ここでムーブ
    },
};

let event = LoanExtended {
    loan_id,  // ローカル変数を使用
    old_due_date,
    // ...
};
```

**利点:**
1. **明示的**: 必要な値を事前に取得していることが明確
2. **Copy非依存**: `Copy`がなくても動作する（`.clone()`に変えればOK）
3. **保守性**: 将来の型変更に強い
4. **意図が明確**: コードレビュアーが理解しやすい

---

## ID型にCopyを実装する判断

### RustにおけるID型の実装パターン

#### パターン1: Copyあり（一般的）

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct UserId(u64);

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct SessionId(Uuid);
```

**メリット:**
- 使いやすい
- パフォーマンス（スタックコピー）
- Rustの慣習

**デメリット:**
- ムーブセマンティクスが隠蔽される
- 所有権の意図が不明確になる可能性

#### パターン2: Copyなし（厳格）

```rust
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct UserId(String);

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct SessionId(Uuid);  // Copyできるが、あえて実装しない
```

**メリット:**
- 所有権が明確
- ムーブが明示的
- use-after-moveがコンパイルエラーになる

**デメリット:**
- `&id`や`.clone()`を多用
- やや冗長

### 判断基準

| 条件 | Copy推奨 | Copy非推奨 |
|---|---|---|
| **サイズ** | ≤16バイト | >16バイト |
| **ベース型** | `u32`, `u64`, `Uuid` | `String`, `Vec` |
| **チーム経験** | 高い | 初心者多い |
| **プロジェクト規模** | 小〜中規模 | 大規模 |
| **将来の拡張性** | 単純なID | メタデータ追加予定 |

### このプロジェクトの判断

**結論: Copyあり（現状維持）**

理由:
1. ✅ UUIDベース（16バイト）で技術的に妥当
2. ✅ Rustの一般的な慣習に沿う
3. ✅ 実用的で簡潔
4. ✅ 今回の修正で明示的なパターンを採用済み

ただし:
- **Copyに暗黙的に依存しない**
- **必要な値を事前に取得する**パターンを守る
- ドキュメント化する

---

## ベストプラクティス

### 1. ムーブ前に値を取得

```rust
// ✅ Good: 明示的
let id = loan.loan_id;
let due_date = loan.due_date;

let new_loan = ActiveLoan {
    core: LoanCore {
        ..loan.core  // ムーブ
    },
};

let event = LoanExtended {
    loan_id: id,  // ローカル変数
    // ...
};
```

```rust
// ❌ Bad: Copyに暗黙的に依存
let new_loan = ActiveLoan {
    core: LoanCore {
        ..loan.core  // ムーブ
    },
};

let event = LoanExtended {
    loan_id: loan.loan_id,  // ムーブ後のアクセス
    // ...
};
```

### 2. ExtensionCountのような小さな値型

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ExtensionCount(u8);

impl ExtensionCount {
    // self を所有権で受け取る（Copyなので自然）
    pub fn increment(self) -> Result<Self, ExtensionError> {
        if self.0 >= 1 {
            return Err(ExtensionError::LimitExceeded);
        }
        Ok(Self(self.0 + 1))
    }
}
```

**これは完璧な設計:**
- `Copy`により関数型スタイルが自然に書ける
- イミュータブルで副作用なし
- 新しい値を返す

### 3. ドキュメント化

```rust
/// 貸出ID
///
/// # Copy実装について
///
/// この型は`Copy`を実装しています：
/// - UUIDベース（16バイト）でコピーが安価
/// - 識別子として値型の性質を持つ
/// - Rustの一般的な慣習に従う
///
/// ## 注意事項
///
/// `Copy`に暗黙的に依存せず、必要な箇所で明示的に値を取得してください：
///
/// ```rust
/// // ✅ Good
/// let loan_id = loan.loan_id;
/// let new_loan = create_new_loan(loan); // loan をムーブ
/// use_id(loan_id); // ローカル変数を使用
///
/// // ❌ Bad
/// let new_loan = create_new_loan(loan); // loan をムーブ
/// use_id(loan.loan_id); // ムーブ後のアクセス（Copyで動くが不適切）
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LoanId(Uuid);
```

### 4. Deref実装との組み合わせ

```rust
impl std::ops::Deref for ActiveLoan {
    type Target = LoanCore;
    fn deref(&self) -> &Self::Target {
        &self.core
    }
}
```

`Deref`がある場合、以下に注意：
- `loan.loan_id`は実際には`loan.core.loan_id`へのアクセス
- `loan.core`をムーブした後、`loan.loan_id`は技術的にアクセスできない
- `LoanId`が`Copy`なので偶然動く

**ベストプラクティス:**
```rust
// Deref経由でアクセスする値も、事前に取得
let loan_id = loan.loan_id;  // Deref経由
let book_id = loan.book_id;  // Deref経由

// その後で core をムーブ
let new_loan = ActiveLoan {
    core: LoanCore {
        ..loan.core
    },
};
```

---

## プロジェクト全体でのCopy実装

### Copy実装箇所の一覧

| 型 | サイズ | Copy実装 | 評価 |
|---|---|---|---|
| `LoanId` | 16バイト | ✅ | 妥当 |
| `BookId` | 16バイト | ✅ | 妥当 |
| `MemberId` | 16バイト | ✅ | 妥当 |
| `StaffId` | 16バイト | ✅ | 妥当 |
| `ExtensionCount` | 1バイト | ✅ | **非常に適切** |
| `LoanStatus` | 1バイト | ✅ | 一時的（Task 1.1dで削除予定） |

### 特に優れた設計: ExtensionCount

```rust
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExtensionCount(u8);

impl ExtensionCount {
    pub fn increment(self) -> Result<Self, ExtensionError> {
        //          ^^^^ 所有権で受け取る（Copyなので自然）
        if self.0 >= 1 {
            return Err(ExtensionError::LimitExceeded);
        }
        Ok(Self(self.0 + 1))  // 新しい値を返す（イミュータブル）
    }
}
```

**この設計が優れている理由:**
1. **関数型プログラミングスタイル** - イミュータブル
2. **副作用なし** - 純粋関数
3. **Copy活用** - 値渡しが自然に書ける
4. **型安全** - ビジネスルール（0または1のみ）を型で強制

---

## 参考資料

### Rustの公式ドキュメント

- [The Rust Programming Language - Ownership](https://doc.rust-lang.org/book/ch04-00-understanding-ownership.html)
- [The Copy Trait](https://doc.rust-lang.org/std/marker/trait.Copy.html)
- [The Clone Trait](https://doc.rust-lang.org/std/clone/trait.Clone.html)

### 関連するRustコミュニティの議論

- [When should I implement Copy?](https://stackoverflow.com/questions/31012923/when-is-it-appropriate-to-implement-copy)
- [Rust API Guidelines - C-COPY](https://rust-lang.github.io/api-guidelines/interoperability.html#types-eagerly-implement-common-traits-c-common-traits)

### このプロジェクトの関連ドキュメント

- [02_domain_model.md](../02_domain_model.md) - ドメインモデル設計
- [05_implementation.md](../05_implementation.md) - 実装ガイド
- [Task 1.1b実装](../phase/tasks/01_1_loan_state_refactor.md) - 今回のタスク

---

## まとめ

### 重要な学び

1. **Copyトレイトは便利だが、隠蔽される問題がある**
   - use-after-moveがコンパイルエラーにならない
   - 暗黙の依存が生まれる

2. **明示的なパターンを採用すべき**
   - ムーブ前に必要な値を取得
   - Copyに依存しないコードを書く

3. **小さな値型（ExtensionCount）ではCopyが最適**
   - 関数型スタイルが自然に書ける
   - イミュータブルな設計に適している

4. **設計判断はプロジェクトごとに異なる**
   - 「作法」ではなく「設計判断」
   - チーム、規模、将来性を考慮

### アクションアイテム

- ✅ ID型の`Copy`は現状維持
- ✅ 明示的な値取得パターンを採用
- ✅ ドキュメント化（この文書）
- 📝 将来（Phase 2以降）で再評価

---

**作成者:** Claude Code (Anthropic)
**レビュー:** CodeRabbit AI Review
