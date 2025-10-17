use chrono::{Duration, Utc};
use sqlx::{SqlitePool, Row};
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
    db: SqlitePool,
}

impl AuthService {
    pub fn new(db: SqlitePool, jwt_secret: &str) -> Self {
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
        let result = sqlx::query("SELECT 1 FROM token_blacklist WHERE jti = $1 AND expires_at > CURRENT_TIMESTAMP")
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

    /// Get user info by ID (includes timestamps from database)
    pub async fn get_user_info(&self, user_id: Uuid) -> Result<UserInfo, AuthError> {
        // Get user from database
        let user = sqlx::query_as::<_, User>(
            "SELECT id, email, password_hash, created_at, updated_at FROM users WHERE id = $1"
        )
        .bind(user_id)
        .fetch_optional(&self.db)
        .await
        .map_err(AuthError::Database)?
        .ok_or(AuthError::UserNotFound)?;

        // Get user role
        let role = self.get_user_role(user.id).await?.unwrap_or(UserRole::Athlete);

        Ok(UserInfo {
            id: user.id,
            email: user.email,
            role,
            created_at: user.created_at,
            updated_at: user.updated_at,
        })
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
             ON CONFLICT (user_id) DO UPDATE SET role = $2, updated_at = CURRENT_TIMESTAMP"
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
             WHERE user_id = $1 AND token_hash = $2 AND expires_at > CURRENT_TIMESTAMP AND revoked = 0"
        )
        .bind(user_id)
        .bind(token_hash)
        .fetch_optional(&self.db)
        .await
        .map_err(AuthError::Database)?;

        Ok(result.is_some())
    }

