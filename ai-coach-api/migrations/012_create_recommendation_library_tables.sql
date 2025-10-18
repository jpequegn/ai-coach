-- Migration: Create Recommendation Library Tables (SQLite)
-- Description: Foundation for intelligent recovery recommendation system
-- Issue: #186 - Migrate Recommendation System to SQLite
-- Original: migrations-postgres/024_create_recommendation_library_tables.sql

-- ============================================================================
-- Recommendation Templates Table
-- ============================================================================
-- Stores reusable recommendation templates with effectiveness tracking
CREATE TABLE IF NOT EXISTS recommendation_templates (
    id TEXT PRIMARY KEY,  -- UUID stored as TEXT

    -- Categorization
    category TEXT NOT NULL CHECK (category IN (
        'sleep',
        'nutrition',
        'active_recovery',
        'stress_management',
        'training_modification'
    )),

    -- Template Content
    title TEXT NOT NULL,
    description TEXT NOT NULL,
    action TEXT NOT NULL,

    -- Priority and Difficulty
    priority_default TEXT NOT NULL DEFAULT 'medium' CHECK (priority_default IN (
        'low',
        'medium',
        'high',
        'critical'
    )),
    difficulty TEXT NOT NULL DEFAULT 'easy' CHECK (difficulty IN (
        'easy',
        'moderate',
        'challenging',
        'advanced'
    )),

    -- Expected Impact
    expected_impact TEXT,
    time_required_minutes INTEGER,

    -- Trigger Conditions (JSON stored as TEXT)
    -- Example: {"poor_sleep": true, "low_hrv": true, "readiness_score_below": 60}
    trigger_conditions TEXT NOT NULL DEFAULT '{}',

    -- User Constraints (JSON stored as TEXT)
    -- Example: {"equipment_needed": ["yoga_mat"], "location": "home", "time_of_day": "evening"}
    user_constraints TEXT NOT NULL DEFAULT '{}',

    -- Effectiveness Tracking
    effectiveness_score REAL NOT NULL DEFAULT 0.5 CHECK (
        effectiveness_score >= 0.0 AND effectiveness_score <= 1.0
    ),
    times_suggested INTEGER NOT NULL DEFAULT 0,
    times_completed INTEGER NOT NULL DEFAULT 0,

    -- Metadata (JSON stored as TEXT)
    metadata TEXT DEFAULT '{}',
    is_active INTEGER NOT NULL DEFAULT 1,  -- Boolean: 0 = false, 1 = true

    -- Timestamps (ISO-8601 format)
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- ============================================================================
-- Indexes for efficient querying
-- ============================================================================
CREATE INDEX IF NOT EXISTS idx_recommendation_templates_category
    ON recommendation_templates(category);

CREATE INDEX IF NOT EXISTS idx_recommendation_templates_active
    ON recommendation_templates(is_active);

CREATE INDEX IF NOT EXISTS idx_recommendation_templates_effectiveness
    ON recommendation_templates(effectiveness_score DESC);

-- Note: GIN indexes for JSONB not applicable in SQLite
-- JSON querying will be done in application layer

-- ============================================================================
-- Recommendation Content Table
-- ============================================================================
-- Links educational content to recommendation templates
CREATE TABLE IF NOT EXISTS recommendation_content (
    id TEXT PRIMARY KEY,  -- UUID stored as TEXT
    recommendation_template_id TEXT NOT NULL REFERENCES recommendation_templates(id) ON DELETE CASCADE,

    -- Content Details
    content_type TEXT NOT NULL CHECK (content_type IN (
        'article',
        'video',
        'guide',
        'research',
        'tutorial'
    )),
    title TEXT NOT NULL,
    description TEXT,
    url TEXT,

    -- Content Metadata
    duration_minutes INTEGER,
    author TEXT,
    source TEXT,

    -- Timestamps
    created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- ============================================================================
-- Indexes for content lookups
-- ============================================================================
CREATE INDEX IF NOT EXISTS idx_recommendation_content_template_id
    ON recommendation_content(recommendation_template_id);

CREATE INDEX IF NOT EXISTS idx_recommendation_content_type
    ON recommendation_content(content_type);

-- ============================================================================
-- Triggers for auto-updating timestamps
-- ============================================================================
-- SQLite uses triggers instead of PostgreSQL functions
CREATE TRIGGER IF NOT EXISTS update_recommendation_templates_updated_at
    AFTER UPDATE ON recommendation_templates
    FOR EACH ROW
BEGIN
    UPDATE recommendation_templates
    SET updated_at = datetime('now')
    WHERE id = NEW.id;
END;

-- ============================================================================
-- Notes (SQLite doesn't support COMMENT, documented here)
-- ============================================================================
-- recommendation_templates: Reusable recommendation templates for recovery optimization
-- category: Recommendation category: sleep, nutrition, active_recovery, stress_management, training_modification
-- trigger_conditions: JSON conditions that trigger this recommendation (e.g., poor_sleep, low_hrv)
-- user_constraints: JSON constraints for recommendation applicability (e.g., equipment, location, time)
-- effectiveness_score: Learned effectiveness score from 0.0 to 1.0, updated based on user outcomes
--
-- recommendation_content: Educational content linked to recommendation templates
-- content_type: Type of content: article, video, guide, research, tutorial
