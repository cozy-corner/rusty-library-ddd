use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Response},
};
use std::sync::Arc;
use uuid::Uuid;

use crate::domain::value_objects::{LoanId, MemberId};
use crate::ports::loan_read_model::LoanReadModel;

use super::types::{ErrorResponse, ListLoansQuery, LoanResponse};

/// APIハンドラーの共有状態
///
/// クエリハンドラーに必要な依存関係を保持する。
#[derive(Clone)]
pub struct ApiState {
    pub loan_read_model: Arc<dyn LoanReadModel>,
}

/// GET /loans/:id - 貸出詳細をIDで取得
///
/// 見つかった場合は貸出情報を返し、見つからない場合は404を返す。
pub async fn get_loan_by_id(
    State(state): State<ApiState>,
    Path(loan_id): Path<Uuid>,
) -> Result<Json<LoanResponse>, AppError> {
    let loan_id = LoanId::from_uuid(loan_id);

    match state.loan_read_model.get_by_id(loan_id).await {
        Ok(Some(loan_view)) => Ok(Json(LoanResponse::from(loan_view))),
        Ok(None) => Err(AppError::NotFound(format!(
            "Loan {} not found",
            loan_id.value()
        ))),
        Err(e) => Err(AppError::InternalError(e.to_string())),
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
    State(state): State<ApiState>,
    Query(query): Query<ListLoansQuery>,
) -> Result<Json<Vec<LoanResponse>>, AppError> {
    // member_idを必須とする
    let member_id = query
        .member_id
        .ok_or_else(|| AppError::BadRequest("member_id query parameter is required".to_string()))?;

    let member_id = MemberId::from_uuid(member_id);

    // 会員の貸出を取得
    let loans = state
        .loan_read_model
        .find_by_member_id(member_id)
        .await
        .map_err(|e| AppError::InternalError(e.to_string()))?;

    // ステータスフィルタが指定されている場合は適用
    let filtered_loans: Vec<LoanResponse> = if let Some(status_str) = &query.status {
        let status = super::types::parse_status_filter(status_str).map_err(AppError::BadRequest)?;

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

/// APIハンドラー用のアプリケーションエラー型
#[derive(Debug)]
pub enum AppError {
    NotFound(String),
    BadRequest(String),
    InternalError(String),
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        let (status, error_message) = match self {
            AppError::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            AppError::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            AppError::InternalError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
        };

        let body = Json(ErrorResponse::new(error_message));
        (status, body).into_response()
    }
}
