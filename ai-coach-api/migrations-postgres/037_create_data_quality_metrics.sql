-- Create data quality metrics table for tracking user data completeness and reliability
-- Migration: 037_create_data_quality_metrics

-- Data quality metrics table
CREATE TABLE data_quality_metrics (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    metric_date DATE NOT NULL,

    -- Quality scores (0-100%)
    completeness_score DOUBLE PRECISION NOT NULL CHECK (completeness_score >= 0 AND completeness_score <= 100),
    consistency_score DOUBLE PRECISION CHECK (consistency_score >= 0 AND consistency_score <= 100),
    reliability_score DOUBLE PRECISION CHECK (reliability_score >= 0 AND reliability_score <= 100),

    -- Data tracking
    last_data_timestamp TIMESTAMPTZ,
    days_without_data INT NOT NULL DEFAULT 0 CHECK (days_without_data >= 0),
    data_sources JSONB, -- Active data sources: {"hrv": true, "sleep": true, "rhr": false}

    -- Metadata
    created_at TIMESTAMPTZ DEFAULT NOW() NOT NULL,
    updated_at TIMESTAMPTZ DEFAULT NOW() NOT NULL,

    -- Ensure one metric per user per day
    UNIQUE(user_id, metric_date)
);

-- Indexes for efficient queries
CREATE INDEX idx_data_quality_user_date ON data_quality_metrics(user_id, metric_date DESC);
CREATE INDEX idx_data_quality_completeness ON data_quality_metrics(completeness_score)
    WHERE completeness_score < 50.0;
CREATE INDEX idx_data_quality_missing_data ON data_quality_metrics(days_without_data)
    WHERE days_without_data >= 3;
CREATE INDEX idx_data_quality_date ON data_quality_metrics(metric_date DESC);

-- Trigger to update updated_at timestamp
CREATE OR REPLACE FUNCTION update_data_quality_metrics_updated_at()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ LANGUAGE plpgsql;

CREATE TRIGGER update_data_quality_metrics_updated_at_trigger
    BEFORE UPDATE ON data_quality_metrics
    FOR EACH ROW
    EXECUTE FUNCTION update_data_quality_metrics_updated_at();

-- Comments for documentation
COMMENT ON TABLE data_quality_metrics IS 'Daily data quality metrics for user recovery data tracking';
COMMENT ON COLUMN data_quality_metrics.completeness_score IS 'Percentage of days with complete recovery data (0-100)';
COMMENT ON COLUMN data_quality_metrics.consistency_score IS 'Data consistency score based on day-to-day variation';
COMMENT ON COLUMN data_quality_metrics.reliability_score IS 'Data reliability score based on source quality';
COMMENT ON COLUMN data_quality_metrics.last_data_timestamp IS 'Timestamp of most recent recovery data entry';
COMMENT ON COLUMN data_quality_metrics.days_without_data IS 'Number of consecutive days without any recovery data';
COMMENT ON COLUMN data_quality_metrics.data_sources IS 'JSON object tracking active data sources and their status';
