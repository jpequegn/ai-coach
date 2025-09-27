use anyhow::{anyhow, Result};
use std::collections::HashSet;

/// Email validation
pub fn validate_email(email: &str) -> Result<()> {
    if email.is_empty() {
        return Err(anyhow!("Email cannot be empty"));
    }

    if !email.contains('@') || !email.contains('.') {
        return Err(anyhow!("Invalid email format"));
    }

    if email.len() > 255 {
        return Err(anyhow!("Email cannot be longer than 255 characters"));
    }

    Ok(())
}

/// Password validation
pub fn validate_password(password: &str) -> Result<()> {
    if password.is_empty() {
        return Err(anyhow!("Password cannot be empty"));
    }

    if password.len() < 8 {
        return Err(anyhow!("Password must be at least 8 characters long"));
    }

    if password.len() > 128 {
        return Err(anyhow!("Password cannot be longer than 128 characters"));
    }

    Ok(())
}

/// Validate sport type
pub fn validate_sport(sport: &str) -> Result<()> {
    let valid_sports: HashSet<&str> = [
        "cycling",
        "running",
        "swimming",
        "triathlon",
        "weightlifting",
        "rowing",
        "climbing",
        "other"
    ].iter().cloned().collect();

    if sport.is_empty() {
        return Err(anyhow!("Sport cannot be empty"));
    }

    if !valid_sports.contains(sport.to_lowercase().as_str()) {
        return Err(anyhow!("Invalid sport type. Must be one of: cycling, running, swimming, triathlon, weightlifting, rowing, climbing, other"));
    }

    Ok(())
}

/// Validate heart rate values
pub fn validate_heart_rate(hr: i32, field_name: &str) -> Result<()> {
    if hr < 30 || hr > 220 {
        return Err(anyhow!("{} must be between 30 and 220 bpm", field_name));
    }
    Ok(())
}

/// Validate FTP (Functional Threshold Power)
pub fn validate_ftp(ftp: i32) -> Result<()> {
    if ftp < 50 || ftp > 600 {
        return Err(anyhow!("FTP must be between 50 and 600 watts"));
    }
    Ok(())
}

/// Validate confidence score
pub fn validate_confidence(confidence: f64) -> Result<()> {
    if !(0.0..=1.0).contains(&confidence) {
        return Err(anyhow!("Confidence must be between 0.0 and 1.0"));
    }
    Ok(())
}

/// Validate recommendation type
pub fn validate_recommendation_type(rec_type: &str) -> Result<()> {
    let valid_types: HashSet<&str> = [
        "training_adjustment",
        "recovery",
        "nutrition",
        "technique",
        "goal_adjustment",
        "equipment",
        "pacing",
        "other"
    ].iter().cloned().collect();

    if rec_type.is_empty() {
        return Err(anyhow!("Recommendation type cannot be empty"));
    }

    if !valid_types.contains(rec_type.to_lowercase().as_str()) {
        return Err(anyhow!("Invalid recommendation type"));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_email_validation() {
        assert!(validate_email("test@example.com").is_ok());
        assert!(validate_email("").is_err());
        assert!(validate_email("invalid").is_err());
        assert!(validate_email("test@").is_err());
    }

    #[test]
    fn test_password_validation() {
        assert!(validate_password("password123").is_ok());
        assert!(validate_password("").is_err());
        assert!(validate_password("short").is_err());
    }

    #[test]
    fn test_sport_validation() {
        assert!(validate_sport("cycling").is_ok());
        assert!(validate_sport("running").is_ok());
        assert!(validate_sport("invalid_sport").is_err());
        assert!(validate_sport("").is_err());
    }

    #[test]
    fn test_heart_rate_validation() {
        assert!(validate_heart_rate(150, "max_hr").is_ok());
        assert!(validate_heart_rate(25, "max_hr").is_err());
        assert!(validate_heart_rate(250, "max_hr").is_err());
    }

    #[test]
    fn test_confidence_validation() {
        assert!(validate_confidence(0.75).is_ok());
        assert!(validate_confidence(-0.1).is_err());
        assert!(validate_confidence(1.5).is_err());
    }
}