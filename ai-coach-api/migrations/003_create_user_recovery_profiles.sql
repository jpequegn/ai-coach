-- Create user recovery profiles table (SQLite-compatible)
CREATE TABLE user_recovery_profiles (
    id TEXT PRIMARY KEY NOT NULL,
    user_id TEXT NOT NULL UNIQUE REFERENCES users(id) ON DELETE CASCADE,

    -- Preferred recovery activities and constraints (JSON as TEXT)
    preferred_recovery_activities TEXT DEFAULT '[]',
    available_equipment TEXT DEFAULT '[]',
    dietary_preferences TEXT DEFAULT '{}',
    sleep_schedule TEXT DEFAULT '{}',
    stress_triggers TEXT DEFAULT '[]',

    -- Auto-learned effectiveness data (JSON as TEXT)
    effective_techniques TEXT DEFAULT '[]',
    completion_stats TEXT DEFAULT '{}',

    -- Profile metadata
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Create index on user_id for fast lookups
CREATE INDEX idx_user_recovery_profiles_user_id ON user_recovery_profiles(user_id);

-- SQLite trigger to update updated_at timestamp
CREATE TRIGGER trigger_update_user_recovery_profile_updated_at
    AFTER UPDATE ON user_recovery_profiles
    FOR EACH ROW
BEGIN
    UPDATE user_recovery_profiles SET updated_at = CURRENT_TIMESTAMP WHERE id = NEW.id;
END;
