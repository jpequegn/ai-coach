-- Migration: Seed Recovery Protocols
-- Description: Pre-built recovery protocol templates
-- Issue: #178 - Phase 8.6.1: Recovery Protocols

-- Protocol 1: Post-Hard-Workout Recovery (24h, sequential)
INSERT INTO recovery_protocols (
    id, name, description, duration_days, sequence_type, category, target_scenarios,
    effectiveness_score, is_active
) VALUES (
    '00000000-0000-0000-0000-000000000001',
    'Post-Hard-Workout Recovery',
    'Comprehensive 24-hour recovery protocol optimized for hard training sessions. Includes immediate nutrition, active recovery, and sleep optimization to maximize adaptation and minimize fatigue accumulation.',
    1,
    'sequential',
    'training_modification',
    '["post_hard_workout", "high_training_load", "competition_recovery"]',
    0.75,
    1
);

-- Protocol 2: Sleep Optimization Week (7-day, phased)
INSERT INTO recovery_protocols (
    id, name, description, duration_days, sequence_type, category, target_scenarios,
    effectiveness_score, is_active
) VALUES (
    '00000000-0000-0000-0000-000000000002',
    'Sleep Optimization Week',
    '7-day progressive protocol to establish healthy sleep patterns and optimize sleep quality. Starts with basics and builds to advanced techniques for sustained improvement.',
    7,
    'phased',
    'sleep',
    '["poor_sleep_week", "sleep_debt", "inconsistent_sleep", "new_to_sleep_optimization"]',
    0.80,
    1
);

-- Protocol 3: Stress Reset (3-day, phased)
INSERT INTO recovery_protocols (
    id, name, description, duration_days, sequence_type, category, target_scenarios,
    effectiveness_score, is_active
) VALUES (
    '00000000-0000-0000-0000-000000000003',
    'Stress Reset',
    'Intensive 3-day protocol to reduce accumulated stress and restore mental resilience. Combines breathing work, nature connection, and integration practices.',
    3,
    'phased',
    'stress_management',
    '["high_stress", "burnout_prevention", "mental_fatigue", "competition_anxiety"]',
    0.70,
    1
);

-- Protocol 4: Travel Recovery (3-5 day, phased)
INSERT INTO recovery_protocols (
    id, name, description, duration_days, sequence_type, category, target_scenarios,
    effectiveness_score, is_active
) VALUES (
    '00000000-0000-0000-0000-000000000004',
    'Travel Recovery',
    'Multi-phase protocol for before, during, and after travel. Minimizes jet lag, maintains training adaptations, and accelerates recovery from travel stress.',
    5,
    'phased',
    'general',
    '["traveling", "jet_lag", "time_zone_change", "competition_travel"]',
    0.65,
    1
);

-- Protocol 5: Injury Prevention (ongoing, parallel)
INSERT INTO recovery_protocols (
    id, name, description, duration_days, sequence_type, category, target_scenarios,
    effectiveness_score, is_active
) VALUES (
    '00000000-0000-0000-0000-000000000005',
    'Injury Prevention Protocol',
    'Ongoing parallel protocol combining daily mobility work, regular strengthening, and recovery practices to prevent common training injuries and maintain long-term athletic health.',
    30,
    'parallel',
    'active_recovery',
    '["injury_prone", "high_training_volume", "return_from_injury", "injury_prevention"]',
    0.72,
    1
);

-- Note: Protocol recommendations linking to specific recommendation templates
-- will be added in a separate migration once recommendation_templates table is migrated to SQLite
-- For now, the protocol framework is in place and ready to be populated with specific recommendations
