use crate::domain::value_objects::{BookId, LoanId, MemberId};
use crate::ports::loan_read_model::{
    LoanReadModel as LoanReadModelTrait, LoanStatus, LoanView, Result,
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sqlx::{PgPool, Row, postgres::PgRow};
use std::str::FromStr;

/// PostgreSQLの行データをLoanViewに変換する
///
/// データベースから取得した行を、ドメインの値オブジェクトとLoanViewに変換する。
/// extension_countのi16からu8への変換とLoanStatusの文字列からの変換で
/// エラーハンドリングを行う。
fn map_row_to_loan_view(row: &PgRow) -> Result<LoanView> {
    let extension_count_i16: i16 = row.get("extension_count");
    let extension_count: u8 = extension_count_i16.try_into().map_err(|_| {
        Box::new(std::io::Error::new(
            std::io::ErrorKind::InvalidData,
            format!("extension_count out of range: {}", extension_count_i16),
        )) as Box<dyn std::error::Error + Send + Sync>
    })?;

    let status_str: &str = row.get("status");
    let status = LoanStatus::from_str(status_str).map_err(|e| {
        Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, e))
            as Box<dyn std::error::Error + Send + Sync>
    })?;

    Ok(LoanView {
        loan_id: LoanId::from_uuid(row.get("loan_id")),
        book_id: BookId::from_uuid(row.get("book_id")),
        member_id: MemberId::from_uuid(row.get("member_id")),
        loaned_at: row.get("loaned_at"),
        due_date: row.get("due_date"),
        returned_at: row.get("returned_at"),
        extension_count,
        status,
        created_at: row.get("created_at"),
        updated_at: row.get("updated_at"),
    })
}

/// LoanReadModelのPostgreSQL実装
///
/// CQRSパターンの読み取り側として、クエリに最適化された
/// 非正規化ビューを提供する。
#[allow(dead_code)]
pub struct LoanReadModel {
    pool: PgPool,
}

#[allow(dead_code)]
impl LoanReadModel {
    /// PostgreSQLコネクションプールから新しいLoanReadModelを作成
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl LoanReadModelTrait for LoanReadModel {
    /// 貸出ビューをRead Modelに保存（upsert）
    ///
    /// INSERT ... ON CONFLICT UPDATEを使用して冪等性を保証する。
    /// これにより、Read Modelは常にイベントストリームから再構築された
    /// 完全な状態を反映する。
    async fn save(&self, loan_view: LoanView) -> Result<()> {
        sqlx::query(
            r#"
            INSERT INTO loans_view (
                loan_id,
                book_id,
                member_id,
                loaned_at,
                due_date,
                returned_at,
                extension_count,
                status,
                created_at,
                updated_at
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            ON CONFLICT (loan_id)
            DO UPDATE SET
                book_id = EXCLUDED.book_id,
                member_id = EXCLUDED.member_id,
                loaned_at = EXCLUDED.loaned_at,
                due_date = EXCLUDED.due_date,
                returned_at = EXCLUDED.returned_at,
                extension_count = EXCLUDED.extension_count,
                status = EXCLUDED.status,
                updated_at = EXCLUDED.updated_at
            "#,
        )
        .bind(loan_view.loan_id.value())
        .bind(loan_view.book_id.value())
        .bind(loan_view.member_id.value())
        .bind(loan_view.loaned_at)
        .bind(loan_view.due_date)
        .bind(loan_view.returned_at)
        .bind(loan_view.extension_count as i16)
        .bind(loan_view.status.as_str())
        .bind(loan_view.created_at)
        .bind(loan_view.updated_at)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// 会員の貸出中の貸出を取得（貸出上限確認用）
    ///
    /// (member_id, status)の部分インデックスを使用してパフォーマンスを最適化。
    async fn get_active_loans_for_member(&self, member_id: MemberId) -> Result<Vec<LoanView>> {
        let rows = sqlx::query(
            r#"
            SELECT
                loan_id,
                book_id,
                member_id,
                loaned_at,
                due_date,
                returned_at,
                extension_count,
                status,
                created_at,
                updated_at
            FROM loans_view
            WHERE member_id = $1 AND status = 'active'
            ORDER BY loaned_at DESC
            "#,
        )
        .bind(member_id.value())
        .fetch_all(&self.pool)
        .await?;

        rows.iter().map(map_row_to_loan_view).collect()
    }

    /// 延滞候補を検索（バッチ延滞検知用）
    ///
    /// 返却期限を過ぎた貸出中の貸出を返す。
    /// (status, due_date)の部分インデックスを使用してパフォーマンスを最適化。
    async fn find_overdue_candidates(&self, cutoff_date: DateTime<Utc>) -> Result<Vec<LoanView>> {
        let rows = sqlx::query(
            r#"
            SELECT
                loan_id,
                book_id,
                member_id,
                loaned_at,
                due_date,
                returned_at,
                extension_count,
                status,
                created_at,
                updated_at
            FROM loans_view
            WHERE status = 'active' AND due_date < $1
            ORDER BY due_date ASC
            "#,
        )
        .bind(cutoff_date)
        .fetch_all(&self.pool)
        .await?;

        rows.iter().map(map_row_to_loan_view).collect()
    }

    /// IDで貸出を取得
    async fn get_by_id(&self, loan_id: LoanId) -> Result<Option<LoanView>> {
        let row = sqlx::query(
            r#"
            SELECT
                loan_id,
                book_id,
                member_id,
                loaned_at,
                due_date,
                returned_at,
                extension_count,
                status,
                created_at,
                updated_at
            FROM loans_view
            WHERE loan_id = $1
            "#,
        )
        .bind(loan_id.value())
        .fetch_optional(&self.pool)
        .await?;

        row.as_ref().map(map_row_to_loan_view).transpose()
    }

    /// 会員の全貸出を検索（貸出履歴）
    async fn find_by_member_id(&self, member_id: MemberId) -> Result<Vec<LoanView>> {
        let rows = sqlx::query(
            r#"
            SELECT
                loan_id,
                book_id,
                member_id,
                loaned_at,
                due_date,
                returned_at,
                extension_count,
                status,
                created_at,
                updated_at
            FROM loans_view
            WHERE member_id = $1
            ORDER BY loaned_at DESC
            "#,
        )
        .bind(member_id.value())
        .fetch_all(&self.pool)
        .await?;

        rows.iter().map(map_row_to_loan_view).collect()
    }
}
