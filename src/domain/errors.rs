#![allow(dead_code)]

use super::ExtensionError;

/// 貸出のエラー
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoanBookError {
    // 現時点では発生しないが、将来的にアプリケーション層で追加される可能性
    // 例: MemberNotFound, BookNotAvailable, MemberHasOverdueLoan など
}

/// 延長のエラー
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExtendLoanError {
    /// 既に返却済み
    AlreadyReturned,
    /// 延長回数の上限を超えた
    ExtensionLimitExceeded,
    /// 延滞中のため延長不可
    CannotExtendOverdue,
}

impl From<ExtensionError> for ExtendLoanError {
    fn from(err: ExtensionError) -> Self {
        match err {
            ExtensionError::LimitExceeded => ExtendLoanError::ExtensionLimitExceeded,
        }
    }
}

/// 返却のエラー
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ReturnBookError {
    /// 既に返却済み
    AlreadyReturned,
}
