use axum::{
    extract::{Query, Request, State},
    middleware,
    response::Json,
    routing::{delete, get, post, put},
    Extension,
    Router,
};
use serde::{Deserialize, Serialize};

use crate::auth::{
    extract_user_session, jwt_auth_middleware, AuditLogEntry, AuthError, AuthResponse,
    AuthService, ChangePasswordRequest, ForgotPasswordRequest, LoginRequest, MessageResponse,
    RefreshTokenRequest, RegisterRequest, ResetPasswordRequest, TokenResponse,
    UpdateProfileRequest, UserInfo, UserRole, UserSession,
};

/// Authentication routes
pub fn auth_routes(auth_service: AuthService) -> Router {
    Router::new()
        .route("/register", post(register))
        .route("/login", post(login))
        .route("/refresh", post(refresh_token))
        .route("/logout", post(logout))
        .route("/forgot-password", post(forgot_password))
        .route("/reset-password", post(reset_password))
        .route(
            "/profile",
            get(get_profile)
                .put(update_profile)
                .route_layer(middleware::from_fn_with_state(
                    auth_service.clone(),
                    jwt_auth_middleware,
                )),
        )
        .route(
            "/change-password",
            post(change_password).route_layer(middleware::from_fn_with_state(
                auth_service.clone(),
                jwt_auth_middleware,
            )),
        )
        .with_state(auth_service)
}

/// Register a new user
#[tracing::instrument(skip(auth_service, request))]
async fn register(
    State(auth_service): State<AuthService>,
    Json(request): Json<RegisterRequest>,
) -> Result<Json<AuthResponse>, AuthError> {
    let response = auth_service.register(request).await?;
    Ok(Json(response))
}

/// Login user
#[tracing::instrument(skip(auth_service, request))]
async fn login(
    State(auth_service): State<AuthService>,
    Json(request): Json<LoginRequest>,
) -> Result<Json<AuthResponse>, AuthError> {
    let response = auth_service.login(request).await?;
    Ok(Json(response))
}

/// Refresh access token
#[tracing::instrument(skip(auth_service, request))]
async fn refresh_token(
    State(auth_service): State<AuthService>,
    Json(request): Json<RefreshTokenRequest>,
) -> Result<Json<TokenResponse>, AuthError> {
    let response = auth_service.refresh_token(request).await?;
    Ok(Json(response))
}

/// Logout user
#[tracing::instrument(skip(auth_service, request))]
async fn logout(
    State(auth_service): State<AuthService>,
    request: Request,
) -> Result<Json<MessageResponse>, AuthError> {
    // Extract the token from the authorization header
    let auth_header = request
        .headers()
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|header| header.to_str().ok())
        .ok_or(AuthError::MissingAuthHeader)?;

    let token = crate::auth::extract_bearer_token(auth_header)?;
    let response = auth_service.logout(token).await?;
    Ok(Json(response))
}

/// Get user profile
#[tracing::instrument(skip(auth_service, request))]
async fn get_profile(
    State(auth_service): State<AuthService>,
    request: Request,
) -> Result<Json<UserInfo>, AuthError> {
    let session = extract_user_session(&request)?;

    // Get user info from database with actual timestamps
    let user_info = auth_service.get_user_info(session.user_id).await?;

    Ok(Json(user_info))
}

/// Update user profile
#[tracing::instrument(skip(auth_service, update_request))]
async fn update_profile(
    State(auth_service): State<AuthService>,
    Json(update_request): Json<UpdateProfileRequest>,
) -> Result<Json<MessageResponse>, AuthError> {
    // Placeholder implementation - profile update logic not yet implemented
    // Future: Connect to user service and update profile data

    Ok(Json(MessageResponse {
        message: "Profile updated successfully".to_string(),
    }))
}

/// Change user password
#[tracing::instrument(skip(auth_service, change_request))]
async fn change_password(
    State(auth_service): State<AuthService>,
    Json(change_request): Json<ChangePasswordRequest>,
) -> Result<Json<MessageResponse>, AuthError> {
    // Placeholder implementation - password change logic not yet implemented
    // Future: Validate current password and update with new hash

    Ok(Json(MessageResponse {
        message: "Password changed successfully".to_string(),
    }))
}

/// Forgot password
#[tracing::instrument(skip(auth_service, request))]
async fn forgot_password(
    State(auth_service): State<AuthService>,
    Json(request): Json<ForgotPasswordRequest>,
) -> Result<Json<MessageResponse>, AuthError> {
    // Placeholder implementation - forgot password flow not yet implemented
    // Future: Generate reset token and send email
    // This should:
    // 1. Check if user exists
    // 2. Generate reset token
    // 3. Send email with reset link
    // 4. Store reset token in database

    Ok(Json(MessageResponse {
        message: "If an account with that email exists, a password reset link has been sent.".to_string(),
    }))
}

/// Reset password
#[tracing::instrument(skip(auth_service, request))]
async fn reset_password(
    State(auth_service): State<AuthService>,
    Json(request): Json<ResetPasswordRequest>,
) -> Result<Json<MessageResponse>, AuthError> {
    // Placeholder implementation - password reset flow not yet implemented
    // Future: Validate reset token and update password
    // This should:
    // 1. Validate reset token
    // 2. Check if token is not expired
    // 3. Update user password
    // 4. Mark token as used

    Ok(Json(MessageResponse {
        message: "Password reset successfully".to_string(),
    }))
}

