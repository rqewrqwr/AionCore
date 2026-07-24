-- Persist tenant ownership in the core database so authorization does not
-- depend on renderer state or gateway response filtering.

ALTER TABLE providers ADD COLUMN owner_user_id TEXT NOT NULL DEFAULT 'system_default_user';
CREATE INDEX IF NOT EXISTS idx_providers_owner_user_id
    ON providers(owner_user_id, created_at);

-- SQLite cannot drop the legacy global UNIQUE(name) constraint in place.
-- Rebuild the table so two users may safely use the same MCP display name.
CREATE TABLE mcp_servers_scoped (
    id               TEXT PRIMARY KEY NOT NULL,
    owner_user_id    TEXT NOT NULL DEFAULT 'system_default_user',
    name             TEXT NOT NULL,
    description      TEXT,
    enabled          INTEGER NOT NULL DEFAULT 0,
    transport_type   TEXT NOT NULL,
    transport_config TEXT NOT NULL,
    tools            TEXT,
    last_test_status TEXT NOT NULL DEFAULT 'disconnected',
    last_connected   INTEGER,
    original_json    TEXT,
    builtin          INTEGER NOT NULL DEFAULT 0,
    deleted_at       INTEGER,
    created_at       INTEGER NOT NULL,
    updated_at       INTEGER NOT NULL,
    UNIQUE(owner_user_id, name)
);

INSERT INTO mcp_servers_scoped (
    id, owner_user_id, name, description, enabled, transport_type,
    transport_config, tools, last_test_status, last_connected, original_json,
    builtin, deleted_at, created_at, updated_at
)
SELECT
    id,
    CASE WHEN builtin = 1 THEN '__shared__' ELSE 'system_default_user' END,
    name, description, enabled, transport_type, transport_config, tools,
    last_test_status, last_connected, original_json, builtin, deleted_at,
    created_at, updated_at
FROM mcp_servers;

DROP TABLE mcp_servers;
ALTER TABLE mcp_servers_scoped RENAME TO mcp_servers;
CREATE INDEX IF NOT EXISTS idx_mcp_servers_name ON mcp_servers(name);
CREATE INDEX IF NOT EXISTS idx_mcp_servers_owner_name
    ON mcp_servers(owner_user_id, name);
CREATE INDEX IF NOT EXISTS idx_mcp_servers_enabled
    ON mcp_servers(owner_user_id, enabled);
CREATE INDEX IF NOT EXISTS idx_mcp_servers_deleted_at ON mcp_servers(deleted_at);

ALTER TABLE cron_jobs ADD COLUMN owner_user_id TEXT NOT NULL DEFAULT 'system_default_user';
UPDATE cron_jobs
SET owner_user_id = COALESCE(
    (SELECT conversations.user_id
     FROM conversations
     WHERE conversations.id = cron_jobs.conversation_id),
    'system_default_user'
);
CREATE INDEX IF NOT EXISTS idx_cron_jobs_owner_user_id
    ON cron_jobs(owner_user_id, created_at);

-- The enterprise gateway already writes this table. Defining it in the core
-- migration makes runtime authorization available before the first gateway
-- request and keeps local/test databases deterministic.
CREATE TABLE IF NOT EXISTS zigo_resource_ownership (
    resource_type TEXT NOT NULL,
    resource_id TEXT NOT NULL,
    owner_subject_id TEXT NOT NULL,
    scope TEXT NOT NULL DEFAULT 'PERSONAL'
        CHECK (scope IN ('PERSONAL', 'SYSTEM')),
    created_at INTEGER NOT NULL,
    updated_at INTEGER NOT NULL,
    PRIMARY KEY (resource_type, resource_id)
);
CREATE INDEX IF NOT EXISTS idx_zigo_resource_owner
    ON zigo_resource_ownership(owner_subject_id, resource_type);
