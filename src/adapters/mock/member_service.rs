use crate::domain::value_objects::MemberId;
use crate::ports::member_service::{MemberService as MemberServiceTrait, Result};
use async_trait::async_trait;

/// Mock implementation of MemberService
///
/// Returns fixed values for testing purposes.
/// Does not store any data.
#[allow(dead_code)]
pub struct MemberService;

#[allow(dead_code)]
impl MemberService {
    pub fn new() -> Self {
        Self
    }
}

impl Default for MemberService {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl MemberServiceTrait for MemberService {
    /// Always returns true (member exists)
    async fn exists(&self, _member_id: MemberId) -> Result<bool> {
        Ok(true)
    }

    /// Always returns false (no overdue loans)
    async fn has_overdue_loans(&self, _member_id: MemberId) -> Result<bool> {
        Ok(false)
    }
}
