#![allow(dead_code)]

use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// 貸出ID - 貸出管理コンテキストの集約ID
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct LoanId(Uuid);

impl LoanId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    pub fn value(&self) -> Uuid {
        self.0
    }
}

impl Default for LoanId {
    fn default() -> Self {
        Self::new()
    }
}

/// 書籍ID - カタログ管理コンテキストへの参照
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct BookId(Uuid);

impl BookId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    pub fn value(&self) -> Uuid {
        self.0
    }
}

impl Default for BookId {
    fn default() -> Self {
        Self::new()
    }
}

/// 会員ID - 会員管理コンテキストへの参照
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct MemberId(Uuid);

impl MemberId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    pub fn value(&self) -> Uuid {
        self.0
    }
}

impl Default for MemberId {
    fn default() -> Self {
        Self::new()
    }
}

/// 職員ID - 職員管理コンテキストへの参照
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct StaffId(Uuid);

impl StaffId {
    pub fn new() -> Self {
        Self(Uuid::new_v4())
    }

    pub fn from_uuid(uuid: Uuid) -> Self {
        Self(uuid)
    }

    pub fn value(&self) -> Uuid {
        self.0
    }
}

impl Default for StaffId {
    fn default() -> Self {
        Self::new()
    }
}

/// 延長回数エラー
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExtensionError {
    /// 延長回数の上限を超えた
    LimitExceeded,
}

/// 延長回数
///
/// 不変条件：延長は1回まで（公立図書館のビジネスルール）
/// 型システムでこの制約を強制し、不正な値（2以上）を作成できないようにする。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ExtensionCount(u8);

impl ExtensionCount {
    /// 新規作成（0回）
    pub fn new() -> Self {
        Self(0)
    }

    /// 延長回数を増やす
    ///
    /// # エラー
    /// 既に1回延長済みの場合は`ExtensionError::LimitExceeded`を返す
    pub fn increment(self) -> Result<Self, ExtensionError> {
        if self.0 >= 1 {
            return Err(ExtensionError::LimitExceeded);
        }
        Ok(Self(self.0 + 1))
    }

    /// 現在の回数
    pub fn value(&self) -> u8 {
        self.0
    }

    /// 延長可能か（まだ延長していないか）
    pub fn can_extend(&self) -> bool {
        self.0 < 1
    }
}

impl Default for ExtensionCount {
    fn default() -> Self {
        Self::new()
    }
}

impl TryFrom<u8> for ExtensionCount {
    type Error = ExtensionError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        if value > 1 {
            return Err(ExtensionError::LimitExceeded);
        }
        Ok(Self(value))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // TDD: ExtensionCount のテスト
    #[test]
    fn test_extension_count_new() {
        let count = ExtensionCount::new();
        assert_eq!(count.value(), 0);
    }

    #[test]
    fn test_extension_count_can_extend_initially() {
        let count = ExtensionCount::new();
        assert!(count.can_extend());
    }

    #[test]
    fn test_extension_count_increment_success() {
        let count = ExtensionCount::new();
        let result = count.increment();
        assert!(result.is_ok());
        let new_count = result.unwrap();
        assert_eq!(new_count.value(), 1);
    }

    #[test]
    fn test_extension_count_cannot_extend_after_one() {
        let count = ExtensionCount::new().increment().unwrap();
        assert!(!count.can_extend());
    }

    #[test]
    fn test_extension_count_increment_fails_after_one() {
        let count = ExtensionCount::new().increment().unwrap();
        let result = count.increment();
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), ExtensionError::LimitExceeded);
    }

    // ID value objects のテスト
    #[test]
    fn test_loan_id_creation() {
        let id1 = LoanId::new();
        let id2 = LoanId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_loan_id_from_uuid() {
        let uuid = Uuid::new_v4();
        let id = LoanId::from_uuid(uuid);
        assert_eq!(id.value(), uuid);
    }

    #[test]
    fn test_book_id_creation() {
        let id1 = BookId::new();
        let id2 = BookId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_member_id_creation() {
        let id1 = MemberId::new();
        let id2 = MemberId::new();
        assert_ne!(id1, id2);
    }

    #[test]
    fn test_staff_id_creation() {
        let id1 = StaffId::new();
        let id2 = StaffId::new();
        assert_ne!(id1, id2);
    }

    // TDD: ExtensionCount TryFrom のテスト
    #[test]
    fn test_extension_count_try_from_valid() {
        let count = ExtensionCount::try_from(0);
        assert!(count.is_ok());
        assert_eq!(count.unwrap().value(), 0);

        let count = ExtensionCount::try_from(1);
        assert!(count.is_ok());
        assert_eq!(count.unwrap().value(), 1);
    }

    #[test]
    fn test_extension_count_try_from_invalid() {
        let count = ExtensionCount::try_from(2);
        assert!(count.is_err());
        assert_eq!(count.unwrap_err(), ExtensionError::LimitExceeded);

        let count = ExtensionCount::try_from(255);
        assert!(count.is_err());
    }
}
