use rusty_library_ddd::{
    adapters::mock::{
        book_service::BookService as MockBookService,
        member_service::MemberService as MockMemberService,
    },
    adapters::postgres::{
        event_store::EventStore as PostgresEventStore,
        loan_read_model::LoanReadModel as PostgresLoanReadModel,
    },
    api::{handlers::AppState, router::create_router},
    application::loan::ServiceDependencies,
};
use std::sync::Arc;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() {
    // トレーシングの初期化
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "rusty_library_ddd=debug,tower_http=debug,axum=trace".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // データベース接続URL
    // 現時点ではプレースホルダー - 実際のDB接続はTask 7（統合）で実装
    let database_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| "postgres://localhost/library".into());

    tracing::info!("Connecting to database...");

    // データベース接続プールの初期化
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to connect to database");

    // アダプターの初期化
    let event_store = Arc::new(PostgresEventStore::new(pool.clone()));
    let loan_read_model = Arc::new(PostgresLoanReadModel::new(pool.clone()));
    let member_service = Arc::new(MockMemberService::new());
    let book_service = Arc::new(MockBookService::new());

    // サービス依存関係の作成
    let service_deps = ServiceDependencies {
        event_store,
        loan_read_model,
        member_service,
        book_service,
    };

    // アプリケーション状態の作成
    let app_state = Arc::new(AppState { service_deps });

    // ルーターの作成
    let app = create_router(app_state);

    // サーバー設定
    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".into());
    let addr = format!("0.0.0.0:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("Failed to bind to address");

    tracing::info!("Server listening on {}", addr);

    // サーバー起動
    axum::serve(listener, app)
        .await
        .expect("Failed to start server");
}
