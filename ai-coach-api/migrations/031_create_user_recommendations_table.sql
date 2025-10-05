-- Migration: Create User Recommendations Table
-- Description: Track individual recommendations per user with lifecycle management
-- Issue: #130 - Phase 8.2: User Recommendation Tracking & Lifecycle

-- User Recommendations Table
-- Tracks individual recommendations shown to users with status and feedback
CREATE TABLE IF NOT EXISTS user_recommendations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),

    -- References
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    recommendation_template_id UUID NOT NULL REFERENCES recommendation_templates(id) ON DELETE CASCADE,
    recovery_score_id UUID REFERENCES recovery_scores(id) ON DELETE SET NULL,

    -- Status Tracking
    status VARCHAR(20) NOT NULL DEFAULT 'pending' CHECK (status IN (
        'pending',
        'completed',
        'skipped',
        'expired'
    )),

    -- Effectiveness & Feedback
    effectiveness_rating INT CHECK (effectiveness_rating >= 1 AND effectiveness_rating <= 5),
    user_feedback TEXT,
    skip_reason TEXT,

    -- Lifecycle Timestamps
    shown_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ,
    skipped_at TIMESTAMPTZ,
    expired_at TIMESTAMPTZ,
    rated_at TIMESTAMPTZ,

    -- Metadata
    metadata JSONB DEFAULT '{}',
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Indexes for efficient querying
CREATE INDEX idx_user_recommendations_user_id ON user_recommendations(user_id);
CREATE INDEX idx_user_recommendations_status ON user_recommendations(status);
CREATE INDEX idx_user_recommendations_template_id ON user_recommendations(recommendation_template_id);
CREATE INDEX idx_user_recommendations_shown_at ON user_recommendations(shown_at DESC);
CREATE INDEX idx_user_recommendations_user_status ON user_recommendations(user_id, status);

-- Composite index for history queries
CREATE INDEX idx_user_recommendations_history ON user_recommendations(user_id, shown_at DESC)
    WHERE status IN ('completed', 'skipped', 'expired');

-- Update timestamp trigger
CREATE OR REPLACE FUNCTION update_user_recommendation_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_update_user_recommendation_updated_at
    BEFORE UPDATE ON user_recommendations
    FOR EACH ROW
    EXECUTE FUNCTION update_user_recommendation_updated_at();

-- Automatic expiration trigger (set expired_at when status changes to expired)
CREATE OR REPLACE FUNCTION set_user_recommendation_expired_at()
RETURNS TRIGGER AS $$
BEGIN
    IF NEW.status = 'expired' AND OLD.status != 'expired' THEN
        NEW.expired_at = NOW();
    END IF;
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_set_user_recommendation_expired_at
    BEFORE UPDATE ON user_recommendations
    FOR EACH ROW
    WHEN (NEW.status = 'expired')
    EXECUTE FUNCTION set_user_recommendation_expired_at();

-- Comments for documentation
COMMENT ON TABLE user_recommendations IS 'Individual recommendations shown to users with lifecycle tracking and feedback';
COMMENT ON COLUMN user_recommendations.status IS 'Recommendation lifecycle status: pending, completed, skipped, expired';
COMMENT ON COLUMN user_recommendations.effectiveness_rating IS 'User rating 1-5 stars on recommendation effectiveness';
COMMENT ON COLUMN user_recommendations.skip_reason IS 'Optional reason why user skipped the recommendation';
COMMENT ON COLUMN user_recommendations.shown_at IS 'When recommendation was first shown to user';
COMMENT ON COLUMN user_recommendations.completed_at IS 'When user marked recommendation as completed';
COMMENT ON COLUMN user_recommendations.skipped_at IS 'When user skipped the recommendation';
COMMENT ON COLUMN user_recommendations.expired_at IS 'When recommendation expired (7 days after shown_at)';
COMMENT ON COLUMN user_recommendations.rated_at IS 'When user rated the recommendation effectiveness';
