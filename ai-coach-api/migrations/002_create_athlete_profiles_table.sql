-- Create athlete_profiles table (SQLite-compatible)
CREATE TABLE athlete_profiles (
    id TEXT PRIMARY KEY NOT NULL,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    sport TEXT NOT NULL,
    ftp INTEGER,
    lthr INTEGER,
    max_heart_rate INTEGER,
    threshold_pace REAL,
    zones TEXT,  -- JSON stored as TEXT
    created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
    updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
);

-- Index for user_id lookups
CREATE INDEX idx_athlete_profiles_user_id ON athlete_profiles(user_id);

-- Index for sport filtering
CREATE INDEX idx_athlete_profiles_sport ON athlete_profiles(sport);
