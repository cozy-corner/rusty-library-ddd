use chrono::Utc;
use rusty_library_ddd::application::loan::{
    ServiceDependencies, detect_overdue_loans, extend_loan, loan_book, return_book,
};
use rusty_library_ddd::domain::commands::*;
use rusty_library_ddd::domain::events::DomainEvent;
use rusty_library_ddd::domain::value_objects::*;
use rusty_library_ddd::ports::*;
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

// ============================================================================
// インメモリモック実装（テスト用）
// ============================================================================

/// インメモリEventStore実装
struct InMemoryEventStore {
    events: Mutex<HashMap<LoanId, Vec<DomainEvent>>>,
}

impl InMemoryEventStore {
    fn new() -> Self {
        Self {
            events: Mutex::new(HashMap::new()),
        }
    }
}

#[async_trait::async_trait]
impl EventStore for InMemoryEventStore {
    async fn append(
        &self,
        aggregate_id: LoanId,
        events: Vec<DomainEvent>,
    ) -> event_store::Result<()> {
        let mut store = self.events.lock().unwrap();
        store.entry(aggregate_id).or_default().extend(events);
        Ok(())
    }

    async fn load(&self, aggregate_id: LoanId) -> event_store::Result<Vec<DomainEvent>> {
        let store = self.events.lock().unwrap();
        Ok(store.get(&aggregate_id).cloned().unwrap_or_default())
    }

    fn stream_all(&self) -> futures::stream::BoxStream<'_, event_store::Result<DomainEvent>> {
        unimplemented!("stream_all not needed for these tests")
    }
}

/// インメモリLoanReadModel実装
struct InMemoryLoanReadModel {
    loans: Mutex<HashMap<LoanId, LoanView>>,
}

impl InMemoryLoanReadModel {
    fn new() -> Self {
        Self {
            loans: Mutex::new(HashMap::new()),
        }
    }
}

#[async_trait::async_trait]
impl LoanReadModel for InMemoryLoanReadModel {
    async fn insert(&self, loan_view: LoanView) -> loan_read_model::Result<()> {
        let mut loans = self.loans.lock().unwrap();
        loans.insert(loan_view.loan_id, loan_view);
        Ok(())
    }

    async fn update_status(
        &self,
        loan_id: LoanId,
        status: LoanStatus,
        returned_at: Option<chrono::DateTime<Utc>>,
    ) -> loan_read_model::Result<()> {
        let mut loans = self.loans.lock().unwrap();
        if let Some(loan) = loans.get_mut(&loan_id) {
            loan.status = status;
            loan.returned_at = returned_at;
        }
        Ok(())
    }

    async fn update_due_date(
        &self,
        loan_id: LoanId,
        new_due_date: chrono::DateTime<Utc>,
    ) -> loan_read_model::Result<()> {
        let mut loans = self.loans.lock().unwrap();
        if let Some(loan) = loans.get_mut(&loan_id) {
            loan.due_date = new_due_date;
        }
        Ok(())
    }

    async fn get_active_loans_for_member(
        &self,
        member_id: MemberId,
    ) -> loan_read_model::Result<Vec<LoanView>> {
        let loans = self.loans.lock().unwrap();
        Ok(loans
            .values()
            .filter(|l| l.member_id == member_id && matches!(l.status, LoanStatus::Active))
            .cloned()
            .collect())
    }

    async fn find_overdue_candidates(
        &self,
        cutoff_date: chrono::DateTime<Utc>,
    ) -> loan_read_model::Result<Vec<LoanView>> {
        let loans = self.loans.lock().unwrap();
        Ok(loans
            .values()
            .filter(|l| matches!(l.status, LoanStatus::Active) && l.due_date < cutoff_date)
            .cloned()
            .collect())
    }

    async fn get_by_id(&self, loan_id: LoanId) -> loan_read_model::Result<Option<LoanView>> {
        let loans = self.loans.lock().unwrap();
        Ok(loans.get(&loan_id).cloned())
    }

    async fn find_by_member_id(
        &self,
        member_id: MemberId,
    ) -> loan_read_model::Result<Vec<LoanView>> {
        let loans = self.loans.lock().unwrap();
        Ok(loans
            .values()
            .filter(|l| l.member_id == member_id)
            .cloned()
            .collect())
    }
}

/// モックMemberService実装
struct MockMemberService {
    existing_members: Mutex<Vec<MemberId>>,
    overdue_members: Mutex<Vec<MemberId>>,
}

impl MockMemberService {
    fn new() -> Self {
        Self {
            existing_members: Mutex::new(Vec::new()),
            overdue_members: Mutex::new(Vec::new()),
        }
    }

    fn add_member(&self, member_id: MemberId) {
        self.existing_members.lock().unwrap().push(member_id);
    }
}

#[async_trait::async_trait]
impl MemberService for MockMemberService {
    async fn exists(&self, member_id: MemberId) -> member_service::Result<bool> {
        Ok(self.existing_members.lock().unwrap().contains(&member_id))
    }

