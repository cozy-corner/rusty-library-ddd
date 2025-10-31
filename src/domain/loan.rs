#![allow(dead_code)]

use chrono::{DateTime, Duration, Utc};
use serde::{Deserialize, Serialize};

use super::{
    BookId, BookLoaned, BookReturned, DomainEvent, ExtendLoanError, ExtensionCount, LoanBookError,
    LoanExtended, LoanId, MemberId, ReturnBookError, StaffId,
};

/// 貸出期間（日数）
pub const LOAN_PERIOD_DAYS: i64 = 14;

// ============================================================================
// 型安全な状態パターン
// ============================================================================

/// Loan集約の共通フィールド
///
/// すべての貸出状態（Active, Overdue, Returned）で共有されるコアデータ。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct LoanCore {
    // 識別子
    pub loan_id: LoanId,

    // 他の集約への参照（IDのみ）
    pub book_id: BookId,
    pub member_id: MemberId,

    // 貸出管理の責務
    pub loaned_at: DateTime<Utc>,
    pub due_date: DateTime<Utc>,
    pub extension_count: ExtensionCount,

    // 監査情報
    pub created_by: StaffId,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 貸出中状態
///
/// ビジネスルール：
/// - 返却期限内
/// - 延長可能（extension_count < 1）
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ActiveLoan {
    #[serde(flatten)]
    pub core: LoanCore,
}

impl std::ops::Deref for ActiveLoan {
    type Target = LoanCore;

    fn deref(&self) -> &Self::Target {
        &self.core
    }
}

/// 延滞中状態
///
/// ビジネスルール：
/// - 返却期限を過ぎている
/// - 延長不可
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OverdueLoan {
    #[serde(flatten)]
    pub core: LoanCore,
}

impl std::ops::Deref for OverdueLoan {
    type Target = LoanCore;

    fn deref(&self) -> &Self::Target {
        &self.core
    }
}

/// 返却済み状態
///
/// ビジネスルール：
/// - returned_atが必須（型で保証）
/// - 操作不可（読み取り専用）
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ReturnedLoan {
    #[serde(flatten)]
    pub core: LoanCore,
    pub returned_at: DateTime<Utc>,
}

impl std::ops::Deref for ReturnedLoan {
    type Target = LoanCore;

    fn deref(&self) -> &Self::Target {
        &self.core
    }
}

/// Loan集約の統合型
///
/// 型安全な状態パターン：
/// - 不正な状態を型システムで排除
/// - 状態遷移を明示的に表現
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "status")]
pub enum Loan {
    Active(ActiveLoan),
    Overdue(OverdueLoan),
    Returned(ReturnedLoan),
}

// ============================================================================
// 型安全な純粋関数
// ============================================================================

/// 純粋関数：書籍を貸し出す
///
/// ビジネスルール：
/// - 貸出期間は14日間
/// - 状態はActive
/// - 延長回数は0
///
/// 副作用なし。新しいActiveLoanとイベントを返す。
pub fn loan_book(
    book_id: BookId,
    member_id: MemberId,
    loaned_at: DateTime<Utc>,
    staff_id: StaffId,
) -> Result<(ActiveLoan, BookLoaned), LoanBookError> {
    let loan_id = LoanId::new();
    let due_date = loaned_at + Duration::days(LOAN_PERIOD_DAYS);

    let loan = ActiveLoan {
        core: LoanCore {
            loan_id,
            book_id,
            member_id,
            loaned_at,
            due_date,
            extension_count: ExtensionCount::new(),
            created_by: staff_id,
            created_at: loaned_at,
            updated_at: loaned_at,
        },
    };

    let event = BookLoaned {
        loan_id,
        book_id,
        member_id,
        loaned_at,
        due_date,
        loaned_by: staff_id,
    };

    Ok((loan, event))
}

