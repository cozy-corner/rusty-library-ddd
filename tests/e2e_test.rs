use axum::body::Body;
use axum::http::{Request, StatusCode};
use rusty_library_ddd::adapters::mock::{BookService, MemberService};
use rusty_library_ddd::adapters::postgres::{PostgresEventStore, PostgresLoanReadModel};
use rusty_library_ddd::api::handlers::AppState;
use rusty_library_ddd::api::router::create_router;
use rusty_library_ddd::api::types::*;
use rusty_library_ddd::application::loan::ServiceDependencies;
use rusty_library_ddd::domain::value_objects::*;
use serde_json::json;
use serial_test::serial;
use sqlx::PgPool;
use std::sync::Arc;
use tower::ServiceExt;

mod common;

// ============================================================================
// E2Eテスト用のヘルパー関数
// ============================================================================

/// E2Eテスト用のアプリケーションセットアップ
///
/// 実際のPostgreSQLデータベースと実際のAPIルーターを使用します。
/// 各テストの前にデータベースをクリーンアップします。
///
/// モックサービスをテスト側から注入できるように、引数で受け取ります。
async fn setup_e2e_app(
    pool: &PgPool,
    member_service: Arc<MemberService>,
    book_service: Arc<BookService>,
) -> axum::Router {
    // データベースをクリーンアップ
    cleanup_database(pool).await;

    // アダプターの作成
    let event_store = Arc::new(PostgresEventStore::new(pool.clone()));
    let loan_read_model = Arc::new(PostgresLoanReadModel::new(pool.clone()));

    let service_deps = ServiceDependencies {
        event_store,
        loan_read_model,
        member_service,
        book_service,
    };

    let app_state = Arc::new(AppState { service_deps });

    create_router(app_state)
}

/// データベースのクリーンアップ
///
/// テストの独立性を保つため、各テスト前にすべてのデータを削除します。
async fn cleanup_database(pool: &PgPool) {
    sqlx::query("TRUNCATE TABLE loans_view CASCADE")
        .execute(pool)
        .await
        .expect("Failed to truncate loans_view");

    sqlx::query("TRUNCATE TABLE events CASCADE")
        .execute(pool)
        .await
        .expect("Failed to truncate events");
}

/// テスト用のメンバーと本をセットアップ
fn setup_test_entities(
    member_service: &MemberService,
    book_service: &BookService,
) -> (MemberId, BookId) {
    let member_id = MemberId::new();
    let book_id = BookId::new();

    member_service.add_member(member_id);
    book_service.add_available_book(book_id);

    (member_id, book_id)
}

// ============================================================================
// E2Eテスト: 正常系フロー
// ============================================================================

