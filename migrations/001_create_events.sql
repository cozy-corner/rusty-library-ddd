-- Create events table for event sourcing
CREATE TABLE events (
    event_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    aggregate_id UUID NOT NULL,
    aggregate_version INTEGER NOT NULL,
    aggregate_type VARCHAR(50) NOT NULL,
    event_type VARCHAR(100) NOT NULL,
    event_data JSONB NOT NULL,
    occurred_at TIMESTAMPTZ NOT NULL,
    sequence_number BIGSERIAL NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    UNIQUE (aggregate_id, aggregate_version)
);

-- Note: UNIQUE constraint on (aggregate_id, aggregate_version) automatically creates an index
-- which is used for loading events by aggregate ID

-- Index for global stream ordering
CREATE INDEX idx_events_sequence_number ON events(sequence_number);

-- Index for filtering by event type
CREATE INDEX idx_events_event_type ON events(event_type);

-- Index for temporal queries
CREATE INDEX idx_events_occurred_at ON events(occurred_at);
