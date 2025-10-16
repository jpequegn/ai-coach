-- Create user_roles table (SQLite-compatible)
CREATE TABLE user_roles (
    user_id TEXT PRIMARY KEY NOT NULL,
    role TEXT NOT NULL DEFAULT 'athlete',
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE
);

-- Index for role lookups
CREATE INDEX idx_user_roles_role ON user_roles(role);
