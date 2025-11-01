use crate::domain::value_objects::BookId;
use crate::ports::book_service::{BookService as BookServiceTrait, Result};
use async_trait::async_trait;
use std::collections::HashSet;
use std::sync::Mutex;

/// Mock implementation of BookService
///
/// Supports stateful testing by storing book IDs.
/// Can register books as available for loan.
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

    /// Add a book as available for loan for testing purposes
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
    /// Check if book is available in the registered books
    async fn is_available_for_loan(&self, book_id: BookId) -> Result<bool> {
        Ok(self.available_books.lock().unwrap().contains(&book_id))
    }

    /// Returns a fixed book title
    async fn get_book_title(&self, _book_id: BookId) -> Result<String> {
        Ok("Mock Book Title".to_string())
    }
}
