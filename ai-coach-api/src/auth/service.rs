use chrono::{Duration, Utc};
use sqlx::{PgPool, Row};
use uuid::Uuid;

use crate::auth::{
    AuthError, AuthResponse, Claims, JwtService, LoginRequest, MessageResponse, RefreshTokenRequest,
    RegisterRequest, TokenResponse, UserInfo, UserRole, UserSession,
};
use crate::auth::password::{hash_password, verify_password, generate_reset_token};

/// Simple user model for authentication
#[derive(Debug, Clone, sqlx::FromRow)]
pub struct User {
    pub id: Uuid,
    pub email: String,
    pub password_hash: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Clone)]
pub struct AuthService {
    jwt_service: JwtService,
    db: PgPool,
}

impl AuthService {
    pub fn new(db: PgPool, jwt_secret: &str) -> Self {
        Self {
            jwt_service: JwtService::new(jwt_secret),
            db,
        }
    }

    /// Register a new user
    pub async fn register(&self, request: RegisterRequest) -> Result<AuthResponse, AuthError> {
        // Check if user already exists
        if self.get_user_by_email(&request.email).await?.is_some() {
            return Err(AuthError::EmailAlreadyExists);
        }

        // Hash password
        let password_hash = hash_password(&request.password)?;
        let role = request.role.unwrap_or(UserRole::Athlete);
        let user_id = Uuid::new_v4();
        let now = chrono::Utc::now();

        // Create user
        let user = sqlx::query_as::<_, User>(
            "INSERT INTO users (id, email, password_hash, created_at, updated_at)
             VALUES ($1, $2, $3, $4, $5)
             RETURNING id, email, password_hash, created_at, updated_at"
        )
        .bind(user_id)
        .bind(&request.email)
        .bind(&password_hash)
        .bind(now)
        .bind(now)
        .fetch_one(&self.db)
        .await
        .map_err(AuthError::Database)?;

        // Add role to user
        self.update_user_role(user.id, &role).await?;

        // Generate tokens
        let (access_token, refresh_token) = self
            .jwt_service
            .create_token_pair(user.id, &user.email, role.clone())?;

        // Store refresh token
        self.store_refresh_token(user.id, &refresh_token).await?;

        Ok(AuthResponse {
            access_token,
            refresh_token,
            token_type: "Bearer".to_string(),
            expires_in: self.jwt_service.access_token_expires_in_seconds(),
            user: UserInfo {
                id: user.id,
                email: user.email,
                role,
                created_at: user.created_at,
                updated_at: user.updated_at,
            },
        })
    }

    /// Login user
    pub async fn login(&self, request: LoginRequest) -> Result<AuthResponse, AuthError> {
        // Get user with password hash for verification
        let user = self.get_user_with_password(&request.email).await?;

        // Verify password
        if !verify_password(&request.password, &user.password_hash)? {
            return Err(AuthError::InvalidCredentials);
        }

        // Get user role
        let role = self.get_user_role(user.id).await?.unwrap_or(UserRole::Athlete);

        // Generate tokens
        let (access_token, refresh_token) = self
            .jwt_service
            .create_token_pair(user.id, &user.email, role.clone())?;

        // Store refresh token
        self.store_refresh_token(user.id, &refresh_token).await?;

        Ok(AuthResponse {
            access_token,
            refresh_token,
            token_type: "Bearer".to_string(),
            expires_in: self.jwt_service.access_token_expires_in_seconds(),
            user: UserInfo {
                id: user.id,
                email: user.email,
                role,
                created_at: user.created_at,
                updated_at: user.updated_at,
            },
        })
    }

    /// Refresh access token
    pub async fn refresh_token(&self, request: RefreshTokenRequest) -> Result<TokenResponse, AuthError> {
        // Validate refresh token
        let claims = self.jwt_service.validate_token(&request.refresh_token)?;

        // Check if refresh token exists in database
        let user_id = Uuid::parse_str(&claims.sub).map_err(|_| AuthError::InvalidToken)?;
        if !self.is_refresh_token_valid(user_id, &request.refresh_token).await? {
            return Err(AuthError::InvalidToken);
        }

        // Create new access token
        let access_token = self
            .jwt_service
            .create_access_token(user_id, &claims.email, claims.role)?;

        Ok(TokenResponse {
            access_token,
            token_type: "Bearer".to_string(),
            expires_in: self.jwt_service.access_token_expires_in_seconds(),
        })
    }

    /// Logout user (blacklist token)
    pub async fn logout(&self, token: &str) -> Result<MessageResponse, AuthError> {
        let jti = self.jwt_service.extract_jti(token)?;
        let claims = self.jwt_service.validate_token(token)?;
        let user_id = Uuid::parse_str(&claims.sub).map_err(|_| AuthError::InvalidToken)?;

        // Blacklist the access token
        self.blacklist_token(&jti, claims.exp as i64).await?;

        // Revoke refresh tokens for this user
        self.revoke_user_refresh_tokens(user_id).await?;

        Ok(MessageResponse {
            message: "Successfully logged out".to_string(),
        })
    }

