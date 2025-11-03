use crate::domain::events::DomainEvent;
use crate::domain::loan::Loan;
use crate::ports::loan_read_model::{LoanReadModel, LoanStatus, LoanView};

/// ドメインイベントをRead Modelに投影する
///
/// イベントから集約の状態を再構築し、loans_viewテーブルに反映する。
///
/// # イベントソーシングの原則
///
/// プロジェクターはイベントソーシングパターンに従う：
/// 1. イベントが真実の情報源
/// 2. Read Modelはイベントから導出される
/// 3. 各イベントは完全な状態再構築をトリガーする
/// 4. Read Modelは集約の完全な状態で更新される
///
/// # 引数
/// * `read_model` - 更新するRead Model
/// * `events` - 集約の全イベント（時系列順）
///
/// # 戻り値
/// 成功または失敗を示すResult
#[allow(dead_code)]
pub async fn project_loan_events(
    read_model: &dyn LoanReadModel,
    events: &[DomainEvent],
) -> Result<(), Box<dyn std::error::Error + Send + Sync>> {
    if events.is_empty() {
        return Ok(());
    }

    // 全イベントから集約の状態を再構築
    let loan = crate::domain::loan::replay_events(events)
        .ok_or("Failed to reconstruct loan from events")?;

    // LoanViewに変換して保存
    let loan_view = build_loan_view_from_aggregate(&loan);
    read_model.save(loan_view).await?;

    Ok(())
}

