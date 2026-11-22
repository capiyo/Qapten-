CREATE TABLE IF NOT EXISTS users (
    id TEXT PRIMARY KEY NOT NULL,
    auth_provider TEXT NOT NULL,
    provider_subject TEXT NOT NULL,
    created_at TEXT NOT NULL,
    UNIQUE (auth_provider, provider_subject)
);

CREATE TABLE IF NOT EXISTS workspace_memberships (
    workspace_id TEXT NOT NULL,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    role TEXT NOT NULL CHECK (role IN ('owner', 'member')),
    created_at TEXT NOT NULL,
    PRIMARY KEY (workspace_id, user_id)
);

CREATE INDEX IF NOT EXISTS idx_memberships_user
    ON workspace_memberships (user_id, created_at);
CREATE INDEX IF NOT EXISTS idx_memberships_workspace
    ON workspace_memberships (workspace_id, role);
ALTER TABLE users ADD COLUMN personal_workspace_id TEXT;
