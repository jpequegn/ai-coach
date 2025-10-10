-- Add timezone support for daily recovery calculation scheduling
-- Migration: 035_add_user_timezone

-- Add timezone field to users table
ALTER TABLE users
ADD COLUMN IF NOT EXISTS timezone VARCHAR(50) DEFAULT 'UTC' NOT NULL;

-- Add active field if it doesn't exist (for filtering active users)
ALTER TABLE users
ADD COLUMN IF NOT EXISTS active BOOLEAN DEFAULT true NOT NULL;

-- Create index for timezone queries (only for active users)
CREATE INDEX IF NOT EXISTS idx_users_timezone ON users(timezone) WHERE active = true;

-- Create index for active users
CREATE INDEX IF NOT EXISTS idx_users_active ON users(active) WHERE active = true;

-- Comments for documentation
COMMENT ON COLUMN users.timezone IS 'User timezone in IANA format (e.g., America/New_York, Europe/London, UTC)';
COMMENT ON COLUMN users.active IS 'Whether the user account is active (for filtering in background jobs)';
