-- Recovery Monitoring Tables
-- Issue #54: Feature - Recovery Monitoring - Wearable Integration & Data Collection

-- Enable UUID extension if not already enabled
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";

-- HRV (Heart Rate Variability) Readings
CREATE TABLE hrv_readings (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    measurement_date DATE NOT NULL,
    measurement_timestamp TIMESTAMPTZ NOT NULL,
    rmssd DOUBLE PRECISION NOT NULL CHECK (rmssd >= 0 AND rmssd <= 200),
    sdnn DOUBLE PRECISION CHECK (sdnn IS NULL OR (sdnn >= 0 AND sdnn <= 200)),
    pnn50 DOUBLE PRECISION CHECK (pnn50 IS NULL OR (pnn50 >= 0 AND pnn50 <= 100)),
    source VARCHAR(50) NOT NULL CHECK (source IN ('oura', 'whoop', 'manual', 'apple_health', 'garmin', 'polar', 'fitbit')),
    metadata JSONB,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(user_id, measurement_timestamp, source)
);

-- Sleep Data
CREATE TABLE sleep_data (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    sleep_date DATE NOT NULL,
    total_sleep_hours DOUBLE PRECISION NOT NULL CHECK (total_sleep_hours >= 0 AND total_sleep_hours <= 24),
    deep_sleep_hours DOUBLE PRECISION CHECK (deep_sleep_hours IS NULL OR (deep_sleep_hours >= 0 AND deep_sleep_hours <= 24)),
    rem_sleep_hours DOUBLE PRECISION CHECK (rem_sleep_hours IS NULL OR (rem_sleep_hours >= 0 AND rem_sleep_hours <= 24)),
    light_sleep_hours DOUBLE PRECISION CHECK (light_sleep_hours IS NULL OR (light_sleep_hours >= 0 AND light_sleep_hours <= 24)),
    awake_hours DOUBLE PRECISION CHECK (awake_hours IS NULL OR (awake_hours >= 0 AND awake_hours <= 24)),
    sleep_efficiency DOUBLE PRECISION CHECK (sleep_efficiency IS NULL OR (sleep_efficiency >= 0 AND sleep_efficiency <= 100)),
    sleep_latency_minutes INTEGER CHECK (sleep_latency_minutes IS NULL OR sleep_latency_minutes >= 0),
    bedtime TIMESTAMPTZ,
    wake_time TIMESTAMPTZ,
    source VARCHAR(50) NOT NULL CHECK (source IN ('oura', 'whoop', 'manual', 'apple_health', 'garmin', 'polar', 'fitbit')),
    metadata JSONB,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(user_id, sleep_date, source)
);

-- Resting Heart Rate Data
CREATE TABLE resting_hr_data (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    measurement_date DATE NOT NULL,
    measurement_timestamp TIMESTAMPTZ NOT NULL,
    resting_hr DOUBLE PRECISION NOT NULL CHECK (resting_hr >= 30 AND resting_hr <= 150),
    source VARCHAR(50) NOT NULL CHECK (source IN ('oura', 'whoop', 'manual', 'apple_health', 'garmin', 'polar', 'fitbit')),
    metadata JSONB,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(user_id, measurement_timestamp, source)
);

-- Recovery Baselines (calculated averages)
CREATE TABLE recovery_baselines (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE UNIQUE,
    hrv_baseline_rmssd DOUBLE PRECISION CHECK (hrv_baseline_rmssd IS NULL OR (hrv_baseline_rmssd >= 0 AND hrv_baseline_rmssd <= 200)),
    rhr_baseline DOUBLE PRECISION CHECK (rhr_baseline IS NULL OR (rhr_baseline >= 30 AND rhr_baseline <= 150)),
    typical_sleep_hours DOUBLE PRECISION CHECK (typical_sleep_hours IS NULL OR (typical_sleep_hours >= 0 AND typical_sleep_hours <= 24)),
    calculated_at TIMESTAMPTZ NOT NULL,
    data_points_count INTEGER NOT NULL CHECK (data_points_count >= 0),
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW()
);

-- Wearable Device Connections
CREATE TABLE wearable_connections (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    provider VARCHAR(50) NOT NULL CHECK (provider IN ('oura', 'whoop', 'apple_health', 'garmin', 'polar', 'fitbit')),
    access_token TEXT,
    refresh_token TEXT,
    token_expires_at TIMESTAMPTZ,
    provider_user_id VARCHAR(255),
    scopes TEXT[],
    connected_at TIMESTAMPTZ DEFAULT NOW(),
    last_sync_at TIMESTAMPTZ,
    is_active BOOLEAN DEFAULT TRUE,
    metadata JSONB,
    created_at TIMESTAMPTZ DEFAULT NOW(),
    updated_at TIMESTAMPTZ DEFAULT NOW(),
    UNIQUE(user_id, provider)
);

-- Indexes for efficient querying
CREATE INDEX idx_hrv_user_date ON hrv_readings(user_id, measurement_date DESC);
CREATE INDEX idx_hrv_user_timestamp ON hrv_readings(user_id, measurement_timestamp DESC);
CREATE INDEX idx_sleep_user_date ON sleep_data(user_id, sleep_date DESC);
CREATE INDEX idx_rhr_user_date ON resting_hr_data(user_id, measurement_date DESC);
CREATE INDEX idx_rhr_user_timestamp ON resting_hr_data(user_id, measurement_timestamp DESC);
CREATE INDEX idx_wearable_connections_user ON wearable_connections(user_id, is_active);
CREATE INDEX idx_wearable_connections_provider ON wearable_connections(provider, is_active);

-- Comments for documentation
COMMENT ON TABLE hrv_readings IS 'Heart Rate Variability readings from wearables and manual input';
COMMENT ON TABLE sleep_data IS 'Sleep data including stages and efficiency from wearables and manual input';
COMMENT ON TABLE resting_hr_data IS 'Resting heart rate measurements from wearables and manual input';
COMMENT ON TABLE recovery_baselines IS 'Calculated baseline values for recovery metrics per user';
COMMENT ON TABLE wearable_connections IS 'OAuth connections to wearable device platforms';

COMMENT ON COLUMN hrv_readings.rmssd IS 'Root Mean Square of Successive Differences (primary HRV metric, ms)';
COMMENT ON COLUMN hrv_readings.sdnn IS 'Standard Deviation of NN intervals (ms)';
COMMENT ON COLUMN hrv_readings.pnn50 IS 'Percentage of successive NN intervals differing >50ms (%)';
COMMENT ON COLUMN sleep_data.sleep_efficiency IS 'Sleep efficiency percentage (0-100)';
COMMENT ON COLUMN sleep_data.sleep_latency_minutes IS 'Time to fall asleep (minutes)';
COMMENT ON COLUMN recovery_baselines.data_points_count IS 'Number of data points used to calculate baselines';
