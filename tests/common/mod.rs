use sqlx::PgPool;

/// テスト用データベースプールを作成し、マイグレーションを実行
///
/// DATABASE_URL環境変数からデータベースURLを取得し、
/// sqlx migrateを使用してマイグレーションを適用します。
/// 本番環境と同じマイグレーションファイルを使用することで、
/// テストと本番の一貫性を保証します。
pub async fn create_test_pool() -> PgPool {
    let database_url = std::env::var("DATABASE_URL")
        .unwrap_or_else(|_| "postgres://postgres:postgres@localhost/rusty_library".to_string());

    let pool = PgPool::connect(&database_url)
        .await
        .expect("Failed to connect to test database");

    // sqlx migrateでマイグレーションを実行（本番と同じ方法）
    sqlx::migrate!("./migrations")
        .run(&pool)
        .await
        .expect("Failed to run migrations");

    pool
}
