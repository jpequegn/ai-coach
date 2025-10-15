-- Create training plan generation and adaptation tables

-- Plan type enum
CREATE TYPE plan_type AS ENUM (
    'goal_based',
    'event_based',
    'progressive',
    'maintenance',
    'recovery',
    'custom'
);

-- Intensity preference enum
CREATE TYPE intensity_preference AS ENUM (
    'low_intensity_high_volume',
    'moderate_intensity_moderate_volume',
    'high_intensity_low_volume',
    'polarized',
    'pyramidal'
);

-- Equipment enum
CREATE TYPE equipment AS ENUM (
    'road',
    'mountain',
    'gravel',
    'tt',
    'trainer',
    'power_meter',
    'heart_rate_monitor',
    'cadence',
    'smart_trainer',
    'rollers',
    'gym',
    'pool',
    'track'
);

-- Training location enum
CREATE TYPE training_location AS ENUM (
    'outdoor',
    'indoor',
    'mixed',
    'gym',
    'home'
);

-- Experience level enum
CREATE TYPE experience_level AS ENUM (
    'beginner',
    'intermediate',
    'advanced',
    'expert'
);

-- Recovery level enum
CREATE TYPE recovery_level AS ENUM (
    'fast',
    'normal',
    'slow',
    'variable'
);

-- Adaptation type enum
CREATE TYPE adaptation_type AS ENUM (
    'volume_increase',
    'volume_decrease',
    'intensity_increase',
    'intensity_decrease',
    'frequency_change',
    'recovery_increase',
    'goal_adjustment',
    'event_rescheduling',
    'injury_accommodation',
    'progress_acceleration',
    'progress_deceleration'
);

-- Workout type enum
CREATE TYPE workout_type AS ENUM (
    'recovery',
    'endurance',
    'tempo',
    'sweet_spot',
    'threshold',
    'vo2_max',
    'neuromuscular',
    'strength',
    'cross_train',
    'test',
    'race'
);

-- Intensity zone enum
CREATE TYPE intensity_zone AS ENUM (
    'zone1',
    'zone2',
    'zone3',
    'zone4',
    'zone5',
    'zone6',
    'mixed'
);

-- Insight type enum
CREATE TYPE insight_type AS ENUM (
    'progress_optimization',
    'recovery_recommendation',
    'volume_adjustment',
    'intensity_adjustment',
    'goal_alignment',
    'risk_mitigation',
    'opportunity_highlight'
);

-- Importance level enum
CREATE TYPE importance_level AS ENUM (
    'low',
    'medium',
    'high',
    'critical'
);

