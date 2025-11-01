use crate::domain::{
    events::DomainEvent,
    value_objects::{BookId, LoanId, MemberId},
};
use async_trait::async_trait;
use chrono::{DateTime, Utc};
use futures::stream::BoxStream;

#[allow(dead_code)]
pub type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync>>;

/// イベントストアポート
///
/// ドメインイベントの永続化と取得を抽象化する。
/// イベントは追記専用ログに保存される不変の事実。
#[allow(dead_code)]
#[async_trait]
pub trait EventStore: Send + Sync {
    /// 集約のイベントを追加する
    ///
    /// イベントは追記専用ログに保存され、変更・削除不可。
    /// イベントの順序は保持される。
    async fn append(&self, aggregate_id: LoanId, events: Vec<DomainEvent>) -> Result<()>;

    /// 集約のすべてのイベントを読み込む
    ///
    /// 追加された順序でイベントを返す。
    /// replay_events による集約状態の復元に使用される。
    async fn load(&self, aggregate_id: LoanId) -> Result<Vec<DomainEvent>>;

    /// すべての集約のイベントをストリーム配信する
    ///
    /// 延滞検知などのバッチ操作に使用される。
    /// イベントは挿入順にストリーム配信される。
    fn stream_all(&self) -> BoxStream<'static, Result<DomainEvent>>;
}

/// 貸出ビュー（Read Model）
///
/// クエリに最適化された非正規化ビュー（CQRSパターン）。
/// イベント永続化時に非同期で更新される。
#[allow(dead_code)]
#[derive(Debug, Clone)]
pub struct LoanView {
    pub loan_id: LoanId,
    pub book_id: BookId,
    pub member_id: MemberId,
    pub loaned_at: DateTime<Utc>,
    pub due_date: DateTime<Utc>,
    pub returned_at: Option<DateTime<Utc>>,
    pub extension_count: u8,
    /// ステータス: "active", "overdue", "returned"
    pub status: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// 貸出Read Modelポート
#[allow(dead_code)]
#[async_trait]
pub trait LoanReadModel: Send + Sync {
    /// 新規貸出ビューレコードを挿入する
    ///
    /// BookLoanedイベント処理時に呼ばれる。
    async fn insert(&self, loan_view: LoanView) -> Result<()>;

    /// 貸出ステータスと返却日時を更新する
    ///
    /// BookReturnedまたはLoanBecameOverdueイベント処理時に呼ばれる。
    async fn update_status(
        &self,
        loan_id: LoanId,
        status: &str,
        returned_at: Option<DateTime<Utc>>,
    ) -> Result<()>;

    /// 貸出返却期限を更新する
    ///
    /// LoanExtendedイベント処理時に呼ばれる。
    async fn update_due_date(&self, loan_id: LoanId, new_due_date: DateTime<Utc>) -> Result<()>;

    /// 会員の貸出中の貸出を取得する
    ///
    /// 貸出上限（会員ごと最大5冊）の確認に使用される。
    async fn get_active_loans_for_member(&self, member_id: MemberId) -> Result<Vec<LoanView>>;

    /// 延滞候補の貸出を検索する
    ///
    /// due_date < cutoff_date かつ status が "active" の貸出を返す。
    /// バッチジョブでの延滞検知に使用される。
    async fn find_overdue_candidates(&self, cutoff_date: DateTime<Utc>) -> Result<Vec<LoanView>>;

    /// IDで貸出を取得する
    async fn get_by_id(&self, loan_id: LoanId) -> Result<Option<LoanView>>;

    /// 会員の全貸出を検索する
    ///
    /// 会員の貸出履歴表示に使用される。
    async fn find_by_member_id(&self, member_id: MemberId) -> Result<Vec<LoanView>>;
}

/// 会員サービスポート
///
/// 貸出コンテキストと会員コンテキストの境界を維持する。
/// 貸出コンテキストはMemberIDのみを知り、会員詳細は知らない。
#[allow(dead_code)]
#[async_trait]
pub trait MemberService: Send + Sync {
    /// 会員が存在するか確認する
    ///
    /// 貸出作成前の会員バリデーションに使用される。
    async fn exists(&self, member_id: MemberId) -> Result<bool>;

    /// 会員が延滞中の貸出を持っているか確認する
    ///
    /// ビジネスルール: 延滞中の会員には貸出不可。
    async fn has_overdue_loans(&self, member_id: MemberId) -> Result<bool>;
}

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

/// 通知サービスポート
///
/// 会員への通知配信メカニズムを抽象化する。
/// 実装はメール、SMS、プッシュ通知などが考えられる。
#[allow(dead_code)]
#[async_trait]
pub trait NotificationService: Send + Sync {
    /// 延滞通知を会員に送信する
    ///
    /// LoanBecameOverdueイベント処理時に呼ばれる。
    async fn send_overdue_notification(
        &self,
        member_id: MemberId,
        book_title: &str,
        due_date: DateTime<Utc>,
    ) -> Result<()>;

    /// 延長確認通知を会員に送信する
    ///
    /// LoanExtendedイベント処理時に呼ばれる。
    async fn send_extension_confirmation(
        &self,
        member_id: MemberId,
        book_title: &str,
        new_due_date: DateTime<Utc>,
    ) -> Result<()>;

    /// 返却確認通知を会員に送信する
    ///
    /// BookReturnedイベント処理時に呼ばれる。
    async fn send_return_confirmation(
        &self,
        member_id: MemberId,
        book_title: &str,
        was_overdue: bool,
    ) -> Result<()>;
}
