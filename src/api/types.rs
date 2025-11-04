use crate::domain::value_objects::{BookId, MemberId, StaffId};
use crate::ports::loan_read_model::{LoanStatus, LoanView};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

// ============================================================================
// Command operations (POST) - Request/Response types
// ============================================================================

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

// ============================================================================
// Query operations (GET) - Request/Response types
// ============================================================================

/// 貸出一覧取得のクエリパラメータ
#[derive(Debug, Deserialize)]
pub struct ListLoansQuery {
    /// 会員IDでフィルタリング
    pub member_id: Option<Uuid>,
    /// ステータスでフィルタリング
    pub status: Option<String>,
}

/// 貸出レスポンス（GET /loans/:id と GET /loans）
#[derive(Debug, Serialize)]
pub struct LoanResponse {
    pub loan_id: Uuid,
    pub book_id: Uuid,
    pub member_id: Uuid,
    pub loaned_at: DateTime<Utc>,
    pub due_date: DateTime<Utc>,
    pub returned_at: Option<DateTime<Utc>>,
    pub extension_count: u8,
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl From<LoanView> for LoanResponse {
    fn from(view: LoanView) -> Self {
        Self {
            loan_id: view.loan_id.value(),
            book_id: view.book_id.value(),
            member_id: view.member_id.value(),
            loaned_at: view.loaned_at,
            due_date: view.due_date,
            returned_at: view.returned_at,
            extension_count: view.extension_count,
            status: view.status.as_str().to_string(),
            created_at: view.created_at,
            updated_at: view.updated_at,
        }
    }
}

// ============================================================================
// Common types
// ============================================================================

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

/// ステータスクエリパラメータのパースとバリデーション
pub fn parse_status_filter(status: &str) -> Result<LoanStatus, String> {
    status.parse::<LoanStatus>()
}
