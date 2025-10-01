use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

/// User roles for role-based access control
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum UserRole {
    Athlete,
    Coach,
    Admin,
}

impl UserRole {
    pub fn as_str(&self) -> &'static str {
        match self {
            UserRole::Athlete => "athlete",
            UserRole::Coach => "coach",
            UserRole::Admin => "admin",
        }
    }

    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "athlete" => Some(UserRole::Athlete),
            "coach" => Some(UserRole::Coach),
            "admin" => Some(UserRole::Admin),
            _ => None,
        }
    }

    /// Check if this role has permission to access another role's resources
    pub fn can_access(&self, target_role: &UserRole) -> bool {
        match self {
            UserRole::Admin => true, // Admin can access everything
            UserRole::Coach => matches!(target_role, UserRole::Athlete | UserRole::Coach),
            UserRole::Athlete => matches!(target_role, UserRole::Athlete),
        }
    }
}

/// JWT token claims
#[derive(Debug, Serialize, Deserialize)]
pub struct Claims {
    pub sub: String,      // Subject (user ID)
    pub email: String,    // User email
    pub role: UserRole,   // User role
    pub exp: usize,       // Expiration time
    pub iat: usize,       // Issued at
    pub jti: String,      // JWT ID (for revocation)
}

/// Authentication request models
#[derive(Debug, Deserialize)]
pub struct RegisterRequest {
    pub email: String,
    pub password: String,
    pub role: Option<UserRole>, // Optional, defaults to Athlete
}

#[derive(Debug, Deserialize)]
pub struct LoginRequest {
    pub email: String,
    pub password: String,
}

#[derive(Debug, Deserialize)]
pub struct RefreshTokenRequest {
    pub refresh_token: String,
}

#[derive(Debug, Deserialize)]
pub struct ForgotPasswordRequest {
    pub email: String,
}

#[derive(Debug, Deserialize)]
pub struct ResetPasswordRequest {
    pub token: String,
    pub new_password: String,
}

#[derive(Debug, Deserialize)]
pub struct ChangePasswordRequest {
    pub current_password: String,
    pub new_password: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateProfileRequest {
    pub email: Option<String>,
}

/// Authentication response models
#[derive(Debug, Serialize)]
pub struct AuthResponse {
    pub access_token: String,
    pub refresh_token: String,
    pub token_type: String,
    pub expires_in: usize,
    pub user: UserInfo,
}

#[derive(Debug, Serialize)]
pub struct UserInfo {
    pub id: Uuid,
    pub email: String,
    pub role: UserRole,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct TokenResponse {
    pub access_token: String,
    pub token_type: String,
    pub expires_in: usize,
}

#[derive(Debug, Serialize)]
pub struct MessageResponse {
    pub message: String,
}

/// Password reset token model
#[derive(Debug, Clone)]
pub struct PasswordResetToken {
    pub user_id: Uuid,
    pub token: String,
    pub expires_at: DateTime<Utc>,
    pub used: bool,
}

/// Refresh token model
#[derive(Debug, Clone)]
pub struct RefreshToken {
    pub id: Uuid,
    pub user_id: Uuid,
    pub token_hash: String,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub revoked: bool,
}

/// Token blacklist model (for logout)
#[derive(Debug, Clone)]
pub struct TokenBlacklist {
    pub jti: String,
    pub expires_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

/// User session information
#[derive(Debug, Clone)]
pub struct UserSession {
    pub user_id: Uuid,
    pub email: String,
    pub role: UserRole,
    pub jti: String,
}

impl UserSession {
    pub fn from_claims(claims: &Claims) -> Result<Self, uuid::Error> {
        Ok(Self {
            user_id: Uuid::parse_str(&claims.sub)?,
            email: claims.email.clone(),
            role: claims.role.clone(),
            jti: claims.jti.clone(),
        })
    }
}

/// Rate limiting models
#[derive(Debug, Clone)]
pub struct RateLimitConfig {
    pub max_requests: u32,
    pub window_seconds: u64,
}

impl Default for RateLimitConfig {
    fn default() -> Self {
        Self {
            max_requests: 5,      // 5 requests
            window_seconds: 300,  // per 5 minutes
        }
    }
}