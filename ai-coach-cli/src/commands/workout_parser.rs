use anyhow::{Context, Result};
use regex::Regex;

/// Parsed workout from natural language description
#[derive(Debug, PartialEq)]
pub struct ParsedWorkout {
    pub exercise_type: String,
    pub duration_minutes: Option<u32>,
    pub distance_km: Option<f64>,
}

impl ParsedWorkout {
    pub fn new(exercise_type: String) -> Self {
        Self {
            exercise_type,
            duration_minutes: None,
            distance_km: None,
        }
    }
}

/// Parse natural language workout descriptions
pub struct WorkoutParser {
    // Patterns for different workout types
    running_patterns: Vec<Regex>,
    cycling_patterns: Vec<Regex>,
    swimming_patterns: Vec<Regex>,
    walking_patterns: Vec<Regex>,
    strength_patterns: Vec<Regex>,

    // Unit conversion patterns
    distance_patterns: Vec<Regex>,
    duration_patterns: Vec<Regex>,
}

impl Default for WorkoutParser {
    fn default() -> Self {
        Self::new()
    }
}

impl WorkoutParser {
    pub fn new() -> Self {
        // Running patterns
        let running_patterns = vec![
            Regex::new(r"(?i)\b(ran|running|run|jog|jogging)\b").unwrap(),
        ];

        // Cycling patterns
        let cycling_patterns = vec![
            Regex::new(r"(?i)\b(cycl(e|ed|ing)|bik(e|ed|ing)|rode)\b").unwrap(),
        ];

        // Swimming patterns
        let swimming_patterns = vec![
            Regex::new(r"(?i)\b(swim|swimming|swam)\b").unwrap(),
        ];

        // Walking patterns
        let walking_patterns = vec![
            Regex::new(r"(?i)\b(walk|walking|walked|hike|hiking|hiked)\b").unwrap(),
        ];

        // Strength patterns
        let strength_patterns = vec![
            Regex::new(r"(?i)\b(lift|lifting|lifted|strength|weights?|gym)\b").unwrap(),
        ];

        // Distance patterns (km, miles, meters) - more specific to avoid matching times
        let distance_patterns = vec![
            Regex::new(r"(\d+\.?\d*)\s*(km|kilometers?|kilometres?)\b").unwrap(),
            Regex::new(r"(\d+\.?\d*)\s*(mi|miles?)\b").unwrap(),
            Regex::new(r"(\d+\.?\d*)\s*meters?").unwrap(), // meters but not "minutes"
        ];

        // Duration patterns (minutes, hours)
        let duration_patterns = vec![
            Regex::new(r"(\d+\.?\d*)\s*(min|minutes?)").unwrap(),
            Regex::new(r"(\d+\.?\d*)\s*(h|hr|hrs|hours?)").unwrap(),
            Regex::new(r"in\s+(\d+)\s+min").unwrap(),
            Regex::new(r"for\s+(\d+)\s+min").unwrap(),
        ];

        Self {
            running_patterns,
            cycling_patterns,
            swimming_patterns,
            walking_patterns,
            strength_patterns,
            distance_patterns,
            duration_patterns,
        }
    }

    /// Parse a workout description
    pub fn parse(&self, description: &str) -> Result<ParsedWorkout> {
        let description_lower = description.to_lowercase();

        // Determine exercise type
        let exercise_type = self.detect_exercise_type(&description_lower)?;

        // Extract distance
        let distance_km = self.extract_distance(&description_lower);

        // Extract duration
        let duration_minutes = self.extract_duration(&description_lower);

        Ok(ParsedWorkout {
            exercise_type,
            duration_minutes,
            distance_km,
        })
    }

    fn detect_exercise_type(&self, description: &str) -> Result<String> {
        if self.running_patterns.iter().any(|r| r.is_match(description)) {
            return Ok("running".to_string());
        }

        if self.cycling_patterns.iter().any(|r| r.is_match(description)) {
            return Ok("cycling".to_string());
        }

        if self.swimming_patterns.iter().any(|r| r.is_match(description)) {
            return Ok("swimming".to_string());
        }

        if self.walking_patterns.iter().any(|r| r.is_match(description)) {
            return Ok("walking".to_string());
        }

        if self.strength_patterns.iter().any(|r| r.is_match(description)) {
            return Ok("strength".to_string());
        }

        Err(anyhow::anyhow!("Could not detect exercise type from description"))
    }

