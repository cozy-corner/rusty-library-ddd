use thiserror::Error;

/// 貸出管理アプリケーション層のエラー
#[derive(Debug, Error)]
pub enum LoanApplicationError {
    /// 会員が存在しない
    #[error("Member not found")]
    MemberNotFound,

    /// 書籍が貸出不可
    #[error("Book is not available for loan")]
    BookNotAvailable,

    /// 会員に延滞中の貸出がある
    #[error("Member has overdue loan")]
    MemberHasOverdueLoan,

    /// 貸出上限（5冊）を超えている
    #[error("Loan limit exceeded (max 5 books)")]
    LoanLimitExceeded,

    /// 貸出が見つからない
    #[error("Loan not found")]
    LoanNotFound,

    /// 貸出の状態が不正（例: Activeを期待したがReturnedだった）
    #[error("Invalid loan state: {0}")]
    InvalidLoanState(String),

    /// ドメイン層のエラー
    #[error("Domain error: {0}")]
    DomainError(String),

    /// EventStoreのエラー
    #[error("Event store error")]
    EventStoreError(#[source] Box<dyn std::error::Error + Send + Sync>),

    /// ReadModelのエラー
    #[error("Read model error")]
    ReadModelError(#[source] Box<dyn std::error::Error + Send + Sync>),

    /// MemberServiceのエラー
    #[error("Member service error")]
    MemberServiceError(#[source] Box<dyn std::error::Error + Send + Sync>),

    /// BookServiceのエラー
    #[error("Book service error")]
    BookServiceError(#[source] Box<dyn std::error::Error + Send + Sync>),
}

/// アプリケーション層の Result型
pub type Result<T> = std::result::Result<T, LoanApplicationError>;
