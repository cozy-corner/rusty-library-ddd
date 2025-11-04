use crate::application::loan::{
    LoanApplicationError, ServiceDependencies, extend_loan as execute_extend_loan,
    loan_book as execute_loan_book, return_book as execute_return_book,
};
use crate::domain::value_objects::LoanId;
use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
};
use std::sync::Arc;
use uuid::Uuid;

use super::{
    error::ApiError,
    types::{BookReturnedResponse, LoanBookRequest, LoanCreatedResponse, LoanExtendedResponse},
};

/// ハンドラー間で共有されるアプリケーション状態
#[derive(Clone)]
pub struct AppState {
    pub service_deps: ServiceDependencies,
}

/// POST /loans - 新しい貸出を作成
///
/// 会員への書籍の貸出を作成する。
///
/// 強制されるビジネスルール:
/// - 会員が存在すること
/// - 書籍が貸出可能であること
/// - 会員に延滞中の貸出がないこと
/// - 会員の貸出数が上限（5冊）を超えないこと
pub async fn create_loan(
    State(state): State<Arc<AppState>>,
    Json(req): Json<LoanBookRequest>,
) -> Result<(StatusCode, Json<LoanCreatedResponse>), ApiError> {
    let cmd = req.to_command();

    let loan_id = execute_loan_book(&state.service_deps, cmd.clone()).await?;

    // 作成された貸出を取得して完全な情報を返す
    let loan_view = state
        .service_deps
        .loan_read_model
        .get_by_id(loan_id)
        .await
        .map_err(|e| ApiError::from(LoanApplicationError::ReadModelError(e)))?
        .ok_or_else(|| ApiError::from(LoanApplicationError::LoanNotFound))?;

    let response = LoanCreatedResponse {
        loan_id: loan_id.value(),
        book_id: cmd.book_id.value(),
        member_id: cmd.member_id.value(),
        loaned_at: loan_view.loaned_at,
        due_date: loan_view.due_date,
    };

    Ok((StatusCode::CREATED, Json(response)))
}

/// POST /loans/:id/extend - 貸出を延長
///
/// 貸出期間を2週間延長する。
///
/// 強制されるビジネスルール:
/// - 貸出が存在すること
/// - 貸出がActive状態であること（OverdueまたはReturnedでないこと）
/// - 延長回数が1未満であること（最大1回まで延長可能）
pub async fn extend_loan(
    State(state): State<Arc<AppState>>,
    Path(loan_id): Path<Uuid>,
) -> Result<(StatusCode, Json<LoanExtendedResponse>), ApiError> {
    let loan_id = LoanId::from_uuid(loan_id);

    let cmd = crate::domain::commands::ExtendLoan {
        loan_id,
        extended_at: chrono::Utc::now(),
    };

    execute_extend_loan(&state.service_deps, cmd).await?;

    // 更新された貸出を取得して新しい情報を返す
    let loan_view = state
        .service_deps
        .loan_read_model
        .get_by_id(loan_id)
        .await
        .map_err(|e| ApiError::from(LoanApplicationError::ReadModelError(e)))?
        .ok_or_else(|| ApiError::from(LoanApplicationError::LoanNotFound))?;

    let response = LoanExtendedResponse {
        loan_id: loan_id.value(),
        new_due_date: loan_view.due_date,
        extension_count: loan_view.extension_count,
    };

    Ok((StatusCode::OK, Json(response)))
}

/// POST /loans/:id/return - 書籍を返却
///
/// 貸出中の書籍の返却を処理する。
///
/// 強制されるビジネスルール:
/// - 貸出が存在すること
/// - 既に返却済みでないこと
/// - 延滞中の貸出も返却可能（公立図書館のため延滞料金なし）
pub async fn return_book(
    State(state): State<Arc<AppState>>,
    Path(loan_id): Path<Uuid>,
) -> Result<(StatusCode, Json<BookReturnedResponse>), ApiError> {
    let loan_id = LoanId::from_uuid(loan_id);

    let cmd = crate::domain::commands::ReturnBook {
        loan_id,
        returned_at: chrono::Utc::now(),
    };

    execute_return_book(&state.service_deps, cmd).await?;

    // 更新された貸出を取得して返却を確認
    let loan_view = state
        .service_deps
        .loan_read_model
        .get_by_id(loan_id)
        .await
        .map_err(|e| ApiError::from(LoanApplicationError::ReadModelError(e)))?
        .ok_or_else(|| ApiError::from(LoanApplicationError::LoanNotFound))?;

    let response = BookReturnedResponse {
        loan_id: loan_id.value(),
        returned_at: loan_view.returned_at.unwrap_or_else(chrono::Utc::now),
    };

    Ok((StatusCode::OK, Json(response)))
}
