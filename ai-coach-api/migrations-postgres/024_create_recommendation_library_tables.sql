-- Migration: Create Recommendation Library Tables
-- Description: Foundation for intelligent recovery recommendation system
-- Issue: #129 - Phase 8.1: Recommendation Library Foundation

-- Recommendation Templates Table
-- Stores reusable recommendation templates with effectiveness tracking
CREATE TABLE IF NOT EXISTS recommendation_templates (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    -- Categorization
    category VARCHAR(50) NOT NULL CHECK (category IN (
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
    priority_default VARCHAR(20) NOT NULL DEFAULT 'medium' CHECK (priority_default IN (
        'low',
        'medium',
        'high',
        'critical'
    )),
    difficulty VARCHAR(20) NOT NULL DEFAULT 'easy' CHECK (difficulty IN (
        'easy',
        'moderate',
        'challenging',
        'advanced'
    )),

    -- Expected Impact
    expected_impact TEXT,
    time_required_minutes INT,

    -- Trigger Conditions (JSONB for flexibility)
    -- Example: {"poor_sleep": true, "low_hrv": true, "readiness_score_below": 60}
    trigger_conditions JSONB NOT NULL DEFAULT '{}',

    -- User Constraints (JSONB for flexibility)
    -- Example: {"equipment_needed": ["yoga_mat"], "location": "home", "time_of_day": "evening"}
    user_constraints JSONB NOT NULL DEFAULT '{}',

    -- Effectiveness Tracking
    effectiveness_score DOUBLE PRECISION NOT NULL DEFAULT 0.5 CHECK (
        effectiveness_score >= 0.0 AND effectiveness_score <= 1.0
    ),
    times_suggested INT NOT NULL DEFAULT 0,
    times_completed INT NOT NULL DEFAULT 0,

    -- Metadata
    metadata JSONB DEFAULT '{}',
    is_active BOOLEAN NOT NULL DEFAULT true,

    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes for efficient querying
CREATE INDEX IF NOT EXISTS idx_recommendation_templates_category ON recommendation_templates(category);
CREATE INDEX IF NOT EXISTS idx_recommendation_templates_active ON recommendation_templates(is_active);
CREATE INDEX IF NOT EXISTS idx_recommendation_templates_effectiveness ON recommendation_templates(effectiveness_score DESC);
CREATE INDEX IF NOT EXISTS idx_recommendation_templates_trigger_conditions ON recommendation_templates USING gin(trigger_conditions);

-- Recommendation Content Table
-- Links educational content to recommendation templates
CREATE TABLE IF NOT EXISTS recommendation_content (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    recommendation_template_id UUID NOT NULL REFERENCES recommendation_templates(id) ON DELETE CASCADE,

    -- Content Details
    content_type VARCHAR(20) NOT NULL CHECK (content_type IN (
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
    duration_minutes INT,
    author VARCHAR(255),
    source VARCHAR(255),

    -- Timestamps
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes for content lookups
CREATE INDEX IF NOT EXISTS idx_recommendation_content_template_id ON recommendation_content(recommendation_template_id);
CREATE INDEX IF NOT EXISTS idx_recommendation_content_type ON recommendation_content(content_type);

-- Update timestamp trigger for recommendation_templates
CREATE OR REPLACE FUNCTION update_recommendation_template_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

DROP TRIGGER IF EXISTS trigger_update_recommendation_template_updated_at ON recommendation_templates;
CREATE TRIGGER trigger_update_recommendation_template_updated_at
    BEFORE UPDATE ON recommendation_templates
    FOR EACH ROW
    EXECUTE FUNCTION update_recommendation_template_updated_at();

-- Comments for documentation
COMMENT ON TABLE recommendation_templates IS 'Reusable recommendation templates for recovery optimization';
COMMENT ON COLUMN recommendation_templates.category IS 'Recommendation category: sleep, nutrition, active_recovery, stress_management, training_modification';
COMMENT ON COLUMN recommendation_templates.trigger_conditions IS 'JSON conditions that trigger this recommendation (e.g., poor_sleep, low_hrv)';
COMMENT ON COLUMN recommendation_templates.user_constraints IS 'JSON constraints for recommendation applicability (e.g., equipment, location, time)';
COMMENT ON COLUMN recommendation_templates.effectiveness_score IS 'Learned effectiveness score from 0.0 to 1.0, updated based on user outcomes';

COMMENT ON TABLE recommendation_content IS 'Educational content linked to recommendation templates';
COMMENT ON COLUMN recommendation_content.content_type IS 'Type of content: article, video, guide, research, tutorial';
