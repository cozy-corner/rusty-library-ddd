mod common;

use chrono::{DateTime, Utc};
use rusty_library_ddd::adapters::postgres::{loan_read_model::LoanReadModel, projector};
use rusty_library_ddd::domain::events::{
    BookLoaned, BookReturned, DomainEvent, LoanBecameOverdue, LoanExtended,
};
use rusty_library_ddd::domain::value_objects::{BookId, LoanId, MemberId, StaffId};
use rusty_library_ddd::ports::loan_read_model::{
    LoanReadModel as LoanReadModelTrait, LoanStatus, LoanView,
};
use sqlx::PgPool;

/// PostgreSQLの時刻精度（マイクロ秒）に合わせて丸める
///
/// PostgreSQL TIMESTAMPTZはマイクロ秒精度（6桁）だが、
/// RustのDateTime<Utc>はナノ秒精度（9桁）を持つ。
/// DBへの保存・取得で精度が変わるため、テストでは比較前に統一する。
fn truncate_to_micros(dt: DateTime<Utc>) -> DateTime<Utc> {
    let micros = dt.timestamp_micros();
    DateTime::from_timestamp_micros(micros).expect("Invalid timestamp")
}

/// テストデータをクリーンアップ
async fn cleanup_loan(pool: &PgPool, loan_id: LoanId) {
    sqlx::query("DELETE FROM loans_view WHERE loan_id = $1")
        .bind(loan_id.value())
        .execute(pool)
        .await
        .expect("Failed to cleanup test loan");
}

#[tokio::test]
async fn test_loan_read_model_save_and_get_by_id() {
    let pool = common::create_test_pool().await;
    let read_model = LoanReadModel::new(pool.clone());

    let loan_id = LoanId::new();
    let book_id = BookId::new();
    let member_id = MemberId::new();
    let now = Utc::now();

    let loan_view = LoanView {
        loan_id,
        book_id,
        member_id,
        loaned_at: now,
        due_date: now + chrono::Duration::days(14),
        returned_at: None,
        extension_count: 0,
        status: LoanStatus::Active,
        created_at: now,
        updated_at: now,
    };

    // Save loan view
    read_model
        .save(loan_view.clone())
        .await
        .expect("Failed to save loan view");

    // Get by ID
    let retrieved = read_model
        .get_by_id(loan_id)
        .await
        .expect("Failed to get loan by id");

    assert!(retrieved.is_some());
    let retrieved = retrieved.unwrap();
    assert_eq!(retrieved.loan_id, loan_id);
    assert_eq!(retrieved.book_id, book_id);
    assert_eq!(retrieved.member_id, member_id);
    assert_eq!(retrieved.status, LoanStatus::Active);
    assert_eq!(retrieved.extension_count, 0);

    // Cleanup
    cleanup_loan(&pool, loan_id).await;
}

#[tokio::test]
async fn test_loan_read_model_upsert() {
    let pool = common::create_test_pool().await;
    let read_model = LoanReadModel::new(pool.clone());

    let loan_id = LoanId::new();
    let book_id = BookId::new();
    let member_id = MemberId::new();
    let now = Utc::now();

    let loan_view = LoanView {
        loan_id,
        book_id,
        member_id,
        loaned_at: now,
        due_date: now + chrono::Duration::days(14),
        returned_at: None,
        extension_count: 0,
        status: LoanStatus::Active,
        created_at: now,
        updated_at: now,
    };

    // First save
    read_model
        .save(loan_view.clone())
        .await
        .expect("Failed to save loan view");

    // Update (upsert)
    let updated_loan_view = LoanView {
        extension_count: 1,
        due_date: now + chrono::Duration::days(28),
        updated_at: now + chrono::Duration::days(1),
        ..loan_view
    };

    read_model
        .save(updated_loan_view)
        .await
        .expect("Failed to update loan view");

    // Verify update
    let retrieved = read_model
        .get_by_id(loan_id)
        .await
        .expect("Failed to get loan by id")
        .unwrap();

    assert_eq!(retrieved.extension_count, 1);
    assert_eq!(
        retrieved.due_date,
        truncate_to_micros(now + chrono::Duration::days(28))
    );

    // Cleanup
    cleanup_loan(&pool, loan_id).await;
}

