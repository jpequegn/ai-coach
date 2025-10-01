use chrono::{Duration, Utc};
use jsonwebtoken::{decode, encode, Algorithm, DecodingKey, EncodingKey, Header, Validation};
use uuid::Uuid;

use crate::auth::{AuthError, Claims, UserRole, UserSession};

/// JWT token service for creating and validating tokens
#[derive(Clone)]
pub struct JwtService {
    encoding_key: EncodingKey,
    decoding_key: DecodingKey,
    access_token_expires_in: Duration,
    refresh_token_expires_in: Duration,
}

impl std::fmt::Debug for JwtService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("JwtService")
            .field("encoding_key", &"[REDACTED]")
            .field("decoding_key", &"[REDACTED]")
            .field("access_token_expires_in", &self.access_token_expires_in)
            .field("refresh_token_expires_in", &self.refresh_token_expires_in)
            .finish()
    }
}

impl JwtService {
    /// Create a new JWT service with the given secret
    pub fn new(secret: &str) -> Self {
        Self {
            encoding_key: EncodingKey::from_secret(secret.as_bytes()),
            decoding_key: DecodingKey::from_secret(secret.as_bytes()),
            access_token_expires_in: Duration::minutes(15), // 15 minutes
            refresh_token_expires_in: Duration::days(30),   // 30 days
        }
    }

    /// Create an access token for a user
    pub fn create_access_token(
        &self,
        user_id: Uuid,
        email: &str,
        role: UserRole,
    ) -> Result<String, AuthError> {
        let now = Utc::now();
        let exp = now + self.access_token_expires_in;
        let jti = Uuid::new_v4().to_string();

        let claims = Claims {
            sub: user_id.to_string(),
            email: email.to_string(),
            role,
            exp: exp.timestamp() as usize,
            iat: now.timestamp() as usize,
            jti,
        };

        encode(&Header::default(), &claims, &self.encoding_key)
            .map_err(AuthError::Jwt)
    }

    /// Create a refresh token for a user
    pub fn create_refresh_token(
        &self,
        user_id: Uuid,
        email: &str,
        role: UserRole,
    ) -> Result<String, AuthError> {
        let now = Utc::now();
        let exp = now + self.refresh_token_expires_in;
        let jti = Uuid::new_v4().to_string();

        let claims = Claims {
            sub: user_id.to_string(),
            email: email.to_string(),
            role,
            exp: exp.timestamp() as usize,
            iat: now.timestamp() as usize,
            jti,
        };

        encode(&Header::default(), &claims, &self.encoding_key)
            .map_err(AuthError::Jwt)
    }

    /// Validate and decode a token
    pub fn validate_token(&self, token: &str) -> Result<Claims, AuthError> {
        let validation = Validation::new(Algorithm::HS256);

        decode::<Claims>(token, &self.decoding_key, &validation)
            .map(|token_data| token_data.claims)
            .map_err(|err| match err.kind() {
                jsonwebtoken::errors::ErrorKind::ExpiredSignature => AuthError::TokenExpired,
                _ => AuthError::InvalidToken,
            })
    }

    /// Extract user session from token
    pub fn extract_user_session(&self, token: &str) -> Result<UserSession, AuthError> {
        let claims = self.validate_token(token)?;
        UserSession::from_claims(&claims)
            .map_err(|_| AuthError::InvalidToken)
    }

    /// Get access token expiration time in seconds
    pub fn access_token_expires_in_seconds(&self) -> usize {
        self.access_token_expires_in.num_seconds() as usize
    }

    /// Get refresh token expiration time in seconds
    pub fn refresh_token_expires_in_seconds(&self) -> usize {
        self.refresh_token_expires_in.num_seconds() as usize
    }

    /// Check if a token is expired (without validating signature)
    pub fn is_token_expired(&self, token: &str) -> bool {
        match decode::<Claims>(
            token,
            &self.decoding_key,
            &Validation::new(Algorithm::HS256),
        ) {
            Ok(token_data) => {
                let exp = token_data.claims.exp as i64;
                Utc::now().timestamp() > exp
            }
            Err(_) => true, // If we can't decode, consider it expired
        }
    }

    /// Extract JWT ID from token (for blacklisting)
    pub fn extract_jti(&self, token: &str) -> Result<String, AuthError> {
        let claims = self.validate_token(token)?;
        Ok(claims.jti)
    }

    /// Create token pair (access + refresh)
    pub fn create_token_pair(
        &self,
        user_id: Uuid,
        email: &str,
        role: UserRole,
    ) -> Result<(String, String), AuthError> {
        let access_token = self.create_access_token(user_id, email, role.clone())?;
        let refresh_token = self.create_refresh_token(user_id, email, role)?;
        Ok((access_token, refresh_token))
    }
}

/// Extract bearer token from authorization header
pub fn extract_bearer_token(auth_header: &str) -> Result<&str, AuthError> {
    if !auth_header.starts_with("Bearer ") {
        return Err(AuthError::InvalidAuthHeaderFormat);
    }

    let token = auth_header.strip_prefix("Bearer ").unwrap();
    if token.is_empty() {
        return Err(AuthError::InvalidAuthHeaderFormat);
    }

    Ok(token)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_jwt_creation_and_validation() {
        let jwt_service = JwtService::new("test_secret");
        let user_id = Uuid::new_v4();
        let email = "test@example.com";
        let role = UserRole::Athlete;

        // Create token
        let token = jwt_service
            .create_access_token(user_id, email, role.clone())
            .unwrap();

        // Validate token
        let claims = jwt_service.validate_token(&token).unwrap();

        assert_eq!(claims.sub, user_id.to_string());
        assert_eq!(claims.email, email);
        assert_eq!(claims.role, role);
    }

    #[test]
    fn test_bearer_token_extraction() {
        assert_eq!(
            extract_bearer_token("Bearer test_token").unwrap(),
            "test_token"
        );

        assert!(extract_bearer_token("Invalid header").is_err());
        assert!(extract_bearer_token("Bearer ").is_err());
    }

    #[test]
    fn test_user_session_extraction() {
        let jwt_service = JwtService::new("test_secret");
        let user_id = Uuid::new_v4();
        let email = "test@example.com";
        let role = UserRole::Coach;

        let token = jwt_service
            .create_access_token(user_id, email, role.clone())
            .unwrap();

        let session = jwt_service.extract_user_session(&token).unwrap();

        assert_eq!(session.user_id, user_id);
        assert_eq!(session.email, email);
        assert_eq!(session.role, role);
    }

    #[test]
    fn test_token_pair_creation() {
        let jwt_service = JwtService::new("test_secret");
        let user_id = Uuid::new_v4();
        let email = "test@example.com";
        let role = UserRole::Admin;

        let (access_token, refresh_token) = jwt_service
            .create_token_pair(user_id, email, role)
            .unwrap();

        // Both tokens should be valid
        assert!(jwt_service.validate_token(&access_token).is_ok());
        assert!(jwt_service.validate_token(&refresh_token).is_ok());

        // JTIs should be different
        let access_jti = jwt_service.extract_jti(&access_token).unwrap();
        let refresh_jti = jwt_service.extract_jti(&refresh_token).unwrap();
        assert_ne!(access_jti, refresh_jti);
    }
}