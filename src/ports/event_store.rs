use crate::domain::events::DomainEvent;
use async_trait::async_trait;
use futures::stream::BoxStream;
use uuid::Uuid;

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
    async fn append(
        &self,
        aggregate_id: Uuid,
        aggregate_type: &str,
        events: Vec<DomainEvent>,
    ) -> Result<()>;

    /// 集約のすべてのイベントを読み込む
    ///
    /// 追加された順序でイベントを返す。
    /// replay_events による集約状態の復元に使用される。
    async fn load(&self, aggregate_id: Uuid) -> Result<Vec<DomainEvent>>;

    /// すべての集約のイベントをストリーム配信する
    ///
    /// 延滞検知などのバッチ操作に使用される。
    /// イベントは挿入順にストリーム配信される。
    fn stream_all(&self) -> BoxStream<'_, Result<DomainEvent>>;
}
