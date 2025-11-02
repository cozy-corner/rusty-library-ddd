-- Create events table for event sourcing
CREATE TABLE events (
    sequence_number BIGSERIAL PRIMARY KEY,
    aggregate_id UUID NOT NULL,
    aggregate_type VARCHAR(50) NOT NULL,
    event_type VARCHAR(100) NOT NULL,
    event_data JSONB NOT NULL,
    occurred_at TIMESTAMPTZ NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Index for loading events by aggregate ID
CREATE INDEX idx_events_aggregate_id ON events(aggregate_id, sequence_number);

-- Index for filtering by event type
CREATE INDEX idx_events_event_type ON events(event_type);

-- Index for temporal queries
CREATE INDEX idx_events_occurred_at ON events(occurred_at);

-- Index for aggregate type queries
CREATE INDEX idx_events_aggregate_type ON events(aggregate_type);