#[tokio::test]
async fn test_get_active_loans_for_member() {
    let pool = common::create_test_pool().await;
    let read_model = LoanReadModel::new(pool.clone());

    let member_id = MemberId::new();
    let now = Utc::now();

    // Create 3 active loans and 1 returned loan for the member
    let mut loan_ids = Vec::new();
    for i in 0..3 {
        let loan_id = LoanId::new();
        loan_ids.push(loan_id);

        let loan_view = LoanView {
            loan_id,
            book_id: BookId::new(),
            member_id,
            loaned_at: now - chrono::Duration::days(i),
            due_date: now + chrono::Duration::days(14),
            returned_at: None,
            extension_count: 0,
            status: LoanStatus::Active,
            created_at: now,
            updated_at: now,
        };

        read_model.save(loan_view).await.unwrap();
    }

    // Add a returned loan (should not be included)
    let returned_loan_id = LoanId::new();
    loan_ids.push(returned_loan_id);

    let returned_loan = LoanView {
        loan_id: returned_loan_id,
        book_id: BookId::new(),
        member_id,
        loaned_at: now - chrono::Duration::days(10),
        due_date: now - chrono::Duration::days(3),
        returned_at: Some(now),
        extension_count: 0,
        status: LoanStatus::Returned,
        created_at: now,
        updated_at: now,
    };

    read_model.save(returned_loan).await.unwrap();

    // Get active loans for member
    let active_loans = read_model
        .get_active_loans_for_member(member_id)
        .await
        .expect("Failed to get active loans");

    assert_eq!(active_loans.len(), 3);
    for loan in active_loans {
        assert_eq!(loan.status, LoanStatus::Active);
        assert_eq!(loan.member_id, member_id);
    }

    // Cleanup
    for loan_id in loan_ids {
        cleanup_loan(&pool, loan_id).await;
    }
}

#[tokio::test]
async fn test_find_overdue_candidates() {
    let pool = common::create_test_pool().await;
    let read_model = LoanReadModel::new(pool.clone());

    let now = Utc::now();

    // Create overdue loan (due_date in the past, status active)
    let overdue_loan_id = LoanId::new();
    let overdue_loan = LoanView {
        loan_id: overdue_loan_id,
        book_id: BookId::new(),
        member_id: MemberId::new(),
        loaned_at: now - chrono::Duration::days(30),
        due_date: now - chrono::Duration::days(5), // Past due
        returned_at: None,
        extension_count: 0,
        status: LoanStatus::Active,
        created_at: now - chrono::Duration::days(30),
        updated_at: now - chrono::Duration::days(30),
    };

    read_model.save(overdue_loan).await.unwrap();

    // Create active loan (not overdue)
    let active_loan_id = LoanId::new();
    let active_loan = LoanView {
        loan_id: active_loan_id,
        book_id: BookId::new(),
        member_id: MemberId::new(),
        loaned_at: now,
        due_date: now + chrono::Duration::days(14), // Future
        returned_at: None,
        extension_count: 0,
        status: LoanStatus::Active,
        created_at: now,
        updated_at: now,
    };

    read_model.save(active_loan).await.unwrap();

    // Find overdue candidates
    let candidates = read_model
        .find_overdue_candidates(now)
        .await
        .expect("Failed to find overdue candidates");

    assert_eq!(candidates.len(), 1);
    assert_eq!(candidates[0].loan_id, overdue_loan_id);
    assert_eq!(candidates[0].status, LoanStatus::Active);
    assert!(candidates[0].due_date < now);

    // Cleanup
    cleanup_loan(&pool, overdue_loan_id).await;
    cleanup_loan(&pool, active_loan_id).await;
}

