-- Migration: Create Recommendation Progression Table
-- Description: Track progressive variants of recommendations for different experience levels
-- SQLite-compatible version

CREATE TABLE IF NOT EXISTS recommendation_progression (
    id TEXT PRIMARY KEY,  -- UUID as TEXT
    recommendation_template_id TEXT NOT NULL REFERENCES recommendation_templates(id) ON DELETE CASCADE,
    version TEXT NOT NULL CHECK (version IN ('beginner', 'intermediate', 'advanced', 'expert')),
    difficulty_modifier REAL DEFAULT 1.0,  -- Multiplier for base difficulty
    time_modifier REAL DEFAULT 1.0,  -- Multiplier for time_required_minutes
    complexity_description TEXT,  -- User-friendly description of this version's complexity
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),

    -- Ensure only one version per template
    UNIQUE(recommendation_template_id, version)
);

-- Trigger to auto-update updated_at timestamp
CREATE TRIGGER IF NOT EXISTS update_recommendation_progression_timestamp
    AFTER UPDATE ON recommendation_progression
    FOR EACH ROW
BEGIN
    UPDATE recommendation_progression SET updated_at = datetime('now') WHERE id = NEW.id;
END;

-- Indexes for efficient querying
CREATE INDEX IF NOT EXISTS idx_rec_progression_template
ON recommendation_progression(recommendation_template_id);

CREATE INDEX IF NOT EXISTS idx_rec_progression_version
ON recommendation_progression(version);

-- Comments explaining the schema
-- version: User experience level this variant is designed for
-- difficulty_modifier: 0.5 = easier, 1.0 = baseline, 2.0 = twice as hard
-- time_modifier: 0.5 = half the time, 1.0 = baseline, 2.0 = twice the time
-- complexity_description: User-facing explanation of what makes this version different
