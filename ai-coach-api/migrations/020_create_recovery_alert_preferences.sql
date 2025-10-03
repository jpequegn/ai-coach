-- Recovery Alert Preferences
-- Issue #55: Feature - Recovery Monitoring - Analysis & Dashboard (Phase 4: Alert System)

-- Alert preferences for users
CREATE TABLE recovery_alert_preferences (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE UNIQUE,
    enabled BOOLEAN NOT NULL DEFAULT TRUE,
    push_notifications BOOLEAN NOT NULL DEFAULT TRUE,
    email_notifications BOOLEAN NOT NULL DEFAULT FALSE,
    poor_recovery_threshold DOUBLE PRECISION NOT NULL DEFAULT 40.0 CHECK (poor_recovery_threshold >= 0 AND poor_recovery_threshold <= 100),
    critical_recovery_threshold DOUBLE PRECISION NOT NULL DEFAULT 20.0 CHECK (critical_recovery_threshold >= 0 AND critical_recovery_threshold <= 100),
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- Index for efficient user lookup
CREATE INDEX idx_recovery_alert_prefs_user ON recovery_alert_preferences(user_id);

-- Comments for documentation
COMMENT ON TABLE recovery_alert_preferences IS 'User preferences for recovery alerts and notifications';
COMMENT ON COLUMN recovery_alert_preferences.enabled IS 'Whether recovery alerts are enabled for this user';
COMMENT ON COLUMN recovery_alert_preferences.push_notifications IS 'Send push notifications for alerts';
COMMENT ON COLUMN recovery_alert_preferences.email_notifications IS 'Send email notifications for alerts';
COMMENT ON COLUMN recovery_alert_preferences.poor_recovery_threshold IS 'Readiness score below this triggers poor recovery alerts';
COMMENT ON COLUMN recovery_alert_preferences.critical_recovery_threshold IS 'Readiness score below this triggers critical recovery alerts';
