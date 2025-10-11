-- Create data_quality_metrics table for monitoring user data completeness, consistency, and reliability
CREATE TABLE IF NOT EXISTS data_quality_metrics (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,

    -- Quality scores (0.0 to 1.0)
    completeness_score DOUBLE PRECISION NOT NULL DEFAULT 0.0 CHECK (completeness_score >= 0.0 AND completeness_score <= 1.0),
    consistency_score DOUBLE PRECISION NOT NULL DEFAULT 0.0 CHECK (consistency_score >= 0.0 AND consistency_score <= 1.0),
    reliability_score DOUBLE PRECISION NOT NULL DEFAULT 0.0 CHECK (reliability_score >= 0.0 AND reliability_score <= 1.0),
    overall_score DOUBLE PRECISION NOT NULL DEFAULT 0.0 CHECK (overall_score >= 0.0 AND overall_score <= 1.0),

    -- Analysis metadata
    days_analyzed INTEGER NOT NULL DEFAULT 30 CHECK (days_analyzed > 0),
    missing_hrv_days INTEGER NOT NULL DEFAULT 0 CHECK (missing_hrv_days >= 0),
    missing_sleep_days INTEGER NOT NULL DEFAULT 0 CHECK (missing_sleep_days >= 0),
    missing_rhr_days INTEGER NOT NULL DEFAULT 0 CHECK (missing_rhr_days >= 0),

    -- Last reading timestamps
    last_hrv_reading TIMESTAMP WITH TIME ZONE,
    last_sleep_reading TIMESTAMP WITH TIME ZONE,
    last_rhr_reading TIMESTAMP WITH TIME ZONE,

    -- Wearable connection status
    wearable_connected BOOLEAN NOT NULL DEFAULT false,

    -- Reminder tracking
    reminder_sent_at TIMESTAMP WITH TIME ZONE,

    -- Timestamps
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),

    -- Ensure only one metric record per user (updated in place)
    UNIQUE(user_id)
);

-- Create index for efficient user lookups
CREATE INDEX idx_data_quality_metrics_user_id ON data_quality_metrics(user_id);

-- Create index for finding users needing reminders
CREATE INDEX idx_data_quality_metrics_reminder ON data_quality_metrics(wearable_connected, reminder_sent_at, overall_score);

-- Create index for analyzing quality issues
CREATE INDEX idx_data_quality_metrics_quality ON data_quality_metrics(overall_score, completeness_score, consistency_score);

-- Create trigger to update updated_at timestamp
CREATE OR REPLACE FUNCTION update_data_quality_metrics_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER trigger_update_data_quality_metrics_updated_at
    BEFORE UPDATE ON data_quality_metrics
    FOR EACH ROW
    EXECUTE FUNCTION update_data_quality_metrics_updated_at();
