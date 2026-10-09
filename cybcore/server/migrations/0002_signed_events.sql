-- The first event store is scoped to the author's verified Ed25519 key.
-- Authorization and multi-tenant ACL must be added before shared-channel events.
CREATE TABLE IF NOT EXISTS signed_events (
    author_key TEXT NOT NULL,
    event_id TEXT NOT NULL,
    kind TEXT NOT NULL,
    created_at_ms BIGINT NOT NULL,
    content TEXT NOT NULL,
    signature TEXT NOT NULL,
    received_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    PRIMARY KEY (author_key, event_id)
);
CREATE INDEX IF NOT EXISTS signed_events_author_time
    ON signed_events(author_key, received_at DESC);
