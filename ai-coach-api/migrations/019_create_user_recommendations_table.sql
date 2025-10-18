-- Migration: Create User Recommendations Table (SQLite)
-- Description: Track individual recommendations per user with lifecycle management
-- Issue: #186 - Migrate Recommendation System to SQLite
-- Original: migrations-postgres/031_create_user_recommendations_table.sql

-- User Recommendations Table
-- Tracks individual recommendations shown to users with status and feedback
CREATE TABLE IF NOT EXISTS user_recommendations (
    id TEXT PRIMARY KEY,

    -- References
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    recommendation_template_id TEXT NOT NULL REFERENCES recommendation_templates(id) ON DELETE CASCADE,
    recovery_score_id TEXT REFERENCES recovery_scores(id) ON DELETE SET NULL,

    -- Status Tracking
    status TEXT NOT NULL DEFAULT 'pending' CHECK (status IN (
        'pending',
        'completed',
        'skipped',
        'expired'
    )),

    -- Effectiveness & Feedback
    effectiveness_rating INTEGER CHECK (effectiveness_rating >= 1 AND effectiveness_rating <= 5),
    user_feedback TEXT,
    skip_reason TEXT,

    -- Lifecycle Timestamps (ISO-8601 format in TEXT)
    shown_at TEXT NOT NULL DEFAULT (datetime('now')),
    completed_at TEXT,
    skipped_at TEXT,
    expired_at TEXT,
    rated_at TEXT,

    -- Metadata (JSON stored as TEXT)
    metadata TEXT DEFAULT '{}',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Indexes for efficient querying
CREATE INDEX idx_user_recommendations_user_id ON user_recommendations(user_id);
CREATE INDEX idx_user_recommendations_status ON user_recommendations(status);
CREATE INDEX idx_user_recommendations_template_id ON user_recommendations(recommendation_template_id);
CREATE INDEX idx_user_recommendations_shown_at ON user_recommendations(shown_at DESC);
CREATE INDEX idx_user_recommendations_user_status ON user_recommendations(user_id, status);

-- Composite index for history queries (SQLite supports partial indexes)
CREATE INDEX idx_user_recommendations_history ON user_recommendations(user_id, shown_at DESC)
    WHERE status IN ('completed', 'skipped', 'expired');

-- Update timestamp trigger
CREATE TRIGGER IF NOT EXISTS update_user_recommendations_updated_at
    AFTER UPDATE ON user_recommendations
    FOR EACH ROW
BEGIN
    UPDATE user_recommendations
    SET updated_at = datetime('now')
    WHERE id = NEW.id;
END;

-- Automatic expiration trigger (set expired_at when status changes to expired)
CREATE TRIGGER IF NOT EXISTS set_user_recommendation_expired_at
    AFTER UPDATE ON user_recommendations
    FOR EACH ROW
    WHEN NEW.status = 'expired' AND OLD.status != 'expired'
BEGIN
    UPDATE user_recommendations
    SET expired_at = datetime('now')
    WHERE id = NEW.id;
END;

-- Automatic completion timestamp trigger
CREATE TRIGGER IF NOT EXISTS set_user_recommendation_completed_at
    AFTER UPDATE ON user_recommendations
    FOR EACH ROW
    WHEN NEW.status = 'completed' AND OLD.status != 'completed'
BEGIN
    UPDATE user_recommendations
    SET completed_at = datetime('now')
    WHERE id = NEW.id;
END;

-- Automatic skip timestamp trigger
CREATE TRIGGER IF NOT EXISTS set_user_recommendation_skipped_at
    AFTER UPDATE ON user_recommendations
    FOR EACH ROW
    WHEN NEW.status = 'skipped' AND OLD.status != 'skipped'
BEGIN
    UPDATE user_recommendations
    SET skipped_at = datetime('now')
    WHERE id = NEW.id;
END;
