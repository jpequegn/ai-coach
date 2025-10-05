-- Migration: Seed Nutrition & Hydration Recommendations
-- Description: Initial 15+ nutrition and hydration recommendations for recovery optimization
-- Issue: #129 - Phase 8.1: Recommendation Library Foundation

-- Post-Workout Nutrition (4 recommendations)
INSERT INTO recommendation_templates (
    category, title, description, action, priority_default, difficulty,
    expected_impact, time_required_minutes, trigger_conditions, user_constraints, metadata
) VALUES
(
    'nutrition',
    'Post-Workout Carb+Protein Window',
    'Consume 3:1 or 4:1 carb-to-protein ratio within 30-60 minutes post-training',
    'Eat meal or snack with 30-40g carbs + 10-20g protein within 30-60 min of finishing training. Examples: chocolate milk, banana + protein shake, rice bowl with chicken.',
    'high',
    'easy',
    'Accelerated glycogen replenishment, reduced muscle breakdown, enhanced recovery within 24 hours',
    30,
    '{"post_hard_training": true, "glycogen_depleted": true, "training_duration_above_minutes": 60}',
    '{"timing": "post_workout", "dietary_restrictions": "flexible", "equipment_needed": ["food_prep"]}',
    '{"evidence_level": "strong", "carb_grams": [30, 40], "protein_grams": [10, 20], "timing_window_minutes": 60}'
),
(
    'nutrition',
    'High-Intensity Recovery Meal',
    'After high-intensity or long sessions (>90min), consume larger recovery meal',
    'Within 2 hours: 60-80g carbs + 20-30g protein. Examples: pasta with lean meat, rice bowl, sandwich + fruit + yogurt.',
    'high',
    'moderate',
    'Complete glycogen restoration, optimal muscle repair, reduced next-day fatigue',
    45,
    '{"high_intensity_session": true, "training_duration_above_minutes": 90, "significant_glycogen_depletion": true}',
    '{"timing": "post_workout", "meal_prep_required": true}',
    '{"evidence_level": "strong", "carb_grams": [60, 80], "protein_grams": [20, 30], "timing_window_hours": 2}'
),
(
    'nutrition',
    'Evening Training Recovery Nutrition',
    'Don''t skip post-workout nutrition even for late evening sessions',
    'After evening training, have easily digestible recovery snack: protein shake + banana, Greek yogurt + honey, or small sandwich.',
    'medium',
    'easy',
    'Better overnight recovery, reduced muscle soreness, improved morning readiness',
    15,
    '{"evening_training": true, "training_end_after": "20:00", "low_appetite": true}',
    '{"timing": "evening", "easy_digestion": "required"}',
    '{"evidence_level": "strong", "light_meal_focus": true, "timing_before_bed_hours": 1.5}'
),
(
    'nutrition',
    'Fasted Training Recovery',
    'If training fasted, prioritize immediate post-workout nutrition',
    'Break fast within 15-30 min with balanced meal: oatmeal + protein + fruit, or eggs + toast + avocado.',
    'critical',
    'easy',
    'Prevention of muscle catabolism, rapid recovery initiation, restored energy levels',
    30,
    '{"fasted_training": true, "morning_training": true, "fasting_duration_hours": 12}',
    '{"timing": "immediately_post_workout", "meal_prep_required": false}',
    '{"evidence_level": "strong", "fasted_training_priority": "critical", "timing_window_minutes": 30}'
);

