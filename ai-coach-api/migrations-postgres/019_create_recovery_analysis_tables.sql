-- Recovery Analysis Tables
-- Issue #55: Feature - Recovery Monitoring - Analysis & Dashboard

-- Enable UUID extension if not already enabled
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- Recovery Scores (calculated daily)
CREATE TABLE recovery_scores (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    score_date DATE NOT NULL,
    readiness_score DOUBLE PRECISION NOT NULL CHECK (readiness_score >= 0 AND readiness_score <= 100),
    hrv_trend VARCHAR(20) NOT NULL CHECK (hrv_trend IN ('improving', 'stable', 'declining', 'insufficient_data')),
    hrv_deviation DOUBLE PRECISION,
    sleep_quality_score DOUBLE PRECISION CHECK (sleep_quality_score IS NULL OR (sleep_quality_score >= 0 AND sleep_quality_score <= 100)),
    recovery_adequacy DOUBLE PRECISION CHECK (recovery_adequacy IS NULL OR (recovery_adequacy >= 0 AND recovery_adequacy <= 100)),
    rhr_deviation DOUBLE PRECISION,
    training_strain DOUBLE PRECISION,
    recovery_status VARCHAR(20) NOT NULL CHECK (recovery_status IN ('optimal', 'good', 'fair', 'poor', 'critical')),
    recommended_tss_adjustment DOUBLE PRECISION,
    calculated_at TIMESTAMPTZ DEFAULT NOW(),
    model_version VARCHAR(20),
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(user_id, score_date)
);

-- Recovery Alerts
CREATE TABLE recovery_alerts (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    alert_type VARCHAR(50) NOT NULL,
    severity VARCHAR(20) NOT NULL CHECK (severity IN ('info', 'warning', 'critical')),
    recovery_score_id UUID REFERENCES recovery_scores(id) ON DELETE SET NULL,
    message TEXT NOT NULL,
    recommendations JSONB,
    acknowledged_at TIMESTAMPTZ,
    created_at TIMESTAMPTZ DEFAULT NOW()
);

-- Indexes for efficient querying
CREATE INDEX idx_recovery_scores_user_date ON recovery_scores(user_id, score_date DESC);
CREATE INDEX idx_recovery_scores_date ON recovery_scores(score_date DESC);
CREATE INDEX idx_recovery_alerts_user ON recovery_alerts(user_id, created_at DESC);
CREATE INDEX idx_recovery_alerts_unack ON recovery_alerts(user_id, acknowledged_at)
    WHERE acknowledged_at IS NULL;
CREATE INDEX idx_recovery_alerts_severity ON recovery_alerts(severity, created_at DESC);

-- Comments for documentation
COMMENT ON TABLE recovery_scores IS 'Daily calculated recovery scores and metrics for users';
COMMENT ON TABLE recovery_alerts IS 'Recovery-based alerts and warnings for users';

COMMENT ON COLUMN recovery_scores.readiness_score IS 'Overall readiness score (0-100)';
COMMENT ON COLUMN recovery_scores.hrv_trend IS 'Heart rate variability trend direction';
COMMENT ON COLUMN recovery_scores.hrv_deviation IS 'HRV deviation from baseline (percentage)';
COMMENT ON COLUMN recovery_scores.sleep_quality_score IS 'Sleep quality score (0-100)';
COMMENT ON COLUMN recovery_scores.recovery_adequacy IS 'Recovery adequacy score (0-100)';
COMMENT ON COLUMN recovery_scores.rhr_deviation IS 'Resting HR deviation from baseline (percentage)';
COMMENT ON COLUMN recovery_scores.training_strain IS 'Training stress balance';
COMMENT ON COLUMN recovery_scores.recovery_status IS 'Overall recovery status classification';
COMMENT ON COLUMN recovery_scores.recommended_tss_adjustment IS 'Recommended TSS adjustment multiplier (1.0 = no change)';
COMMENT ON COLUMN recovery_scores.model_version IS 'Version of calculation algorithm used';

COMMENT ON COLUMN recovery_alerts.alert_type IS 'Type of alert (poor_recovery, declining_hrv, high_strain, etc.)';
COMMENT ON COLUMN recovery_alerts.severity IS 'Alert severity level';
COMMENT ON COLUMN recovery_alerts.recommendations IS 'JSON array of actionable recommendations';
