CREATE TABLE IF NOT EXISTS chats (
    id TEXT PRIMARY KEY NOT NULL,
    workspace_id TEXT NOT NULL,
    owner_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    title TEXT NOT NULL,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    archived INTEGER NOT NULL DEFAULT 0 CHECK (archived IN (0, 1)),
    memory_version INTEGER NOT NULL DEFAULT 0
);
CREATE INDEX IF NOT EXISTS idx_chats_workspace_updated
    ON chats (workspace_id, archived, updated_at DESC);

CREATE TABLE IF NOT EXISTS chat_messages (
    id TEXT PRIMARY KEY NOT NULL,
    chat_id TEXT NOT NULL REFERENCES chats(id) ON DELETE CASCADE,
    role TEXT NOT NULL CHECK (role IN ('user', 'assistant', 'system')),
    content TEXT NOT NULL,
    created_at TEXT NOT NULL,
    answered_by TEXT
);
CREATE INDEX IF NOT EXISTS idx_chat_messages_chat_created
    ON chat_messages (chat_id, created_at, id);

CREATE TABLE IF NOT EXISTS memory_events (
    sequence INTEGER PRIMARY KEY AUTOINCREMENT,
    event_id TEXT NOT NULL UNIQUE,
    workspace_id TEXT NOT NULL,
    chat_id TEXT NOT NULL REFERENCES chats(id) ON DELETE CASCADE,
    memory_entry_id TEXT NOT NULL,
    event_type TEXT NOT NULL CHECK (event_type IN ('created', 'approved', 'edited', 'rejected', 'deleted')),
    payload_json TEXT NOT NULL,
    actor TEXT NOT NULL,
    occurred_at TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_memory_events_chat_sequence
    ON memory_events (chat_id, sequence);
CREATE TRIGGER IF NOT EXISTS memory_events_reject_update
BEFORE UPDATE ON memory_events
BEGIN
    SELECT RAISE(ABORT, 'memory_events are append-only');
END;
CREATE TRIGGER IF NOT EXISTS memory_events_reject_delete
BEFORE DELETE ON memory_events
BEGIN
    SELECT RAISE(ABORT, 'memory_events are append-only');
END;

CREATE TABLE IF NOT EXISTS resume_brief_cache (
    chat_id TEXT PRIMARY KEY NOT NULL REFERENCES chats(id) ON DELETE CASCADE,
    memory_version INTEGER NOT NULL,
    last_message_id TEXT,
    prompt_version TEXT NOT NULL,
    brief TEXT NOT NULL,
    generated_at TEXT NOT NULL
);