-- Hydration Strategies (4 recommendations)
INSERT INTO recommendation_templates (
    category, title, description, action, priority_default, difficulty,
    expected_impact, time_required_minutes, trigger_conditions, user_constraints, metadata
) VALUES
(
    'nutrition',
    'Baseline Daily Hydration',
    'Maintain baseline hydration: 0.5-0.7 oz per pound bodyweight daily',
    'Calculate target: bodyweight (lbs) × 0.6 = oz per day. Track intake using water bottle or app. Spread evenly throughout day.',
    'medium',
    'easy',
    'Maintained performance, better recovery markers, optimal cellular function',
    5,
    '{"dehydration_signs": true, "dark_urine": true, "low_energy": true}',
    '{"equipment_needed": ["water_bottle", "tracking_app"], "lifestyle": "flexible"}',
    '{"evidence_level": "strong", "calculation": "bodyweight_lbs * 0.6 = oz_per_day"}'
),
(
    'nutrition',
    'Pre-Training Hydration Boost',
    'Drink 16-20 oz water 2-3 hours before training',
    'Consume 16-20 oz water 2-3 hours pre-training, then another 8-10 oz 15-20 min before start. Monitor urine color (pale yellow ideal).',
    'high',
    'easy',
    'Optimized performance, delayed fatigue, better thermoregulation during training',
    5,
    '{"upcoming_training": true, "hot_conditions": true, "training_duration_above_minutes": 60}',
    '{"timing": "pre_training", "equipment_needed": ["water_bottle"]}',
    '{"evidence_level": "strong", "primary_intake_oz": [16, 20], "secondary_intake_oz": [8, 10]}'
),
(
    'nutrition',
    'During-Training Hydration',
    'Consume 6-12 oz every 15-20 minutes during exercise',
    'Set timer for 15-20 min intervals. Drink 6-12 oz per interval. Add electrolytes if sweating heavily or training >60 min.',
    'high',
    'moderate',
    'Maintained performance, prevention of dehydration, optimal cardiovascular function',
    1,
    '{"training_duration_above_minutes": 45, "hot_conditions": true, "high_sweat_rate": true}',
    '{"timing": "during_training", "equipment_needed": ["water_bottle", "timer"]}',
    '{"evidence_level": "strong", "intake_oz_per_interval": [6, 12], "interval_minutes": [15, 20]}'
),
(
    'nutrition',
    'Post-Training Rehydration',
    'Drink 16-24 oz per pound lost during training session',
    'Weigh before and after training. For each pound lost, drink 16-24 oz fluid over next 2-4 hours. Include sodium if loss >2 lbs.',
    'high',
    'moderate',
    'Complete fluid restoration, improved recovery, restored blood volume and cardiovascular function',
    10,
    '{"post_training": true, "weight_loss_during_training": true, "training_duration_above_minutes": 60}',
    '{"timing": "post_training", "equipment_needed": ["scale", "water_bottle"], "rehydration_hours": 4}',
    '{"evidence_level": "strong", "oz_per_lb_lost": [16, 24], "electrolyte_threshold_lbs": 2}'
);

-- Anti-Inflammatory Nutrition (4 recommendations)
INSERT INTO recommendation_templates (
    category, title, description, action, priority_default, difficulty,
    expected_impact, time_required_minutes, trigger_conditions, user_constraints, metadata
) VALUES
(
    'nutrition',
    'Tart Cherry Juice for Recovery',
    'Consume 8-12 oz tart cherry juice post-training for anti-inflammatory benefits',
    'Drink 8-12 oz tart cherry juice (not concentrate) within 1 hour post-training and again before bed. Use for 3-5 days during hard training blocks.',
    'medium',
    'easy',
    'Reduced muscle soreness, accelerated recovery, improved sleep quality',
    5,
    '{"high_muscle_soreness": true, "hard_training_block": true, "poor_recovery": true}',
    '{"dietary_restrictions": "none", "timing": "post_training_and_evening"}',
    '{"evidence_level": "moderate", "dosage_oz": [8, 12], "frequency_per_day": 2}'
),
(
    'nutrition',
    'Omega-3 Supplementation',
    'Take 2-3g EPA+DHA daily to reduce exercise-induced inflammation',
    'Consume high-quality fish oil (2-3g EPA+DHA) with meals. Split dose: 1-1.5g at 2 different meals for better absorption.',
    'medium',
    'easy',
    'Reduced inflammation, enhanced recovery, improved cardiovascular health over 2-4 weeks',
    2,
    '{"chronic_inflammation": true, "slow_recovery": true, "high_training_load": true}',
    '{"dietary_restrictions": "no_fish_allergy", "equipment_needed": ["supplements"]}',
    '{"evidence_level": "strong", "epa_dha_grams": [2, 3], "split_dosing": true}'
),
(
    'nutrition',
    'Increase Colorful Vegetables',
    'Consume 5+ servings of colorful vegetables daily for antioxidant support',
    'Add vegetable serving to each meal: berries at breakfast, salad at lunch, roasted vegetables at dinner. Focus on variety and color.',
    'medium',
    'moderate',
    'Enhanced antioxidant status, reduced oxidative stress, better long-term recovery',
    20,
    '{"high_oxidative_stress": true, "inadequate_vegetable_intake": true, "hard_training_block": true}',
    '{"meal_prep_required": true, "grocery_shopping": true}',
    '{"evidence_level": "strong", "servings_per_day": 5, "variety_emphasis": true}'
),
(
    'nutrition',
    'Turmeric Anti-Inflammatory Protocol',
    'Add 1 tsp turmeric + black pepper to meals or take curcumin supplement',
    'Mix 1 tsp turmeric with black pepper into smoothies, soups, or eggs. Or take 500mg curcumin supplement with piperine 2x daily.',
    'low',
    'easy',
    'Reduced inflammation markers, decreased muscle soreness, enhanced recovery',
    5,
    '{"chronic_inflammation": true, "persistent_soreness": true, "recovery_issues": true}',
    '{"dietary_restrictions": "none", "equipment_needed": ["spices_or_supplements"]}',
    '{"evidence_level": "moderate", "turmeric_tsp": 1, "black_pepper_required": true, "curcumin_mg": 500}'
);

