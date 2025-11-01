use crate::domain::value_objects::BookId;
use crate::ports::book_service::{BookService as BookServiceTrait, Result};
use async_trait::async_trait;

/// Mock implementation of BookService
///
/// Returns fixed values for testing purposes.
/// Does not store any data.
#[allow(dead_code)]
pub struct BookService;

#[allow(dead_code)]
impl BookService {
    pub fn new() -> Self {
        Self
    }
}

impl Default for BookService {
    fn default() -> Self {
        Self::new()
    }
}

#[async_trait]
impl BookServiceTrait for BookService {
    /// Always returns true (book is available for loan)
    async fn is_available_for_loan(&self, _book_id: BookId) -> Result<bool> {
        Ok(true)
    }

    /// Returns a fixed book title
    async fn get_book_title(&self, _book_id: BookId) -> Result<String> {
        Ok("Mock Book Title".to_string())
    }
}
