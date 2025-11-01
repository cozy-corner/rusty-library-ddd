use crate::domain::{self, DomainEvent, commands::*, value_objects::*};
use crate::ports::*;
use std::sync::Arc;

use super::errors::{LoanApplicationError, Result};

/// 会員1人あたりの最大貸出冊数
const MAX_ACTIVE_LOANS: usize = 5;

/// サービスの依存関係
///
/// 関数型DDDの原則に従い、データ構造として定義。
/// 振る舞い（メソッド）は持たず、純粋な関数に依存関係を渡す。
///
/// このパターンにより：
/// - すべての依存が明示的
/// - データと振る舞いの分離
/// - 関数合成が容易
/// - テストが明確
#[derive(Clone)]
#[allow(dead_code)]
pub struct ServiceDependencies {
    pub event_store: Arc<dyn EventStore>,
    pub loan_read_model: Arc<dyn LoanReadModel>,
    pub member_service: Arc<dyn MemberService>,
    pub book_service: Arc<dyn BookService>,
}

/// イベントストアから貸出集約を復元するヘルパー関数
///
/// extend_loan, return_book, overdue_detectionで共通利用される。
///
/// # 引数
/// * `event_store` - イベントストア
/// * `loan_id` - 貸出ID
///
/// # 戻り値
/// 復元された貸出集約
///
/// # エラー
/// - EventStoreError: イベント読み込み失敗
/// - LoanNotFound: イベントが存在しない、または復元に失敗
async fn load_loan(
    event_store: &Arc<dyn EventStore>,
    loan_id: LoanId,
) -> Result<domain::loan::Loan> {
    let events = event_store
        .load(loan_id)
        .await
        .map_err(LoanApplicationError::EventStoreError)?;

    domain::loan::replay_events(&events).ok_or(LoanApplicationError::LoanNotFound)
}

/// 貸出集約からRead Model用のビューを構築するヘルパー関数
///
/// イベントソーシングの原則に従い、集約の完全な状態を
/// Read Modelのビューとして変換する。
///
/// # 引数
/// * `loan` - 貸出集約（Active/Overdue/Returned）
///
/// # 戻り値
/// Read Model用の完全な貸出ビュー
pub(super) fn build_loan_view(loan: &domain::loan::Loan) -> LoanView {
    match loan {
        domain::loan::Loan::Active(active) => LoanView {
            loan_id: active.loan_id,
            book_id: active.book_id,
            member_id: active.member_id,
            loaned_at: active.loaned_at,
            due_date: active.due_date,
            returned_at: None,
            extension_count: active.extension_count.value(),
            status: LoanStatus::Active,
            created_at: active.created_at,
            updated_at: active.updated_at,
        },
        domain::loan::Loan::Overdue(overdue) => LoanView {
            loan_id: overdue.loan_id,
            book_id: overdue.book_id,
            member_id: overdue.member_id,
            loaned_at: overdue.loaned_at,
            due_date: overdue.due_date,
            returned_at: None,
            extension_count: overdue.extension_count.value(),
            status: LoanStatus::Overdue,
            created_at: overdue.created_at,
            updated_at: overdue.updated_at,
        },
        domain::loan::Loan::Returned(returned) => LoanView {
            loan_id: returned.loan_id,
            book_id: returned.book_id,
            member_id: returned.member_id,
            loaned_at: returned.loaned_at,
            due_date: returned.due_date,
            returned_at: Some(returned.returned_at),
            extension_count: returned.extension_count.value(),
            status: LoanStatus::Returned,
            created_at: returned.created_at,
            updated_at: returned.updated_at,
        },
    }
}

/// 書籍を貸し出す（純粋な関数）
///
/// ビジネスルール：
/// - 会員が存在すること
/// - 書籍が貸出可能であること
/// - 会員に延滞中の貸出がないこと
/// - 会員の貸出中の冊数が5冊未満であること
///
/// すべての依存が引数として明示的に渡される（関数型の原則）。
///
/// # 一貫性保証
///
/// この関数は**結果整合性（Eventual Consistency）**を提供します。
///
/// - EventStore（書き込み）とReadModel（読み取り）は独立して更新されます
/// - ReadModel更新がEventStore保存後に失敗した場合、一時的に不整合が発生します
/// - 将来の拡張（Phase 5以降）でイベントプロジェクションワーカーによる自動修復を予定
///
/// # 冪等性
///
/// **警告**: この関数は冪等ではありません。重複した呼び出しは重複イベントを生成します。
/// 将来の拡張で冪等キーによる重複検出を予定しています。
///
/// # 引数
/// * `deps` - サービスの依存関係
/// * `cmd` - 貸出コマンド
///
/// # 戻り値
/// 成功時は作成された貸出のID
#[allow(dead_code)]
pub async fn loan_book(deps: &ServiceDependencies, cmd: LoanBook) -> Result<LoanId> {
    // 1. 会員の存在確認
    let member_exists = deps
        .member_service
        .exists(cmd.member_id)
        .await
        .map_err(LoanApplicationError::MemberServiceError)?;

    if !member_exists {
        return Err(LoanApplicationError::MemberNotFound);
    }

    // 2. 書籍の貸出可能性確認
    let book_available = deps
        .book_service
        .is_available_for_loan(cmd.book_id)
        .await
        .map_err(LoanApplicationError::BookServiceError)?;

    if !book_available {
        return Err(LoanApplicationError::BookNotAvailable);
    }

    // 3. 会員の延滞確認
    let has_overdue = deps
        .member_service
        .has_overdue_loans(cmd.member_id)
        .await
        .map_err(LoanApplicationError::MemberServiceError)?;

    if has_overdue {
        return Err(LoanApplicationError::MemberHasOverdueLoan);
    }

    // 4. 貸出上限確認（5冊まで）
    let active_loans = deps
        .loan_read_model
        .get_active_loans_for_member(cmd.member_id)
        .await
        .map_err(LoanApplicationError::ReadModelError)?;

    if active_loans.len() >= MAX_ACTIVE_LOANS {
        return Err(LoanApplicationError::LoanLimitExceeded);
    }

    // 5. ドメイン層の純粋関数を呼び出し
    let (active_loan, event) =
        domain::loan::loan_book(cmd.book_id, cmd.member_id, cmd.loaned_at, cmd.staff_id)
            .map_err(|e| LoanApplicationError::DomainError(format!("{:?}", e)))?;

    let loan_id = active_loan.loan_id;

    // 6. イベントストアに保存
    deps.event_store
        .append(loan_id, vec![DomainEvent::BookLoaned(event.clone())])
        .await
        .map_err(LoanApplicationError::EventStoreError)?;

    // 7. Read Modelを更新（完全な状態を保存）
    let loan_view = build_loan_view(&domain::loan::Loan::Active(active_loan));
    deps.loan_read_model
        .save(loan_view)
        .await
        .map_err(LoanApplicationError::ReadModelError)?;

    Ok(loan_id)
}

