use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Json},
    routing::{get, post, put},
    Extension, Router,
};
use serde_json::json;
use std::sync::Arc;
use uuid::Uuid;

use crate::auth::Claims;
use crate::models::recovery_protocol::*;
use crate::services::RecoveryProtocolService;

/// Get all available recovery protocols with optional filtering
pub async fn get_protocols(
    State(service): State<Arc<RecoveryProtocolService>>,
    Query(query): Query<ProtocolQuery>,
) -> impl IntoResponse {
    match service
        .get_applicable_protocols(query.scenario, query.duration_max, query.category)
        .await
    {
        Ok(protocols) => {
            let responses: Vec<_> = protocols
                .into_iter()
                .map(|protocol| {
                    let target_scenarios_parsed: Vec<String> =
                        serde_json::from_str(&protocol.target_scenarios).unwrap_or_default();
                    ProtocolResponse {
                        protocol,
                        recommendations: vec![], // Will be populated once recommendation system is integrated
                        target_scenarios_parsed,
                    }
                })
                .collect();

            (StatusCode::OK, Json(json!({ "protocols": responses }))).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

/// Get a specific protocol by ID
pub async fn get_protocol(
    State(service): State<Arc<RecoveryProtocolService>>,
    Path(protocol_id): Path<Uuid>,
) -> impl IntoResponse {
    match service.get_protocol(protocol_id).await {
        Ok(Some(protocol)) => {
            let target_scenarios_parsed: Vec<String> =
                serde_json::from_str(&protocol.target_scenarios).unwrap_or_default();
            let response = ProtocolResponse {
                protocol,
                recommendations: vec![], // Will be populated once recommendation system is integrated
                target_scenarios_parsed,
            };
            (StatusCode::OK, Json(response)).into_response()
        }
        Ok(None) => (
            StatusCode::NOT_FOUND,
            Json(json!({ "error": "Protocol not found" })),
        )
            .into_response(),
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

/// Activate a protocol for the authenticated user
pub async fn activate_protocol(
    State(service): State<Arc<RecoveryProtocolService>>,
    Extension(claims): Extension<Claims>,
    Json(request): Json<ActivateProtocolRequest>,
) -> impl IntoResponse {
    let user_id = match Uuid::parse_str(&claims.sub) {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "Invalid user ID" })),
            )
                .into_response()
        }
    };

    match service.activate_protocol(user_id, request.protocol_id).await {
        Ok(activation) => {
            // Fetch protocol details
            let protocol = match service.get_protocol(request.protocol_id).await {
                Ok(Some(p)) => p,
                Ok(None) => {
                    return (
                        StatusCode::NOT_FOUND,
                        Json(json!({ "error": "Protocol not found" })),
                    )
                        .into_response()
                }
                Err(e) => {
                    return (
                        StatusCode::INTERNAL_SERVER_ERROR,
                        Json(json!({ "error": e.to_string() })),
                    )
                        .into_response()
                }
            };

            let response = ProtocolActivationResponse {
                activation_id: activation.id,
                protocol,
                status: activation.status,
                next_recommendations: vec![], // Will be populated once recommendation system is integrated
                message: format!("Successfully activated protocol. Your protocol journey begins now!"),
            };

            (StatusCode::CREATED, Json(response)).into_response()
        }
        Err(e) => {
            let status = if e.to_string().contains("already active") {
                StatusCode::CONFLICT
            } else if e.to_string().contains("not found") {
                StatusCode::NOT_FOUND
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };

            (status, Json(json!({ "error": e.to_string() }))).into_response()
        }
    }
}

/// Get active protocols for the authenticated user
pub async fn get_active_protocols(
    State(service): State<Arc<RecoveryProtocolService>>,
    Extension(claims): Extension<Claims>,
) -> impl IntoResponse {
    let user_id = match Uuid::parse_str(&claims.sub) {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "Invalid user ID" })),
            )
                .into_response()
        }
    };

    match service.get_active_protocols(user_id).await {
        Ok(activations) => {
            // Fetch protocol details for each activation
            let mut responses = Vec::new();
            for activation in activations {
                if let Ok(Some(protocol)) = service.get_protocol(activation.protocol_id).await {
                    // Parse progress
                    let progress_json: serde_json::Value =
                        serde_json::from_str(&activation.progress).unwrap_or(json!({}));
                    let completed_count = progress_json
                        .as_object()
                        .map(|obj| obj.len() as i32)
                        .unwrap_or(0);

                    // For now, assume total recommendations will come from protocol_recommendations table
                    let total_recommendations = 0; // Will be calculated once recommendations are linked
                    let completion_percentage = if total_recommendations > 0 {
                        (completed_count as f64 / total_recommendations as f64) * 100.0
                    } else {
                        0.0
                    };

                    let progress = ProtocolProgress {
                        total_recommendations,
                        completed_count,
                        completion_percentage,
                    };

                    responses.push(ActiveProtocolResponse {
                        activation_id: activation.id,
                        protocol,
                        status: activation.status,
                        progress,
                        activated_at: activation.activated_at,
                        recommendations: vec![], // Will be populated once recommendation system is integrated
                    });
                }
            }

            (StatusCode::OK, Json(json!({ "active_protocols": responses }))).into_response()
        }
        Err(e) => (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({ "error": e.to_string() })),
        )
            .into_response(),
    }
}

