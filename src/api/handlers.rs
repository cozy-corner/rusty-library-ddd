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

/// Application state shared across handlers
#[derive(Clone)]
pub struct AppState {
    pub service_deps: ServiceDependencies,
}

/// POST /loans - Create a new loan
///
/// Creates a new book loan for a member.
///
/// Business rules enforced:
/// - Member must exist
/// - Book must be available
/// - Member must not have overdue loans
/// - Member must not exceed loan limit (5 books)
pub async fn create_loan(
    State(state): State<Arc<AppState>>,
    Json(req): Json<LoanBookRequest>,
) -> Result<(StatusCode, Json<LoanCreatedResponse>), ApiError> {
    let cmd = req.to_command();

    let loan_id = execute_loan_book(&state.service_deps, cmd.clone()).await?;

    // Fetch the created loan to return complete information
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

/// POST /loans/:id/extend - Extend a loan
///
/// Extends the loan period by 2 weeks.
///
/// Business rules enforced:
/// - Loan must exist
/// - Loan must be in Active state (not Overdue or Returned)
/// - Extension count must be less than 1 (max 1 extension allowed)
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

    // Fetch the updated loan to return new information
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

/// POST /loans/:id/return - Return a book
///
/// Processes the return of a loaned book.
///
/// Business rules enforced:
/// - Loan must exist
/// - Loan must not already be returned
/// - Overdue loans can still be returned (no late fees for public libraries)
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

    // Fetch the updated loan to confirm return
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
