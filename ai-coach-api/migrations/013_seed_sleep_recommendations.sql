-- Migration: Seed Sleep Optimization Recommendations (SQLite)
-- Description: Initial 20 sleep recommendations for recovery optimization
-- Issue: #186 - Migrate Recommendation System to SQLite
-- Original: migrations-postgres/025_seed_sleep_recommendations.sql

-- ============================================================================
-- Sleep Duration Adjustments (5 recommendations)
-- ============================================================================
INSERT INTO recommendation_templates (
    id, category, title, description, action, priority_default, difficulty,
    expected_impact, time_required_minutes, trigger_conditions, user_constraints, metadata
) VALUES
(
    '10000000-0000-0000-0000-000000000001',
    'sleep',
    'Extend Sleep Duration by 30-60 Minutes',
    'Increase nightly sleep by 30-60 minutes to improve recovery from high training load',
    'Go to bed 30-60 minutes earlier than your current bedtime. Set a bedtime alarm to remind you.',
    'high',
    'easy',
    'Improved recovery markers, reduced fatigue, better HRV within 3-5 days',
    30,
    '{"poor_sleep": true, "low_hrv": true, "high_training_load": true, "readiness_score_below": 60}',
    '{"time_of_day": "evening", "equipment_needed": []}',
    '{"evidence_level": "strong", "research_citations": ["walker_2017_sleep_recovery"]}'
),
(
    '10000000-0000-0000-0000-000000000002',
    'sleep',
    'Sleep Extension for Hard Training Blocks',
    'Add 1-2 hours of sleep during intense training periods or competition prep',
    'Schedule 9-10 hours of sleep opportunity during hard training weeks. Adjust your evening routine to support earlier bedtime.',
    'high',
    'moderate',
    'Significantly improved recovery, reduced injury risk, better performance adaptations',
    60,
    '{"high_training_load": true, "consecutive_hard_days": true, "readiness_score_below": 65}',
    '{"schedule_flexibility": "required", "time_of_day": "evening"}',
    '{"evidence_level": "strong", "target_duration_hours": 9}'
),
(
    '10000000-0000-0000-0000-000000000003',
    'sleep',
    'Recovery Sleep After Poor Night',
    'Extend sleep by 1-2 hours following a night of insufficient sleep',
    'If you slept <6 hours, aim for 8-9 hours the following night. Consider an early evening nap if possible.',
    'critical',
    'easy',
    'Restored cognitive function, improved next-day readiness, reduced accumulated sleep debt',
    60,
    '{"sleep_hours_below": 6, "consecutive_poor_sleep": false}',
    '{"schedule_flexibility": "moderate"}',
    '{"evidence_level": "strong", "compensatory_sleep": true}'
),
(
    '10000000-0000-0000-0000-000000000004',
    'sleep',
    'Reduce Sleep When Over-Rested',
    'If consistently over-sleeping (>9h) with poor sleep quality, reduce by 30 minutes',
    'Wake 30 minutes earlier for 3-4 days. Monitor sleep quality and morning readiness.',
    'low',
    'easy',
    'Improved sleep efficiency, better sleep quality, consistent energy levels',
    30,
    '{"sleep_hours_above": 9, "poor_sleep_quality": true, "sleep_efficiency_below": 85}',
    '{"time_of_day": "morning"}',
    '{"evidence_level": "moderate", "trial_duration_days": 4}'
),
(
    '10000000-0000-0000-0000-000000000005',
    'sleep',
    'Optimal 7-9 Hour Sleep Window',
    'Maintain consistent 7-9 hour sleep duration for baseline recovery',
    'Establish a consistent bedtime and wake time that allows for 7.5-8.5 hours of sleep.',
    'medium',
    'easy',
    'Consistent recovery, stable HRV, optimal readiness scores',
    30,
    '{"sleep_variability_high": true, "inconsistent_schedule": true}',
    '{"schedule_flexibility": "moderate"}',
    '{"evidence_level": "strong", "optimal_range_hours": [7, 9]}'
);

