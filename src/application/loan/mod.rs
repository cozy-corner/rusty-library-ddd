mod errors;
mod loan_service;
mod overdue_detection;

#[allow(unused_imports)]
pub use errors::{LoanApplicationError, Result};
#[allow(unused_imports)]
pub use loan_service::{ServiceDependencies, extend_loan, loan_book, return_book};
#[allow(unused_imports)]
pub use overdue_detection::detect_overdue_loans;