    async fn has_overdue_loans(&self, member_id: MemberId) -> member_service::Result<bool> {
        Ok(self.overdue_members.lock().unwrap().contains(&member_id))
    }
}

/// モックBookService実装
struct MockBookService {
    available_books: Mutex<Vec<BookId>>,
}

impl MockBookService {
    fn new() -> Self {
        Self {
            available_books: Mutex::new(Vec::new()),
        }
    }

    fn add_available_book(&self, book_id: BookId) {
        self.available_books.lock().unwrap().push(book_id);
    }
}

#[async_trait::async_trait]
impl BookService for MockBookService {
    async fn is_available_for_loan(&self, book_id: BookId) -> book_service::Result<bool> {
        Ok(self.available_books.lock().unwrap().contains(&book_id))
    }

    async fn get_book_title(&self, _book_id: BookId) -> book_service::Result<String> {
        Ok("Test Book".to_string())
    }
}

// ============================================================================
// 統合テスト（関数型DDD - 関数ベースのAPI）
// ============================================================================

#[tokio::test]
async fn test_loan_book_success() {
    // Arrange: 依存関係のセットアップ
    let event_store = Arc::new(InMemoryEventStore::new());
    let loan_read_model = Arc::new(InMemoryLoanReadModel::new());
    let member_service = Arc::new(MockMemberService::new());
    let book_service = Arc::new(MockBookService::new());

    let member_id = MemberId::new();
    let book_id = BookId::new();
    let staff_id = StaffId::new();

    member_service.add_member(member_id);
    book_service.add_available_book(book_id);

    let deps = ServiceDependencies {
        event_store: event_store.clone(),
        loan_read_model: loan_read_model.clone(),
        member_service,
        book_service,
    };

    // Act: 貸出実行（純粋な関数呼び出し）
    let cmd = LoanBook {
        book_id,
        member_id,
        loaned_at: Utc::now(),
        staff_id,
    };

    let result = loan_book(&deps, cmd).await;

    // Assert: 成功を確認
    assert!(result.is_ok());
    let loan_id = result.unwrap();

    // イベントが保存されたことを確認
    let events = event_store.load(loan_id).await.unwrap();
    assert_eq!(events.len(), 1);
    assert!(matches!(events[0], DomainEvent::BookLoaned(_)));

    // Read Modelが更新されたことを確認
    let loan_view = loan_read_model.get_by_id(loan_id).await.unwrap();
    assert!(loan_view.is_some());
    assert_eq!(loan_view.unwrap().status, LoanStatus::Active);
}

#[tokio::test]
async fn test_loan_book_member_not_found() {
    // Arrange
    let event_store = Arc::new(InMemoryEventStore::new());
    let loan_read_model = Arc::new(InMemoryLoanReadModel::new());
    let member_service = Arc::new(MockMemberService::new());
    let book_service = Arc::new(MockBookService::new());

    let member_id = MemberId::new();
    let book_id = BookId::new();
    let staff_id = StaffId::new();

    // 会員を登録しない（存在しない会員）
    book_service.add_available_book(book_id);

    let deps = ServiceDependencies {
        event_store,
        loan_read_model,
        member_service,
        book_service,
    };

    // Act
    let cmd = LoanBook {
        book_id,
        member_id,
        loaned_at: Utc::now(),
        staff_id,
    };

    let result = loan_book(&deps, cmd).await;

    // Assert: MemberNotFoundエラーを確認
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        rusty_library_ddd::application::loan::LoanApplicationError::MemberNotFound
    ));
}