/// 純粋関数：貸出を延長する
///
/// ビジネスルール：
/// - 延長は1回まで
/// - ActiveLoanのみ受け付ける（型で保証）
/// - 延長時：現在の返却期限 + 14日間
///
/// 副作用なし。新しいActiveLoanとイベントを返す。
pub fn extend_loan(
    loan: ActiveLoan,
    extended_at: DateTime<Utc>,
) -> Result<(ActiveLoan, LoanExtended), ExtendLoanError> {
    // バリデーション：延長可能か（回数制限）
    if !loan.extension_count.can_extend() {
        return Err(ExtendLoanError::ExtensionLimitExceeded);
    }

    // 新しい返却期限を計算（必要な値を先に確保してから move）
    let loan_id = loan.loan_id;
    let old_due_date = loan.due_date;
    let new_due_date = old_due_date + Duration::days(LOAN_PERIOD_DAYS);
    let new_extension_count = loan.extension_count.increment()?;

    // 新しいActiveLoanを生成
    let new_loan = ActiveLoan {
        core: LoanCore {
            due_date: new_due_date,
            extension_count: new_extension_count,
            updated_at: extended_at,
            ..loan.core
        },
    };

    let event = LoanExtended {
        loan_id,
        old_due_date,
        new_due_date,
        extended_at,
        extension_count: new_extension_count.value(),
    };

    Ok((new_loan, event))
}

/// 純粋関数：書籍を返却する
///
/// ビジネスルール：
/// - ActiveまたはOverdueLoanを受け付ける
/// - 延滞していても返却は受け付ける
/// - 延滞料金なし（公立図書館）
///
/// 副作用なし。ReturnedLoanとイベントを返す。
pub fn return_book(
    loan: Loan,
    returned_at: DateTime<Utc>,
) -> Result<(ReturnedLoan, BookReturned), ReturnBookError> {
    match loan {
        Loan::Active(active) => {
            // 先にID類を取り出してから core を move
            let loan_id = active.loan_id;
            let book_id = active.book_id;
            let member_id = active.member_id;
            let was_overdue = returned_at > active.due_date;

            let returned_loan = ReturnedLoan {
                core: LoanCore {
                    updated_at: returned_at,
                    ..active.core
                },
                returned_at,
            };

            let event = BookReturned {
                loan_id,
                book_id,
                member_id,
                returned_at,
                was_overdue,
            };

            Ok((returned_loan, event))
        }
        Loan::Overdue(overdue) => {
            // 先にID類を取り出してから core を move
            let loan_id = overdue.loan_id;
            let book_id = overdue.book_id;
            let member_id = overdue.member_id;

            let returned_loan = ReturnedLoan {
                core: LoanCore {
                    updated_at: returned_at,
                    ..overdue.core
                },
                returned_at,
            };

            let event = BookReturned {
                loan_id,
                book_id,
                member_id,
                returned_at,
                was_overdue: true,
            };

            Ok((returned_loan, event))
        }
        Loan::Returned(_) => Err(ReturnBookError::AlreadyReturned),
    }
}

/// 純粋関数：延滞判定
///
/// パターンマッチで状態判定を行う。
pub fn is_overdue(loan: &Loan, now: DateTime<Utc>) -> bool {
    match loan {
        Loan::Overdue(_) => true,
        Loan::Active(a) => now > a.due_date,
        Loan::Returned(_) => false,
    }
}

