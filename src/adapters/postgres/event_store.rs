use crate::domain::{events::DomainEvent, value_objects::LoanId};
use crate::ports::event_store::{EventStore as EventStoreTrait, Result};
use async_trait::async_trait;
use futures::stream::{BoxStream, StreamExt};
use sqlx::{PgPool, Row};

/// PostgreSQL implementation of EventStore
///
/// Stores domain events in an append-only event log.
/// Events are serialized as JSONB for flexible schema evolution.
#[allow(dead_code)]
pub struct EventStore {
    pool: PgPool,
}

#[allow(dead_code)]
impl EventStore {
    /// Create a new EventStore with a PostgreSQL connection pool
    pub fn new(pool: PgPool) -> Self {
        Self { pool }
    }

    /// Get the event type discriminator from a DomainEvent
    fn event_type(event: &DomainEvent) -> &'static str {
        match event {
            DomainEvent::BookLoaned(_) => "BookLoaned",
            DomainEvent::LoanExtended(_) => "LoanExtended",
            DomainEvent::BookReturned(_) => "BookReturned",
            DomainEvent::LoanBecameOverdue(_) => "LoanBecameOverdue",
        }
    }

    /// Extract the occurred_at timestamp from a DomainEvent
    fn occurred_at(event: &DomainEvent) -> chrono::DateTime<chrono::Utc> {
        match event {
            DomainEvent::BookLoaned(e) => e.loaned_at,
            DomainEvent::LoanExtended(e) => e.extended_at,
            DomainEvent::BookReturned(e) => e.returned_at,
            DomainEvent::LoanBecameOverdue(e) => e.detected_at,
        }
    }
}

#[async_trait]
impl EventStoreTrait for EventStore {
    /// Append events to the event store
    ///
    /// Events are stored with versioning for optimistic concurrency control.
    /// All events for a single aggregate are stored atomically within a transaction.
    /// The aggregate_version is automatically incremented for each event.
    /// Uses batch INSERT with UNNEST for optimal performance.
    async fn append(&self, aggregate_id: LoanId, events: Vec<DomainEvent>) -> Result<()> {
        if events.is_empty() {
            return Ok(());
        }

        let mut tx = self.pool.begin().await?;

        // Get the current version of the aggregate
        // COALESCE handles NULL when no events exist for this aggregate
        let current_version: i32 = sqlx::query_scalar(
            r#"
            SELECT COALESCE(MAX(aggregate_version), 0)
            FROM events
            WHERE aggregate_id = $1
            "#,
        )
        .bind(aggregate_id.value())
        .fetch_one(&mut *tx)
        .await?;

        // Prepare batch data
        let mut versions = Vec::with_capacity(events.len());
        let mut event_types = Vec::with_capacity(events.len());
        let mut event_data_list = Vec::with_capacity(events.len());
        let mut occurred_at_list = Vec::with_capacity(events.len());

        for (i, event) in events.iter().enumerate() {
            versions.push(current_version + (i as i32) + 1);
            event_types.push(Self::event_type(event));
            event_data_list.push(serde_json::to_value(event)?);
            occurred_at_list.push(Self::occurred_at(event));
        }

        // Batch INSERT using UNNEST
        // aggregate_type is constant for all events in this batch
        let aggregate_types = vec!["Loan"; events.len()];

        sqlx::query(
            r#"
            INSERT INTO events (
                aggregate_id,
                aggregate_version,
                aggregate_type,
                event_type,
                event_data,
                occurred_at
            )
            SELECT $1, * FROM UNNEST($2::int[], $3::varchar[], $4::varchar[], $5::jsonb[], $6::timestamptz[])
            "#,
        )
        .bind(aggregate_id.value())
        .bind(&versions)
        .bind(&aggregate_types)
        .bind(&event_types)
        .bind(&event_data_list)
        .bind(&occurred_at_list)
        .execute(&mut *tx)
        .await?;

        tx.commit().await?;
        Ok(())
    }

    /// Load all events for an aggregate in chronological order
    ///
    /// Events are returned in the order they were appended (by aggregate_version).
    /// Used to reconstruct aggregate state through event replay.
    async fn load(&self, aggregate_id: LoanId) -> Result<Vec<DomainEvent>> {
        let rows = sqlx::query(
            r#"
            SELECT event_data
            FROM events
            WHERE aggregate_id = $1
            ORDER BY aggregate_version ASC
            "#,
        )
        .bind(aggregate_id.value())
        .fetch_all(&self.pool)
        .await?;

        let mut events = Vec::with_capacity(rows.len());
        for row in rows {
            let event_data: serde_json::Value = row.get("event_data");
            let event: DomainEvent = serde_json::from_value(event_data)?;
            events.push(event);
        }

        Ok(events)
    }

    /// Stream all events in insertion order
    ///
    /// Returns a stream of events ordered by sequence_number.
    /// Used for batch processing operations like overdue detection.
    fn stream_all(&self) -> BoxStream<'_, Result<DomainEvent>> {
        // Create a stream from the query
        let stream = sqlx::query(
            r#"
            SELECT event_data
            FROM events
            ORDER BY sequence_number ASC
            "#,
        )
        .fetch(&self.pool)
        .map(|row_result| {
            let row = row_result?;
            let event_data: serde_json::Value = row.get("event_data");
            let event: DomainEvent = serde_json::from_value(event_data)
                .map_err(|e| Box::new(e) as Box<dyn std::error::Error + Send + Sync>)?;
            Ok(event)
        });

