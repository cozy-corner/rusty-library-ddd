use crate::domain::value_objects::{BookId, LoanId, MemberId};
use async_trait::async_trait;
use chrono::{DateTime, Utc};

#[allow(dead_code)]
pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// 貸出ステータス（Read Model用）
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoanStatus {
    /// 貸出中
    Active,
    /// 延滞中
    Overdue,
    /// 返却済み
    Returned,
}

impl LoanStatus {
    /// 文字列表現を取得する
    #[allow(dead_code)]
    pub fn as_str(&self) -> &'static str {
        match self {
            LoanStatus::Active => "active",
            LoanStatus::Overdue => "overdue",
            LoanStatus::Returned => "returned",
        }
    }
}

impl std::str::FromStr for LoanStatus {
    type Err = String;

    fn from_str(s: &str) -> std::result::Result<Self, Self::Err> {
        match s {
            "active" => Ok(LoanStatus::Active),
            "overdue" => Ok(LoanStatus::Overdue),
            "returned" => Ok(LoanStatus::Returned),
            _ => Err(format!("Invalid loan status: {}", s)),
        }
    }
}

/// 貸出ビュー（Read Model）
///
/// クエリに最適化された非正規化ビュー（CQRSパターン）。
/// イベント永続化時に非同期で更新される。
#[allow(dead_code)] // フィールドは将来のAPI層で使用
#[derive(Debug, Clone)]
pub struct LoanView {
    pub loan_id: LoanId,
    pub book_id: BookId,
    pub member_id: MemberId,
    pub loaned_at: DateTime<Utc>,
    pub due_date: DateTime<Utc>,
    pub returned_at: Option<DateTime<Utc>>,
    pub extension_count: u8,
    pub status: LoanStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 貸出Read Modelポート
#[allow(dead_code)]
#[async_trait]
pub trait LoanReadModel: Send + Sync {
    /// 貸出の現在状態をRead Modelに保存
    ///
    /// イベントストアから復元した集約の完全な状態を保存する。
    /// 新規の場合はINSERT、既存の場合はUPDATE（upsert）を実行する。
    ///
    /// イベントソーシングの原則として、Read Modelは常にイベントから
    /// 復元した集約の完全な状態を反映すべきであり、部分更新は行わない。
    async fn save(&self, loan_view: LoanView) -> Result<()>;

    /// 会員の貸出中の貸出を取得する
    ///
    /// 貸出上限（会員ごと最大5冊）の確認に使用される。
    async fn get_active_loans_for_member(&self, member_id: MemberId) -> Result<Vec<LoanView>>;

    /// 延滞候補の貸出を検索する
    ///
    /// due_date < cutoff_date かつ status が "active" の貸出を返す。
    /// バッチジョブでの延滞検知に使用される。
    async fn find_overdue_candidates(&self, cutoff_date: DateTime<Utc>) -> Result<Vec<LoanView>>;

    /// IDで貸出を取得する
    async fn get_by_id(&self, loan_id: LoanId) -> Result<Option<LoanView>>;

    /// 会員の全貸出を検索する
    ///
    /// 会員の貸出履歴表示に使用される。
    async fn find_by_member_id(&self, member_id: MemberId) -> Result<Vec<LoanView>>;
}
