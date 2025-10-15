-- Create indexes for performance
CREATE INDEX idx_users_email ON users(email);
CREATE INDEX idx_athlete_profiles_user_id ON athlete_profiles(user_id);
CREATE INDEX idx_athlete_profiles_sport ON athlete_profiles(sport);
CREATE INDEX idx_training_sessions_user_id ON training_sessions(user_id);
CREATE INDEX idx_training_sessions_date ON training_sessions(date);
CREATE INDEX idx_training_sessions_user_date ON training_sessions(user_id, date);
CREATE INDEX idx_coaching_recommendations_user_id ON coaching_recommendations(user_id);
CREATE INDEX idx_coaching_recommendations_type ON coaching_recommendations(recommendation_type);
CREATE INDEX idx_coaching_recommendations_created_at ON coaching_recommendations(created_at);
CREATE INDEX idx_training_plans_user_id ON training_plans(user_id);
CREATE INDEX idx_training_plans_status ON training_plans(status);
CREATE INDEX idx_training_plans_dates ON training_plans(start_date, end_date);
CREATE INDEX idx_model_predictions_user_id ON model_predictions(user_id);
CREATE INDEX idx_model_predictions_type ON model_predictions(prediction_type);
CREATE INDEX idx_model_predictions_created_at ON model_predictions(created_at);