-- ============================================================================
-- Sleep Timing Optimization (5 recommendations)
-- ============================================================================
INSERT INTO recommendation_templates (
    id, category, title, description, action, priority_default, difficulty,
    expected_impact, time_required_minutes, trigger_conditions, user_constraints, metadata
) VALUES
(
    '10000000-0000-0000-0000-000000000006',
    'sleep',
    'Consistent Sleep-Wake Schedule',
    'Go to bed and wake up at the same time every day, including weekends',
    'Set consistent bedtime and wake time. Use alarms for both. Vary by no more than 30 minutes on weekends.',
    'high',
    'moderate',
    'Improved sleep quality, better circadian alignment, enhanced recovery within 1-2 weeks',
    14,
    '{"sleep_timing_variability": true, "irregular_schedule": true, "poor_sleep_quality": true}',
    '{"schedule_flexibility": "required", "lifestyle": "structured"}',
    '{"evidence_level": "strong", "consistency_window_minutes": 30}'
),
(
    '10000000-0000-0000-0000-000000000007',
    'sleep',
    'Earlier Bedtime for Morning Training',
    'Shift bedtime 30-60 minutes earlier if you train early morning',
    'If training before 7am, go to bed by 9:30-10pm to ensure adequate sleep before early wake-up.',
    'high',
    'moderate',
    'Better recovery from morning sessions, improved training quality, reduced fatigue',
    45,
    '{"morning_training": true, "wake_time_before": "07:00", "sleep_hours_below": 7.5}',
    '{"time_of_day": "evening", "schedule_flexibility": "required"}',
    '{"evidence_level": "strong", "morning_training_sleep_priority": true}'
),
(
    '10000000-0000-0000-0000-000000000008',
    'sleep',
    'Circadian Alignment Optimization',
    'Align sleep timing with natural circadian rhythm (typically 10pm-6am)',
    'Gradually shift bedtime to 10-11pm over 1-2 weeks. Maximize morning light exposure.',
    'medium',
    'challenging',
    'Enhanced sleep quality, better hormonal balance, improved recovery markers',
    30,
    '{"late_bedtime": true, "bedtime_after": "23:30", "poor_sleep_quality": true}',
    '{"lifestyle": "flexible", "schedule_flexibility": "high", "gradual_change_weeks": 2}',
    '{"evidence_level": "strong", "circadian_optimal_bedtime": "22:00-23:00"}'
),
(
    '10000000-0000-0000-0000-000000000009',
    'sleep',
    'Post-Evening Training Sleep Adjustment',
    'Allow 2-3 hours between evening training and bedtime for optimal sleep',
    'If training ends after 7pm, schedule bedtime 2-3 hours later. Use relaxation techniques to facilitate wind-down.',
    'medium',
    'moderate',
    'Improved sleep onset, better sleep quality, enhanced recovery from evening sessions',
    30,
    '{"evening_training": true, "training_end_after": "19:00", "sleep_onset_issues": true}',
    '{"time_of_day": "evening", "schedule_flexibility": "moderate"}',
    '{"evidence_level": "moderate", "cooldown_period_hours": 2.5}'
),
(
    '10000000-0000-0000-0000-00000000000A',
    'sleep',
    'Weekend Sleep Consistency',
    'Maintain weekday sleep schedule on weekends within 1 hour',
    'Wake within 1 hour of weekday wake time on weekends. Use strategic naps if needed for recovery.',
    'medium',
    'moderate',
    'Reduced social jet lag, more consistent readiness scores, better Monday performance',
    14,
    '{"weekend_sleep_delay": true, "weekend_wake_time_delay_hours": 2}',
    '{"lifestyle": "social", "schedule_flexibility": "moderate"}',
    '{"evidence_level": "moderate", "social_jetlag_prevention": true}'
);