#[tokio::test]
async fn test_find_by_member_id() {
    let pool = common::create_test_pool().await;
    let read_model = LoanReadModel::new(pool.clone());

    let member_id = MemberId::new();
    let other_member_id = MemberId::new();
    let now = Utc::now();

    // Create 2 loans for member
    let loan_id1 = LoanId::new();
    let loan_id2 = LoanId::new();

    let loan1 = LoanView {
        loan_id: loan_id1,
        book_id: BookId::new(),
        member_id,
        loaned_at: now - chrono::Duration::days(10),
        due_date: now + chrono::Duration::days(4),
        returned_at: None,
        extension_count: 0,
        status: LoanStatus::Active,
        created_at: now - chrono::Duration::days(10),
        updated_at: now - chrono::Duration::days(10),
    };

    let loan2 = LoanView {
        loan_id: loan_id2,
        book_id: BookId::new(),
        member_id,
        loaned_at: now - chrono::Duration::days(20),
        due_date: now - chrono::Duration::days(6),
        returned_at: Some(now - chrono::Duration::days(5)),
        extension_count: 0,
        status: LoanStatus::Returned,
        created_at: now - chrono::Duration::days(20),
        updated_at: now - chrono::Duration::days(5),
    };

    read_model.save(loan1).await.unwrap();
    read_model.save(loan2).await.unwrap();

    // Create loan for other member (should not be included)
    let other_loan_id = LoanId::new();
    let other_loan = LoanView {
        loan_id: other_loan_id,
        book_id: BookId::new(),
        member_id: other_member_id,
        loaned_at: now,
        due_date: now + chrono::Duration::days(14),
        returned_at: None,
        extension_count: 0,
        status: LoanStatus::Active,
        created_at: now,
        updated_at: now,
    };

    read_model.save(other_loan).await.unwrap();

    // Find by member ID
    let loans = read_model
        .find_by_member_id(member_id)
        .await
        .expect("Failed to find loans by member id");

    assert_eq!(loans.len(), 2);
    for loan in &loans {
        assert_eq!(loan.member_id, member_id);
    }

    // Cleanup
    cleanup_loan(&pool, loan_id1).await;
    cleanup_loan(&pool, loan_id2).await;
    cleanup_loan(&pool, other_loan_id).await;
}

#[tokio::test]
async fn test_projector_book_loaned() {
    let pool = common::create_test_pool().await;
    let read_model = LoanReadModel::new(pool.clone());

    let loan_id = LoanId::new();
    let book_id = BookId::new();
    let member_id = MemberId::new();
    let staff_id = StaffId::new();
    let now = Utc::now();
    let due_date = now + chrono::Duration::days(14);

    let events = vec![DomainEvent::BookLoaned(BookLoaned {
        loan_id,
        book_id,
        member_id,
        loaned_at: now,
        due_date,
        loaned_by: staff_id,
    })];

    // Project events
    projector::project_loan_events(&read_model, &events)
        .await
        .expect("Failed to project events");

    // Verify read model
    let loan_view = read_model
        .get_by_id(loan_id)
        .await
        .expect("Failed to get loan")
        .expect("Loan not found");

    assert_eq!(loan_view.loan_id, loan_id);
    assert_eq!(loan_view.book_id, book_id);
    assert_eq!(loan_view.member_id, member_id);
    assert_eq!(loan_view.status, LoanStatus::Active);
    assert_eq!(loan_view.extension_count, 0);
    assert!(loan_view.returned_at.is_none());

    // Cleanup
    cleanup_loan(&pool, loan_id).await;
}