/// Loan集約からLoanViewを構築
///
/// ドメイン集約の状態をRead Modelビューに変換する。
/// アプリケーション層のbuild_loan_view()と同じ役割を持つが、
/// アダプター層の関心事として独立して保守される。
#[allow(dead_code)]
fn build_loan_view_from_aggregate(loan: &Loan) -> LoanView {
    match loan {
        Loan::Active(active) => LoanView {
            loan_id: active.loan_id,
            book_id: active.book_id,
            member_id: active.member_id,
            loaned_at: active.loaned_at,
            due_date: active.due_date,
            returned_at: None,
            extension_count: active.extension_count.value(),
            status: LoanStatus::Active,
            created_at: active.created_at,
            updated_at: active.updated_at,
        },
        Loan::Overdue(overdue) => LoanView {
            loan_id: overdue.loan_id,
            book_id: overdue.book_id,
            member_id: overdue.member_id,
            loaned_at: overdue.loaned_at,
            due_date: overdue.due_date,
            returned_at: None,
            extension_count: overdue.extension_count.value(),
            status: LoanStatus::Overdue,
            created_at: overdue.created_at,
            updated_at: overdue.updated_at,
        },
        Loan::Returned(returned) => LoanView {
            loan_id: returned.loan_id,
            book_id: returned.book_id,
            member_id: returned.member_id,
            loaned_at: returned.loaned_at,
            due_date: returned.due_date,
            returned_at: Some(returned.returned_at),
            extension_count: returned.extension_count.value(),
            status: LoanStatus::Returned,
            created_at: returned.created_at,
            updated_at: returned.updated_at,
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::events::{BookLoaned, BookReturned, LoanBecameOverdue, LoanExtended};
    use crate::domain::value_objects::{BookId, LoanId, MemberId, StaffId};
    use crate::ports::loan_read_model::LoanView;
    use chrono::Utc;
    use std::collections::HashMap;
    use std::sync::{Arc, Mutex};

    /// Mock LoanReadModel for testing
    struct MockLoanReadModel {
        loans: Arc<Mutex<HashMap<LoanId, LoanView>>>,
    }

    impl MockLoanReadModel {
        fn new() -> Self {
            Self {
                loans: Arc::new(Mutex::new(HashMap::new())),
            }
        }

        fn get(&self, loan_id: LoanId) -> Option<LoanView> {
            self.loans.lock().unwrap().get(&loan_id).cloned()
        }
    }

    #[async_trait::async_trait]
    impl LoanReadModel for MockLoanReadModel {
        async fn save(&self, loan_view: LoanView) -> crate::ports::loan_read_model::Result<()> {
            self.loans
                .lock()
                .unwrap()
                .insert(loan_view.loan_id, loan_view);
            Ok(())
        }

        async fn get_active_loans_for_member(
            &self,
            _member_id: MemberId,
        ) -> crate::ports::loan_read_model::Result<Vec<LoanView>> {
            unimplemented!()
        }

        async fn find_overdue_candidates(
            &self,
            _cutoff_date: chrono::DateTime<Utc>,
        ) -> crate::ports::loan_read_model::Result<Vec<LoanView>> {
            unimplemented!()
        }

        async fn get_by_id(
            &self,
            loan_id: LoanId,
        ) -> crate::ports::loan_read_model::Result<Option<LoanView>> {
            Ok(self.get(loan_id))
        }

        async fn find_by_member_id(
            &self,
            _member_id: MemberId,
        ) -> crate::ports::loan_read_model::Result<Vec<LoanView>> {
            unimplemented!()
        }
    }

    #[tokio::test]
    async fn test_project_book_loaned_event() {
        let read_model = MockLoanReadModel::new();
        let loan_id = LoanId::new();
        let book_id = BookId::new();
        let member_id = MemberId::new();
        let staff_id = StaffId::new();
        let now = Utc::now();

        let events = vec![DomainEvent::BookLoaned(BookLoaned {
            loan_id,
            book_id,
            member_id,
            loaned_at: now,
            due_date: now + chrono::Duration::days(14),
            loaned_by: staff_id,
        })];

        project_loan_events(&read_model, &events).await.unwrap();

        let loan_view = read_model.get(loan_id).unwrap();
        assert_eq!(loan_view.loan_id, loan_id);
        assert_eq!(loan_view.status, LoanStatus::Active);
        assert_eq!(loan_view.extension_count, 0);
        assert!(loan_view.returned_at.is_none());
    }

    #[tokio::test]
    async fn test_project_loan_extended_event() {
        let read_model = MockLoanReadModel::new();
        let loan_id = LoanId::new();
        let book_id = BookId::new();
        let member_id = MemberId::new();
        let staff_id = StaffId::new();
        let now = Utc::now();
        let old_due_date = now + chrono::Duration::days(14);
        let new_due_date = old_due_date + chrono::Duration::days(14);

        let events = vec![
            DomainEvent::BookLoaned(BookLoaned {
                loan_id,
                book_id,
                member_id,
                loaned_at: now,
                due_date: old_due_date,
                loaned_by: staff_id,
            }),
            DomainEvent::LoanExtended(LoanExtended {
                loan_id,
                old_due_date,
                new_due_date,
                extended_at: now + chrono::Duration::days(5),
                extension_count: 1,
            }),
        ];

        project_loan_events(&read_model, &events).await.unwrap();

        let loan_view = read_model.get(loan_id).unwrap();
        assert_eq!(loan_view.status, LoanStatus::Active);
        assert_eq!(loan_view.extension_count, 1);
        assert_eq!(loan_view.due_date, new_due_date);
    }

    #[tokio::test]
    async fn test_project_book_returned_event() {
        let read_model = MockLoanReadModel::new();
        let loan_id = LoanId::new();
        let book_id = BookId::new();
        let member_id = MemberId::new();
        let staff_id = StaffId::new();
        let now = Utc::now();
        let returned_at = now + chrono::Duration::days(7);

        let events = vec![
            DomainEvent::BookLoaned(BookLoaned {
                loan_id,
                book_id,
                member_id,
                loaned_at: now,
                due_date: now + chrono::Duration::days(14),
                loaned_by: staff_id,
            }),
            DomainEvent::BookReturned(BookReturned {
                loan_id,
                book_id,
                member_id,
                returned_at,
                was_overdue: false,
            }),
        ];

        project_loan_events(&read_model, &events).await.unwrap();

        let loan_view = read_model.get(loan_id).unwrap();
        assert_eq!(loan_view.status, LoanStatus::Returned);
        assert_eq!(loan_view.returned_at, Some(returned_at));
    }

    #[tokio::test]
    async fn test_project_loan_became_overdue_event() {
        let read_model = MockLoanReadModel::new();
        let loan_id = LoanId::new();
        let book_id = BookId::new();
        let member_id = MemberId::new();
        let staff_id = StaffId::new();
        let now = Utc::now();
        let due_date = now + chrono::Duration::days(14);
        let detected_at = now + chrono::Duration::days(20);

        let events = vec![
            DomainEvent::BookLoaned(BookLoaned {
                loan_id,
                book_id,
                member_id,
                loaned_at: now,
                due_date,
                loaned_by: staff_id,
            }),
            DomainEvent::LoanBecameOverdue(LoanBecameOverdue {
                loan_id,
                book_id,
                member_id,
                due_date,
                detected_at,
            }),
        ];

        project_loan_events(&read_model, &events).await.unwrap();

        let loan_view = read_model.get(loan_id).unwrap();
        assert_eq!(loan_view.status, LoanStatus::Overdue);
        assert!(loan_view.returned_at.is_none());
    }

    #[tokio::test]
    async fn test_project_empty_events() {
        let read_model = MockLoanReadModel::new();
        let events: Vec<DomainEvent> = vec![];

        let result = project_loan_events(&read_model, &events).await;
        assert!(result.is_ok());
    }
}
