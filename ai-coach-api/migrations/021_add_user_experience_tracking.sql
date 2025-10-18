-- Migration: Add User Experience Tracking Fields
-- Description: Add experience level and mastery tracking to user_recovery_profiles
-- SQLite-compatible version

-- Add experience tracking columns to user_recovery_profiles
ALTER TABLE user_recovery_profiles
ADD COLUMN experience_level TEXT DEFAULT 'beginner'
CHECK (experience_level IN ('beginner', 'intermediate', 'advanced', 'expert'));

ALTER TABLE user_recovery_profiles
ADD COLUMN mastery_by_category TEXT DEFAULT '{}';  -- JSON object mapping categories to mastery levels

ALTER TABLE user_recovery_profiles
ADD COLUMN total_recommendations_completed INTEGER DEFAULT 0;

ALTER TABLE user_recovery_profiles
ADD COLUMN weeks_active INTEGER DEFAULT 0;

-- Create index for efficient experience level queries
CREATE INDEX IF NOT EXISTS idx_user_recovery_profiles_experience
ON user_recovery_profiles(experience_level);
