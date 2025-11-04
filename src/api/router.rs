use axum::{
    Router,
    routing::{get, post},
};
use std::sync::Arc;
use tower_http::trace::TraceLayer;

use super::handlers::{AppState, create_loan, extend_loan, return_book};

/// Creates the API router with all loan management endpoints
///
/// Command endpoints (Write operations):
/// - POST /loans - Create a new loan
/// - POST /loans/:id/extend - Extend a loan
/// - POST /loans/:id/return - Return a book
///
/// Future query endpoints (Read operations - Task 6.2):
/// - GET /loans - List loans with filters
/// - GET /loans/:id - Get loan details
pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        // Health check endpoint
        .route("/health", get(health_check))
        // Command endpoints (Write operations)
        .route("/loans", post(create_loan))
        .route("/loans/:id/extend", post(extend_loan))
        .route("/loans/:id/return", post(return_book))
        // Add tracing middleware
        .layer(TraceLayer::new_for_http())
        // Add application state
        .with_state(state)
}

/// Health check endpoint
async fn health_check() -> &'static str {
    "OK"
}