#[tokio::test]
async fn test_projector_loan_extended() {
    let pool = common::create_test_pool().await;
    let read_model = LoanReadModel::new(pool.clone());

    let loan_id = LoanId::new();
    let book_id = BookId::new();
    let member_id = MemberId::new();
    let staff_id = StaffId::new();
    let now = Utc::now();
    let old_due_date = now + chrono::Duration::days(14);
    let new_due_date = old_due_date + chrono::Duration::days(14);

    let events = vec![
        DomainEvent::BookLoaned(BookLoaned {
            loan_id,
            book_id,
            member_id,
            loaned_at: now,
            due_date: old_due_date,
            loaned_by: staff_id,
        }),
        DomainEvent::LoanExtended(LoanExtended {
            loan_id,
            old_due_date,
            new_due_date,
            extended_at: now + chrono::Duration::days(5),
            extension_count: 1,
        }),
    ];

    // Project events
    projector::project_loan_events(&read_model, &events)
        .await
        .expect("Failed to project events");

    // Verify read model
    let loan_view = read_model
        .get_by_id(loan_id)
        .await
        .expect("Failed to get loan")
        .expect("Loan not found");

    assert_eq!(loan_view.status, LoanStatus::Active);
    assert_eq!(loan_view.extension_count, 1);
    assert_eq!(loan_view.due_date, truncate_to_micros(new_due_date));

    // Cleanup
    cleanup_loan(&pool, loan_id).await;
}

#[tokio::test]
async fn test_projector_book_returned() {
    let pool = common::create_test_pool().await;
    let read_model = LoanReadModel::new(pool.clone());

    let loan_id = LoanId::new();
    let book_id = BookId::new();
    let member_id = MemberId::new();
    let staff_id = StaffId::new();
    let now = Utc::now();
    let returned_at = now + chrono::Duration::days(7);

    let events = vec![
        DomainEvent::BookLoaned(BookLoaned {
            loan_id,
            book_id,
            member_id,
            loaned_at: now,
            due_date: now + chrono::Duration::days(14),
            loaned_by: staff_id,
        }),
        DomainEvent::BookReturned(BookReturned {
            loan_id,
            book_id,
            member_id,
            returned_at,
            was_overdue: false,
        }),
    ];

    // Project events
    projector::project_loan_events(&read_model, &events)
        .await
        .expect("Failed to project events");

    // Verify read model
    let loan_view = read_model
        .get_by_id(loan_id)
        .await
        .expect("Failed to get loan")
        .expect("Loan not found");

    assert_eq!(loan_view.status, LoanStatus::Returned);
    assert_eq!(loan_view.returned_at, Some(truncate_to_micros(returned_at)));

    // Cleanup
    cleanup_loan(&pool, loan_id).await;
}

#[tokio::test]
async fn test_projector_loan_became_overdue() {
    let pool = common::create_test_pool().await;
    let read_model = LoanReadModel::new(pool.clone());

    let loan_id = LoanId::new();
    let book_id = BookId::new();
    let member_id = MemberId::new();
    let staff_id = StaffId::new();
    let now = Utc::now();
    let due_date = now + chrono::Duration::days(14);
    let detected_at = now + chrono::Duration::days(20);

    let events = vec![
        DomainEvent::BookLoaned(BookLoaned {
            loan_id,
            book_id,
            member_id,
            loaned_at: now,
            due_date,
            loaned_by: staff_id,
        }),
        DomainEvent::LoanBecameOverdue(LoanBecameOverdue {
            loan_id,
            book_id,
            member_id,
            due_date,
            detected_at,
        }),
    ];

    // Project events
    projector::project_loan_events(&read_model, &events)
        .await
        .expect("Failed to project events");

    // Verify read model
    let loan_view = read_model
        .get_by_id(loan_id)
        .await
        .expect("Failed to get loan")
        .expect("Loan not found");

    assert_eq!(loan_view.status, LoanStatus::Overdue);
    assert!(loan_view.returned_at.is_none());

    // Cleanup
    cleanup_loan(&pool, loan_id).await;
}
