use crate::domain::{self, events::*};

use super::errors::{LoanApplicationError, Result};
use super::loan_service::{ServiceDependencies, build_loan_view};

/// 延滞検出バッチ（純粋な関数）
///
/// 定期的に実行され、延滞した貸出を検出してLoanBecameOverdueイベントを発行する。
///
/// ビジネスルール：
/// - 返却期限（due_date）を過ぎたActive状態の貸出を延滞とする
/// - 既にOverdue状態の貸出は処理しない（重複イベント防止）
/// - Returned状態の貸出は処理しない
///
/// すべての依存が引数として明示的に渡される（関数型の原則）。
///
/// 処理フロー：
/// 1. Read Modelから延滞候補を取得
/// 2. 各候補について：
///    - イベントストアから完全な履歴を取得
///    - イベントから現在の状態を復元
///    - Active状態かつ延滞している場合のみ処理
///    - LoanBecameOverdueイベントを生成・保存
///    - Read Modelを更新
/// 3. 処理件数を返す
///
/// # 引数
/// * `deps` - サービスの依存関係
///
/// # 戻り値
/// 延滞として検出した貸出の件数
///
/// # エラー
/// ポート層のI/Oエラー（EventStore, LoanReadModel）
#[allow(dead_code)]
pub async fn detect_overdue_loans(deps: &ServiceDependencies) -> Result<usize> {
    let now = chrono::Utc::now();
    let mut detected_count = 0;

    // 1. Read Modelから延滞候補を取得
    let candidates = deps
        .loan_read_model
        .find_overdue_candidates(now)
        .await
        .map_err(LoanApplicationError::ReadModelError)?;

    // 2. 各候補について延滞判定
    for loan_view in candidates {
        // 2.1. イベントストアから完全な履歴を取得
        let events = deps
            .event_store
            .load(loan_view.loan_id.value())
            .await
            .map_err(LoanApplicationError::EventStoreError)?;

        // 2.2. イベントから現在の状態を復元
        let loan = match domain::loan::replay_events(&events) {
            Some(loan) => loan,
            None => continue, // イベントがない場合はスキップ
        };

        // 2.3. ActiveLoanかつ延滞している場合のみ処理
        match loan {
            domain::loan::Loan::Active(active) => {
                // 延滞判定
                if domain::loan::is_overdue(&domain::loan::Loan::Active(active.clone()), now) {
                    // LoanBecameOverdueイベントを生成
                    let event = LoanBecameOverdue {
                        loan_id: active.loan_id,
                        book_id: active.book_id,
                        member_id: active.member_id,
                        due_date: active.due_date,
                        detected_at: now,
                    };

                    // イベントストアに保存
                    deps.event_store
                        .append(
                            active.loan_id.value(),
                            "Loan",
                            vec![DomainEvent::LoanBecameOverdue(event.clone())],
                        )
                        .await
                        .map_err(LoanApplicationError::EventStoreError)?;

                    // Read Modelを更新（完全な状態を保存）
                    // イベントを適用して更新後の状態を取得
                    let updated_loan = domain::loan::apply_event(
                        Some(domain::loan::Loan::Active(active)),
                        &DomainEvent::LoanBecameOverdue(event),
                    );
                    let loan_view = build_loan_view(&updated_loan);
                    deps.loan_read_model
                        .save(loan_view)
                        .await
                        .map_err(LoanApplicationError::ReadModelError)?;

                    detected_count += 1;
                }
            }
            // Overdue, Returnedの場合はスキップ
            _ => continue,
        }
    }

    Ok(detected_count)
}
