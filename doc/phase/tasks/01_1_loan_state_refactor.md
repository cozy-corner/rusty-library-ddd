# Task 1.1: Loanを型安全な状態パターンにリファクタリング

## 概要

Task 1で実装したLoan集約を、より型安全な状態パターンにリファクタリングする。
`LoanStatus` enumではなく、型で状態を表現することで、不正な状態を型システムで排除する。

## 背景・動機

### 現在の問題

```rust
pub struct Loan {
    pub status: LoanStatus,
    pub returned_at: Option<DateTime<Utc>>,
    // ...
}
```

**問題点：**
- `status = Returned` かつ `returned_at = None` という不正な状態が作れる
- 型システムがビジネスルールを強制できない
- コンパイル時に不正な状態遷移を検出できない

### 目標設計

```rust
/// 共通フィールド
pub struct LoanCore { ... }

/// 貸出中
pub struct ActiveLoan { pub core: LoanCore }

/// 延滞中
pub struct OverdueLoan { pub core: LoanCore }

/// 返却済み（returned_atが必須！）
pub struct ReturnedLoan {
    pub core: LoanCore,
    pub returned_at: DateTime<Utc>,  // 型で保証
}

/// 貸出の統合型
pub enum Loan {
    Active(ActiveLoan),
    Overdue(OverdueLoan),
    Returned(ReturnedLoan),
}
```

**型安全性の改善：**
- ✅ `ReturnedLoan`は必ず`returned_at`を持つ（型で保証）
- ✅ `extend_loan()`は`ActiveLoan`しか受け付けない（コンパイルエラーで防止）
- ✅ 状態遷移が明示的（パターンマッチで全ケース網羅）
- ✅ 不正な状態を作れない

## リファクタリング戦略

### 段階的アプローチ

Task 1.1を4つのサブタスクに分割する：
- **1.1a**: 型定義の追加（既存コードと共存、破壊的変更なし）
- **1.1b**: 純粋関数の移行（新旧関数が共存）
- **1.1c**: イベントソーシングの移行（全面移行）
- **1.1d**: クリーンアップ（旧実装削除）

各サブタスクは独立したPRとし、段階的にマージする。

---

## Task 1.1a: 型定義の追加

**ブランチ:** `vk/47bc-task-1-1a-type-definitions`

**スコープ:** 新しい型定義のみ追加。既存コードは一切変更しない。

### 実装内容

- [ ] `LoanCore` structを定義
  - loan_id, book_id, member_id, loaned_at, due_date, extension_count, created_by, created_at, updated_at
- [ ] `ActiveLoan`, `OverdueLoan`, `ReturnedLoan` structを定義
  - 各structは`core: LoanCore`を持つ
  - `ReturnedLoan`のみ`returned_at: DateTime<Utc>`を追加
- [ ] `Deref` traitを実装（3つの状態型すべてに）
  - `loan.loan_id`のように直接アクセス可能に
- [ ] `LoanV2` enumを定義（既存の`Loan`と共存）
  - `Active(ActiveLoan)`, `Overdue(OverdueLoan)`, `Returned(ReturnedLoan)`
- [ ] Serialize/Deserialize実装
  - serde(flatten)でJSONシリアライズを最適化
  - serde(tag = "status")で状態をタグ化

### テスト

- [ ] `ActiveLoan`の基本的な生成と`Deref`動作
- [ ] `OverdueLoan`の基本的な生成と`Deref`動作
- [ ] `ReturnedLoan`の生成（`returned_at`必須）
- [ ] `LoanV2` enumのパターンマッチ
- [ ] Serialize/Deserializeの動作確認

### ファイル

- `src/domain/loan.rs`（行15の前に新しい型を追加）

### 確認ポイント

- [ ] 既存の32テストがすべてpass（変更していないので当然）
- [ ] 新しい型のテストが追加されpass
- [ ] `cargo clippy`で警告なし
- [ ] `cargo fmt --check`でフォーマット済み

### 依存

なし（Task 1完了後、mainから直接分岐）

### 推定時間

30-60分

### 文脈の引き継ぎ

次のTask 1.1bでは、この新しい型を使って純粋関数（`loan_book()`, `extend_loan()`, `return_book()`）を移行する。