/// イベントを適用して新しい状態を生成する純粋関数
///
/// イベントソーシングのfoldパターンで使用される。
/// 型安全な状態遷移を実装。不正な遷移はpanicする。
///
/// # 引数
/// * `loan` - 現在の貸出状態（Noneは初期状態）
/// * `event` - 適用するドメインイベント
///
/// # 戻り値
/// 新しい貸出状態
///
/// # Panics
/// 不正な状態遷移（例: Returned状態からの延長）の場合にpanicする
pub fn apply_event(loan: Option<Loan>, event: &DomainEvent) -> Loan {
    match (loan, event) {
        // BookLoaned: 初期状態（None）からのみ受け入れる
        (None, DomainEvent::BookLoaned(e)) => Loan::Active(ActiveLoan {
            core: LoanCore {
                loan_id: e.loan_id,
                book_id: e.book_id,
                member_id: e.member_id,
                loaned_at: e.loaned_at,
                due_date: e.due_date,
                extension_count: ExtensionCount::new(),
                created_by: e.loaned_by,
                created_at: e.loaned_at,
                updated_at: e.loaned_at,
            },
        }),
        (Some(_), DomainEvent::BookLoaned(e)) => panic!(
            "Invalid state transition: BookLoaned({:?}) cannot apply to an existing loan",
            e.loan_id
        ),

        // LoanExtended: Active状態からのみ可能
        (Some(Loan::Active(active)), DomainEvent::LoanExtended(e)) => {
            assert_eq!(
                active.loan_id, e.loan_id,
                "LoanExtended loan_id does not match current loan"
            );
            let extension_count = ExtensionCount::try_from(e.extension_count)
                .expect("Invalid extension_count in persisted event");

            Loan::Active(ActiveLoan {
                core: LoanCore {
                    due_date: e.new_due_date,
                    extension_count,
                    updated_at: e.extended_at,
                    ..active.core
                },
            })
        }

        // BookReturned: ActiveまたはOverdue状態から可能
        (Some(Loan::Active(active)), DomainEvent::BookReturned(e)) => {
            assert_eq!(
                active.loan_id, e.loan_id,
                "BookReturned loan_id does not match current loan"
            );
            Loan::Returned(ReturnedLoan {
                core: LoanCore {
                    updated_at: e.returned_at,
                    ..active.core
                },
                returned_at: e.returned_at,
            })
        }
        (Some(Loan::Overdue(overdue)), DomainEvent::BookReturned(e)) => {
            assert_eq!(
                overdue.loan_id, e.loan_id,
                "BookReturned loan_id does not match current loan"
            );
            Loan::Returned(ReturnedLoan {
                core: LoanCore {
                    updated_at: e.returned_at,
                    ..overdue.core
                },
                returned_at: e.returned_at,
            })
        }

        // LoanBecameOverdue: Active状態からのみ可能
        (Some(Loan::Active(active)), DomainEvent::LoanBecameOverdue(e)) => {
            assert_eq!(
                active.loan_id, e.loan_id,
                "LoanBecameOverdue loan_id does not match current loan"
            );
            Loan::Overdue(OverdueLoan {
                core: LoanCore {
                    updated_at: e.detected_at,
                    ..active.core
                },
            })
        }

        // 不正な状態遷移
        (loan, event) => panic!(
            "Invalid state transition: loan={:?}, event={:?}",
            loan, event
        ),
    }
}

