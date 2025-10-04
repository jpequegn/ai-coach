-- Training Recovery Settings
-- Issue #115: Phase 5 - Training Plan Integration with Recovery Scores

-- Enable UUID extension if not already enabled
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- Training Recovery Settings Table
CREATE TABLE training_recovery_settings (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE UNIQUE,
    auto_adjust_enabled BOOLEAN DEFAULT FALSE,
    adjustment_aggressiveness VARCHAR(20) DEFAULT 'moderate' CHECK (adjustment_aggressiveness IN ('conservative', 'moderate', 'aggressive')),
    min_rest_days_per_week INTEGER DEFAULT 1 CHECK (min_rest_days_per_week >= 0 AND min_rest_days_per_week <= 7),
    max_consecutive_training_days INTEGER DEFAULT 6 CHECK (max_consecutive_training_days >= 1 AND max_consecutive_training_days <= 14),
    allow_intensity_reduction BOOLEAN DEFAULT TRUE,
    allow_volume_reduction BOOLEAN DEFAULT TRUE,
    allow_workout_swap BOOLEAN DEFAULT FALSE,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- Indexes
CREATE INDEX idx_training_recovery_settings_user ON training_recovery_settings(user_id);

-- Add trigger to update updated_at timestamp
CREATE TRIGGER update_training_recovery_settings_updated_at BEFORE UPDATE ON training_recovery_settings
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

-- Comments for documentation
COMMENT ON TABLE training_recovery_settings IS 'User preferences for recovery-based training adjustments';

COMMENT ON COLUMN training_recovery_settings.auto_adjust_enabled IS 'Whether to automatically adjust training based on recovery';
COMMENT ON COLUMN training_recovery_settings.adjustment_aggressiveness IS 'How aggressively to adjust workouts (conservative/moderate/aggressive)';
COMMENT ON COLUMN training_recovery_settings.min_rest_days_per_week IS 'Minimum number of rest days per week';
COMMENT ON COLUMN training_recovery_settings.max_consecutive_training_days IS 'Maximum consecutive training days before forced rest';
COMMENT ON COLUMN training_recovery_settings.allow_intensity_reduction IS 'Allow reduction of workout intensity';
COMMENT ON COLUMN training_recovery_settings.allow_volume_reduction IS 'Allow reduction of workout volume';
COMMENT ON COLUMN training_recovery_settings.allow_workout_swap IS 'Allow swapping workouts based on recovery';
