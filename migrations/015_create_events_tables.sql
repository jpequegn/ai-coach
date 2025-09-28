-- Create events and event planning system tables

-- Event type enum
CREATE TYPE event_type AS ENUM (
    'race',
    'competition',
    'training',
    'group_ride',
    'clinic',
    'workshop',
    'social',
    'volunteer',
    'personal'
);

-- Sport enum
CREATE TYPE sport AS ENUM (
    'cycling',
    'running',
    'swimming',
    'triathlon',
    'duathlon',
    'cross_training',
    'strength',
    'yoga',
    'other'
);

-- Event status enum
CREATE TYPE event_status AS ENUM (
    'planned',
    'registered',
    'confirmed',
    'in_progress',
    'completed',
    'cancelled',
    'missed'
);

-- Event priority enum
CREATE TYPE event_priority AS ENUM (
    'low',
    'medium',
    'high',
    'critical'
);

-- Phase type enum
CREATE TYPE phase_type AS ENUM (
    'base',
    'build',
    'peak',
    'taper',
    'recovery',
    'transition'
);

-- Conflict type enum
CREATE TYPE conflict_type AS ENUM (
    'date_overlap',
    'too_close',
    'training_conflict',
    'recovery_needed',
    'travel_conflict'
);

-- Conflict severity enum
CREATE TYPE conflict_severity AS ENUM (
    'low',
    'medium',
    'high',
    'critical'
);

-- Event recommendation type enum
CREATE TYPE event_recommendation_type AS ENUM (
    'register_soon',
    'adjust_training',
    'book_travel',
    'check_equipment',
    'nutrition_plan',
    'taper_start',
    'recovery_plan',
    'conflict_resolution'
);

-- Events table
CREATE TABLE events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    event_type event_type NOT NULL,
    sport sport NOT NULL,
    event_date DATE NOT NULL,
    event_time TIME,
    location VARCHAR(255),
    distance DECIMAL(10,2),
    distance_unit VARCHAR(20),
    elevation_gain DECIMAL(10,2),
    expected_duration INTEGER, -- minutes
    registration_deadline DATE,
    cost DECIMAL(10,2),
    website_url VARCHAR(500),
    notes TEXT,
    status event_status NOT NULL DEFAULT 'planned',
    priority event_priority NOT NULL DEFAULT 'medium',
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Event plans table for periodization planning
CREATE TABLE event_plans (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    event_id UUID NOT NULL REFERENCES events(id) ON DELETE CASCADE,
    user_id UUID NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    training_phases JSONB NOT NULL, -- JSON structure for periodization
    peak_date DATE NOT NULL,
    taper_start_date DATE NOT NULL,
    base_training_weeks INTEGER NOT NULL,
    build_training_weeks INTEGER NOT NULL,
    peak_training_weeks INTEGER NOT NULL,
    taper_weeks INTEGER NOT NULL,
    recovery_weeks INTEGER NOT NULL,
    travel_considerations TEXT,
    logistics_notes TEXT,
    equipment_checklist JSONB,
    nutrition_plan TEXT,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Event conflicts table
CREATE TABLE event_conflicts (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    event1_id UUID NOT NULL REFERENCES events(id) ON DELETE CASCADE,
    event2_id UUID NOT NULL REFERENCES events(id) ON DELETE CASCADE,
    conflict_type conflict_type NOT NULL,
    severity conflict_severity NOT NULL,
    description TEXT NOT NULL,
    suggested_resolution TEXT NOT NULL,
    resolved_at TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Event recommendations table
CREATE TABLE event_recommendations (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    event_id UUID NOT NULL REFERENCES events(id) ON DELETE CASCADE,
    recommendation_type event_recommendation_type NOT NULL,
    title VARCHAR(255) NOT NULL,
    description TEXT NOT NULL,
    priority event_priority NOT NULL,
    action_required BOOLEAN NOT NULL DEFAULT FALSE,
    deadline DATE,
    acknowledged_at TIMESTAMP WITH TIME ZONE,
    created_at TIMESTAMP WITH TIME ZONE NOT NULL DEFAULT NOW()
);

-- Add foreign key constraint for goals.event_id now that events table exists
ALTER TABLE goals ADD CONSTRAINT fk_goals_event_id
    FOREIGN KEY (event_id) REFERENCES events(id) ON DELETE SET NULL;

-- Indexes for performance
CREATE INDEX idx_events_user_id ON events(user_id);
CREATE INDEX idx_events_event_date ON events(event_date);
CREATE INDEX idx_events_status ON events(status);
CREATE INDEX idx_events_priority ON events(priority);
CREATE INDEX idx_events_sport ON events(sport);
CREATE INDEX idx_events_event_type ON events(event_type);

CREATE INDEX idx_event_plans_event_id ON event_plans(event_id);
CREATE INDEX idx_event_plans_user_id ON event_plans(user_id);
CREATE INDEX idx_event_plans_peak_date ON event_plans(peak_date);

CREATE INDEX idx_event_conflicts_event1_id ON event_conflicts(event1_id);
CREATE INDEX idx_event_conflicts_event2_id ON event_conflicts(event2_id);
CREATE INDEX idx_event_conflicts_severity ON event_conflicts(severity);

CREATE INDEX idx_event_recommendations_event_id ON event_recommendations(event_id);
CREATE INDEX idx_event_recommendations_deadline ON event_recommendations(deadline);

-- Triggers for updated_at
CREATE TRIGGER update_events_updated_at BEFORE UPDATE ON events
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();

CREATE TRIGGER update_event_plans_updated_at BEFORE UPDATE ON event_plans
    FOR EACH ROW EXECUTE FUNCTION update_updated_at_column();