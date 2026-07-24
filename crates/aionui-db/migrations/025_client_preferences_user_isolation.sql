-- OAuth credentials are private per user even when two users connect to the
-- same MCP endpoint.
CREATE TABLE oauth_tokens_scoped (
    owner_user_id TEXT NOT NULL DEFAULT 'system_default_user',
    server_url    TEXT NOT NULL,
    access_token  TEXT NOT NULL,
    refresh_token TEXT,
    token_type    TEXT NOT NULL DEFAULT 'bearer',
    expires_at    INTEGER,
    created_at    INTEGER NOT NULL,
    updated_at    INTEGER NOT NULL,
    PRIMARY KEY (owner_user_id, server_url)
);
INSERT INTO oauth_tokens_scoped (
    owner_user_id, server_url, access_token, refresh_token, token_type,
    expires_at, created_at, updated_at
)
SELECT 'system_default_user', server_url, access_token, refresh_token,
       token_type, expires_at, created_at, updated_at
FROM oauth_tokens;
DROP TABLE oauth_tokens;
ALTER TABLE oauth_tokens_scoped RENAME TO oauth_tokens;
CREATE INDEX IF NOT EXISTS idx_oauth_tokens_owner
    ON oauth_tokens(owner_user_id, created_at);

-- Browser/desktop preferences (theme, font sizes, recent selections, etc.)
-- must follow the authenticated account across devices without leaking into
-- another account. Legacy single-user values remain with the local principal.
CREATE TABLE client_preferences_scoped (
    owner_user_id TEXT NOT NULL DEFAULT 'system_default_user',
    key           TEXT NOT NULL,
    value         TEXT NOT NULL,
    updated_at    INTEGER NOT NULL,
    PRIMARY KEY (owner_user_id, key)
);
INSERT INTO client_preferences_scoped (owner_user_id, key, value, updated_at)
SELECT 'system_default_user', key, value, updated_at
FROM client_preferences;
DROP TABLE client_preferences;
ALTER TABLE client_preferences_scoped RENAME TO client_preferences;
CREATE INDEX IF NOT EXISTS idx_client_preferences_owner
    ON client_preferences(owner_user_id, updated_at);