---

## Task 1.1b: 純粋関数の移行

**ブランチ:** `vk/47bc-task-1-1b-pure-functions`

**スコープ:** 純粋関数を新しい型に対応させる。既存関数は残したまま、新関数を追加。

### 前提条件

Task 1.1aがマージ済みであること（`LoanCore`, `ActiveLoan`等が定義済み）

### 実装内容

- [ ] `loan_book_v2()` を実装
  - シグネチャ: `Result<(ActiveLoan, BookLoaned), LoanBookError>`
  - `ActiveLoan { core: LoanCore { ... } }`を返す
  - 既存の`loan_book()`は残す
- [ ] `extend_loan_v2()` を実装
  - シグネチャ: `extend_loan_v2(loan: ActiveLoan, ...) -> Result<(ActiveLoan, ...), ...>`
  - **型システムで`ActiveLoan`のみ受け付ける**（Overdue/Returnedはコンパイルエラー）
  - 既存の`extend_loan()`は残す
- [ ] `return_book_v2()` を実装
  - シグネチャ: `return_book_v2(loan: LoanV2, ...) -> Result<(ReturnedLoan, ...), ...>`
  - パターンマッチで`Active`/`Overdue`から`ReturnedLoan`に遷移
  - 既存の`return_book()`は残す
- [ ] `is_overdue_v2()` を実装
  - パターンマッチで状態判定
  ```rust
  match loan {
      LoanV2::Overdue(_) => true,
      LoanV2::Active(a) => now > a.due_date,
      LoanV2::Returned(_) => false,
  }
  ```

### テスト

- [ ] `test_loan_book_v2_*`（4テスト）
  - `ActiveLoan`を返すことを確認
  - `core.due_date`が正しいことを確認
- [ ] `test_extend_loan_v2_*`（4テスト）
  - `ActiveLoan`を引数に取ることを確認
  - 型システムで不正な引数を防止（コンパイルレベル）
- [ ] `test_return_book_v2_*`（3テスト）
  - `ReturnedLoan.returned_at`が必須であることを確認
  - パターンマッチで全ケースカバー
- [ ] `test_is_overdue_v2_*`（3テスト）

### ファイル

- `src/domain/loan.rs`（新関数を追加）

### 確認ポイント

- [ ] 既存の32テストがすべてpass（既存関数は変更していない）
- [ ] 新しい14テストが追加されpass
- [ ] 型システムで不正な呼び出しがコンパイルエラーになることを確認
- [ ] `cargo clippy`で警告なし

### 依存

Task 1.1a完了

### 推定時間

60-90分

### 文脈の引き継ぎ

次のTask 1.1cでは、イベントソーシング関数（`apply_event()`, `replay_events()`）を新しい型に完全移行し、既存関数を置き換える。

---

## Task 1.1c: イベントソーシングの移行

**ブランチ:** `vk/47bc-task-1-1c-event-sourcing`

**スコープ:** イベントソーシング関数を新型に完全移行。既存関数を置き換え。

### 前提条件

Task 1.1bがマージ済みであること（`loan_book_v2()`等が定義済み）

### 実装内容

- [ ] `apply_event()`を完全に書き換え
  - シグネチャ: `apply_event(loan: Option<LoanV2>, event: &DomainEvent) -> LoanV2`
  - パターンマッチで状態遷移を実装
  ```rust
  match (loan, event) {
      (_, DomainEvent::BookLoaned(e)) => LoanV2::Active(ActiveLoan { ... }),
      (Some(LoanV2::Active(a)), DomainEvent::LoanExtended(e)) => LoanV2::Active(...),
      (Some(LoanV2::Active(a) | LoanV2::Overdue(o)), DomainEvent::BookReturned(e)) => LoanV2::Returned(...),
      (Some(LoanV2::Active(a)), DomainEvent::LoanBecameOverdue(e)) => LoanV2::Overdue(...),
      _ => panic!("Invalid state transition"),
  }
  ```
- [ ] `replay_events()`を完全に書き換え
  - シグネチャ: `replay_events(events: &[DomainEvent]) -> Option<LoanV2>`
  - `fold`で`Option<LoanV2>`を蓄積
- [ ] `Loan::empty()`を削除（もう不要）