/// Admin endpoints
pub fn admin_routes(auth_service: AuthService) -> Router {
    Router::new()
        .route("/users", get(list_users))
        .route("/users/count", get(count_users))
        .route("/users/search", get(search_users))
        .route("/users/:id", delete(delete_user))
        .route("/users/:id/role", put(update_user_role))
        .route("/audit-logs", get(query_audit_logs))
        .route_layer(middleware::from_fn_with_state(
            auth_service.clone(),
            jwt_auth_middleware,
        ))
        .route_layer(middleware::from_fn(crate::auth::admin_only_middleware))
        .with_state(auth_service)
}

#[derive(Deserialize)]
struct ListUsersQuery {
    page: Option<u32>,
    limit: Option<u32>,
}

/// List all users (admin only)
async fn list_users(
    State(auth_service): State<AuthService>,
    Query(params): Query<ListUsersQuery>,
) -> Result<Json<Vec<UserInfo>>, AuthError> {
    // Default pagination: page 1, limit 50
    let page = params.page.unwrap_or(1);
    let limit = params.limit.unwrap_or(50).min(100); // Max 100 users per page

    let users = auth_service.list_all_users(page, limit).await?;
    Ok(Json(users))
}

#[derive(Deserialize)]
struct UpdateRoleRequest {
    role: UserRole,
}

/// Update user role (admin only)
async fn update_user_role(
    State(auth_service): State<AuthService>,
    Extension(session): Extension<UserSession>,
    axum::extract::Path(user_id): axum::extract::Path<uuid::Uuid>,
    Json(request): Json<UpdateRoleRequest>,
) -> Result<Json<MessageResponse>, AuthError> {
    // Verify user exists
    auth_service.get_user_info(user_id).await?;

    // Update role with audit logging
    auth_service
        .update_user_role_admin(user_id, &request.role, session.user_id)
        .await?;

    Ok(Json(MessageResponse {
        message: "User role updated successfully".to_string(),
    }))
}

#[derive(Deserialize)]
struct CountUsersQuery {
    role: Option<String>,
}

#[derive(Serialize)]
struct CountResponse {
    count: i64,
    role_filter: Option<String>,
}

/// Count users with optional role filter (admin only)
async fn count_users(
    State(auth_service): State<AuthService>,
    Query(params): Query<CountUsersQuery>,
) -> Result<Json<CountResponse>, AuthError> {
    // Parse role filter if provided
    let role_filter = if let Some(role_str) = &params.role {
        Some(
            UserRole::from_str(role_str)
                .ok_or_else(|| AuthError::ValidationError("Invalid role".to_string()))?,
        )
    } else {
        None
    };

    let count = auth_service.count_users(role_filter.as_ref()).await?;

    Ok(Json(CountResponse {
        count,
        role_filter: params.role,
    }))
}

#[derive(Deserialize)]
struct SearchUsersQuery {
    email: Option<String>,
    role: Option<String>,
    page: Option<u32>,
    limit: Option<u32>,
}

/// Search users by email and/or role (admin only)
async fn search_users(
    State(auth_service): State<AuthService>,
    Query(params): Query<SearchUsersQuery>,
) -> Result<Json<Vec<UserInfo>>, AuthError> {
    // Default pagination
    let page = params.page.unwrap_or(1);
    let limit = params.limit.unwrap_or(50).min(100);

    // Parse role filter if provided
    let role_filter = if let Some(role_str) = &params.role {
        Some(
            UserRole::from_str(role_str)
                .ok_or_else(|| AuthError::ValidationError("Invalid role".to_string()))?,
        )
    } else {
        None
    };

    let users = auth_service
        .search_users(
            params.email.as_deref(),
            role_filter.as_ref(),
            page,
            limit,
        )
        .await?;

    Ok(Json(users))
}

#[derive(Deserialize)]
struct AuditLogsQuery {
    user_id: Option<uuid::Uuid>,
    admin_id: Option<uuid::Uuid>,
    action: Option<String>,
    page: Option<u32>,
    limit: Option<u32>,
}

/// Query audit logs with filters (admin only)
async fn query_audit_logs(
    State(auth_service): State<AuthService>,
    Query(params): Query<AuditLogsQuery>,
) -> Result<Json<Vec<AuditLogEntry>>, AuthError> {
    // Default pagination
    let page = params.page.unwrap_or(1);
    let limit = params.limit.unwrap_or(50).min(100);

    let logs = auth_service
        .query_audit_logs(
            params.user_id,
            params.admin_id,
            params.action.as_deref(),
            page,
            limit,
        )
        .await?;

    Ok(Json(logs))
}

/// Delete user (admin only)
async fn delete_user(
    State(auth_service): State<AuthService>,
    Extension(session): Extension<UserSession>,
    axum::extract::Path(user_id): axum::extract::Path<uuid::Uuid>,
) -> Result<Json<MessageResponse>, AuthError> {
    // Prevent self-deletion
    if user_id == session.user_id {
        return Err(AuthError::ValidationError(
            "Cannot delete your own account".to_string(),
        ));
    }

    // Verify user exists
    auth_service.get_user_info(user_id).await?;

    // Delete user with audit logging
    auth_service
        .delete_user_admin(user_id, session.user_id)
        .await?;

    Ok(Json(MessageResponse {
        message: "User deleted successfully".to_string(),
    }))
}