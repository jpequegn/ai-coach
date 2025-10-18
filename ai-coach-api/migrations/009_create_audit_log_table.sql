-- Create audit_log table for tracking admin actions
CREATE TABLE audit_log (
    id TEXT PRIMARY KEY NOT NULL,
    user_id TEXT NOT NULL,
    admin_id TEXT NOT NULL,
    action TEXT NOT NULL,
    entity_type TEXT NOT NULL,
    entity_id TEXT,
    old_value TEXT,
    new_value TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (admin_id) REFERENCES users(id) ON DELETE CASCADE
);

-- Index for querying audit logs by user
CREATE INDEX idx_audit_log_user_id ON audit_log(user_id);

-- Index for querying audit logs by admin
CREATE INDEX idx_audit_log_admin_id ON audit_log(admin_id);

-- Index for querying audit logs by action
CREATE INDEX idx_audit_log_action ON audit_log(action);

-- Index for querying recent audit logs
CREATE INDEX idx_audit_log_created_at ON audit_log(created_at DESC);
