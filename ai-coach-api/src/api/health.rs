use axum::{extract::State, http::StatusCode, response::Json};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sqlx::SqlitePool;
use std::sync::Arc;
use std::time::Instant;

// MVP: JobHealthStatus disabled with job models
// use crate::models::JobHealthStatus;
// Disabled for MVP
// use crate::services::RecoveryJobScheduler;

/// Overall health status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum HealthStatus {
    Healthy,
    Degraded,
    Unhealthy,
}

/// Component health check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComponentHealth {
    pub status: HealthStatus,
    pub latency_ms: Option<i64>,
    pub message: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub details: Option<serde_json::Value>,
}

impl ComponentHealth {
    pub fn healthy(latency_ms: i64) -> Self {
        Self {
            status: HealthStatus::Healthy,
            latency_ms: Some(latency_ms),
            message: None,
            details: None,
        }
    }

    pub fn healthy_with_details(latency_ms: i64, details: serde_json::Value) -> Self {
        Self {
            status: HealthStatus::Healthy,
            latency_ms: Some(latency_ms),
            message: None,
            details: Some(details),
        }
    }

    pub fn degraded(message: impl Into<String>) -> Self {
        Self {
            status: HealthStatus::Degraded,
            latency_ms: None,
            message: Some(message.into()),
            details: None,
        }
    }

    pub fn unhealthy(message: impl Into<String>) -> Self {
        Self {
            status: HealthStatus::Unhealthy,
            latency_ms: None,
            message: Some(message.into()),
            details: None,
        }
    }
}

/// Application health state
pub struct HealthState {
    pub scheduler: Option<Arc<()>>,  // MVP: Disabled RecoveryJobScheduler
    pub db: SqlitePool,
}

/// Basic health check endpoint
///
/// Returns service health status without detailed checks
pub async fn health_check() -> Result<Json<Value>, StatusCode> {
    Ok(Json(json!({
        "status": "healthy",
        "service": "ai-coach",
        "version": env!("CARGO_PKG_VERSION"),
        "timestamp": chrono::Utc::now().to_rfc3339()
    })))
}

/// Check database health
async fn check_database_health(db: &SqlitePool) -> ComponentHealth {
    let start = Instant::now();

    match sqlx::query("SELECT 1 as health_check")
        .fetch_one(db)
        .await
    {
        Ok(_) => {
            let latency_ms = start.elapsed().as_millis() as i64;

            // Get connection pool stats
            let details = json!({
                "pool_size": db.size(),
                "idle_connections": db.num_idle(),
            });

            // Warn if latency is high
            if latency_ms > 100 {
                ComponentHealth {
                    status: HealthStatus::Degraded,
                    latency_ms: Some(latency_ms),
                    message: Some("High database latency".to_string()),
                    details: Some(details),
                }
            } else {
                ComponentHealth::healthy_with_details(latency_ms, details)
            }
        }
        Err(e) => ComponentHealth::unhealthy(format!("Database connection failed: {}", e)),
    }
}

/// Check Redis health
async fn check_redis_health() -> ComponentHealth {
    // Check if Redis URL is configured
    match std::env::var("REDIS_URL") {
        Ok(redis_url) => {
            let start = Instant::now();

            // Try to connect to Redis
            match redis::Client::open(redis_url) {
                Ok(client) => {
                    match client.get_async_connection().await {
                        Ok(mut conn) => {
                            // Try a simple ping command
                            match redis::cmd("PING")
                                .query_async::<_, String>(&mut conn)
                                .await
                            {
                                Ok(_) => {
                                    let latency_ms = start.elapsed().as_millis() as i64;

                                    if latency_ms > 50 {
                                        ComponentHealth {
                                            status: HealthStatus::Degraded,
                                            latency_ms: Some(latency_ms),
                                            message: Some("High Redis latency".to_string()),
                                            details: None,
                                        }
                                    } else {
                                        ComponentHealth::healthy(latency_ms)
                                    }
                                }
                                Err(e) => ComponentHealth::unhealthy(format!("Redis ping failed: {}", e)),
                            }
                        }
                        Err(e) => ComponentHealth::unhealthy(format!("Redis connection failed: {}", e)),
                    }
                }
                Err(e) => ComponentHealth::unhealthy(format!("Invalid Redis URL: {}", e)),
            }
        }
        Err(_) => ComponentHealth {
            status: HealthStatus::Healthy,
            latency_ms: None,
            message: Some("Redis not configured (optional)".to_string()),
            details: None,
        },
    }
}

/* MVP: Disabled scheduler health check
/// Check job scheduler health
async fn check_scheduler_health(scheduler: &RecoveryJobScheduler) -> ComponentHealth {
    match scheduler.check_all_jobs_health().await {
        Ok(job_health) => {
            let status = match job_health.overall_status {
                JobHealthStatus::Healthy => HealthStatus::Healthy,
                JobHealthStatus::Degraded => HealthStatus::Degraded,
                JobHealthStatus::Unhealthy => HealthStatus::Unhealthy,
            };

            let details = json!({
                "total": job_health.total_jobs,
                "healthy": job_health.healthy_jobs,
                "degraded": job_health.degraded_jobs,
                "unhealthy": job_health.unhealthy_jobs,
            });

            ComponentHealth {
                status,
                latency_ms: None,
                message: None,
                details: Some(details),
            }
        }
        Err(e) => ComponentHealth::unhealthy(format!("Failed to check jobs: {}", e)),
    }
}
*/

/// Determine overall health status from component statuses
fn determine_overall_status(components: &[&ComponentHealth]) -> HealthStatus {
    let mut has_degraded = false;

    for component in components {
        match component.status {
            HealthStatus::Unhealthy => return HealthStatus::Unhealthy,
            HealthStatus::Degraded => has_degraded = true,
            HealthStatus::Healthy => {}
        }
    }

    if has_degraded {
        HealthStatus::Degraded
    } else {
        HealthStatus::Healthy
    }
}

/// Detailed health check with comprehensive component monitoring
///
/// Returns comprehensive health including:
/// - Database connection and latency
/// - Redis connection and latency (if configured)
/// - Background job scheduler status
/// - Overall system health
pub async fn health_check_detailed(
    State(state): State<Arc<HealthState>>,
) -> Result<(StatusCode, Json<Value>), StatusCode> {
    // Check all components
    let db_health = check_database_health(&state.db).await;
    let redis_health = check_redis_health().await;

    // MVP: Scheduler health check disabled
    // let scheduler_health = if let Some(scheduler) = &state.scheduler {
    //     Some(check_scheduler_health(scheduler).await)
    // } else {
    //     None
    // };

    // Determine overall status
    let components = vec![&db_health, &redis_health];
    // MVP: No scheduler health
    // if let Some(ref sched_health) = scheduler_health {
    //     components.push(sched_health);
    // }

    let overall_status = determine_overall_status(&components);

    // Build response
    let mut response = json!({
        "status": overall_status,
        "service": "ai-coach",
        "version": env!("CARGO_PKG_VERSION"),
        "timestamp": chrono::Utc::now().to_rfc3339(),
        "components": {
            "database": db_health,
            "redis": redis_health,
        }
    });

    // MVP: No scheduler health to add
    // if let Some(sched_health) = scheduler_health {
    //     response["components"]["scheduler"] = json!(sched_health);
    // }

    // Set HTTP status code based on overall health
    let status_code = match overall_status {
        HealthStatus::Healthy => StatusCode::OK,
        HealthStatus::Degraded => StatusCode::OK, // Still operational
        HealthStatus::Unhealthy => StatusCode::SERVICE_UNAVAILABLE,
    };

    Ok((status_code, Json(response)))
}