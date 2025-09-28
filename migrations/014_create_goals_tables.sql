-- Create enhanced goals system tables

-- Goal type enum
CREATE TYPE goal_type AS ENUM (
    'power',
    'pace',
    'race_time',
    'distance',
    'heart_rate',
    'consistency',
    'weekly_tss',
    'weekly_volume',
    'recovery_metrics',
    'event_preparation',
    'peak_performance',
    'taper_execution',
    'weight',
    'body_composition',
    'strength',
    'flexibility',
    'custom'
);

-- Goal category enum
CREATE TYPE goal_category AS ENUM (
    'performance',
    'process',
    'event',
    'health',
    'training',
    'competition'
);

-- Goal status enum
CREATE TYPE goal_status AS ENUM (
    'draft',
    'active',
    'on_track',
    'at_risk',
    'completed',
    'failed',
    'paused',
    'cancelled'
);

-- Goal priority enum
CREATE TYPE goal_priority AS ENUM (
    'low',
    'medium',
    'high',
    'critical'
);

-- Trend direction enum
CREATE TYPE trend_direction AS ENUM (
    'improving',
    'stable',
    'declining',
    'insufficient'
);

-- Recommendation type enum
CREATE TYPE recommendation_type AS ENUM (
    'adjust_target',
    'extend_deadline',
    'increase_effort',
    'change_strategy',
    'celebration',
    'warning'
);

-- Goals table
CREATE TABLE goals (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    title VARCHAR(255) NOT NULL,
    description TEXT,
    goal_type goal_type NOT NULL,
    goal_category goal_category NOT NULL,
    target_value DECIMAL(10,2),
    current_value DECIMAL(10,2),
    unit VARCHAR(50),
    target_date DATE,
    status goal_status NOT NULL DEFAULT 'draft',
    priority goal_priority NOT NULL DEFAULT 'medium',
    event_id UUID, -- Will reference events table
    parent_goal_id UUID REFERENCES goals(id) ON DELETE SET NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Goal progress tracking table
CREATE TABLE goal_progress (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    goal_id UUID NOT NULL REFERENCES goals(id) ON DELETE CASCADE,
    value DECIMAL(10,2) NOT NULL,
    date DATE NOT NULL DEFAULT CURRENT_DATE,
    note TEXT,
    milestone_achieved VARCHAR(255),
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Goal recommendations table
CREATE TABLE goal_recommendations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    goal_id UUID NOT NULL REFERENCES goals(id) ON DELETE CASCADE,
    recommendation_type recommendation_type NOT NULL,
    title VARCHAR(255) NOT NULL,
    description TEXT NOT NULL,
    priority goal_priority NOT NULL,
    suggested_actions JSONB,
    generated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    acknowledged_at TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Indexes for performance
CREATE INDEX idx_goals_user_id ON goals(user_id);
CREATE INDEX idx_goals_status ON goals(status);
CREATE INDEX idx_goals_target_date ON goals(target_date);
CREATE INDEX idx_goals_priority ON goals(priority);
CREATE INDEX idx_goals_event_id ON goals(event_id);
CREATE INDEX idx_goals_parent_goal_id ON goals(parent_goal_id);

CREATE INDEX idx_goal_progress_goal_id ON goal_progress(goal_id);
CREATE INDEX idx_goal_progress_date ON goal_progress(date);

CREATE INDEX idx_goal_recommendations_goal_id ON goal_recommendations(goal_id);
CREATE INDEX idx_goal_recommendations_generated_at ON goal_recommendations(generated_at);

-- Update timestamp trigger function
CREATE OR REPLACE FUNCTION update_updated_at_column()
RETURNS TRIGGER AS $$
BEGIN
    NEW.updated_at = NOW();
    RETURN NEW;
END;
$$ language 'plpgsql';

-- Triggers for updated_at
CREATE TRIGGER update_goals_updated_at BEFORE UPDATE ON goals
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();