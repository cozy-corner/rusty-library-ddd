use crate::domain::value_objects::MemberId;
use crate::ports::member_service::{MemberService as MemberServiceTrait, Result};
use async_trait::async_trait;
use std::collections::HashSet;
use std::sync::Mutex;

/// Mock implementation of MemberService
///
/// Supports stateful testing by storing member IDs.
/// Can register members and mark them as having overdue loans.
#[allow(dead_code)]
pub struct MemberService {
    existing_members: Mutex<HashSet<MemberId>>,
    overdue_members: Mutex<HashSet<MemberId>>,
}

#[allow(dead_code)]
impl MemberService {
    pub fn new() -> Self {
        Self {
            existing_members: Mutex::new(HashSet::new()),
            overdue_members: Mutex::new(HashSet::new()),
        }
    }

    /// Add a member for testing purposes
    pub fn add_member(&self, member_id: MemberId) {
        self.existing_members.lock().unwrap().insert(member_id);
    }

    /// Mark a member as having overdue loans for testing purposes
    pub fn mark_overdue(&self, member_id: MemberId) {
        self.overdue_members.lock().unwrap().insert(member_id);
    }
}

impl Default for MemberService {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl MemberServiceTrait for MemberService {
    /// Check if member exists in the registered members
    async fn exists(&self, member_id: MemberId) -> Result<bool> {
        Ok(self.existing_members.lock().unwrap().contains(&member_id))
    }

    /// Check if member has overdue loans
    async fn has_overdue_loans(&self, member_id: MemberId) -> Result<bool> {
        Ok(self.overdue_members.lock().unwrap().contains(&member_id))
    }
}
