pub mod handlers;
pub mod types;

use axum::{Router, routing::get};

use handlers::ApiState;

/// 全てのクエリエンドポイントを含むAPIルーターを作成
///
/// エンドポイント:
/// - GET /loans/:id - 貸出詳細をIDで取得
/// - GET /loans - オプションフィルタ付き貸出一覧取得（member_id, status）
pub fn create_router(state: ApiState) -> Router {
    Router::new()
        .route("/loans/:id", get(handlers::get_loan_by_id))
        .route("/loans", get(handlers::list_loans))
        .with_state(state)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::value_objects::{BookId, LoanId, MemberId};
    use crate::ports::loan_read_model::{LoanReadModel, LoanStatus, LoanView};
    use async_trait::async_trait;
    use chrono::Utc;
    use std::sync::Arc;

    // テスト用のモック実装
    struct MockLoanReadModel {
        loans: Vec<LoanView>,
    }

    type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

    #[async_trait]
    impl LoanReadModel for MockLoanReadModel {
        async fn save(&self, _loan_view: LoanView) -> Result<()> {
            Ok(())
        }

        async fn get_active_loans_for_member(&self, member_id: MemberId) -> Result<Vec<LoanView>> {
            Ok(self
                .loans
                .iter()
                .filter(|loan| loan.member_id == member_id && loan.status == LoanStatus::Active)
                .cloned()
                .collect())
        }

        async fn find_overdue_candidates(
            &self,
            _cutoff_date: chrono::DateTime<Utc>,
        ) -> Result<Vec<LoanView>> {
            Ok(vec![])
        }

        async fn get_by_id(&self, loan_id: LoanId) -> Result<Option<LoanView>> {
            Ok(self
                .loans
                .iter()
                .find(|loan| loan.loan_id == loan_id)
                .cloned())
        }

        async fn find_by_member_id(&self, member_id: MemberId) -> Result<Vec<LoanView>> {
            Ok(self
                .loans
                .iter()
                .filter(|loan| loan.member_id == member_id)
                .cloned()
                .collect())
        }
    }

    fn create_test_loan_view(loan_id: LoanId, member_id: MemberId, status: LoanStatus) -> LoanView {
        let now = Utc::now();
        LoanView {
            loan_id,
            book_id: BookId::new(),
            member_id,
            loaned_at: now,
            due_date: now,
            returned_at: None,
            extension_count: 0,
            status,
            created_at: now,
            updated_at: now,
        }
    }

    #[tokio::test]
    async fn test_router_can_be_created() {
        let mock_read_model = MockLoanReadModel { loans: vec![] };
        let state = ApiState {
            loan_read_model: Arc::new(mock_read_model),
        };

        let _router = create_router(state);
    }

    #[tokio::test]
    async fn test_mock_read_model_works() {
        let loan_id = LoanId::new();
        let member_id = MemberId::new();
        let loan = create_test_loan_view(loan_id, member_id, LoanStatus::Active);

        let mock_read_model = MockLoanReadModel {
            loans: vec![loan.clone()],
        };

        // Test get_by_id
        let result = mock_read_model.get_by_id(loan_id).await.unwrap();
        assert!(result.is_some());
        assert_eq!(result.unwrap().loan_id, loan_id);

        // Test find_by_member_id
        let results = mock_read_model.find_by_member_id(member_id).await.unwrap();
        assert_eq!(results.len(), 1);
        assert_eq!(results[0].member_id, member_id);
    }
}
