-- Migration: Seed Active Recovery Recommendations
-- Description: Initial 10+ active recovery recommendations for recovery optimization
-- Issue: #129 - Phase 8.1: Recommendation Library Foundation

-- Movement-Based Recovery (5 recommendations)
INSERT INTO recommendation_templates (
    category, title, description, action, priority_default, difficulty,
    expected_impact, time_required_minutes, trigger_conditions, user_constraints, metadata
) VALUES
(
    'active_recovery',
    'Easy Aerobic Recovery Session',
    'Perform 20-40 minutes of easy aerobic exercise at conversational pace',
    'Do easy bike, swim, or jog at 60-70% max HR. Should be able to hold conversation. Focus on movement, not intensity.',
    'medium',
    'easy',
    'Increased blood flow, enhanced waste removal, reduced muscle stiffness within 24 hours',
    30,
    '{"muscle_soreness": true, "post_hard_training": true, "stiffness": true}',
    '{"equipment_needed": ["bike_pool_or_shoes"], "location": "flexible", "weather": "any"}',
    '{"evidence_level": "strong", "heart_rate_percent": [60, 70], "duration_minutes": [20, 40]}'
),
(
    'active_recovery',
    'Recovery Walk',
    'Take 30-60 minute easy walk outdoors for active recovery',
    'Walk at comfortable pace outdoors. Focus on relaxed breathing and gentle movement. Can be split into 2 shorter walks.',
    'low',
    'easy',
    'Gentle blood flow enhancement, psychological recovery, reduced perceived fatigue',
    45,
    '{"general_fatigue": true, "low_motivation": true, "poor_sleep": true}',
    '{"location": "outdoor", "equipment_needed": [], "weather": "mild"}',
    '{"evidence_level": "moderate", "duration_minutes": [30, 60], "intensity": "very_low"}'
),
(
    'active_recovery',
    'Swimming Recovery Session',
    'Swim easy 15-30 minutes for low-impact active recovery',
    'Easy swimming with focus on technique and relaxation. Use different strokes. Keep effort conversational.',
    'medium',
    'moderate',
    'Reduced impact stress, enhanced circulation, muscle relaxation, improved mobility',
    30,
    '{"high_impact_training": true, "joint_stress": true, "muscle_soreness": true}',
    '{"equipment_needed": ["pool_access"], "location": "pool", "skill_level": "basic_swimming"}',
    '{"evidence_level": "strong", "duration_minutes": [15, 30], "low_impact": true}'
),
(
    'active_recovery',
    'Yoga Flow for Recovery',
    'Practice 20-45 minutes gentle yoga flow focusing on mobility',
    'Follow recovery-focused yoga sequence: gentle flows, long holds, emphasis on breathing. Use props for support.',
    'medium',
    'moderate',
    'Improved flexibility, reduced muscle tension, enhanced mind-body connection, better sleep',
    35,
    '{"muscle_tightness": true, "stress": true, "poor_mobility": true}',
    '{"equipment_needed": ["yoga_mat"], "location": "home_or_studio", "guidance": "video_or_instructor"}',
    '{"evidence_level": "moderate", "duration_minutes": [20, 45], "focus": "gentle_flow"}'
),
(
    'active_recovery',
    'Dynamic Mobility Circuit',
    'Perform 15-20 minute dynamic mobility routine targeting key areas',
    'Complete circuit: leg swings, arm circles, hip openers, spinal rotations, ankle mobility. 2 rounds of 10-12 exercises.',
    'medium',
    'easy',
    'Enhanced range of motion, reduced stiffness, improved movement quality',
    20,
    '{"poor_mobility": true, "muscle_stiffness": true, "movement_restrictions": true}',
    '{"equipment_needed": ["minimal"], "location": "anywhere", "guidance": "basic"}',
    '{"evidence_level": "strong", "duration_minutes": [15, 20], "circuit_rounds": 2}'
);

