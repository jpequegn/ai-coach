use anyhow::Result;
use bcrypt::{hash, verify, DEFAULT_COST};
use regex::Regex;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum PasswordError {
    #[error("Password must be at least 8 characters long")]
    TooShort,
    #[error("Password must be no more than 128 characters long")]
    TooLong,
    #[error("Password must contain at least one uppercase letter")]
    NoUppercase,
    #[error("Password must contain at least one lowercase letter")]
    NoLowercase,
    #[error("Password must contain at least one number")]
    NoNumber,
    #[error("Password must contain at least one special character")]
    NoSpecialChar,
    #[error("Failed to hash password")]
    HashingFailed,
    #[error("Failed to verify password")]
    VerificationFailed,
}

/// Password strength requirements
#[derive(Debug, Clone)]
pub struct PasswordPolicy {
    pub min_length: usize,
    pub max_length: usize,
    pub require_uppercase: bool,
    pub require_lowercase: bool,
    pub require_number: bool,
    pub require_special_char: bool,
}

impl Default for PasswordPolicy {
    fn default() -> Self {
        Self {
            min_length: 8,
            max_length: 128,
            require_uppercase: true,
            require_lowercase: true,
            require_number: true,
            require_special_char: true,
        }
    }
}

/// Validate password strength according to policy
pub fn validate_password_strength(password: &str, policy: &PasswordPolicy) -> Result<(), PasswordError> {
    // Length checks
    if password.len() < policy.min_length {
        return Err(PasswordError::TooShort);
    }

    if password.len() > policy.max_length {
        return Err(PasswordError::TooLong);
    }

    // Character type checks
    if policy.require_uppercase && !password.chars().any(|c| c.is_uppercase()) {
        return Err(PasswordError::NoUppercase);
    }

    if policy.require_lowercase && !password.chars().any(|c| c.is_lowercase()) {
        return Err(PasswordError::NoLowercase);
    }

    if policy.require_number && !password.chars().any(|c| c.is_numeric()) {
        return Err(PasswordError::NoNumber);
    }

    if policy.require_special_char {
        let special_chars = Regex::new(r"[^a-zA-Z0-9]").unwrap();
        if !special_chars.is_match(password) {
            return Err(PasswordError::NoSpecialChar);
        }
    }

    Ok(())
}

/// Hash a password using bcrypt
pub fn hash_password(password: &str) -> Result<String, PasswordError> {
    // Validate password strength first
    validate_password_strength(password, &PasswordPolicy::default())?;

    hash(password, DEFAULT_COST)
        .map_err(|_| PasswordError::HashingFailed)
}

/// Verify a password against its hash
pub fn verify_password(password: &str, hash: &str) -> Result<bool, PasswordError> {
    verify(password, hash)
        .map_err(|_| PasswordError::VerificationFailed)
}

/// Generate a secure random password reset token
pub fn generate_reset_token() -> String {
    use rand::Rng;
    const CHARSET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ\
                            abcdefghijklmnopqrstuvwxyz\
                            0123456789";
    const TOKEN_LEN: usize = 32;

    let mut rng = rand::thread_rng();

    (0..TOKEN_LEN)
        .map(|_| {
            let idx = rng.gen_range(0..CHARSET.len());
            CHARSET[idx] as char
        })
        .collect()
}

/// Check password strength and return a score (0-100)
pub fn calculate_password_strength(password: &str) -> u8 {
    let mut score: u8 = 0;

    // Length bonus
    match password.len() {
        0..=7 => score += 0,
        8..=11 => score += 25,
        12..=15 => score += 35,
        _ => score += 45,
    }

    // Character variety bonus
    if password.chars().any(|c| c.is_lowercase()) {
        score += 10;
    }
    if password.chars().any(|c| c.is_uppercase()) {
        score += 10;
    }
    if password.chars().any(|c| c.is_numeric()) {
        score += 10;
    }

    let special_chars = Regex::new(r"[^a-zA-Z0-9]").unwrap();
    if special_chars.is_match(password) {
        score += 15;
    }

    // Complexity bonus
    let unique_chars = password.chars().collect::<std::collections::HashSet<_>>().len();
    if unique_chars >= 8 {
        score += 10;
    }

    // Penalize common patterns
    if password.to_lowercase().contains("password") ||
       password.to_lowercase().contains("123456") ||
       password == "qwerty" {
        score = score.saturating_sub(30);
    }

    score.min(100)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_password_validation() {
        let policy = PasswordPolicy::default();

        // Too short
        assert!(matches!(
            validate_password_strength("short", &policy),
            Err(PasswordError::TooShort)
        ));

        // Missing uppercase
        assert!(matches!(
            validate_password_strength("lowercase123!", &policy),
            Err(PasswordError::NoUppercase)
        ));

        // Missing lowercase
        assert!(matches!(
            validate_password_strength("UPPERCASE123!", &policy),
            Err(PasswordError::NoLowercase)
        ));

        // Missing number
        assert!(matches!(
            validate_password_strength("Password!", &policy),
            Err(PasswordError::NoNumber)
        ));

        // Missing special character
        assert!(matches!(
            validate_password_strength("Password123", &policy),
            Err(PasswordError::NoSpecialChar)
        ));

        // Valid password
        assert!(validate_password_strength("Password123!", &policy).is_ok());
    }

    #[test]
    fn test_password_hashing() {
        let password = "TestPassword123!";
        let hash = hash_password(password).unwrap();

        assert!(verify_password(password, &hash).unwrap());
        assert!(!verify_password("WrongPassword", &hash).unwrap());
    }

    #[test]
    fn test_password_strength_calculation() {
        assert!(calculate_password_strength("weak") < 50);
        assert!(calculate_password_strength("StrongPassword123!") > 80);
    }

    #[test]
    fn test_reset_token_generation() {
        let token1 = generate_reset_token();
        let token2 = generate_reset_token();

        assert_eq!(token1.len(), 32);
        assert_eq!(token2.len(), 32);
        assert_ne!(token1, token2); // Should be different
    }
}