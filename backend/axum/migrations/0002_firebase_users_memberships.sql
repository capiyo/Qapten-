CREATE TABLE IF NOT EXISTS users (
    id TEXT PRIMARY KEY NOT NULL,
    provider TEXT NOT NULL,
    subject TEXT NOT NULL,
    email TEXT,
    personal_workspace_id TEXT UNIQUE,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL,
    UNIQUE(provider, subject)
);

CREATE TABLE IF NOT EXISTS workspace_memberships (
    user_id TEXT NOT NULL,
    workspace_id TEXT NOT NULL,
    role TEXT NOT NULL CHECK(role IN ('owner', 'member')),
    created_at TEXT NOT NULL,
    PRIMARY KEY(user_id, workspace_id),
    FOREIGN KEY(user_id) REFERENCES users(id)
);

CREATE INDEX IF NOT EXISTS idx_memberships_workspace
    ON workspace_memberships(workspace_id, user_id);

CREATE TRIGGER IF NOT EXISTS workspace_memberships_require_workspace
BEFORE INSERT ON workspace_memberships
WHEN NOT EXISTS (
    SELECT 1 FROM entities WHERE kind = 'workspace' AND id = NEW.workspace_id
)
BEGIN
    SELECT RAISE(ABORT, 'workspace membership requires an existing workspace');
END;