-- ============================================================================
-- Sleep Hygiene (5 recommendations)
-- ============================================================================
INSERT INTO recommendation_templates (
    id, category, title, description, action, priority_default, difficulty,
    expected_impact, time_required_minutes, trigger_conditions, user_constraints, metadata
) VALUES
(
    '10000000-0000-0000-0000-00000000000B',
    'sleep',
    'Optimize Bedroom Environment',
    'Create ideal sleep environment: dark, cool (65-68°F), quiet',
    'Use blackout curtains, set thermostat to 65-68°F, use white noise if needed. Remove electronic devices.',
    'high',
    'easy',
    'Faster sleep onset, fewer nighttime awakenings, improved sleep quality',
    30,
    '{"poor_sleep_quality": true, "sleep_environment_issues": true, "frequent_awakenings": true}',
    '{"location": "home", "equipment_needed": ["blackout_curtains", "thermometer"]}',
    '{"evidence_level": "strong", "optimal_temp_fahrenheit": [65, 68]}'
),
(
    '10000000-0000-0000-0000-00000000000C',
    'sleep',
    'Digital Sunset Routine',
    'Stop all screen use 60-90 minutes before bed',
    'Set phone to "Do Not Disturb" at 9pm. Use blue light filters if screens are necessary. Replace screen time with reading or relaxation.',
    'high',
    'moderate',
    'Improved melatonin production, faster sleep onset, better sleep quality within 3-5 days',
    60,
    '{"poor_sleep_onset": true, "evening_screen_use": true, "sleep_latency_above_minutes": 20}',
    '{"time_of_day": "evening", "equipment_needed": ["blue_light_filter"]}',
    '{"evidence_level": "strong", "screen_cutoff_minutes_before_bed": 90}'
),
(
    '10000000-0000-0000-0000-00000000000D',
    'sleep',
    'Pre-Sleep Wind-Down Routine',
    'Establish 30-60 minute relaxing routine before bed',
    'Create consistent routine: dim lights, light reading, gentle stretching, or meditation. Same order each night.',
    'medium',
    'easy',
    'Conditioned sleep response, faster sleep onset, improved sleep quality',
    45,
    '{"poor_sleep_onset": true, "inconsistent_bedtime_routine": true}',
    '{"time_of_day": "evening", "location": "home"}',
    '{"evidence_level": "strong", "routine_duration_minutes": 45}'
),
(
    '10000000-0000-0000-0000-00000000000E',
    'sleep',
    'Avoid Late Caffeine',
    'No caffeine after 2pm to prevent sleep disruption',
    'Set a 2pm caffeine cutoff. Switch to decaf or herbal tea in afternoon/evening. Monitor sleep quality changes.',
    'high',
    'easy',
    'Reduced sleep latency, fewer awakenings, improved deep sleep within 2-3 days',
    5,
    '{"poor_sleep_onset": true, "afternoon_caffeine_use": true, "sleep_latency_above_minutes": 20}',
    '{"lifestyle": "flexible", "dietary_restrictions": "none"}',
    '{"evidence_level": "strong", "caffeine_cutoff_time": "14:00"}'
),
(
    '10000000-0000-0000-0000-00000000000F',
    'sleep',
    'Manage Evening Fluid Intake',
    'Limit fluids 2 hours before bed to reduce nighttime awakenings',
    'Last significant fluid intake by 8pm. Sip only small amounts if needed. Use bathroom before bed.',
    'low',
    'easy',
    'Fewer nighttime bathroom trips, more continuous sleep, better sleep quality',
    5,
    '{"frequent_nighttime_awakenings": true, "bathroom_trips_above": 2}',
    '{"time_of_day": "evening"}',
    '{"evidence_level": "moderate", "fluid_cutoff_hours_before_bed": 2}'
);

