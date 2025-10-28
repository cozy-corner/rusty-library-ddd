use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::{BookId, LoanId, MemberId, StaffId};

/// イベント：書籍が貸出された
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BookLoaned {
    pub loan_id: LoanId,
    pub book_id: BookId,
    pub member_id: MemberId,
    pub loaned_at: DateTime<Utc>,
    pub due_date: DateTime<Utc>,
    pub loaned_by: StaffId,
}

/// イベント：貸出が延長された
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoanExtended {
    pub loan_id: LoanId,
    pub old_due_date: DateTime<Utc>,
    pub new_due_date: DateTime<Utc>,
    pub extended_at: DateTime<Utc>,
    pub extension_count: u8,
}

/// イベント：書籍が返却された
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct BookReturned {
    pub loan_id: LoanId,
    pub book_id: BookId,
    pub member_id: MemberId,
    pub returned_at: DateTime<Utc>,
    pub was_overdue: bool,
}

/// イベント：貸出が延滞した
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoanBecameOverdue {
    pub loan_id: LoanId,
    pub book_id: BookId,
    pub member_id: MemberId,
    pub due_date: DateTime<Utc>,
    pub detected_at: DateTime<Utc>,
}

/// ドメインイベント統合型
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum DomainEvent {
    BookLoaned(BookLoaned),
    LoanExtended(LoanExtended),
    BookReturned(BookReturned),
    LoanBecameOverdue(LoanBecameOverdue),
}
