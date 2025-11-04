use crate::domain::value_objects::{BookId, MemberId, StaffId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 貸出作成リクエスト
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LoanBookRequest {
    pub book_id: Uuid,
    pub member_id: Uuid,
    pub staff_id: Uuid,
}

impl LoanBookRequest {
    /// ドメインコマンドへ変換
    pub fn to_command(&self) -> crate::domain::commands::LoanBook {
        crate::domain::commands::LoanBook {
            book_id: BookId::from_uuid(self.book_id),
            member_id: MemberId::from_uuid(self.member_id),
            loaned_at: Utc::now(),
            staff_id: StaffId::from_uuid(self.staff_id),
        }
    }
}

/// 貸出作成成功レスポンス
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoanCreatedResponse {
    pub loan_id: Uuid,
    pub book_id: Uuid,
    pub member_id: Uuid,
    pub loaned_at: DateTime<Utc>,
    pub due_date: DateTime<Utc>,
}

/// 貸出延長成功レスポンス
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LoanExtendedResponse {
    pub loan_id: Uuid,
    pub new_due_date: DateTime<Utc>,
    pub extension_count: u8,
}

/// 返却成功レスポンス
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BookReturnedResponse {
    pub loan_id: Uuid,
    pub returned_at: DateTime<Utc>,
}

/// エラーレスポンス
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorResponse {
    pub error: String,
    pub message: String,
}

impl ErrorResponse {
    pub fn new(error: impl Into<String>, message: impl Into<String>) -> Self {
        Self {
            error: error.into(),
            message: message.into(),
        }
    }
}
