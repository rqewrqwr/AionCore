-- Personal server workspaces for the intranet multi-user deployment.
--
-- File contents remain on the server filesystem because agent CLIs write
-- directly to their working directory. The database is authoritative for
-- identity and ownership.

CREATE TABLE IF NOT EXISTS workspaces (
    id            TEXT PRIMARY KEY NOT NULL,
    owner_user_id TEXT NOT NULL,
    name          TEXT NOT NULL,
    root_path     TEXT NOT NULL,
    storage_type  TEXT NOT NULL DEFAULT 'managed'
        CHECK (storage_type IN ('managed', 'local_external')),
    status        TEXT NOT NULL DEFAULT 'active'
        CHECK (status IN ('active', 'archived')),
    created_at    INTEGER NOT NULL,
    updated_at    INTEGER NOT NULL,
    UNIQUE(owner_user_id, root_path)
);

CREATE INDEX IF NOT EXISTS idx_workspaces_owner_status
    ON workspaces(owner_user_id, status, updated_at DESC);

ALTER TABLE conversations ADD COLUMN workspace_id TEXT;
CREATE INDEX IF NOT EXISTS idx_conversations_workspace_id
    ON conversations(workspace_id);

ALTER TABLE teams ADD COLUMN workspace_id TEXT;
CREATE INDEX IF NOT EXISTS idx_teams_workspace_id
    ON teams(workspace_id);
