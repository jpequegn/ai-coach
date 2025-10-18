-- Migration: Create Recommendation Outcomes Table (SQLite)
-- Description: Track outcomes and effectiveness of completed recommendations
-- Issue: #186 - Migrate Recommendation System to SQLite
-- Original: migrations-postgres/033_create_recommendation_outcomes_table.sql

CREATE TABLE IF NOT EXISTS recommendation_outcomes (
    id TEXT PRIMARY KEY,
    user_recommendation_id TEXT NOT NULL REFERENCES user_recommendations(id) ON DELETE CASCADE,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    recommendation_template_id TEXT NOT NULL REFERENCES recommendation_templates(id) ON DELETE CASCADE,

    -- Timing metrics
    shown_at TEXT NOT NULL,
    completed_at TEXT NOT NULL,
    completion_time_hours REAL NOT NULL,

    -- Recovery metrics
    baseline_recovery_score_id TEXT REFERENCES recovery_scores(id) ON DELETE SET NULL,
    baseline_recovery_score REAL NOT NULL,
    next_day_recovery_score_id TEXT REFERENCES recovery_scores(id) ON DELETE SET NULL,
    next_day_recovery_score REAL,
    recovery_improvement REAL,

    -- User feedback
    user_rating INTEGER CHECK (user_rating BETWEEN 1 AND 5),
    user_feedback TEXT,

    -- Calculated effectiveness
    effectiveness_score REAL,

    -- Metadata (JSON stored as TEXT)
    metadata TEXT DEFAULT '{}',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),

    -- Constraints
    UNIQUE(user_recommendation_id),
    CHECK (completion_time_hours > 0),
    CHECK (baseline_recovery_score >= 0 AND baseline_recovery_score <= 100),
    CHECK (next_day_recovery_score IS NULL OR (next_day_recovery_score >= 0 AND next_day_recovery_score <= 100))
);

-- Indexes for performance
CREATE INDEX idx_recommendation_outcomes_user_id ON recommendation_outcomes(user_id);
CREATE INDEX idx_recommendation_outcomes_template_id ON recommendation_outcomes(recommendation_template_id);
CREATE INDEX idx_recommendation_outcomes_completed_at ON recommendation_outcomes(completed_at);
CREATE INDEX idx_recommendation_outcomes_effectiveness ON recommendation_outcomes(effectiveness_score) WHERE effectiveness_score IS NOT NULL;
CREATE INDEX idx_recommendation_outcomes_pending_next_day ON recommendation_outcomes(completed_at) WHERE next_day_recovery_score IS NULL;

-- Trigger to update updated_at timestamp
CREATE TRIGGER IF NOT EXISTS update_recommendation_outcomes_updated_at
    AFTER UPDATE ON recommendation_outcomes
    FOR EACH ROW
BEGIN
    UPDATE recommendation_outcomes
    SET updated_at = datetime('now')
    WHERE id = NEW.id;
END;

-- Trigger to calculate recovery_improvement when next_day_recovery_score is set
CREATE TRIGGER IF NOT EXISTS calculate_recovery_improvement
    AFTER UPDATE ON recommendation_outcomes
    FOR EACH ROW
    WHEN NEW.next_day_recovery_score IS NOT NULL AND OLD.next_day_recovery_score IS NULL
BEGIN
    UPDATE recommendation_outcomes
    SET recovery_improvement = NEW.next_day_recovery_score - NEW.baseline_recovery_score
    WHERE id = NEW.id;
END;