    /// Check if token is blacklisted
    pub async fn is_token_blacklisted(&self, jti: &str) -> Result<bool, AuthError> {
        let result = sqlx::query("SELECT 1 FROM token_blacklist WHERE jti = $1 AND expires_at > NOW()")
            .bind(jti)
            .fetch_optional(&self.db)
            .await
            .map_err(AuthError::Database)?;

        Ok(result.is_some())
    }

    /// Validate user session from token
    pub async fn validate_session(&self, token: &str) -> Result<UserSession, AuthError> {
        let session = self.jwt_service.extract_user_session(token)?;

        // Check if token is blacklisted
        if self.is_token_blacklisted(&session.jti).await? {
            return Err(AuthError::InvalidToken);
        }

        Ok(session)
    }

    // Private helper methods

    async fn get_user_by_email(&self, email: &str) -> Result<Option<User>, AuthError> {
        let user = sqlx::query_as::<_, User>(
            "SELECT id, email, password_hash, created_at, updated_at FROM users WHERE email = $1"
        )
        .bind(email)
        .fetch_optional(&self.db)
        .await
        .map_err(AuthError::Database)?;

        Ok(user)
    }

    async fn get_user_with_password(&self, email: &str) -> Result<User, AuthError> {
        let user = sqlx::query_as::<_, User>(
            "SELECT id, email, password_hash, created_at, updated_at FROM users WHERE email = $1"
        )
        .bind(email)
        .fetch_optional(&self.db)
        .await
        .map_err(AuthError::Database)?
        .ok_or(AuthError::UserNotFound)?;

        Ok(user)
    }

    async fn get_user_role(&self, user_id: Uuid) -> Result<Option<UserRole>, AuthError> {
        let result = sqlx::query("SELECT role FROM user_roles WHERE user_id = $1")
            .bind(user_id)
            .fetch_optional(&self.db)
            .await
            .map_err(AuthError::Database)?;

        Ok(result.and_then(|row| {
            let role_str: String = row.get("role");
            UserRole::from_str(&role_str)
        }))
    }

    async fn update_user_role(&self, user_id: Uuid, role: &UserRole) -> Result<(), AuthError> {
        sqlx::query(
            "INSERT INTO user_roles (user_id, role) VALUES ($1, $2)
             ON CONFLICT (user_id) DO UPDATE SET role = $2, updated_at = NOW()"
        )
        .bind(user_id)
        .bind(role.as_str())
        .execute(&self.db)
        .await
        .map_err(AuthError::Database)?;

        Ok(())
    }

    async fn store_refresh_token(&self, user_id: Uuid, refresh_token: &str) -> Result<(), AuthError> {
        let claims = self.jwt_service.validate_token(refresh_token)?;
        let expires_at = chrono::DateTime::from_timestamp(claims.exp as i64, 0)
            .ok_or(AuthError::InvalidToken)?;

        sqlx::query(
            "INSERT INTO refresh_tokens (id, user_id, token_hash, expires_at)
             VALUES ($1, $2, $3, $4)"
        )
        .bind(Uuid::new_v4())
        .bind(user_id)
        .bind(format!("{:x}", md5::compute(refresh_token)))
        .bind(expires_at)
        .execute(&self.db)
        .await
        .map_err(AuthError::Database)?;

        Ok(())
    }

    async fn is_refresh_token_valid(&self, user_id: Uuid, refresh_token: &str) -> Result<bool, AuthError> {
        let token_hash = format!("{:x}", md5::compute(refresh_token));

        let result = sqlx::query(
            "SELECT 1 FROM refresh_tokens
             WHERE user_id = $1 AND token_hash = $2 AND expires_at > NOW() AND NOT revoked"
        )
        .bind(user_id)
        .bind(token_hash)
        .fetch_optional(&self.db)
        .await
        .map_err(AuthError::Database)?;

        Ok(result.is_some())
    }

    async fn revoke_user_refresh_tokens(&self, user_id: Uuid) -> Result<(), AuthError> {
        sqlx::query("UPDATE refresh_tokens SET revoked = true WHERE user_id = $1")
            .bind(user_id)
            .execute(&self.db)
            .await
            .map_err(AuthError::Database)?;

        Ok(())
    }

    async fn blacklist_token(&self, jti: &str, exp: i64) -> Result<(), AuthError> {
        let expires_at = chrono::DateTime::from_timestamp(exp, 0)
            .ok_or(AuthError::InvalidToken)?;

        sqlx::query(
            "INSERT INTO token_blacklist (jti, expires_at) VALUES ($1, $2)
             ON CONFLICT (jti) DO NOTHING"
        )
        .bind(jti)
        .bind(expires_at)
        .execute(&self.db)
        .await
        .map_err(AuthError::Database)?;

        Ok(())
    }
}