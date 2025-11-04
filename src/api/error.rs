use crate::application::loan::LoanApplicationError;
use axum::{
    Json,
    http::StatusCode,
    response::{IntoResponse, Response},
};

use super::types::ErrorResponse;

/// API層のエラー型
///
/// アプリケーション層のエラーをラップし、HTTPレスポンスへのマッピングを提供する。
#[derive(Debug)]
pub struct ApiError(LoanApplicationError);

impl From<LoanApplicationError> for ApiError {
    fn from(err: LoanApplicationError) -> Self {
        ApiError(err)
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        let (status, error_type, message) = match self.0 {
            // 404 Not Found - リクエストされたリソースが存在しない
            LoanApplicationError::LoanNotFound => {
                (StatusCode::NOT_FOUND, "LOAN_NOT_FOUND", "Loan not found")
            }

            // 422 Unprocessable Entity - ビジネスルール違反
            LoanApplicationError::MemberNotFound => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "MEMBER_NOT_FOUND",
                "Member not found",
            ),
            LoanApplicationError::BookNotAvailable => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "BOOK_NOT_AVAILABLE",
                "Book is not available for loan",
            ),
            LoanApplicationError::MemberHasOverdueLoan => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "MEMBER_HAS_OVERDUE_LOAN",
                "Member has overdue loan and cannot borrow more books",
            ),
            LoanApplicationError::LoanLimitExceeded => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "LOAN_LIMIT_EXCEEDED",
                "Loan limit exceeded (max 5 books per member)",
            ),
            LoanApplicationError::InvalidLoanState(ref msg) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "INVALID_LOAN_STATE",
                msg.as_str(),
            ),
            LoanApplicationError::DomainError(ref msg) => (
                StatusCode::UNPROCESSABLE_ENTITY,
                "DOMAIN_ERROR",
                msg.as_str(),
            ),

            // 500 Internal Server Error - システム障害
            // 内部エラーの詳細はログに記録し、クライアントには一般的なメッセージのみを返す
            LoanApplicationError::EventStoreError(ref e) => {
                tracing::error!("Event store error: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "EVENT_STORE_ERROR",
                    "Failed to store event",
                )
            }
            LoanApplicationError::ReadModelError(ref e) => {
                tracing::error!("Read model error: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "READ_MODEL_ERROR",
                    "Failed to update read model",
                )
            }
            LoanApplicationError::MemberServiceError(ref e) => {
                tracing::error!("Member service error: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "MEMBER_SERVICE_ERROR",
                    "Member service error",
                )
            }
            LoanApplicationError::BookServiceError(ref e) => {
                tracing::error!("Book service error: {}", e);
                (
                    StatusCode::INTERNAL_SERVER_ERROR,
                    "BOOK_SERVICE_ERROR",
                    "Book service error",
                )
            }
        };

        let body = Json(ErrorResponse::new(error_type, message));
        (status, body).into_response()
    }
}
