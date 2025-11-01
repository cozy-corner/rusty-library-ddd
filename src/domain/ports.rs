use crate::domain::{
    events::DomainEvent,
    value_objects::{BookId, LoanId, MemberId},
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use futures::stream::BoxStream;

#[allow(dead_code)]
pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// Event Store port for persisting and retrieving domain events.
///
/// This trait abstracts the persistence layer for event sourcing.
/// Events are immutable facts stored in an append-only log.
#[allow(dead_code)]
#[async_trait]
pub trait EventStore: Send + Sync {
    /// Append events for an aggregate.
    ///
    /// Events are stored in an append-only log and cannot be modified or deleted.
    /// The order of events is preserved.
    async fn append(&self, aggregate_id: LoanId, events: Vec<DomainEvent>) -> Result<()>;

    /// Load all events for an aggregate.
    ///
    /// Returns events in the order they were appended.
    /// Used to reconstruct aggregate state via replay_events.
    async fn load(&self, aggregate_id: LoanId) -> Result<Vec<DomainEvent>>;

    /// Stream all events across all aggregates.
    ///
    /// Used for batch operations like overdue detection.
    /// Events are streamed in insertion order.
    fn stream_all(&self) -> BoxStream<'static, Result<DomainEvent>>;
}

/// Read Model for efficient loan queries (CQRS pattern).
///
/// This is a denormalized view optimized for queries.
/// Updated asynchronously when events are persisted.
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct LoanView {
    pub loan_id: LoanId,
    pub book_id: BookId,
    pub member_id: MemberId,
    pub loaned_at: DateTime<Utc>,
    pub due_date: DateTime<Utc>,
    pub returned_at: Option<DateTime<Utc>>,
    pub extension_count: u8,
    /// Status: "active", "overdue", or "returned"
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[allow(dead_code)]
#[async_trait]
pub trait LoanReadModel: Send + Sync {
    /// Insert a new loan view record.
    ///
    /// Called when BookLoaned event is processed.
    async fn insert(&self, loan_view: LoanView) -> Result<()>;

    /// Update loan status and returned_at timestamp.
    ///
    /// Called when BookReturned or LoanBecameOverdue events are processed.
    async fn update_status(
        &self,
        loan_id: LoanId,
        status: &str,
        returned_at: Option<DateTime<Utc>>,
    ) -> Result<()>;

    /// Update loan due date.
    ///
    /// Called when LoanExtended event is processed.
    async fn update_due_date(&self, loan_id: LoanId, new_due_date: DateTime<Utc>) -> Result<()>;

    /// Get active loans for a member.
    ///
    /// Used to enforce loan limit (max 5 active loans per member).
    async fn get_active_loans_for_member(&self, member_id: MemberId) -> Result<Vec<LoanView>>;

    /// Find loans that might be overdue.
    ///
    /// Returns loans where due_date < cutoff_date and status is "active".
    /// Used by batch job to detect overdue loans.
    async fn find_overdue_candidates(&self, cutoff_date: DateTime<Utc>) -> Result<Vec<LoanView>>;

    /// Get a single loan by ID.
    async fn get_by_id(&self, loan_id: LoanId) -> Result<Option<LoanView>>;

    /// Find all loans for a member.
    ///
    /// Used for member loan history display.
    async fn find_by_member_id(&self, member_id: MemberId) -> Result<Vec<LoanView>>;
}

/// Member Service port for member context operations.
///
/// This port maintains context boundaries between Loan and Member contexts.
/// Loan context only knows MemberId, not member details.
#[allow(dead_code)]
#[async_trait]
pub trait MemberService: Send + Sync {
    /// Check if a member exists.
    ///
    /// Used to validate member before creating a loan.
    async fn exists(&self, member_id: MemberId) -> Result<bool>;

    /// Check if a member has any overdue loans.
    ///
    /// Business rule: Cannot loan to member with overdue loans.
    async fn has_overdue_loans(&self, member_id: MemberId) -> Result<bool>;
}

/// Book Service port for catalog context operations.
///
/// This port maintains context boundaries between Loan and Catalog contexts.
/// Loan context only knows BookId, not book details.
#[allow(dead_code)]
#[async_trait]
pub trait BookService: Send + Sync {
    /// Check if a book is available for loan.
    ///
    /// Business rule: Cannot loan unavailable books.
    async fn is_available_for_loan(&self, book_id: BookId) -> Result<bool>;

    /// Get book title.
    ///
    /// Used in notifications to display friendly messages.
    async fn get_book_title(&self, book_id: BookId) -> Result<String>;
}

/// Notification Service port for sending notifications to members.
///
/// This port abstracts notification delivery mechanism.
/// Implementation could be email, SMS, push notification, etc.
#[allow(dead_code)]
#[async_trait]
pub trait NotificationService: Send + Sync {
    /// Send overdue notification to member.
    ///
    /// Called when LoanBecameOverdue event is processed.
    async fn send_overdue_notification(
        &self,
        member_id: MemberId,
        book_title: &str,
        due_date: DateTime<Utc>,
    ) -> Result<()>;

    /// Send extension confirmation to member.
    ///
    /// Called when LoanExtended event is processed.
    async fn send_extension_confirmation(
        &self,
        member_id: MemberId,
        book_title: &str,
        new_due_date: DateTime<Utc>,
    ) -> Result<()>;

    /// Send return confirmation to member.
    ///
    /// Called when BookReturned event is processed.
    async fn send_return_confirmation(
        &self,
        member_id: MemberId,
        book_title: &str,
        was_overdue: bool,
    ) -> Result<()>;
}
