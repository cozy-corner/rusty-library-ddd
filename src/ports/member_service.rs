use crate::domain::value_objects::MemberId;
use async_trait::async_trait;

#[allow(dead_code)]
pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// 会員サービスポート
///
/// 貸出コンテキストと会員コンテキストの境界を維持する。
/// 貸出コンテキストはMemberIDのみを知り、会員詳細は知らない。
#[allow(dead_code)]
#[async_trait]
pub trait MemberService: Send + Sync {
    /// 会員が存在するか確認する
    ///
    /// 貸出作成前の会員バリデーションに使用される。
    async fn exists(&self, member_id: MemberId) -> Result<bool>;

    /// 会員が延滞中の貸出を持っているか確認する
    ///
    /// ビジネスルール: 延滞中の会員には貸出不可。
    async fn has_overdue_loans(&self, member_id: MemberId) -> Result<bool>;
}
