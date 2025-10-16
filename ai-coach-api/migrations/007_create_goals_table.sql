-- Create goals table for SQLite
CREATE TABLE goals (
    id TEXT PRIMARY KEY NOT NULL DEFAULT (lower(hex(randomblob(16)))),
    user_id TEXT NOT NULL,
    title TEXT NOT NULL,
    description TEXT NOT NULL,
    goal_type TEXT NOT NULL CHECK(goal_type IN (
        'power', 'pace', 'race_time', 'distance', 'heart_rate',
        'consistency', 'weekly_tss', 'weekly_volume', 'recovery_metrics',
        'event_preparation', 'peak_performance', 'taper_execution',
        'weight', 'body_composition', 'strength', 'flexibility',
        'custom'
    )),
    goal_category TEXT NOT NULL CHECK(goal_category IN (
        'performance', 'process', 'event', 'health', 'training', 'competition'
    )),
    target_value REAL,
    current_value REAL,
    unit TEXT,
    target_date TEXT,  -- ISO 8601 date format: YYYY-MM-DD
    status TEXT NOT NULL DEFAULT 'active' CHECK(status IN (
        'draft', 'active', 'on_track', 'at_risk', 'completed', 'failed', 'paused', 'cancelled'
    )),
    priority TEXT NOT NULL DEFAULT 'medium' CHECK(priority IN (
        'low', 'medium', 'high', 'critical'
    )),
    event_id TEXT,
    parent_goal_id TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (user_id) REFERENCES users(id) ON DELETE CASCADE,
    FOREIGN KEY (parent_goal_id) REFERENCES goals(id) ON DELETE CASCADE
);

-- Create indexes for common queries
CREATE INDEX idx_goals_user_id ON goals(user_id);
CREATE INDEX idx_goals_status ON goals(status);
CREATE INDEX idx_goals_target_date ON goals(target_date);
CREATE INDEX idx_goals_priority ON goals(priority);
CREATE INDEX idx_goals_event_id ON goals(event_id);