#[tokio::test]
#[serial]
async fn test_e2e_full_loan_flow() {
    // Arrange: データベースとアプリケーションのセットアップ
    let pool = common::create_test_pool().await;

    // テストデータの準備
    let member_service = Arc::new(MemberService::new());
    let book_service = Arc::new(BookService::new());
    let (member_id, book_id) = setup_test_entities(&member_service, &book_service);

    let app = setup_e2e_app(&pool, member_service, book_service).await;

    // Step 1: 貸出作成（POST /loans）
    let loan_request = json!({
        "book_id": book_id.value(),
        "member_id": member_id.value(),
        "staff_id": StaffId::new().value(),
    });

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/loans")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&loan_request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::CREATED);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let loan_response: LoanCreatedResponse = serde_json::from_slice(&body).unwrap();
    let loan_id = loan_response.loan_id;

    // Step 2: 貸出詳細取得（GET /loans/:id）
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/loans/{}", loan_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let loan_view: LoanResponse = serde_json::from_slice(&body).unwrap();
    assert_eq!(loan_view.loan_id, loan_id);
    assert_eq!(loan_view.book_id, book_id.value());
    assert_eq!(loan_view.member_id, member_id.value());
    assert_eq!(loan_view.status, "active");
    assert_eq!(loan_view.extension_count, 0);

    // Step 3: 延長（POST /loans/:id/extend）
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/loans/{}/extend", loan_id))
                .header("content-type", "application/json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let extend_response: LoanExtendedResponse = serde_json::from_slice(&body).unwrap();
    assert_eq!(extend_response.loan_id, loan_id);

    // 延長後の状態確認
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/loans/{}", loan_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let loan_view: LoanResponse = serde_json::from_slice(&body).unwrap();
    assert_eq!(loan_view.extension_count, 1);

    // Step 4: 返却（POST /loans/:id/return）
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/loans/{}/return", loan_id))
                .header("content-type", "application/json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let return_response: BookReturnedResponse = serde_json::from_slice(&body).unwrap();
    assert_eq!(return_response.loan_id, loan_id);

    // 返却後の状態確認
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/loans/{}", loan_id))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let loan_view: LoanResponse = serde_json::from_slice(&body).unwrap();
    assert_eq!(loan_view.status, "returned");
    assert!(loan_view.returned_at.is_some());
}

// ============================================================================
// E2Eテスト: エラーケース
// ============================================================================

#[tokio::test]
#[serial]
async fn test_e2e_loan_member_not_found() {
    // Arrange
    let pool = common::create_test_pool().await;

    let member_service = Arc::new(MemberService::new());
    let book_service = Arc::new(BookService::new());
    let book_id = BookId::new();
    book_service.add_available_book(book_id);

    let app = setup_e2e_app(&pool, member_service, book_service).await;

    // 存在しない会員IDで貸出を試みる
    let member_id = MemberId::new();
    let loan_request = json!({
        "book_id": book_id.value(),
        "member_id": member_id.value(),
        "staff_id": StaffId::new().value(),
    });

    // Act
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/loans")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&loan_request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Assert
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let error: ErrorResponse = serde_json::from_slice(&body).unwrap();
    assert!(error.message.contains("Member not found"));
}

#[tokio::test]
#[serial]
async fn test_e2e_loan_book_not_available() {
    // Arrange
    let pool = common::create_test_pool().await;

    let member_service = Arc::new(MemberService::new());
    let book_service = Arc::new(BookService::new());
    let member_id = MemberId::new();
    member_service.add_member(member_id);

    let app = setup_e2e_app(&pool, member_service, book_service).await;

    // 存在しない本IDで貸出を試みる
    let book_id = BookId::new();
    let loan_request = json!({
        "book_id": book_id.value(),
        "member_id": member_id.value(),
        "staff_id": StaffId::new().value(),
    });

    // Act
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/loans")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&loan_request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    // Assert
    assert_eq!(response.status(), StatusCode::UNPROCESSABLE_ENTITY);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let error: ErrorResponse = serde_json::from_slice(&body).unwrap();
    assert!(error.message.contains("not available"));
}

#[tokio::test]
#[serial]
async fn test_e2e_extend_loan_not_found() {
    // Arrange
    let pool = common::create_test_pool().await;
    let member_service = Arc::new(MemberService::new());
    let book_service = Arc::new(BookService::new());
    let app = setup_e2e_app(&pool, member_service, book_service).await;

    // 存在しない貸出IDで延長を試みる
    let non_existent_loan_id = LoanId::new();

    // Act
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/loans/{}/extend", non_existent_loan_id.value()))
                .header("content-type", "application/json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Assert
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

#[tokio::test]
#[serial]
async fn test_e2e_return_loan_not_found() {
    // Arrange
    let pool = common::create_test_pool().await;
    let member_service = Arc::new(MemberService::new());
    let book_service = Arc::new(BookService::new());
    let app = setup_e2e_app(&pool, member_service, book_service).await;

    // 存在しない貸出IDで返却を試みる
    let non_existent_loan_id = LoanId::new();

    // Act
    let response = app
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/loans/{}/return", non_existent_loan_id.value()))
                .header("content-type", "application/json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Assert
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}

// ============================================================================
// E2Eテスト: クエリエンドポイント
// ============================================================================

#[tokio::test]
#[serial]
async fn test_e2e_list_loans_by_member() {
    // Arrange: 複数の貸出を作成
    let pool = common::create_test_pool().await;

    let member_service = Arc::new(MemberService::new());
    let book_service = Arc::new(BookService::new());
    let member_id = MemberId::new();
    member_service.add_member(member_id);

    let app = setup_e2e_app(&pool, member_service.clone(), book_service.clone()).await;

    // 3冊借りる
    let mut loan_ids = Vec::new();
    for _ in 0..3 {
        let book_id = BookId::new();
        book_service.add_available_book(book_id);

        let loan_request = json!({
            "book_id": book_id.value(),
            "member_id": member_id.value(),
            "staff_id": StaffId::new().value(),
        });

        let response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/loans")
                    .header("content-type", "application/json")
                    .body(Body::from(serde_json::to_string(&loan_request).unwrap()))
                    .unwrap(),
            )
            .await
            .unwrap();

        let body = axum::body::to_bytes(response.into_body(), usize::MAX)
            .await
            .unwrap();
        let loan_response: LoanCreatedResponse = serde_json::from_slice(&body).unwrap();
        loan_ids.push(loan_response.loan_id);
    }

    // Act: 会員IDで貸出を検索
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/loans?member_id={}", member_id.value()))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Assert
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let loans: Vec<LoanResponse> = serde_json::from_slice(&body).unwrap();
    assert_eq!(loans.len(), 3);

    // すべての貸出が正しい会員IDを持つことを確認
    for loan in loans {
        assert_eq!(loan.member_id, member_id.value());
        assert!(loan_ids.contains(&loan.loan_id));
    }
}

#[tokio::test]
#[serial]
async fn test_e2e_list_loans_by_status() {
    // Arrange: Active と Returned の貸出を作成
    let pool = common::create_test_pool().await;

    let member_service = Arc::new(MemberService::new());
    let book_service = Arc::new(BookService::new());
    let (member_id, book_id) = setup_test_entities(&member_service, &book_service);

    let app = setup_e2e_app(&pool, member_service.clone(), book_service.clone()).await;

    // 1冊目: Active
    let loan_request = json!({
        "book_id": book_id.value(),
        "member_id": member_id.value(),
        "staff_id": StaffId::new().value(),
    });

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/loans")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&loan_request).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let active_loan: LoanCreatedResponse = serde_json::from_slice(&body).unwrap();

    // 2冊目: Returned
    let book_id2 = BookId::new();
    book_service.add_available_book(book_id2);
    let loan_request2 = json!({
        "book_id": book_id2.value(),
        "member_id": member_id.value(),
        "staff_id": StaffId::new().value(),
    });

    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri("/loans")
                .header("content-type", "application/json")
                .body(Body::from(serde_json::to_string(&loan_request2).unwrap()))
                .unwrap(),
        )
        .await
        .unwrap();

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let returned_loan: LoanCreatedResponse = serde_json::from_slice(&body).unwrap();

    // 2冊目を返却
    app.clone()
        .oneshot(
            Request::builder()
                .method("POST")
                .uri(format!("/loans/{}/return", returned_loan.loan_id))
                .header("content-type", "application/json")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Act: Active の貸出のみ取得
    let response = app
        .clone()
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!(
                    "/loans?member_id={}&status=active",
                    member_id.value()
                ))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Assert
    assert_eq!(response.status(), StatusCode::OK);

    let body = axum::body::to_bytes(response.into_body(), usize::MAX)
        .await
        .unwrap();
    let loans: Vec<LoanResponse> = serde_json::from_slice(&body).unwrap();
    assert_eq!(loans.len(), 1);
    assert_eq!(loans[0].loan_id, active_loan.loan_id);
    assert_eq!(loans[0].status, "active");
}

#[tokio::test]
#[serial]
async fn test_e2e_get_loan_not_found() {
    // Arrange
    let pool = common::create_test_pool().await;
    let member_service = Arc::new(MemberService::new());
    let book_service = Arc::new(BookService::new());
    let app = setup_e2e_app(&pool, member_service, book_service).await;

    let non_existent_loan_id = LoanId::new();

    // Act
    let response = app
        .oneshot(
            Request::builder()
                .method("GET")
                .uri(format!("/loans/{}", non_existent_loan_id.value()))
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();

    // Assert
    assert_eq!(response.status(), StatusCode::NOT_FOUND);
}