-- Timing & Frequency Strategies (3 recommendations)
INSERT INTO recommendation_templates (
    category, title, description, action, priority_default, difficulty,
    expected_impact, time_required_minutes, trigger_conditions, user_constraints, metadata
) VALUES
(
    'nutrition',
    'Protein Distribution Throughout Day',
    'Consume 20-40g protein every 3-4 hours across 4-5 meals',
    'Plan meals with 20-40g protein each: breakfast eggs/Greek yogurt, lunch chicken/fish, dinner lean meat, snacks with protein.',
    'medium',
    'moderate',
    'Optimized muscle protein synthesis, better recovery, maintained lean mass',
    45,
    '{"inadequate_protein_distribution": true, "recovery_issues": true, "muscle_loss": true}',
    '{"meal_prep_required": true, "planning_needed": true}',
    '{"evidence_level": "strong", "protein_grams_per_meal": [20, 40], "meal_frequency_hours": [3, 4]}'
),
(
    'nutrition',
    'Pre-Sleep Protein for Overnight Recovery',
    'Consume 20-40g slow-digesting protein 30-60 min before bed',
    'Eat casein protein shake, Greek yogurt, or cottage cheese 30-60 min before bed. Supports overnight muscle repair.',
    'medium',
    'easy',
    'Enhanced overnight recovery, reduced muscle breakdown, improved morning readiness',
    10,
    '{"hard_training_day": true, "muscle_soreness": true, "poor_overnight_recovery": true}',
    '{"timing": "evening", "dietary_restrictions": "flexible"}',
    '{"evidence_level": "strong", "protein_grams": [20, 40], "casein_preferred": true}'
),
(
    'nutrition',
    'Strategic Carb Timing Around Training',
    'Front-load carbs around training: 30% before, 40% after, 30% throughout day',
    'Adjust carb distribution: pre-training snack (30%), post-training meal (40%), remaining meals (30%). Maintain total daily carbs.',
    'medium',
    'challenging',
    'Optimized glycogen availability, enhanced performance, better recovery efficiency',
    30,
    '{"poor_training_energy": true, "slow_glycogen_recovery": true, "performance_plateau": true}',
    '{"meal_planning_required": true, "macro_tracking": "recommended"}',
    '{"evidence_level": "moderate", "carb_distribution": {"pre": 30, "post": 40, "other": 30}}'
);

-- Comments for documentation
COMMENT ON TABLE recommendation_templates IS 'Nutrition and hydration recommendations seeded with 15 evidence-based interventions';