    async fn revoke_user_refresh_tokens(&self, user_id: Uuid) -> Result<(), AuthError> {
        sqlx::query("UPDATE refresh_tokens SET revoked = 1 WHERE user_id = $1")
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

    /// List all users with pagination (admin only)
    pub async fn list_all_users(&self, page: u32, limit: u32) -> Result<Vec<UserInfo>, AuthError> {
        let offset = (page.saturating_sub(1)) * limit;

        // Get users from database with pagination
        let users = sqlx::query_as::<_, User>(
            "SELECT id, email, password_hash, created_at, updated_at
             FROM users
             ORDER BY created_at DESC
             LIMIT $1 OFFSET $2"
        )
        .bind(limit as i64)
        .bind(offset as i64)
        .fetch_all(&self.db)
        .await
        .map_err(AuthError::Database)?;

        // Get user info with roles for each user
        let mut user_infos = Vec::new();
        for user in users {
            let role = self.get_user_role(user.id).await?.unwrap_or(UserRole::Athlete);
            user_infos.push(UserInfo {
                id: user.id,
                email: user.email,
                role,
                created_at: user.created_at,
                updated_at: user.updated_at,
            });
        }

        Ok(user_infos)
    }

    /// Update user role with audit logging (admin only)
    pub async fn update_user_role_admin(
        &self,
        user_id: Uuid,
        new_role: &UserRole,
        admin_id: Uuid,
    ) -> Result<(), AuthError> {
        // Get old role for audit logging
        let old_role = self.get_user_role(user_id).await?.unwrap_or(UserRole::Athlete);

        // Update the role
        self.update_user_role(user_id, new_role).await?;

        // Log audit event
        self.log_audit_event(
            user_id,
            admin_id,
            "update_role",
            "user",
            Some(user_id),
            Some(&old_role.as_str()),
            Some(&new_role.as_str()),
        )
        .await?;

        Ok(())
    }

    /// Log an audit event
    async fn log_audit_event(
        &self,
        user_id: Uuid,
        admin_id: Uuid,
        action: &str,
        entity_type: &str,
        entity_id: Option<Uuid>,
        old_value: Option<&str>,
        new_value: Option<&str>,
    ) -> Result<(), AuthError> {
        let audit_id = Uuid::new_v4();
        let now = chrono::Utc::now();

        sqlx::query(
            "INSERT INTO audit_log (id, user_id, admin_id, action, entity_type, entity_id, old_value, new_value, created_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)"
        )
        .bind(audit_id)
        .bind(user_id)
        .bind(admin_id)
        .bind(action)
        .bind(entity_type)
        .bind(entity_id.map(|id| id.to_string()))
        .bind(old_value)
        .bind(new_value)
        .bind(now)
        .execute(&self.db)
        .await
        .map_err(AuthError::Database)?;

        Ok(())
    }

    /// Count total users, optionally filtered by role (admin only)
    pub async fn count_users(&self, role_filter: Option<&UserRole>) -> Result<i64, AuthError> {
        let count = if let Some(role) = role_filter {
            sqlx::query_scalar::<_, i64>(
                "SELECT COUNT(DISTINCT u.id) FROM users u
                 JOIN user_roles ur ON u.id = ur.user_id
                 WHERE ur.role = $1"
            )
            .bind(role.as_str())
            .fetch_one(&self.db)
            .await
            .map_err(AuthError::Database)?
        } else {
            sqlx::query_scalar::<_, i64>("SELECT COUNT(*) FROM users")
                .fetch_one(&self.db)
                .await
                .map_err(AuthError::Database)?
        };

        Ok(count)
    }

    /// Search users by email or filter by role (admin only)
    pub async fn search_users(
        &self,
        email_query: Option<&str>,
        role_filter: Option<&UserRole>,
        page: u32,
        limit: u32,
    ) -> Result<Vec<UserInfo>, AuthError> {
        let offset = (page.saturating_sub(1)) * limit;

        // Build query based on filters
        let users = if let (Some(email), Some(role)) = (email_query, role_filter) {
            // Both email and role filters
            sqlx::query_as::<_, User>(
                "SELECT u.id, u.email, u.password_hash, u.created_at, u.updated_at
                 FROM users u
                 JOIN user_roles ur ON u.id = ur.user_id
                 WHERE u.email LIKE $1 AND ur.role = $2
                 ORDER BY u.created_at DESC
                 LIMIT $3 OFFSET $4"
            )
            .bind(format!("%{}%", email))
            .bind(role.as_str())
            .bind(limit as i64)
            .bind(offset as i64)
            .fetch_all(&self.db)
            .await
            .map_err(AuthError::Database)?
        } else if let Some(email) = email_query {
            // Email filter only
            sqlx::query_as::<_, User>(
                "SELECT id, email, password_hash, created_at, updated_at
                 FROM users
                 WHERE email LIKE $1
                 ORDER BY created_at DESC
                 LIMIT $2 OFFSET $3"
            )
            .bind(format!("%{}%", email))
            .bind(limit as i64)
            .bind(offset as i64)
            .fetch_all(&self.db)
            .await
            .map_err(AuthError::Database)?
        } else if let Some(role) = role_filter {
            // Role filter only
            sqlx::query_as::<_, User>(
                "SELECT u.id, u.email, u.password_hash, u.created_at, u.updated_at
                 FROM users u
                 JOIN user_roles ur ON u.id = ur.user_id
                 WHERE ur.role = $1
                 ORDER BY u.created_at DESC
                 LIMIT $2 OFFSET $3"
            )
            .bind(role.as_str())
            .bind(limit as i64)
            .bind(offset as i64)
            .fetch_all(&self.db)
            .await
            .map_err(AuthError::Database)?
        } else {
            // No filters - same as list_all_users
            sqlx::query_as::<_, User>(
                "SELECT id, email, password_hash, created_at, updated_at
                 FROM users
                 ORDER BY created_at DESC
                 LIMIT $1 OFFSET $2"
            )
            .bind(limit as i64)
            .bind(offset as i64)
            .fetch_all(&self.db)
            .await
            .map_err(AuthError::Database)?
        };

        // Enrich with roles
        let mut user_infos = Vec::new();
        for user in users {
            let role = self.get_user_role(user.id).await?.unwrap_or(UserRole::Athlete);
            user_infos.push(UserInfo {
                id: user.id,
                email: user.email,
                role,
                created_at: user.created_at,
                updated_at: user.updated_at,
            });
        }

        Ok(user_infos)
    }

    /// Query audit logs with filters (admin only)
    pub async fn query_audit_logs(
        &self,
        user_id_filter: Option<Uuid>,
        admin_id_filter: Option<Uuid>,
        action_filter: Option<&str>,
        page: u32,
        limit: u32,
    ) -> Result<Vec<AuditLogEntry>, AuthError> {
        let offset = (page.saturating_sub(1)) * limit;

        // Build query dynamically based on filters
        let mut query = String::from(
            "SELECT id, user_id, admin_id, action, entity_type, entity_id, old_value, new_value, created_at
             FROM audit_log WHERE 1=1"
        );

        let mut bind_count = 0;
        if user_id_filter.is_some() {
            bind_count += 1;
            query.push_str(&format!(" AND user_id = ${}", bind_count));
        }
        if admin_id_filter.is_some() {
            bind_count += 1;
            query.push_str(&format!(" AND admin_id = ${}", bind_count));
        }
        if action_filter.is_some() {
            bind_count += 1;
            query.push_str(&format!(" AND action = ${}", bind_count));
        }

        bind_count += 1;
        let limit_param = bind_count;
        bind_count += 1;
        let offset_param = bind_count;

        query.push_str(&format!(" ORDER BY created_at DESC LIMIT ${} OFFSET ${}", limit_param, offset_param));

        // Build and execute query with bindings
        let mut sqlx_query = sqlx::query_as::<_, AuditLogEntry>(&query);

        if let Some(user_id) = user_id_filter {
            sqlx_query = sqlx_query.bind(user_id);
        }
        if let Some(admin_id) = admin_id_filter {
            sqlx_query = sqlx_query.bind(admin_id);
        }
        if let Some(action) = action_filter {
            sqlx_query = sqlx_query.bind(action);
        }

        let logs = sqlx_query
            .bind(limit as i64)
            .bind(offset as i64)
            .fetch_all(&self.db)
            .await
            .map_err(AuthError::Database)?;

        Ok(logs)
    }

    /// Delete user with audit logging (admin only)
    pub async fn delete_user_admin(
        &self,
        user_id: Uuid,
        admin_id: Uuid,
    ) -> Result<(), AuthError> {
        // Get user info for audit log
        let user_info = self.get_user_info(user_id).await?;

        // Delete user (cascade will delete related records)
        sqlx::query("DELETE FROM users WHERE id = $1")
            .bind(user_id)
            .execute(&self.db)
            .await
            .map_err(AuthError::Database)?;

        // Log audit event
        self.log_audit_event(
            user_id,
            admin_id,
            "delete_user",
            "user",
            Some(user_id),
            Some(&format!("{}:{}", user_info.email, user_info.role.as_str())),
            None,
        )
        .await?;

        Ok(())
    }
}

/// Audit log entry model
#[derive(Debug, Clone, serde::Serialize, sqlx::FromRow)]
pub struct AuditLogEntry {
    pub id: String,
    pub user_id: String,
    pub admin_id: String,
    pub action: String,
    pub entity_type: String,
    pub entity_id: Option<String>,
    pub old_value: Option<String>,
    pub new_value: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}