-- Generated plans table
CREATE TABLE generated_plans (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    goal_id UUID REFERENCES goals(id) ON DELETE SET NULL,
    event_id UUID REFERENCES events(id) ON DELETE SET NULL,
    plan_name VARCHAR(255) NOT NULL,
    plan_type plan_type NOT NULL,
    start_date DATE NOT NULL,
    end_date DATE NOT NULL,
    total_weeks INTEGER NOT NULL,
    plan_structure JSONB NOT NULL, -- JSON structure containing phases, weeks, workouts
    generation_parameters JSONB NOT NULL, -- Parameters used for generation
    adaptation_history JSONB NOT NULL DEFAULT '[]'::jsonb, -- Track plan adjustments
    status VARCHAR(50) NOT NULL DEFAULT 'draft',
    confidence_score DECIMAL(5,2), -- AI confidence in plan effectiveness
    success_prediction DECIMAL(5,2), -- Predicted success probability
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- User training preferences table
CREATE TABLE user_training_preferences (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    available_days_per_week INTEGER NOT NULL DEFAULT 5,
    preferred_workout_duration INTEGER NOT NULL DEFAULT 60, -- minutes
    max_workout_duration INTEGER NOT NULL DEFAULT 120, -- minutes
    intensity_preference intensity_preference NOT NULL DEFAULT 'moderate_intensity_moderate_volume',
    preferred_training_times JSONB NOT NULL DEFAULT '[]'::jsonb,
    equipment_available JSONB NOT NULL DEFAULT '[]'::jsonb,
    training_location training_location NOT NULL DEFAULT 'mixed',
    experience_level experience_level NOT NULL DEFAULT 'intermediate',
    injury_history JSONB NOT NULL DEFAULT '[]'::jsonb,
    recovery_needs recovery_level NOT NULL DEFAULT 'normal',
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    UNIQUE(user_id)
);

-- Training constraints table
CREATE TABLE training_constraints (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    max_weekly_hours DECIMAL(5,2) NOT NULL DEFAULT 10.0,
    min_weekly_hours DECIMAL(5,2) NOT NULL DEFAULT 3.0,
    max_consecutive_hard_days INTEGER NOT NULL DEFAULT 2,
    required_rest_days INTEGER NOT NULL DEFAULT 1,
    travel_dates JSONB NOT NULL DEFAULT '[]'::jsonb,
    blackout_dates JSONB NOT NULL DEFAULT '[]'::jsonb,
    priority_dates JSONB NOT NULL DEFAULT '[]'::jsonb,
    equipment_limitations JSONB NOT NULL DEFAULT '[]'::jsonb,
    health_considerations JSONB NOT NULL DEFAULT '[]'::jsonb,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    UNIQUE(user_id)
);

-- Plan adaptations table
CREATE TABLE plan_adaptations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    plan_id UUID NOT NULL REFERENCES generated_plans(id) ON DELETE CASCADE,
    adaptation_type adaptation_type NOT NULL,
    trigger_reason TEXT NOT NULL,
    changes_made JSONB NOT NULL,
    effectiveness_score DECIMAL(5,2),
    applied_date TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Plan alternatives table
CREATE TABLE plan_alternatives (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    original_plan_id UUID NOT NULL REFERENCES generated_plans(id) ON DELETE CASCADE,
    alternative_name VARCHAR(255) NOT NULL,
    alternative_description TEXT NOT NULL,
    differences JSONB NOT NULL,
    estimated_effectiveness DECIMAL(5,2) NOT NULL,
    suitability_score DECIMAL(5,2) NOT NULL,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Coaching insights table
CREATE TABLE coaching_insights (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    plan_id UUID NOT NULL REFERENCES generated_plans(id) ON DELETE CASCADE,
    insight_type insight_type NOT NULL,
    title VARCHAR(255) NOT NULL,
    description TEXT NOT NULL,
    recommended_action TEXT,
    importance importance_level NOT NULL,
    acknowledged_at TIMESTAMP WITH TIME ZONE,
    generated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Indexes for performance
CREATE INDEX idx_generated_plans_user_id ON generated_plans(user_id);
CREATE INDEX idx_generated_plans_goal_id ON generated_plans(goal_id);
CREATE INDEX idx_generated_plans_event_id ON generated_plans(event_id);
CREATE INDEX idx_generated_plans_start_date ON generated_plans(start_date);
CREATE INDEX idx_generated_plans_end_date ON generated_plans(end_date);
CREATE INDEX idx_generated_plans_status ON generated_plans(status);

CREATE INDEX idx_plan_adaptations_plan_id ON plan_adaptations(plan_id);
CREATE INDEX idx_plan_adaptations_applied_date ON plan_adaptations(applied_date);

CREATE INDEX idx_plan_alternatives_original_plan_id ON plan_alternatives(original_plan_id);

CREATE INDEX idx_coaching_insights_plan_id ON coaching_insights(plan_id);
CREATE INDEX idx_coaching_insights_importance ON coaching_insights(importance);
CREATE INDEX idx_coaching_insights_generated_at ON coaching_insights(generated_at);

-- Triggers for updated_at
CREATE TRIGGER update_generated_plans_updated_at BEFORE UPDATE ON generated_plans
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_user_training_preferences_updated_at BEFORE UPDATE ON user_training_preferences
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_training_constraints_updated_at BEFORE UPDATE ON training_constraints
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();