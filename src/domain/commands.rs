use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::{BookId, LoanId, MemberId, StaffId};

/// コマンド：書籍を貸し出す
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoanBook {
    pub book_id: BookId,
    pub member_id: MemberId,
    pub loaned_at: DateTime<Utc>,
    pub staff_id: StaffId,
}

/// コマンド：貸出を延長する
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExtendLoan {
    pub loan_id: LoanId,
    pub extended_at: DateTime<Utc>,
}

/// コマンド：書籍を返却する
#[allow(dead_code)]
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReturnBook {
    pub loan_id: LoanId,
    pub returned_at: DateTime<Utc>,
}
