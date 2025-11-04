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
    // Initialize tracing
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "rusty_library_ddd=debug,tower_http=debug,axum=trace".into()),
        )
        .with(tracing_subscriber::fmt::layer())
        .init();

    // Database connection URL
    // For now, using a placeholder - actual database connection will be in Task 7 (Integration)
    let database_url =
        std::env::var("DATABASE_URL").unwrap_or_else(|_| "postgres://localhost/library".into());

    tracing::info!("Database URL: {}", database_url);

    // Initialize database connection pool
    let pool = sqlx::postgres::PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to connect to database");

    // Initialize adapters
    let event_store = Arc::new(PostgresEventStore::new(pool.clone()));
    let loan_read_model = Arc::new(PostgresLoanReadModel::new(pool.clone()));
    let member_service = Arc::new(MockMemberService::new());
    let book_service = Arc::new(MockBookService::new());

    // Create service dependencies
    let service_deps = ServiceDependencies {
        event_store,
        loan_read_model,
        member_service,
        book_service,
    };

    // Create application state
    let app_state = Arc::new(AppState { service_deps });

    // Create router
    let app = create_router(app_state);

    // Server configuration
    let port = std::env::var("PORT").unwrap_or_else(|_| "3000".into());
    let addr = format!("0.0.0.0:{}", port);
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("Failed to bind to address");

    tracing::info!("Server listening on {}", addr);

    // Start server
    axum::serve(listener, app)
        .await
        .expect("Failed to start server");
}
