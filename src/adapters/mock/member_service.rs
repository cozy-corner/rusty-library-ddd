use crate::domain::value_objects::MemberId;
use crate::ports::member_service::{MemberService as MemberServiceTrait, Result};
use async_trait::async_trait;
use std::collections::HashSet;
use std::sync::Mutex;

/// MemberServiceのモック実装
///
/// 会員IDを保存することで状態を持ったテストをサポート。
/// 会員登録や延滞マークが可能。
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

    /// テスト用に会員を登録
    pub fn add_member(&self, member_id: MemberId) {
        self.existing_members.lock().unwrap().insert(member_id);
    }

    /// テスト用に会員を延滞状態にマーク
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
    /// 登録された会員の中に存在するかチェック
    async fn exists(&self, member_id: MemberId) -> Result<bool> {
        Ok(self.existing_members.lock().unwrap().contains(&member_id))
    }

    /// 会員が延滞中の貸出を持っているかチェック
    async fn has_overdue_loans(&self, member_id: MemberId) -> Result<bool> {
        Ok(self.overdue_members.lock().unwrap().contains(&member_id))
    }
}