-- Recovery Modalities (5 recommendations)
INSERT INTO recommendation_templates (
    category, title, description, action, priority_default, difficulty,
    expected_impact, time_required_minutes, trigger_conditions, user_constraints, metadata
) VALUES
(
    'active_recovery',
    'Foam Rolling Session',
    'Perform 10-15 minute foam rolling routine on major muscle groups',
    'Roll each muscle group 30-60 seconds: quads, hamstrings, calves, glutes, back, lats. Avoid rolling directly on joints or bones.',
    'medium',
    'easy',
    'Reduced muscle tension, improved tissue quality, enhanced recovery perception',
    15,
    '{"muscle_tightness": true, "trigger_points": true, "post_hard_training": true}',
    '{"equipment_needed": ["foam_roller"], "location": "home_or_gym"}',
    '{"evidence_level": "moderate", "duration_seconds_per_muscle": [30, 60], "total_minutes": [10, 15]}'
),
(
    'active_recovery',
    'Contrast Water Therapy',
    'Alternate hot (3 min) and cold (1 min) water exposure for 3-4 cycles',
    'After training: hot shower/bath 3 min, cold shower 1 min. Repeat 3-4 times, end with cold. Total 12-16 minutes.',
    'medium',
    'moderate',
    'Enhanced circulation, reduced inflammation, accelerated metabolite clearance',
    16,
    '{"intense_muscle_soreness": true, "post_hard_training": true, "inflammation": true}',
    '{"equipment_needed": ["shower_or_bath"], "location": "home", "cold_tolerance": "moderate"}',
    '{"evidence_level": "moderate", "hot_minutes": 3, "cold_minutes": 1, "cycles": [3, 4]}'
),
(
    'active_recovery',
    'Self-Massage Routine',
    'Use massage tools for 15-20 minute self-myofascial release',
    'Use lacrosse ball, massage stick, or hands on tight areas. Apply sustained pressure 30-90 seconds per spot. Focus on problem areas.',
    'low',
    'easy',
    'Reduced muscle tension, improved tissue mobility, targeted trigger point release',
    20,
    '{"specific_muscle_tightness": true, "trigger_points": true, "localized_soreness": true}',
    '{"equipment_needed": ["massage_tools"], "location": "anywhere"}',
    '{"evidence_level": "moderate", "pressure_duration_seconds": [30, 90], "total_minutes": [15, 20]}'
),
(
    'active_recovery',
    'Compression Garment Protocol',
    'Wear compression garments 2-4 hours post-training',
    'Put on compression tights/socks immediately after training. Wear 2-4 hours while resting or doing light activities.',
    'low',
    'easy',
    'Enhanced venous return, reduced swelling, improved waste removal',
    0,
    '{"leg_heaviness": true, "swelling": true, "post_long_training": true}',
    '{"equipment_needed": ["compression_garments"], "wear_duration_hours": [2, 4]}',
    '{"evidence_level": "moderate", "timing": "immediately_post_training", "duration_hours": [2, 4]}'
),
(
    'active_recovery',
    'Active Stretching Routine',
    'Perform 15-20 minute active stretching sequence',
    'Dynamic stretches with movement: walking lunges, leg swings, arm swings, torso rotations. Hold each 2-3 seconds, repeat 10-15 times.',
    'medium',
    'easy',
    'Improved flexibility, enhanced blood flow, reduced muscle tension, better movement quality',
    20,
    '{"muscle_tightness": true, "pre_mobility_issues": true, "post_training": true}',
    '{"equipment_needed": [], "location": "anywhere", "timing": "post_training_or_rest_day"}',
    '{"evidence_level": "strong", "hold_seconds": [2, 3], "reps": [10, 15], "duration_minutes": [15, 20]}'
);

-- Comments for documentation
COMMENT ON TABLE recommendation_templates IS 'Active recovery recommendations seeded with 10 evidence-based interventions';
