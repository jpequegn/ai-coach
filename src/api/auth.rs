use axum::{
    extract::{Query, Request, State},
    middleware,
    response::Json,
    routing::{get, post, put},
    Router,
};
use serde::Deserialize;

use crate::auth::{
    extract_user_session, jwt_auth_middleware, AuthError, AuthResponse, AuthService,
    ChangePasswordRequest, ForgotPasswordRequest, LoginRequest, MessageResponse,
    RefreshTokenRequest, RegisterRequest, ResetPasswordRequest, TokenResponse,
    UpdateProfileRequest, UserInfo, UserRole,
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
#[tracing::instrument(skip(request))]
async fn get_profile(request: Request) -> Result<Json<UserInfo>, AuthError> {
    let session = extract_user_session(&request)?;

    let user_info = UserInfo {
        id: session.user_id,
        email: session.email.clone(),
        role: session.role.clone(),
        created_at: chrono::Utc::now(), // TODO: Get from database
        updated_at: chrono::Utc::now(), // TODO: Get from database
    };

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
        .route("/users/:id/role", put(update_user_role))
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
    Query(params): Query<ListUsersQuery>,
) -> Result<Json<Vec<UserInfo>>, AuthError> {
    // Placeholder implementation - user listing not yet implemented
    // Future: Return paginated list of users with proper admin authorization
    Ok(Json(vec![]))
}

#[derive(Deserialize)]
struct UpdateRoleRequest {
    role: UserRole,
}

/// Update user role (admin only)
async fn update_user_role(
    axum::extract::Path(user_id): axum::extract::Path<uuid::Uuid>,
    Json(request): Json<UpdateRoleRequest>,
) -> Result<Json<MessageResponse>, AuthError> {
    // Placeholder implementation - role update not yet implemented
    // Future: Update user role with proper admin authorization
    Ok(Json(MessageResponse {
        message: "User role updated successfully".to_string(),
    }))
}