/// イベント列から現在の状態を復元する純粋関数
///
/// イベントソーシングにおいて、永続化されたイベント列から
/// Loan集約の現在の状態を再構築する。
/// foldパターンで各イベントを順次適用する。
///
/// # 引数
/// * `events` - ドメインイベントの列（時系列順）
///
/// # 戻り値
/// * イベントが空の場合は`None`
/// * それ以外は復元されたLoanを`Some`で返す
pub fn replay_events(events: &[DomainEvent]) -> Option<Loan> {
    events
        .iter()
        .fold(None, |loan, event| Some(apply_event(loan, event)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::LoanBecameOverdue;

    // TDD: apply_event() と replay_events() のテスト
    #[test]
    fn test_apply_event_book_loaned() {
        let loan_id = LoanId::new();
        let book_id = BookId::new();
        let member_id = MemberId::new();
        let staff_id = StaffId::new();
        let loaned_at = Utc::now();
        let due_date = loaned_at + Duration::days(14);

        let event = DomainEvent::BookLoaned(BookLoaned {
            loan_id,
            book_id,
            member_id,
            loaned_at,
            due_date,
            loaned_by: staff_id,
        });

        let loan = apply_event(None, &event);

        // Loan::Activeが返されることを確認
        match loan {
            Loan::Active(active) => {
                assert_eq!(active.loan_id, loan_id);
                assert_eq!(active.book_id, book_id);
                assert_eq!(active.member_id, member_id);
                assert_eq!(active.loaned_at, loaned_at);
                assert_eq!(active.due_date, due_date);
                assert_eq!(active.extension_count.value(), 0);
            }
            _ => panic!("Expected Loan::Active"),
        }
    }

    #[test]
    fn test_apply_event_loan_extended() {
        let book_id = BookId::new();
        let member_id = MemberId::new();
        let staff_id = StaffId::new();
        let loaned_at = Utc::now();

        let (active_loan, _) = loan_book(book_id, member_id, loaned_at, staff_id).unwrap();
        let loan_id = active_loan.loan_id;
        let old_due_date = active_loan.due_date;
        let new_due_date = old_due_date + Duration::days(14);
        let extended_at = loaned_at + Duration::days(5);

        let event = DomainEvent::LoanExtended(LoanExtended {
            loan_id,
            old_due_date,
            new_due_date,
            extended_at,
            extension_count: 1,
        });

        let new_loan = apply_event(Some(Loan::Active(active_loan)), &event);

        // Loan::Activeが返されることを確認
        match new_loan {
            Loan::Active(active) => {
                assert_eq!(active.due_date, new_due_date);
                assert_eq!(active.extension_count.value(), 1);
                assert_eq!(active.updated_at, extended_at);
            }
            _ => panic!("Expected Loan::Active"),
        }
    }

    #[test]
    fn test_apply_event_book_returned() {
        let book_id = BookId::new();
        let member_id = MemberId::new();
        let staff_id = StaffId::new();
        let loaned_at = Utc::now();

        let (active_loan, _) = loan_book(book_id, member_id, loaned_at, staff_id).unwrap();
        let loan_id = active_loan.loan_id;
        let returned_at = loaned_at + Duration::days(7);

        let event = DomainEvent::BookReturned(BookReturned {
            loan_id,
            book_id,
            member_id,
            returned_at,
            was_overdue: false,
        });

        let new_loan = apply_event(Some(Loan::Active(active_loan)), &event);

        // Loan::Returnedが返されることを確認
        match new_loan {
            Loan::Returned(returned) => {
                assert_eq!(returned.returned_at, returned_at);
                assert_eq!(returned.updated_at, returned_at);
            }
            _ => panic!("Expected Loan::Returned"),
        }
    }

    #[test]
    fn test_apply_event_loan_became_overdue() {
        let book_id = BookId::new();
        let member_id = MemberId::new();
        let staff_id = StaffId::new();
        let loaned_at = Utc::now();

        let (active_loan, _) = loan_book(book_id, member_id, loaned_at, staff_id).unwrap();
        let loan_id = active_loan.loan_id;
        let detected_at = loaned_at + Duration::days(20);

        let event = DomainEvent::LoanBecameOverdue(LoanBecameOverdue {
            loan_id,
            book_id,
            member_id,
            due_date: active_loan.due_date,
            detected_at,
        });

        let new_loan = apply_event(Some(Loan::Active(active_loan)), &event);

        // Loan::Overdueが返されることを確認
        match new_loan {
            Loan::Overdue(overdue) => {
                assert_eq!(overdue.updated_at, detected_at);
            }
            _ => panic!("Expected Loan::Overdue"),
        }
    }

    #[test]
    fn test_replay_events_empty() {
        let events = vec![];
        let result = replay_events(&events);
        // 空のイベント列はNoneを返す
        assert!(result.is_none());
    }

    #[test]
    fn test_replay_events_full_lifecycle() {
        let loan_id = LoanId::new();
        let book_id = BookId::new();
        let member_id = MemberId::new();
        let staff_id = StaffId::new();
        let loaned_at = Utc::now();
        let due_date = loaned_at + Duration::days(14);
        let returned_at = loaned_at + Duration::days(20);

        // イベント列を作成：貸出 → 延長 → 返却
        let events = vec![
            DomainEvent::BookLoaned(BookLoaned {
                loan_id,
                book_id,
                member_id,
                loaned_at,
                due_date,
                loaned_by: staff_id,
            }),
            DomainEvent::LoanExtended(LoanExtended {
                loan_id,
                old_due_date: due_date,
                new_due_date: due_date + Duration::days(14),
                extended_at: loaned_at + Duration::days(5),
                extension_count: 1,
            }),
            DomainEvent::BookReturned(BookReturned {
                loan_id,
                book_id,
                member_id,
                returned_at,
                was_overdue: false,
            }),
        ];

        let result = replay_events(&events);
        assert!(result.is_some());

        let loan = result.unwrap();

        // 最終的にLoan::Returnedになることを確認
        match loan {
            Loan::Returned(returned) => {
                assert_eq!(returned.loan_id, loan_id);
                assert_eq!(returned.extension_count.value(), 1);
                assert_eq!(returned.returned_at, returned_at);
            }
            _ => panic!("Expected Loan::Returned"),
        }
    }

    // ========================================================================
    // 型安全な状態パターンのテスト
    // ========================================================================

    #[test]
    fn test_active_loan_creation_and_deref() {
        let loan_id = LoanId::new();
        let book_id = BookId::new();
        let member_id = MemberId::new();
        let staff_id = StaffId::new();
        let loaned_at = Utc::now();
        let due_date = loaned_at + Duration::days(14);

        let active_loan = ActiveLoan {
            core: LoanCore {
                loan_id,
                book_id,
                member_id,
                loaned_at,
                due_date,
                extension_count: ExtensionCount::new(),
                created_by: staff_id,
                created_at: loaned_at,
                updated_at: loaned_at,
            },
        };

        // Derefでcore.loan_idに直接アクセスできることを確認
        assert_eq!(active_loan.loan_id, loan_id);
        assert_eq!(active_loan.book_id, book_id);
        assert_eq!(active_loan.member_id, member_id);
        assert_eq!(active_loan.due_date, due_date);
        assert_eq!(active_loan.extension_count.value(), 0);
    }

    #[test]
    fn test_overdue_loan_creation_and_deref() {
        let loan_id = LoanId::new();
        let book_id = BookId::new();
        let member_id = MemberId::new();
        let staff_id = StaffId::new();
        let loaned_at = Utc::now();
        let due_date = loaned_at + Duration::days(14);

        let overdue_loan = OverdueLoan {
            core: LoanCore {
                loan_id,
                book_id,
                member_id,
                loaned_at,
                due_date,
                extension_count: ExtensionCount::new(),
                created_by: staff_id,
                created_at: loaned_at,
                updated_at: loaned_at,
            },
        };

        // Derefでcore.loan_idに直接アクセスできることを確認
        assert_eq!(overdue_loan.loan_id, loan_id);
        assert_eq!(overdue_loan.book_id, book_id);
        assert_eq!(overdue_loan.extension_count.value(), 0);
    }

    #[test]
    fn test_returned_loan_creation_with_returned_at() {
        let loan_id = LoanId::new();
        let book_id = BookId::new();
        let member_id = MemberId::new();
        let staff_id = StaffId::new();
        let loaned_at = Utc::now();
        let due_date = loaned_at + Duration::days(14);
        let returned_at = loaned_at + Duration::days(7);

        let returned_loan = ReturnedLoan {
            core: LoanCore {
                loan_id,
                book_id,
                member_id,
                loaned_at,
                due_date,
                extension_count: ExtensionCount::new(),
                created_by: staff_id,
                created_at: loaned_at,
                updated_at: returned_at,
            },
            returned_at,
        };

        // returned_atが必須であることを型システムが保証
        assert_eq!(returned_loan.returned_at, returned_at);
        // Derefでcoreフィールドにアクセス可能
        assert_eq!(returned_loan.loan_id, loan_id);
        assert_eq!(returned_loan.book_id, book_id);
    }

    #[test]
    fn test_loan_pattern_matching() {
        let loan_id = LoanId::new();
        let book_id = BookId::new();
        let member_id = MemberId::new();
        let staff_id = StaffId::new();
        let loaned_at = Utc::now();
        let due_date = loaned_at + Duration::days(14);

        // ActiveLoan
        let active_loan = ActiveLoan {
            core: LoanCore {
                loan_id,
                book_id,
                member_id,
                loaned_at,
                due_date,
                extension_count: ExtensionCount::new(),
                created_by: staff_id,
                created_at: loaned_at,
                updated_at: loaned_at,
            },
        };
        let loan = Loan::Active(active_loan.clone());

        match loan {
            Loan::Active(a) => {
                assert_eq!(a.loan_id, loan_id);
            }
            _ => panic!("Expected Active variant"),
        }

        // OverdueLoan
        let overdue_loan = OverdueLoan {
            core: active_loan.core.clone(),
        };
        let loan = Loan::Overdue(overdue_loan);

        match loan {
            Loan::Overdue(o) => {
                assert_eq!(o.loan_id, loan_id);
            }
            _ => panic!("Expected Overdue variant"),
        }

        // ReturnedLoan
        let returned_at = loaned_at + Duration::days(7);
        let returned_loan = ReturnedLoan {
            core: active_loan.core.clone(),
            returned_at,
        };
        let loan = Loan::Returned(returned_loan);

        match loan {
            Loan::Returned(r) => {
                assert_eq!(r.loan_id, loan_id);
                assert_eq!(r.returned_at, returned_at);
            }
            _ => panic!("Expected Returned variant"),
        }
    }

    // ========================================================================
    // 純粋関数のテスト
    // ========================================================================

    // TDD: loan_book() のテスト
    #[test]
    fn test_loan_book_creates_active_loan_with_correct_due_date() {
        let book_id = BookId::new();
        let member_id = MemberId::new();
        let staff_id = StaffId::new();
        let loaned_at = Utc::now();

        let result = loan_book(book_id, member_id, loaned_at, staff_id);
        assert!(result.is_ok());

        let (loan, event) = result.unwrap();

        // ActiveLoanを返すことを確認
        assert_eq!(loan.due_date, loaned_at + Duration::days(14));
        assert_eq!(loan.extension_count.value(), 0);
        assert_eq!(loan.book_id, book_id);
        assert_eq!(loan.member_id, member_id);
        assert_eq!(loan.created_by, staff_id);

        // イベントの検証
        assert_eq!(event.loan_id, loan.loan_id);
        assert_eq!(event.book_id, book_id);
        assert_eq!(event.member_id, member_id);
        assert_eq!(event.loaned_at, loaned_at);
        assert_eq!(event.due_date, loan.due_date);
        assert_eq!(event.loaned_by, staff_id);
    }

    #[test]
    fn test_loan_book_returns_active_loan_type() {
        let book_id = BookId::new();
        let member_id = MemberId::new();
        let staff_id = StaffId::new();
        let loaned_at = Utc::now();

        let result = loan_book(book_id, member_id, loaned_at, staff_id);
        assert!(result.is_ok());

        let (loan, _) = result.unwrap();

        // ActiveLoan型であることを確認（コンパイル時に型チェックされる）
        let _active: ActiveLoan = loan;
    }

    #[test]
    fn test_loan_book_core_due_date_is_correct() {
        let book_id = BookId::new();
        let member_id = MemberId::new();
        let staff_id = StaffId::new();
        let loaned_at = Utc::now();

        let (loan, _) = loan_book(book_id, member_id, loaned_at, staff_id).unwrap();

        // core.due_dateが正しいことを確認
        assert_eq!(loan.core.due_date, loaned_at + Duration::days(14));
    }

    #[test]
    fn test_loan_book_initial_extension_count_is_zero() {
        let book_id = BookId::new();
        let member_id = MemberId::new();
        let staff_id = StaffId::new();
        let loaned_at = Utc::now();

        let (loan, _) = loan_book(book_id, member_id, loaned_at, staff_id).unwrap();

        // 初期延長回数は0
        assert_eq!(loan.extension_count.value(), 0);
    }

    // TDD: extend_loan() のテスト
    #[test]
    fn test_extend_loan_success_with_active_loan() {
        let book_id = BookId::new();
        let member_id = MemberId::new();
        let staff_id = StaffId::new();
        let loaned_at = Utc::now();

        let (loan, _) = loan_book(book_id, member_id, loaned_at, staff_id).unwrap();
        let extended_at = loaned_at + Duration::days(5);

        let result = extend_loan(loan.clone(), extended_at);
        assert!(result.is_ok());

        let (new_loan, event) = result.unwrap();

        // 延長後の返却期限は元の期限 + 14日間
        assert_eq!(new_loan.due_date, loan.due_date + Duration::days(14));
        assert_eq!(new_loan.extension_count.value(), 1);

        // イベントの検証
        assert_eq!(event.loan_id, loan.loan_id);
        assert_eq!(event.old_due_date, loan.due_date);
        assert_eq!(event.new_due_date, new_loan.due_date);
        assert_eq!(event.extension_count, 1);
    }

    #[test]
    fn test_extend_loan_fails_when_already_extended() {
        let book_id = BookId::new();
        let member_id = MemberId::new();
        let staff_id = StaffId::new();
        let loaned_at = Utc::now();

        let (loan, _) = loan_book(book_id, member_id, loaned_at, staff_id).unwrap();
        let extended_at = loaned_at + Duration::days(5);

        // 1回目の延長は成功
        let (loan, _) = extend_loan(loan, extended_at).unwrap();

        // 2回目の延長は失敗
        let result = extend_loan(loan, extended_at + Duration::days(1));
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), ExtendLoanError::ExtensionLimitExceeded);
    }

    #[test]
    fn test_extend_loan_type_safety_accepts_only_active_loan() {
        let book_id = BookId::new();
        let member_id = MemberId::new();
        let staff_id = StaffId::new();
        let loaned_at = Utc::now();

        let (active_loan, _) = loan_book(book_id, member_id, loaned_at, staff_id).unwrap();
        let extended_at = loaned_at + Duration::days(5);

        // ActiveLoanを受け付ける（コンパイル成功）
        let result = extend_loan(active_loan, extended_at);
        assert!(result.is_ok());

        // OverdueLoanやReturnedLoanは型システムでコンパイルエラーになる
        // 以下はコンパイルエラーになるためコメントアウト：
        // let overdue_loan = OverdueLoan { core: active_loan.core.clone() };
        // extend_loan(overdue_loan, extended_at); // コンパイルエラー
    }

    #[test]
    fn test_extend_loan_returns_active_loan() {
        let book_id = BookId::new();
        let member_id = MemberId::new();
        let staff_id = StaffId::new();
        let loaned_at = Utc::now();

        let (loan, _) = loan_book(book_id, member_id, loaned_at, staff_id).unwrap();
        let extended_at = loaned_at + Duration::days(5);

        let (new_loan, _) = extend_loan(loan, extended_at).unwrap();

        // ActiveLoan型であることを確認
        let _active: ActiveLoan = new_loan;
    }

    // TDD: return_book() のテスト
    #[test]
    fn test_return_book_success_from_active_loan() {
        let book_id = BookId::new();
        let member_id = MemberId::new();
        let staff_id = StaffId::new();
        let loaned_at = Utc::now();

        let (loan, _) = loan_book(book_id, member_id, loaned_at, staff_id).unwrap();
        let returned_at = loaned_at + Duration::days(7);

        let result = return_book(Loan::Active(loan.clone()), returned_at);
        assert!(result.is_ok());

        let (returned_loan, event) = result.unwrap();

        // ReturnedLoan.returned_atが必須であることを確認
        assert_eq!(returned_loan.returned_at, returned_at);
        assert!(!event.was_overdue);

        // イベントの検証
        assert_eq!(event.loan_id, loan.loan_id);
        assert_eq!(event.book_id, book_id);
        assert_eq!(event.member_id, member_id);
        assert_eq!(event.returned_at, returned_at);
    }

    #[test]
    fn test_return_book_success_from_overdue_loan() {
        let book_id = BookId::new();
        let member_id = MemberId::new();
        let staff_id = StaffId::new();
        let loaned_at = Utc::now();

        let (active_loan, _) = loan_book(book_id, member_id, loaned_at, staff_id).unwrap();
        let overdue_loan = OverdueLoan {
            core: active_loan.core,
        };
        let returned_at = loaned_at + Duration::days(20);

        let result = return_book(Loan::Overdue(overdue_loan), returned_at);
        assert!(result.is_ok());

        let (returned_loan, event) = result.unwrap();

        // 延滞から返却
        assert_eq!(returned_loan.returned_at, returned_at);
        assert!(event.was_overdue);
    }

    #[test]
    fn test_return_book_fails_when_already_returned() {
        let book_id = BookId::new();
        let member_id = MemberId::new();
        let staff_id = StaffId::new();
        let loaned_at = Utc::now();

        let (loan, _) = loan_book(book_id, member_id, loaned_at, staff_id).unwrap();
        let returned_at = loaned_at + Duration::days(7);
        let (returned_loan, _) = return_book(Loan::Active(loan), returned_at).unwrap();

        // 2回目の返却は失敗
        let result = return_book(
            Loan::Returned(returned_loan),
            returned_at + Duration::days(1),
        );
        assert!(result.is_err());
        assert_eq!(result.unwrap_err(), ReturnBookError::AlreadyReturned);
    }

    // TDD: is_overdue() のテスト
    #[test]
    fn test_is_overdue_false_for_active_loan_before_due_date() {
        let book_id = BookId::new();
        let member_id = MemberId::new();
        let staff_id = StaffId::new();
        let loaned_at = Utc::now();

        let (loan, _) = loan_book(book_id, member_id, loaned_at, staff_id).unwrap();
        let check_time = loaned_at + Duration::days(7);

        assert!(!is_overdue(&Loan::Active(loan), check_time));
    }

    #[test]
    fn test_is_overdue_true_for_active_loan_after_due_date() {
        let book_id = BookId::new();
        let member_id = MemberId::new();
        let staff_id = StaffId::new();
        let loaned_at = Utc::now();

        let (loan, _) = loan_book(book_id, member_id, loaned_at, staff_id).unwrap();
        let check_time = loaned_at + Duration::days(20);

        assert!(is_overdue(&Loan::Active(loan), check_time));
    }

    #[test]
    fn test_is_overdue_true_for_overdue_loan() {
        let book_id = BookId::new();
        let member_id = MemberId::new();
        let staff_id = StaffId::new();
        let loaned_at = Utc::now();

        let (active_loan, _) = loan_book(book_id, member_id, loaned_at, staff_id).unwrap();
        let overdue_loan = OverdueLoan {
            core: active_loan.core,
        };
        let check_time = Utc::now();

        // パターンマッチでOverdueLoanは常にtrue
        assert!(is_overdue(&Loan::Overdue(overdue_loan), check_time));
    }
}
