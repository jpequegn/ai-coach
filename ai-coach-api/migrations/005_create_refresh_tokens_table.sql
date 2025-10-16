-- Create refresh_tokens table (SQLite-compatible)
CREATE TABLE refresh_tokens (
    id TEXT PRIMARY KEY NOT NULL,
    user_id TEXT NOT NULL,
    token_hash TEXT NOT NULL,
    expires_at TEXT NOT NULL,
    revoked INTEGER NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

-- Index for user_id lookups
CREATE INDEX idx_refresh_tokens_user_id ON refresh_tokens(user_id);

-- Index for token_hash lookups
CREATE INDEX idx_refresh_tokens_token_hash ON refresh_tokens(token_hash);

-- Index for expiration checks
CREATE INDEX idx_refresh_tokens_expires_at ON refresh_tokens(expires_at);