### テスト

- [ ] 既存の4つの`test_apply_event_*`を**全面的に書き直し**
  - `LoanV2`を返すことを確認
  - 状態遷移パターンマッチのテスト
- [ ] 既存の2つの`test_replay_events_*`を**全面的に書き直し**
  - `Option<LoanV2>`を返すことを確認
  - 完全なライフサイクルテスト

### ファイル

- `src/domain/loan.rs`（`apply_event()`, `replay_events()`を置き換え）

### 確認ポイント

- [ ] 古い`apply_event()`/`replay_events()`のテストがすべて新型に移行
- [ ] 状態遷移の全パターンがカバーされている
- [ ] 不正な状態遷移で`panic!`することを確認
- [ ] `cargo clippy`で警告なし

### 依存

Task 1.1b完了

### 推定時間

90-120分

### 文脈の引き継ぎ

次のTask 1.1dでは、旧実装（`Loan` struct、`loan_book()`等の`_v2`なし関数、`LoanStatus` enum）を完全に削除し、リネームを行う。

---

## Task 1.1d: クリーンアップ

**ブランチ:** `vk/47bc-task-1-1d-cleanup`

**スコープ:** 旧実装の削除とリネーム。最終的な仕上げ。

### 前提条件

Task 1.1cがマージ済みであること（イベントソーシングが新型対応済み）

### 実装内容

- [ ] 旧`Loan` structを削除
- [ ] 旧`loan_book()`, `extend_loan()`, `return_book()`, `is_overdue()`を削除
- [ ] 旧関数のテスト（14個）を削除
- [ ] `LoanV2` → `Loan`にリネーム
- [ ] `*_v2()`関数から`_v2`サフィックスを削除
- [ ] `src/domain/value_objects.rs`から`LoanStatus` enumを削除
- [ ] `LoanStatus`のテスト（3個）を削除
- [ ] importを更新（`use super::LoanStatus`を削除）

### テスト

- [ ] すべてのテストがpass（新型のテストのみ残る）
- [ ] テスト数の確認（32 → 約29テスト: 古いLoanStatus 3個削除）

### ファイル

- `src/domain/loan.rs`（旧実装削除、リネーム）
- `src/domain/value_objects.rs`（`LoanStatus`削除）

### 確認ポイント

- [ ] コードが簡潔になった（冗長な旧実装がない）
- [ ] すべてのテストがpass
- [ ] `cargo clippy --all-targets -- -D warnings`で警告なし
- [ ] `cargo fmt --check`でフォーマット済み
- [ ] ドキュメントコメントが最新

### 依存

Task 1.1c完了

### 推定時間

30分

### 文脈の引き継ぎ

これでTask 1.1（Loanの型安全リファクタリング）が完了。次はTask 2（ポート定義）に進む。

---

## 完成の定義

Task 1.1（全サブタスク）が完成したと言える基準：

### 機能

- [ ] 型システムで不正な状態を表現できない
- [ ] `ReturnedLoan`は必ず`returned_at`を持つ
- [ ] `extend_loan()`は`ActiveLoan`しか受け付けない
- [ ] すべての既存機能が動作する

### テスト

- [ ] 約29テストがすべてpass
- [ ] 型安全性のテストが追加されている
- [ ] 状態遷移の全パターンがカバーされている

### コード品質

- [ ] `cargo clippy`で警告なし
- [ ] `cargo fmt`でフォーマット済み
- [ ] 冗長なコードがない（旧実装削除済み）

### ドキュメント

- [ ] 各structにドキュメントコメントがある
- [ ] 状態遷移が明確に説明されている

---

## 総所要時間見積もり

- Task 1.1a: 30-60分
- Task 1.1b: 60-90分
- Task 1.1c: 90-120分
- Task 1.1d: 30分
- **合計: 3.5-5時間**

---

## 学習ポイント

このリファクタリングを通じて以下を学ぶ：

1. **型でビジネスルールを表現する**（DDDの王道）
2. **コンパイル時の型安全性**（不正な状態を防ぐ）
3. **段階的リファクタリング**（大きな変更を小さなPRに分割）
4. **状態パターン**（enumでの状態管理）
5. **パターンマッチ**（Rustの強力な機能）
