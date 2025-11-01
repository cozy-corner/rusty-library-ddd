pub mod book_service;
pub mod event_store;
pub mod loan_read_model;
pub mod member_service;
pub mod notification_service;

// 明示的に型を再エクスポート（Result型の衝突を避けるため、グロブインポートを使わない）
pub use book_service::BookService;
pub use event_store::EventStore;
pub use loan_read_model::{LoanReadModel, LoanStatus, LoanView};
pub use member_service::MemberService;
#[allow(unused_imports)] // 将来のAPI層で使用予定
pub use notification_service::NotificationService;