    fn extract_distance(&self, description: &str) -> Option<f64> {
        for pattern in &self.distance_patterns {
            if let Some(captures) = pattern.captures(description) {
                if let Some(value_str) = captures.get(1) {
                    if let Ok(value) = value_str.as_str().parse::<f64>() {
                        let unit = captures.get(2).map(|m| m.as_str()).unwrap_or("");

                        // Convert to km
                        return Some(match unit {
                            unit if unit.starts_with("mi") => value * 1.60934,
                            unit if unit.starts_with('m') && !unit.starts_with("mi") => value / 1000.0,
                            _ => value, // Already in km
                        });
                    }
                }
            }
        }
        None
    }

    fn extract_duration(&self, description: &str) -> Option<u32> {
        for pattern in &self.duration_patterns {
            if let Some(captures) = pattern.captures(description) {
                if let Some(value_str) = captures.get(1) {
                    if let Ok(value) = value_str.as_str().parse::<f64>() {
                        // Check if there's a unit (group 2)
                        let unit = captures.get(2).map(|m| m.as_str()).unwrap_or("min");

                        // Convert to minutes
                        let minutes = match unit {
                            unit if unit.starts_with('h') => value * 60.0,
                            _ => value, // Already in minutes
                        };

                        return Some(minutes as u32);
                    }
                }
            }
        }
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_ran_miles_minutes() {
        let parser = WorkoutParser::new();
        let result = parser.parse("Ran 5 miles in 40 minutes").unwrap();

        assert_eq!(result.exercise_type, "running");
        assert_eq!(result.duration_minutes, Some(40));
        assert!(result.distance_km.is_some());
        let dist = result.distance_km.unwrap();
        assert!((dist - 8.0467).abs() < 0.01); // 5 miles ≈ 8.05 km
    }

    #[test]
    fn test_parse_bike_km() {
        let parser = WorkoutParser::new();
        let result = parser.parse("60 min bike ride at 25km").unwrap();

        assert_eq!(result.exercise_type, "cycling");
        assert_eq!(result.duration_minutes, Some(60));
        assert_eq!(result.distance_km, Some(25.0));
    }

    #[test]
    fn test_parse_running_km() {
        let parser = WorkoutParser::new();
        let result = parser.parse("Running 10 kilometers").unwrap();

        assert_eq!(result.exercise_type, "running");
        assert_eq!(result.distance_km, Some(10.0));
    }

    #[test]
    fn test_parse_cycling_duration_only() {
        let parser = WorkoutParser::new();
        let result = parser.parse("Cycled for 45 minutes").unwrap();

        assert_eq!(result.exercise_type, "cycling");
        assert_eq!(result.duration_minutes, Some(45));
        assert_eq!(result.distance_km, None);
    }

    #[test]
    fn test_parse_hours_to_minutes() {
        let parser = WorkoutParser::new();
        let result = parser.parse("Ran for 1.5 hours").unwrap();

        assert_eq!(result.exercise_type, "running");
        assert_eq!(result.duration_minutes, Some(90));
    }

    #[test]
    fn test_parse_walking() {
        let parser = WorkoutParser::new();
        let result = parser.parse("Walked 3 miles").unwrap();

        assert_eq!(result.exercise_type, "walking");
        assert!(result.distance_km.is_some());
    }

    #[test]
    fn test_parse_strength_training() {
        let parser = WorkoutParser::new();
        let result = parser.parse("Strength training for 60 minutes").unwrap();

        assert_eq!(result.exercise_type, "strength");
        assert_eq!(result.duration_minutes, Some(60));
    }

    #[test]
    fn test_parse_unknown_exercise() {
        let parser = WorkoutParser::new();
        let result = parser.parse("Did something for 30 min");

        assert!(result.is_err());
    }
}
