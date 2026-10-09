-- Explicitly provisioned trusted node keys; deny unknown or revoked keys.
CREATE TABLE IF NOT EXISTS trusted_nodes (
    author_key TEXT PRIMARY KEY,
    label TEXT NOT NULL,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    created_at TIMESTAMPTZ NOT NULL DEFAULT now(),
    CHECK (length(author_key) = 64),
    CHECK (length(label) BETWEEN 1 AND 128)
);
CREATE TABLE IF NOT EXISTS node_event_permissions (
    author_key TEXT NOT NULL REFERENCES trusted_nodes(author_key) ON DELETE CASCADE,
    kind TEXT NOT NULL,
    PRIMARY KEY (author_key, kind),
    CHECK (length(kind) BETWEEN 1 AND 128)
);
-- No default nodes and no default permissions. Provision via controlled DBA migration.