/// Update protocol progress
pub async fn update_protocol_progress(
    State(service): State<Arc<RecoveryProtocolService>>,
    Extension(claims): Extension<Claims>,
    Path(activation_id): Path<Uuid>,
    Json(request): Json<UpdateProtocolProgressRequest>,
) -> impl IntoResponse {
    // Verify user owns this activation
    let user_id = match Uuid::parse_str(&claims.sub) {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "Invalid user ID" })),
            )
                .into_response()
        }
    };

    match service
        .update_protocol_progress(activation_id, request.recommendation_id, request.completed)
        .await
    {
        Ok(activation) => {
            // Verify ownership
            if activation.user_id != user_id {
                return (
                    StatusCode::FORBIDDEN,
                    Json(json!({ "error": "Not authorized to update this protocol" })),
                )
                    .into_response();
            }

            (StatusCode::OK, Json(activation)).into_response()
        }
        Err(e) => {
            let status = if e.to_string().contains("not found") {
                StatusCode::NOT_FOUND
            } else if e.to_string().contains("not active") {
                StatusCode::CONFLICT
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };

            (status, Json(json!({ "error": e.to_string() }))).into_response()
        }
    }
}

/// Complete a protocol
pub async fn complete_protocol(
    State(service): State<Arc<RecoveryProtocolService>>,
    Extension(claims): Extension<Claims>,
    Path(activation_id): Path<Uuid>,
) -> impl IntoResponse {
    let user_id = match Uuid::parse_str(&claims.sub) {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "Invalid user ID" })),
            )
                .into_response()
        }
    };

    match service.complete_protocol(activation_id).await {
        Ok(activation) => {
            // Verify ownership
            if activation.user_id != user_id {
                return (
                    StatusCode::FORBIDDEN,
                    Json(json!({ "error": "Not authorized to complete this protocol" })),
                )
                    .into_response();
            }

            (
                StatusCode::OK,
                Json(json!({
                    "activation": activation,
                    "message": "Congratulations! You've completed this recovery protocol."
                })),
            )
                .into_response()
        }
        Err(e) => {
            let status = if e.to_string().contains("not found") {
                StatusCode::NOT_FOUND
            } else if e.to_string().contains("not active") {
                StatusCode::CONFLICT
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };

            (status, Json(json!({ "error": e.to_string() }))).into_response()
        }
    }
}

/// Abandon a protocol
pub async fn abandon_protocol(
    State(service): State<Arc<RecoveryProtocolService>>,
    Extension(claims): Extension<Claims>,
    Path(activation_id): Path<Uuid>,
    Json(request): Json<AbandonProtocolRequest>,
) -> impl IntoResponse {
    let user_id = match Uuid::parse_str(&claims.sub) {
        Ok(id) => id,
        Err(_) => {
            return (
                StatusCode::BAD_REQUEST,
                Json(json!({ "error": "Invalid user ID" })),
            )
                .into_response()
        }
    };

    match service.abandon_protocol(activation_id, request.reason).await {
        Ok(activation) => {
            // Verify ownership
            if activation.user_id != user_id {
                return (
                    StatusCode::FORBIDDEN,
                    Json(json!({ "error": "Not authorized to abandon this protocol" })),
                )
                    .into_response();
            }

            (
                StatusCode::OK,
                Json(json!({
                    "activation": activation,
                    "message": "Protocol abandoned. You can always start a new protocol when ready."
                })),
            )
                .into_response()
        }
        Err(e) => {
            let status = if e.to_string().contains("not found") {
                StatusCode::NOT_FOUND
            } else if e.to_string().contains("not active") {
                StatusCode::CONFLICT
            } else {
                StatusCode::INTERNAL_SERVER_ERROR
            };

            (status, Json(json!({ "error": e.to_string() }))).into_response()
        }
    }
}

/// Create recovery protocol routes
/// Note: This will be integrated into the main router once the recommendation system is fully migrated to SQLite
pub fn recovery_protocol_routes(service: Arc<RecoveryProtocolService>) -> Router {
    Router::new()
        .route("/", get(get_protocols))
        .route("/:protocol_id", get(get_protocol))
        .route("/activate", post(activate_protocol))
        .route("/active", get(get_active_protocols))
        .route("/:activation_id/progress", put(update_protocol_progress))
        .route("/:activation_id/complete", post(complete_protocol))
        .route("/:activation_id/abandon", post(abandon_protocol))
        .with_state(service)
}
