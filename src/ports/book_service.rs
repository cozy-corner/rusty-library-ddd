use crate::domain::value_objects::BookId;
use async_trait::async_trait;

#[allow(dead_code)]
pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// 書籍サービスポート
///
/// 貸出コンテキストとカタログコンテキストの境界を維持する。
/// 貸出コンテキストはBookIDのみを知り、書籍詳細は知らない。
#[allow(dead_code)]
#[async_trait]
pub trait BookService: Send + Sync {
    /// 書籍が貸出可能か確認する
    ///
    /// ビジネスルール: 貸出不可の書籍は貸し出せない。
    async fn is_available_for_loan(&self, book_id: BookId) -> Result<bool>;

    /// 書籍タイトルを取得する
    ///
    /// 通知メッセージでわかりやすい表示をするために使用される。
    async fn get_book_title(&self, book_id: BookId) -> Result<String>;
}
