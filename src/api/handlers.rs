use crate::application::loan::{
    LoanApplicationError, ServiceDependencies, extend_loan as execute_extend_loan,
    loan_book as execute_loan_book, return_book as execute_return_book,
};
use crate::domain::value_objects::{LoanId, MemberId};
use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use std::sync::Arc;
use uuid::Uuid;

use super::{
    error::ApiError,
    types::{
        BookReturnedResponse, ListLoansQuery, LoanBookRequest, LoanCreatedResponse,
        LoanExtendedResponse, LoanResponse,
    },
};

// ============================================================================
// State
// ============================================================================

/// ハンドラー間で共有されるアプリケーション状態
#[derive(Clone)]
pub struct AppState {
    pub service_deps: ServiceDependencies,
}

// ============================================================================
// Command handlers (POST)
// ============================================================================

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

// ============================================================================
// Query handlers (GET)
// ============================================================================

/// GET /loans/:id - 貸出詳細をIDで取得
///
/// 見つかった場合は貸出情報を返し、見つからない場合は404を返す。
pub async fn get_loan_by_id(
    State(state): State<Arc<AppState>>,
    Path(loan_id): Path<Uuid>,
) -> Result<Json<LoanResponse>, QueryError> {
    let loan_id = LoanId::from_uuid(loan_id);

    match state.service_deps.loan_read_model.get_by_id(loan_id).await {
        Ok(Some(loan_view)) => Ok(Json(LoanResponse::from(loan_view))),
        Ok(None) => Err(QueryError::NotFound(format!(
            "Loan {} not found",
            loan_id.value()
        ))),
        Err(e) => Err(QueryError::InternalError(e.to_string())),
    }
}

/// GET /loans - オプションフィルタ付き貸出一覧取得
///
/// クエリパラメータ:
/// - member_id: 会員IDでフィルタリング（必須）
/// - status: ステータスでフィルタリング（active, overdue, returned）（オプション）
///
/// フィルタが指定されない場合は、会員の全貸出を返す。
/// 現在はmember_idパラメータが必須。
pub async fn list_loans(
    State(state): State<Arc<AppState>>,
    Query(query): Query<ListLoansQuery>,
) -> Result<Json<Vec<LoanResponse>>, QueryError> {
    // member_idを必須とする
    let member_id = query.member_id.ok_or_else(|| {
        QueryError::BadRequest("member_id query parameter is required".to_string())
    })?;

    let member_id = MemberId::from_uuid(member_id);

    // 会員の貸出を取得
    let loans = state
        .service_deps
        .loan_read_model
        .find_by_member_id(member_id)
        .await
        .map_err(|e| QueryError::InternalError(e.to_string()))?;

    // ステータスフィルタが指定されている場合は適用
    let filtered_loans: Vec<LoanResponse> = if let Some(status_str) = &query.status {
        let status =
            super::types::parse_status_filter(status_str).map_err(QueryError::BadRequest)?;

        loans
            .into_iter()
            .filter(|loan| loan.status == status)
            .map(LoanResponse::from)
            .collect()
    } else {
        loans.into_iter().map(LoanResponse::from).collect()
    };

    Ok(Json(filtered_loans))
}

// ============================================================================
// Error types
// ============================================================================

/// クエリハンドラー用のエラー型
#[derive(Debug)]
pub enum QueryError {
    NotFound(String),
    BadRequest(String),
    InternalError(String),
}

impl IntoResponse for QueryError {
    fn into_response(self) -> Response {
        let (status, error_type, message) = match self {
            QueryError::NotFound(msg) => (StatusCode::NOT_FOUND, "not_found", msg),
            QueryError::BadRequest(msg) => (StatusCode::BAD_REQUEST, "bad_request", msg),
            QueryError::InternalError(msg) => {
                // 内部エラーの詳細はログに記録し、クライアントには一般的なメッセージのみを返す
                tracing::error!("Internal error in query handler: {}", msg);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "internal_error",
                    "An unexpected error occurred".to_string(),
                )
            }
        };

        let body = Json(super::types::ErrorResponse::new(error_type, message));
        (status, body).into_response()
    }
}
