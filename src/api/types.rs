use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::ports::loan_read_model::{LoanStatus, LoanView};

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

/// エラーレスポンス
#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

impl ErrorResponse {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            error: message.into(),
        }
    }
}

/// ステータスクエリパラメータのパースとバリデーション
pub fn parse_status_filter(status: &str) -> Result<LoanStatus, String> {
    status.parse::<LoanStatus>()
}