        Box::pin(stream)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{
        events::{BookLoaned, BookReturned, LoanExtended},
        value_objects::{BookId, MemberId, StaffId},
    };
    use chrono::Utc;

    /// Helper to create a test database pool
    /// Requires DATABASE_URL environment variable to be set
    async fn create_test_pool() -> PgPool {
        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://postgres:postgres@localhost/rusty_library".to_string());

        PgPool::connect(&database_url)
            .await
            .expect("Failed to connect to test database")
    }

    /// Helper to clean up test data
    async fn cleanup_events(pool: &PgPool, aggregate_id: LoanId) {
        sqlx::query("DELETE FROM events WHERE aggregate_id = $1")
            .bind(aggregate_id.value())
            .execute(pool)
            .await
            .expect("Failed to cleanup test events");
    }

    #[tokio::test]
    async fn test_append_and_load_events() {
        let pool = create_test_pool().await;
        let event_store = EventStore::new(pool.clone());

        let loan_id = LoanId::new();
        let book_id = BookId::new();
        let member_id = MemberId::new();
        let staff_id = StaffId::new();
        let now = Utc::now();

        let events = vec![
            DomainEvent::BookLoaned(BookLoaned {
                loan_id,
                book_id,
                member_id,
                loaned_at: now,
                due_date: now + chrono::Duration::days(14),
                loaned_by: staff_id,
            }),
            DomainEvent::LoanExtended(LoanExtended {
                loan_id,
                old_due_date: now + chrono::Duration::days(14),
                new_due_date: now + chrono::Duration::days(28),
                extended_at: now + chrono::Duration::days(10),
                extension_count: 1,
            }),
        ];

        // Append events
        event_store
            .append(loan_id, events.clone())
            .await
            .expect("Failed to append events");

        // Load events
        let loaded_events = event_store
            .load(loan_id)
            .await
            .expect("Failed to load events");

        assert_eq!(loaded_events.len(), 2);
        assert_eq!(loaded_events, events);

        // Cleanup
        cleanup_events(&pool, loan_id).await;
    }

    #[tokio::test]
    async fn test_load_nonexistent_aggregate() {
        let pool = create_test_pool().await;
        let event_store = EventStore::new(pool);

        let loan_id = LoanId::new();
        let events = event_store
            .load(loan_id)
            .await
            .expect("Failed to load events");

        assert_eq!(events.len(), 0);
    }

    #[tokio::test]
    async fn test_append_empty_events() {
        let pool = create_test_pool().await;
        let event_store = EventStore::new(pool);

        let loan_id = LoanId::new();
        let result = event_store.append(loan_id, vec![]).await;

        assert!(result.is_ok());
    }

    #[tokio::test]
    async fn test_stream_all_events() {
        let pool = create_test_pool().await;
        let event_store = EventStore::new(pool.clone());

        let loan_id = LoanId::new();
        let book_id = BookId::new();
        let member_id = MemberId::new();
        let now = Utc::now();

        let events = vec![
            DomainEvent::BookLoaned(BookLoaned {
                loan_id,
                book_id,
                member_id,
                loaned_at: now,
                due_date: now + chrono::Duration::days(14),
                loaned_by: StaffId::new(),
            }),
            DomainEvent::BookReturned(BookReturned {
                loan_id,
                book_id,
                member_id,
                returned_at: now + chrono::Duration::days(7),
                was_overdue: false,
            }),
        ];

        event_store
            .append(loan_id, events.clone())
            .await
            .expect("Failed to append events");

        // Stream all events
        let mut stream = event_store.stream_all();
        let mut streamed_events = Vec::new();

        while let Some(event_result) = stream.next().await {
            let event = event_result.expect("Failed to stream event");
            // Only collect events for our test aggregate
            match &event {
                DomainEvent::BookLoaned(e) if e.loan_id == loan_id => {
                    streamed_events.push(event);
                }
                DomainEvent::BookReturned(e) if e.loan_id == loan_id => {
                    streamed_events.push(event);
                }
                _ => {}
            }
        }

        assert_eq!(streamed_events.len(), 2);

        // Cleanup
        cleanup_events(&pool, loan_id).await;
    }

    #[tokio::test]
    async fn test_events_ordering() {
        let pool = create_test_pool().await;
        let event_store = EventStore::new(pool.clone());

        let loan_id = LoanId::new();
        let book_id = BookId::new();
        let member_id = MemberId::new();
        let now = Utc::now();

        // Append events in multiple batches
        let event1 = DomainEvent::BookLoaned(BookLoaned {
            loan_id,
            book_id,
            member_id,
            loaned_at: now,
            due_date: now + chrono::Duration::days(14),
            loaned_by: StaffId::new(),
        });

        event_store
            .append(loan_id, vec![event1.clone()])
            .await
            .expect("Failed to append first event");

        let event2 = DomainEvent::LoanExtended(LoanExtended {
            loan_id,
            old_due_date: now + chrono::Duration::days(14),
            new_due_date: now + chrono::Duration::days(28),
            extended_at: now + chrono::Duration::days(10),
            extension_count: 1,
        });

        event_store
            .append(loan_id, vec![event2.clone()])
            .await
            .expect("Failed to append second event");

        // Load events and verify ordering
        let loaded_events = event_store
            .load(loan_id)
            .await
            .expect("Failed to load events");

        assert_eq!(loaded_events.len(), 2);
        assert_eq!(loaded_events[0], event1);
        assert_eq!(loaded_events[1], event2);

        // Cleanup
        cleanup_events(&pool, loan_id).await;
    }
}
