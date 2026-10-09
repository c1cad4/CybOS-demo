-- First migration: shared events are NOT accepted by the API until
-- signature verification and tenant-scoped authorization are implemented.
CREATE TABLE IF NOT EXISTS schema_metadata (
    key TEXT PRIMARY KEY,
    value TEXT NOT NULL
);
INSERT INTO schema_metadata(key, value)
VALUES ('schema_version', '1')
ON CONFLICT (key) DO NOTHING;