-- ============================================================================
-- Nap Strategies (5 recommendations)
-- ============================================================================
INSERT INTO recommendation_templates (
    id, category, title, description, action, priority_default, difficulty,
    expected_impact, time_required_minutes, trigger_conditions, user_constraints, metadata
) VALUES
(
    '10000000-0000-0000-0000-000000000010',
    'sleep',
    'Power Nap for Acute Fatigue',
    'Take 20-minute nap when acutely fatigued during the day',
    'Find quiet space, set 20-minute timer, nap between 1-3pm. Wake immediately when timer sounds.',
    'medium',
    'easy',
    'Restored alertness, improved afternoon performance, no sleep inertia',
    20,
    '{"acute_fatigue": true, "afternoon_energy_dip": true, "poor_previous_night_sleep": true}',
    '{"time_of_day": "afternoon", "location": "home_or_work", "schedule_flexibility": "moderate"}',
    '{"evidence_level": "strong", "optimal_duration_minutes": 20, "optimal_timing": "13:00-15:00"}'
),
(
    '10000000-0000-0000-0000-000000000011',
    'sleep',
    'Recovery Nap After Hard Training',
    'Take 60-90 minute nap after intense morning or midday training',
    'Nap within 2 hours of completing hard session. Use dark, quiet room. Set alarm for 90 minutes max.',
    'high',
    'moderate',
    'Enhanced recovery, improved glycogen resynthesis, reduced muscle fatigue',
    90,
    '{"hard_training_session": true, "morning_or_midday_training": true, "high_fatigue": true}',
    '{"schedule_flexibility": "required", "location": "home", "time_of_day": "post_training"}',
    '{"evidence_level": "strong", "recovery_nap_optimal_minutes": 90, "timing_post_training_hours": 2}'
),
(
    '10000000-0000-0000-0000-000000000012',
    'sleep',
    'Prophylactic Nap Before Sleep Restriction',
    'Take 60-90 minute afternoon nap if expecting late night or early wake',
    'If you know sleep will be short tonight, take preventive nap 1-3pm. Limit to 90 minutes to avoid grogginess.',
    'medium',
    'moderate',
    'Reduced next-day fatigue, maintained performance despite sleep restriction',
    90,
    '{"upcoming_late_night": true, "upcoming_early_wake": true, "predictable_sleep_restriction": true}',
    '{"schedule_flexibility": "required", "advance_planning": true}',
    '{"evidence_level": "moderate", "prophylactic_nap_duration_minutes": 90}'
),
(
    '10000000-0000-0000-0000-000000000013',
    'sleep',
    'Avoid Late Afternoon Naps',
    'Skip naps after 3pm to protect nighttime sleep quality',
    'If fatigued after 3pm, use other alertness strategies (light, movement, cold water). Save sleep for nighttime.',
    'medium',
    'easy',
    'Better nighttime sleep onset, improved sleep quality, more consistent sleep schedule',
    0,
    '{"late_napping_habit": true, "nap_time_after": "15:00", "poor_sleep_onset": true}',
    '{"lifestyle": "flexible"}',
    '{"evidence_level": "strong", "nap_cutoff_time": "15:00"}'
),
(
    '10000000-0000-0000-0000-000000000014',
    'sleep',
    'Split Sleep for Extreme Training Load',
    'Use biphasic sleep (6h night + 90min nap) during peak training blocks',
    'Core sleep 11pm-5am, plus 90-minute nap 1-3pm. Only use during highest training loads (2-3 week max).',
    'medium',
    'advanced',
    'Maintained performance during extreme load, enhanced recovery, sustainable high-volume training',
    90,
    '{"extreme_training_load": true, "multiple_daily_sessions": true, "elite_athlete": true}',
    '{"schedule_flexibility": "high", "elite_context": true, "time_limited_weeks": 3}',
    '{"evidence_level": "moderate", "biphasic_pattern": "6h_night_90min_nap", "max_duration_weeks": 3}'
);
