-- Training Adjustments Tracking
-- Issue #115: Phase 5 - Training Plan Integration with Recovery Scores

-- Enable UUID extension if not already enabled
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- Training Adjustments Table (tracks adjustment decisions and outcomes)
CREATE TABLE training_adjustments (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    adjustment_date DATE NOT NULL,
    recovery_score_id UUID REFERENCES recovery_scores(id) ON DELETE SET NULL,
    original_tss DOUBLE PRECISION,
    recommended_tss DOUBLE PRECISION,
    actual_tss DOUBLE PRECISION,
    adjustment_applied BOOLEAN DEFAULT FALSE,
    adjustment_type VARCHAR(50), -- reduce_intensity, reduce_volume, rest_day, no_change
    outcome_recovery_score DOUBLE PRECISION,
    outcome_training_quality VARCHAR(20), -- excellent, good, fair, poor, terrible
    user_feedback TEXT,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- Indexes for efficient querying
CREATE INDEX idx_training_adjustments_user_date ON training_adjustments(user_id, adjustment_date DESC);
CREATE INDEX idx_training_adjustments_applied ON training_adjustments(user_id, adjustment_applied);
CREATE INDEX idx_training_adjustments_recovery_score ON training_adjustments(recovery_score_id);

-- Add trigger to update updated_at timestamp
CREATE TRIGGER update_training_adjustments_updated_at BEFORE UPDATE ON training_adjustments
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- Comments for documentation
COMMENT ON TABLE training_adjustments IS 'Tracking of training adjustments based on recovery scores and their outcomes';

COMMENT ON COLUMN training_adjustments.adjustment_date IS 'Date when the adjustment was recommended';
COMMENT ON COLUMN training_adjustments.original_tss IS 'Originally planned Training Stress Score';
COMMENT ON COLUMN training_adjustments.recommended_tss IS 'AI-recommended adjusted Training Stress Score';
COMMENT ON COLUMN training_adjustments.actual_tss IS 'Actually completed Training Stress Score';
COMMENT ON COLUMN training_adjustments.adjustment_applied IS 'Whether the user applied the recommended adjustment';
COMMENT ON COLUMN training_adjustments.adjustment_type IS 'Type of adjustment recommended';
COMMENT ON COLUMN training_adjustments.outcome_recovery_score IS 'Recovery score on the following day';
COMMENT ON COLUMN training_adjustments.outcome_training_quality IS 'User-reported quality of the training session';
