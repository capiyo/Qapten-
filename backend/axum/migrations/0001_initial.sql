CREATE TABLE IF NOT EXISTS entities (
    kind TEXT NOT NULL,
    id TEXT NOT NULL,
    workspace_id TEXT,
    created_at TEXT NOT NULL,
    payload_json TEXT NOT NULL,
    PRIMARY KEY (kind, id)
);

CREATE INDEX IF NOT EXISTS idx_entities_workspace_kind
    ON entities (workspace_id, kind, created_at);

CREATE TABLE IF NOT EXISTS audit_events (
    sequence INTEGER PRIMARY KEY AUTOINCREMENT,
    event_id TEXT NOT NULL UNIQUE,
    workspace_id TEXT,
    actor TEXT NOT NULL,
    event_type TEXT NOT NULL,
    entity_type TEXT NOT NULL,
    entity_id TEXT NOT NULL,
    occurred_at TEXT NOT NULL,
    metadata_json TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS idx_audit_workspace_sequence
    ON audit_events (workspace_id, sequence);

CREATE TRIGGER IF NOT EXISTS audit_events_reject_update
BEFORE UPDATE ON audit_events
BEGIN
    SELECT RAISE(ABORT, 'audit_events are append-only');
END;

CREATE TRIGGER IF NOT EXISTS audit_events_reject_delete
BEFORE DELETE ON audit_events
BEGIN
    SELECT RAISE(ABORT, 'audit_events are append-only');
END;
