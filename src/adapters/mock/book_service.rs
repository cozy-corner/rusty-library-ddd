use crate::domain::value_objects::BookId;
use crate::ports::book_service::{BookService as BookServiceTrait, Result};
use async_trait::async_trait;
use std::collections::HashSet;
use std::sync::Mutex;

/// BookServiceのモック実装
///
/// 書籍IDを保存することで状態を持ったテストをサポート。
/// 貸出可能な書籍を登録可能。
#[allow(dead_code)]
pub struct BookService {
    available_books: Mutex<HashSet<BookId>>,
}

#[allow(dead_code)]
impl BookService {
    pub fn new() -> Self {
        Self {
            available_books: Mutex::new(HashSet::new()),
        }
    }

    /// テスト用に貸出可能な書籍を登録
    pub fn add_available_book(&self, book_id: BookId) {
        self.available_books.lock().unwrap().insert(book_id);
    }
}

impl Default for BookService {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl BookServiceTrait for BookService {
    /// 登録された書籍の中で貸出可能かチェック
    async fn is_available_for_loan(&self, book_id: BookId) -> Result<bool> {
        Ok(self.available_books.lock().unwrap().contains(&book_id))
    }

    /// 固定の書籍タイトルを返す
    async fn get_book_title(&self, _book_id: BookId) -> Result<String> {
        Ok("Mock Book Title".to_string())
    }
}
