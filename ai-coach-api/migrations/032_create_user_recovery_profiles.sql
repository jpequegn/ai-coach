-- Create user recovery profiles table
-- This table stores personalized recovery preferences and learned patterns for each user

CREATE TABLE user_recovery_profiles (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL UNIQUE REFERENCES users(id) ON DELETE CASCADE,

    -- Preferred recovery activities and constraints
    preferred_recovery_activities JSONB DEFAULT '[]'::jsonb,
    available_equipment JSONB DEFAULT '[]'::jsonb,
    dietary_preferences JSONB DEFAULT '{}'::jsonb,
    sleep_schedule JSONB DEFAULT '{}'::jsonb,
    stress_triggers JSONB DEFAULT '[]'::jsonb,

    -- Auto-learned effectiveness data
    effective_techniques JSONB DEFAULT '[]'::jsonb,
    completion_stats JSONB DEFAULT '{}'::jsonb,

    -- Profile metadata
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

-- Create index on user_id for fast lookups
CREATE INDEX idx_user_recovery_profiles_user_id ON user_recovery_profiles(user_id);

-- Add trigger to update updated_at timestamp
CREATE OR REPLACE FUNCTION update_user_recovery_profile_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_update_user_recovery_profile_updated_at
    BEFORE UPDATE ON user_recovery_profiles
    FOR EACH ROW
    EXECUTE FUNCTION update_user_recovery_profile_updated_at();

-- Add comments for documentation
COMMENT ON TABLE user_recovery_profiles IS 'Stores personalized recovery preferences and learned patterns for each user';
COMMENT ON COLUMN user_recovery_profiles.preferred_recovery_activities IS 'Array of preferred recovery activity types (e.g., ["yoga", "foam_rolling", "walking"])';
COMMENT ON COLUMN user_recovery_profiles.available_equipment IS 'Array of equipment user has access to (e.g., ["yoga_mat", "foam_roller", "massage_gun"])';
COMMENT ON COLUMN user_recovery_profiles.dietary_preferences IS 'Object with dietary info: {vegetarian: bool, vegan: bool, allergies: [], restrictions: [], supplements: []}';
COMMENT ON COLUMN user_recovery_profiles.sleep_schedule IS 'Object with sleep patterns: {bedtime: "22:00", wake_time: "06:00", target_duration_hours: 8, nap_preference: bool}';
COMMENT ON COLUMN user_recovery_profiles.stress_triggers IS 'Array of known stress triggers for personalization';
COMMENT ON COLUMN user_recovery_profiles.effective_techniques IS 'Array of recommendations that work well for this user with effectiveness scores';
COMMENT ON COLUMN user_recovery_profiles.completion_stats IS 'Object with category-level completion rates and preferences';