#[tokio::test]
async fn test_loan_book_limit_exceeded() {
    // Arrange: 既に5冊借りている会員
    let event_store = Arc::new(InMemoryEventStore::new());
    let loan_read_model = Arc::new(InMemoryLoanReadModel::new());
    let member_service = Arc::new(MockMemberService::new());
    let book_service = Arc::new(MockBookService::new());

    let member_id = MemberId::new();
    let staff_id = StaffId::new();

    member_service.add_member(member_id);

    // 5冊の貸出を事前に登録
    for _ in 0..5 {
        let book_id = BookId::new();
        book_service.add_available_book(book_id);

        let loan_view = LoanView {
            loan_id: LoanId::new(),
            book_id,
            member_id,
            loaned_at: Utc::now(),
            due_date: Utc::now(),
            returned_at: None,
            extension_count: 0,
            status: LoanStatus::Active,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        loan_read_model.insert(loan_view).await.unwrap();
    }

    // 6冊目を借りようとする
    let new_book_id = BookId::new();
    book_service.add_available_book(new_book_id);

    let deps = ServiceDependencies {
        event_store,
        loan_read_model,
        member_service,
        book_service,
    };

    // Act
    let cmd = LoanBook {
        book_id: new_book_id,
        member_id,
        loaned_at: Utc::now(),
        staff_id,
    };

    let result = loan_book(&deps, cmd).await;

    // Assert: LoanLimitExceededエラーを確認
    assert!(result.is_err());
    assert!(matches!(
        result.unwrap_err(),
        rusty_library_ddd::application::loan::LoanApplicationError::LoanLimitExceeded
    ));
}

#[tokio::test]
async fn test_extend_loan_success() {
    // Arrange: 貸出を事前に作成
    let event_store = Arc::new(InMemoryEventStore::new());
    let loan_read_model = Arc::new(InMemoryLoanReadModel::new());
    let member_service = Arc::new(MockMemberService::new());
    let book_service = Arc::new(MockBookService::new());

    let member_id = MemberId::new();
    let book_id = BookId::new();
    let staff_id = StaffId::new();

    member_service.add_member(member_id);
    book_service.add_available_book(book_id);

    let deps = ServiceDependencies {
        event_store: event_store.clone(),
        loan_read_model: loan_read_model.clone(),
        member_service,
        book_service,
    };

    // 貸出作成
    let loan_cmd = LoanBook {
        book_id,
        member_id,
        loaned_at: Utc::now(),
        staff_id,
    };
    let loan_id = loan_book(&deps, loan_cmd).await.unwrap();

    // Act: 延長実行（純粋な関数呼び出し）
    let extend_cmd = ExtendLoan {
        loan_id,
        extended_at: Utc::now(),
    };

    let result = extend_loan(&deps, extend_cmd).await;

    // Assert: 成功を確認
    assert!(result.is_ok());

    // イベントが追加されたことを確認
    let events = event_store.load(loan_id).await.unwrap();
    assert_eq!(events.len(), 2); // BookLoaned + LoanExtended
    assert!(matches!(events[1], DomainEvent::LoanExtended(_)));
}

#[tokio::test]
async fn test_return_book_success() {
    // Arrange: 貸出を事前に作成
    let event_store = Arc::new(InMemoryEventStore::new());
    let loan_read_model = Arc::new(InMemoryLoanReadModel::new());
    let member_service = Arc::new(MockMemberService::new());
    let book_service = Arc::new(MockBookService::new());

    let member_id = MemberId::new();
    let book_id = BookId::new();
    let staff_id = StaffId::new();

    member_service.add_member(member_id);
    book_service.add_available_book(book_id);

    let deps = ServiceDependencies {
        event_store: event_store.clone(),
        loan_read_model: loan_read_model.clone(),
        member_service,
        book_service,
    };

    // 貸出作成
    let loan_cmd = LoanBook {
        book_id,
        member_id,
        loaned_at: Utc::now(),
        staff_id,
    };
    let loan_id = loan_book(&deps, loan_cmd).await.unwrap();

    // Act: 返却実行（純粋な関数呼び出し）
    let return_cmd = ReturnBook {
        loan_id,
        returned_at: Utc::now(),
    };

    let result = return_book(&deps, return_cmd).await;

    // Assert: 成功を確認
    assert!(result.is_ok());

    // イベントが追加されたことを確認
    let events = event_store.load(loan_id).await.unwrap();
    assert_eq!(events.len(), 2); // BookLoaned + BookReturned
    assert!(matches!(events[1], DomainEvent::BookReturned(_)));

    // Read Modelのステータスが更新されたことを確認
    let loan_view = loan_read_model.get_by_id(loan_id).await.unwrap();
    assert!(loan_view.is_some());
    assert_eq!(loan_view.unwrap().status, LoanStatus::Returned);
}

#[tokio::test]
async fn test_detect_overdue_loans() {
    // Arrange: 延滞した貸出を作成
    let event_store = Arc::new(InMemoryEventStore::new());
    let loan_read_model = Arc::new(InMemoryLoanReadModel::new());
    let member_service = Arc::new(MockMemberService::new());
    let book_service = Arc::new(MockBookService::new());

    let member_id = MemberId::new();
    let book_id = BookId::new();
    let staff_id = StaffId::new();

    member_service.add_member(member_id);
    book_service.add_available_book(book_id);

    let deps = ServiceDependencies {
        event_store: event_store.clone(),
        loan_read_model: loan_read_model.clone(),
        member_service,
        book_service,
    };

    // 過去の日付で貸出作成（延滞させる）
    let loaned_at = Utc::now() - chrono::Duration::days(30);
    let loan_cmd = LoanBook {
        book_id,
        member_id,
        loaned_at,
        staff_id,
    };
    let loan_id = loan_book(&deps, loan_cmd).await.unwrap();

    // Act: 延滞検出バッチ実行（純粋な関数呼び出し）
    let result = detect_overdue_loans(&deps).await;

    // Assert: 1件検出されたことを確認
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), 1);

    // LoanBecameOverdueイベントが追加されたことを確認
    let events = event_store.load(loan_id).await.unwrap();
    assert_eq!(events.len(), 2); // BookLoaned + LoanBecameOverdue
    assert!(matches!(events[1], DomainEvent::LoanBecameOverdue(_)));

    // Read Modelのステータスが更新されたことを確認
    let loan_view = loan_read_model.get_by_id(loan_id).await.unwrap();
    assert!(loan_view.is_some());
    assert_eq!(loan_view.unwrap().status, LoanStatus::Overdue);
}