/// 貸出を延長する（純粋な関数）
///
/// ビジネスルール：
/// - 貸出が存在すること
/// - 貸出がActive状態であること（Overdue, Returnedは延長不可）
/// - 延長回数が上限（1回）に達していないこと
///
/// すべての依存が引数として明示的に渡される（関数型の原則）。
///
/// # 一貫性保証
///
/// 結果整合性を提供。詳細は`loan_book()`を参照。
///
/// # 引数
/// * `deps` - サービスの依存関係
/// * `cmd` - 延長コマンド
#[allow(dead_code)]
pub async fn extend_loan(deps: &ServiceDependencies, cmd: ExtendLoan) -> Result<()> {
    // 1. イベントストアから貸出集約を復元
    let loan = load_loan(&deps.event_store, cmd.loan_id).await?;

    // 2. ActiveLoanであることを確認
    let active_loan = match loan {
        domain::loan::Loan::Active(active) => active,
        domain::loan::Loan::Overdue(_) => {
            return Err(LoanApplicationError::InvalidLoanState(
                "Cannot extend overdue loan".to_string(),
            ));
        }
        domain::loan::Loan::Returned(_) => {
            return Err(LoanApplicationError::InvalidLoanState(
                "Cannot extend returned loan".to_string(),
            ));
        }
    };

    // 3. ドメイン層の純粋関数を呼び出し
    let (updated_loan, event) = domain::loan::extend_loan(active_loan, cmd.extended_at)
        .map_err(|e| LoanApplicationError::DomainError(format!("{:?}", e)))?;

    // 4. イベントストアに保存
    deps.event_store
        .append(cmd.loan_id, vec![DomainEvent::LoanExtended(event.clone())])
        .await
        .map_err(LoanApplicationError::EventStoreError)?;

    // 5. Read Modelを更新（完全な状態を保存）
    let loan_view = build_loan_view(&domain::loan::Loan::Active(updated_loan));
    deps.loan_read_model
        .save(loan_view)
        .await
        .map_err(LoanApplicationError::ReadModelError)?;

    Ok(())
}

/// 書籍を返却する（純粋な関数）
///
/// ビジネスルール：
/// - 貸出が存在すること
/// - 貸出がActive, Overdue状態であること（Returnedは返却不可）
/// - 延滞していても返却は受け付ける（公立図書館のため延滞料金なし）
///
/// すべての依存が引数として明示的に渡される（関数型の原則）。
///
/// # 一貫性保証
///
/// 結果整合性を提供。詳細は`loan_book()`を参照。
///
/// # 引数
/// * `deps` - サービスの依存関係
/// * `cmd` - 返却コマンド
#[allow(dead_code)]
pub async fn return_book(deps: &ServiceDependencies, cmd: ReturnBook) -> Result<()> {
    // 1. イベントストアから貸出集約を復元
    let loan = load_loan(&deps.event_store, cmd.loan_id).await?;

    // 2. ドメイン層の純粋関数を呼び出し
    let (returned_loan, event) = domain::loan::return_book(loan, cmd.returned_at)
        .map_err(|e| LoanApplicationError::DomainError(format!("{:?}", e)))?;

    // 3. イベントストアに保存
    deps.event_store
        .append(cmd.loan_id, vec![DomainEvent::BookReturned(event.clone())])
        .await
        .map_err(LoanApplicationError::EventStoreError)?;

    // 4. Read Modelを更新（完全な状態を保存）
    let loan_view = build_loan_view(&domain::loan::Loan::Returned(returned_loan));
    deps.loan_read_model
        .save(loan_view)
        .await
        .map_err(LoanApplicationError::ReadModelError)?;

    Ok(())
}
