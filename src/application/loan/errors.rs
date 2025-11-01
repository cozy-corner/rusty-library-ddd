use std::fmt;

/// 貸出管理アプリケーション層のエラー
#[derive(Debug)]
#[allow(dead_code)]
pub enum LoanApplicationError {
    /// 会員が存在しない
    MemberNotFound,
    /// 書籍が貸出不可
    BookNotAvailable,
    /// 会員に延滞中の貸出がある
    MemberHasOverdueLoan,
    /// 貸出上限（5冊）を超えている
    LoanLimitExceeded,
    /// 貸出が見つからない
    LoanNotFound,
    /// 貸出の状態が不正（例: Activeを期待したがReturnedだった）
    InvalidLoanState(String),
    /// ドメイン層のエラー
    DomainError(String),
    /// ポート層（I/O）のエラー
    PortError(String),
}

impl fmt::Display for LoanApplicationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MemberNotFound => write!(f, "Member not found"),
            Self::BookNotAvailable => write!(f, "Book is not available for loan"),
            Self::MemberHasOverdueLoan => write!(f, "Member has overdue loan"),
            Self::LoanLimitExceeded => write!(f, "Loan limit exceeded (max 5 books)"),
            Self::LoanNotFound => write!(f, "Loan not found"),
            Self::InvalidLoanState(msg) => write!(f, "Invalid loan state: {}", msg),
            Self::DomainError(msg) => write!(f, "Domain error: {}", msg),
            Self::PortError(msg) => write!(f, "Port error: {}", msg),
        }
    }
}

impl std::error::Error for LoanApplicationError {}

/// アプリケーション層の Result型
#[allow(dead_code)]
pub type Result<T> = std::result::Result<T, LoanApplicationError>;
