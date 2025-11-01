use crate::domain::value_objects::MemberId;
use crate::ports::notification_service::{NotificationService as NotificationServiceTrait, Result};
use async_trait::async_trait;
use chrono::{DateTime, Utc};

/// Mock implementation of NotificationService
///
/// Does not send actual notifications.
/// Simply succeeds without performing any action.
#[allow(dead_code)]
pub struct NotificationService;

#[allow(dead_code)]
impl NotificationService {
    pub fn new() -> Self {
        Self
    }
}

impl Default for NotificationService {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl NotificationServiceTrait for NotificationService {
    /// Mock overdue notification (does nothing)
    async fn send_overdue_notification(
        &self,
        _member_id: MemberId,
        _book_title: &str,
        _due_date: DateTime<Utc>,
    ) -> Result<()> {
        Ok(())
    }

    /// Mock extension confirmation (does nothing)
    async fn send_extension_confirmation(
        &self,
        _member_id: MemberId,
        _book_title: &str,
        _new_due_date: DateTime<Utc>,
    ) -> Result<()> {
        Ok(())
    }

    /// Mock return confirmation (does nothing)
    async fn send_return_confirmation(
        &self,
        _member_id: MemberId,
        _book_title: &str,
        _was_overdue: bool,
    ) -> Result<()> {
        Ok(())
    }
}
