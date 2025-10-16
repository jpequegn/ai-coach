-- Create token_blacklist table (SQLite-compatible)
CREATE TABLE token_blacklist (
    jti TEXT PRIMARY KEY NOT NULL,
    expires_at TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Index for expiration checks
CREATE INDEX idx_token_blacklist_expires_at ON token_blacklist(expires_at);
