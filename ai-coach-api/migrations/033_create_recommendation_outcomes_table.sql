-- Create recommendation_outcomes table for tracking effectiveness
-- This table stores outcome data for completed recommendations to measure their effectiveness

CREATE TABLE recommendation_outcomes (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_recommendation_id UUID NOT NULL REFERENCES user_recommendations(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    recommendation_template_id UUID NOT NULL REFERENCES recommendation_templates(id) ON DELETE CASCADE,

    -- Timing metrics
    shown_at TIMESTAMPTZ NOT NULL,
    completed_at TIMESTAMPTZ NOT NULL,
    completion_time_hours DOUBLE PRECISION NOT NULL,

    -- Recovery metrics
    baseline_recovery_score_id UUID REFERENCES recovery_scores(id) ON DELETE SET NULL,
    baseline_recovery_score DOUBLE PRECISION NOT NULL,
    next_day_recovery_score_id UUID REFERENCES recovery_scores(id) ON DELETE SET NULL,
    next_day_recovery_score DOUBLE PRECISION,
    recovery_improvement DOUBLE PRECISION,

    -- User feedback
    user_rating INTEGER CHECK (user_rating BETWEEN 1 AND 5),
    user_feedback TEXT,

    -- Calculated effectiveness
    effectiveness_score DOUBLE PRECISION,

    -- Metadata
    metadata JSONB DEFAULT '{}'::jsonb,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

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
CREATE TRIGGER update_recommendation_outcomes_updated_at
    BEFORE UPDATE ON recommendation_outcomes
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

-- Comments
COMMENT ON TABLE recommendation_outcomes IS 'Tracks outcomes and effectiveness of completed recommendations';
COMMENT ON COLUMN recommendation_outcomes.completion_time_hours IS 'Hours from shown_at to completed_at';
COMMENT ON COLUMN recommendation_outcomes.baseline_recovery_score IS 'Recovery score at the time recommendation was shown';
COMMENT ON COLUMN recommendation_outcomes.next_day_recovery_score IS 'Recovery score 24 hours after completion';
COMMENT ON COLUMN recommendation_outcomes.recovery_improvement IS 'Difference between next_day and baseline scores';
COMMENT ON COLUMN recommendation_outcomes.effectiveness_score IS 'Combined effectiveness score (0-1) based on recovery improvement, rating, and timing';
