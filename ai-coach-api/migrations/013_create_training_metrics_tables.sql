-- Add training metrics tables for PMC and zone settings

-- Performance Management Chart table
CREATE TABLE performance_management_chart (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    date DATE NOT NULL,
    ctl DECIMAL(10,2) NOT NULL DEFAULT 0, -- Chronic Training Load (Fitness)
    atl DECIMAL(10,2) NOT NULL DEFAULT 0, -- Acute Training Load (Fatigue)
    tsb DECIMAL(10,2) NOT NULL DEFAULT 0, -- Training Stress Balance (Form)
    tss_daily DECIMAL(10,2) NOT NULL DEFAULT 0, -- Daily Training Stress Score
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),

    -- Ensure one record per user per date
    UNIQUE(user_id, date)
);

-- Zone settings table
CREATE TABLE zone_settings (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    ftp DECIMAL(8,2), -- Functional Threshold Power (watts)
    lthr INTEGER, -- Lactate Threshold Heart Rate (bpm)
    max_heart_rate INTEGER, -- Maximum Heart Rate (bpm)
    resting_heart_rate INTEGER, -- Resting Heart Rate (bpm)
    threshold_pace DECIMAL(8,2), -- Threshold pace (seconds per meter)
    weight DECIMAL(6,2), -- Body weight (kg)
    power_zones JSONB, -- Custom power zone thresholds
    heart_rate_zones JSONB, -- Custom HR zone thresholds
    pace_zones JSONB, -- Custom pace zone thresholds
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),

    -- Ensure one record per user
    UNIQUE(user_id)
);

-- Power curve table for peak power analysis
CREATE TABLE power_curve (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    session_id UUID REFERENCES training_sessions(id) ON DELETE CASCADE,
    date DATE NOT NULL,
    duration_seconds INTEGER NOT NULL,
    max_power DECIMAL(8,2) NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),

    -- Index for efficient queries
    INDEX idx_power_curve_user_duration (user_id, duration_seconds),
    INDEX idx_power_curve_date (date)
);

-- Critical power model table
CREATE TABLE critical_power_model (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    critical_power DECIMAL(8,2) NOT NULL, -- CP in watts
    work_capacity DECIMAL(10,2) NOT NULL, -- W' in kJ
    model_r_squared DECIMAL(5,4) NOT NULL, -- Goodness of fit (0-1)
    test_duration_range VARCHAR(50) NOT NULL, -- e.g., "3-20 minutes"
    calculated_date TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    created_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE DEFAULT NOW(),

    -- Only keep the latest model per user
    UNIQUE(user_id)
);

-- Create indexes for efficient queries
CREATE INDEX idx_pmc_user_date ON performance_management_chart(user_id, date DESC);
CREATE INDEX idx_zone_settings_user ON zone_settings(user_id);
CREATE INDEX idx_power_curve_user_date ON power_curve(user_id, date DESC);
CREATE INDEX idx_critical_power_user ON critical_power_model(user_id);

-- Create updated_at trigger function if it doesn't exist
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';

-- Add triggers for updated_at columns
CREATE TRIGGER update_pmc_updated_at
    BEFORE UPDATE ON performance_management_chart
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_zone_settings_updated_at
    BEFORE UPDATE ON zone_settings
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_critical_power_updated_at
    BEFORE UPDATE ON critical_power_model
    FOR EACH ROW
    EXECUTE FUNCTION update_updated_at_column();