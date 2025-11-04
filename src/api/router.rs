use axum::{
    Router,
    routing::{get, post},
};
use std::sync::Arc;
use tower_http::trace::TraceLayer;

use super::handlers::{AppState, create_loan, extend_loan, return_book};

/// 貸出管理の全エンドポイントを持つAPIルーターを作成
///
/// コマンドエンドポイント（Write操作）:
/// - POST /loans - 新しい貸出を作成
/// - POST /loans/:id/extend - 貸出を延長
/// - POST /loans/:id/return - 書籍を返却
///
/// 将来のクエリエンドポイント（Read操作 - Task 6.2）:
/// - GET /loans - フィルタ付き貸出一覧
/// - GET /loans/:id - 貸出詳細
pub fn create_router(state: Arc<AppState>) -> Router {
    Router::new()
        // ヘルスチェックエンドポイント
        .route("/health", get(health_check))
        // コマンドエンドポイント（Write操作）
        .route("/loans", post(create_loan))
        .route("/loans/:id/extend", post(extend_loan))
        .route("/loans/:id/return", post(return_book))
        // トレーシングミドルウェアを追加
        .layer(TraceLayer::new_for_http())
        // アプリケーション状態を追加
        .with_state(state)
}

/// ヘルスチェックエンドポイント
async fn health_check() -> &'static str {
    "OK"
}
