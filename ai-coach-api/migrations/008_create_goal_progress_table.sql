-- Create goal_progress table for SQLite
CREATE TABLE goal_progress (
    id TEXT PRIMARY KEY NOT NULL DEFAULT (lower(hex(randomblob(16)))),
    goal_id TEXT NOT NULL,
    value REAL NOT NULL,
    date TEXT NOT NULL,  -- ISO 8601 date format: YYYY-MM-DD
    note TEXT,
    milestone_achieved TEXT,
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    FOREIGN KEY (goal_id) REFERENCES goals(id) ON DELETE CASCADE
);

-- Create indexes for progress queries
CREATE INDEX idx_goal_progress_goal_id ON goal_progress(goal_id);
CREATE INDEX idx_goal_progress_date ON goal_progress(date);
