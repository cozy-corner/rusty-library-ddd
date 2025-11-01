use crate::domain::value_objects::MemberId;
use async_trait::async_trait;
use chrono::{DateTime, Utc};

#[allow(dead_code)]
pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// 通知サービスポート
///
/// 会員への通知配信メカニズムを抽象化する。
/// 実装はメール、SMS、プッシュ通知などが考えられる。
#[allow(dead_code)]
#[async_trait]
pub trait NotificationService: Send + Sync {
    /// 延滞通知を会員に送信する
    ///
    /// LoanBecameOverdueイベント処理時に呼ばれる。
    async fn send_overdue_notification(
        &self,
        member_id: MemberId,
        book_title: &str,
        due_date: DateTime<Utc>,
    ) -> Result<()>;

    /// 延長確認通知を会員に送信する
    ///
    /// LoanExtendedイベント処理時に呼ばれる。
    async fn send_extension_confirmation(
        &self,
        member_id: MemberId,
        book_title: &str,
        new_due_date: DateTime<Utc>,
    ) -> Result<()>;

    /// 返却確認通知を会員に送信する
    ///
    /// BookReturnedイベント処理時に呼ばれる。
    async fn send_return_confirmation(
        &self,
        member_id: MemberId,
        book_title: &str,
        was_overdue: bool,
    ) -> Result<()>;
}
