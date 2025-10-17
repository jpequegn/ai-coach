-- Migration: Create Recovery Protocol Tables
-- Description: Multi-recommendation sequences for comprehensive recovery strategies
-- Issue: #178 - Phase 8.6.1: Recovery Protocols

-- Recovery Protocols Table
-- Stores recovery protocol templates with effectiveness tracking
CREATE TABLE IF NOT EXISTS recovery_protocols (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    description TEXT NOT NULL,
    duration_days INTEGER NOT NULL,
    sequence_type TEXT NOT NULL CHECK (sequence_type IN ('parallel', 'sequential', 'phased')),
    category TEXT NOT NULL CHECK (category IN (
        'sleep',
        'nutrition',
        'active_recovery',
        'stress_management',
        'training_modification',
        'general'
    )),
    target_scenarios TEXT NOT NULL, -- JSON array: ["post_hard_workout", "poor_sleep_week"]
    effectiveness_score REAL NOT NULL DEFAULT 0.5 CHECK (effectiveness_score >= 0.0 AND effectiveness_score <= 1.0),
    times_activated INTEGER NOT NULL DEFAULT 0,
    times_completed INTEGER NOT NULL DEFAULT 0,
    is_active INTEGER NOT NULL DEFAULT 1 CHECK (is_active IN (0, 1)),
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Protocol Recommendations Table
-- Links recommendation templates to protocols with sequencing information
CREATE TABLE IF NOT EXISTS protocol_recommendations (
    id TEXT PRIMARY KEY,
    protocol_id TEXT NOT NULL REFERENCES recovery_protocols(id) ON DELETE CASCADE,
    recommendation_template_id TEXT NOT NULL, -- Will reference recommendation_templates once migrated
    sequence_order INTEGER NOT NULL,
    day_number INTEGER, -- for phased protocols (NULL for parallel/sequential)
    is_required INTEGER NOT NULL DEFAULT 0 CHECK (is_required IN (0, 1)),
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE(protocol_id, sequence_order)
);

-- User Protocol Activations Table
-- Tracks individual protocol activations and progress for users
CREATE TABLE IF NOT EXISTS user_protocol_activations (
    id TEXT PRIMARY KEY,
    user_id TEXT NOT NULL REFERENCES users(id) ON DELETE CASCADE,
    protocol_id TEXT NOT NULL REFERENCES recovery_protocols(id) ON DELETE CASCADE,
    status TEXT NOT NULL DEFAULT 'active' CHECK (status IN ('active', 'completed', 'abandoned')),
    progress TEXT NOT NULL DEFAULT '{}', -- JSON: {recommendation_id: {completed: bool, completed_at: timestamp}}
    activated_at TEXT NOT NULL DEFAULT (datetime('now')),
    completed_at TEXT,
    abandoned_at TEXT,
    abandonment_reason TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Indexes for efficient querying
CREATE INDEX IF NOT EXISTS idx_recovery_protocols_category ON recovery_protocols(category);
CREATE INDEX IF NOT EXISTS idx_recovery_protocols_active ON recovery_protocols(is_active);
CREATE INDEX IF NOT EXISTS idx_recovery_protocols_effectiveness ON recovery_protocols(effectiveness_score DESC);

CREATE INDEX IF NOT EXISTS idx_protocol_recommendations_protocol_id ON protocol_recommendations(protocol_id);
CREATE INDEX IF NOT EXISTS idx_protocol_recommendations_sequence ON protocol_recommendations(protocol_id, sequence_order);

CREATE INDEX IF NOT EXISTS idx_user_protocol_activations_user_id ON user_protocol_activations(user_id);
CREATE INDEX IF NOT EXISTS idx_user_protocol_activations_status ON user_protocol_activations(status);
CREATE INDEX IF NOT EXISTS idx_user_protocol_activations_user_status ON user_protocol_activations(user_id, status);

-- Trigger to update updated_at timestamp for recovery_protocols
CREATE TRIGGER IF NOT EXISTS update_recovery_protocols_updated_at
AFTER UPDATE ON recovery_protocols
FOR EACH ROW
BEGIN
    UPDATE recovery_protocols SET updated_at = datetime('now') WHERE id = NEW.id;
END;

-- Trigger to update updated_at timestamp for user_protocol_activations
CREATE TRIGGER IF NOT EXISTS update_user_protocol_activations_updated_at
AFTER UPDATE ON user_protocol_activations
FOR EACH ROW
BEGIN
    UPDATE user_protocol_activations SET updated_at = datetime('now') WHERE id = NEW.id;
END;

-- Trigger to set completed_at when status changes to completed
CREATE TRIGGER IF NOT EXISTS set_user_protocol_activation_completed_at
AFTER UPDATE OF status ON user_protocol_activations
FOR EACH ROW
WHEN NEW.status = 'completed' AND OLD.status != 'completed'
BEGIN
    UPDATE user_protocol_activations SET completed_at = datetime('now') WHERE id = NEW.id;
END;

-- Trigger to set abandoned_at when status changes to abandoned
CREATE TRIGGER IF NOT EXISTS set_user_protocol_activation_abandoned_at
AFTER UPDATE OF status ON user_protocol_activations
FOR EACH ROW
WHEN NEW.status = 'abandoned' AND OLD.status != 'abandoned'
BEGIN
    UPDATE user_protocol_activations SET abandoned_at = datetime('now') WHERE id = NEW.id;
END;
