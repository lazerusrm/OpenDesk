CREATE TABLE client_access_tokens (
    client_access_token_uuid TEXT PRIMARY KEY NOT NULL,
    token_digest TEXT NOT NULL UNIQUE CHECK (length(token_digest) = 64),
    user_uuid TEXT NOT NULL REFERENCES users(user_uuid) ON DELETE CASCADE,
    rustdesk_id TEXT NOT NULL,
    client_uuid TEXT NOT NULL,
    expires_at TEXT NOT NULL,
    revoked_at TEXT,
    created_at TEXT NOT NULL,
    last_used_at TEXT NOT NULL
);

CREATE INDEX idx_client_access_tokens_user_uuid
    ON client_access_tokens(user_uuid);
CREATE INDEX idx_client_access_tokens_expires_at
    ON client_access_tokens(expires_at);
