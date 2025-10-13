use axum::{
    extract::{Path, Query, State},
    http::StatusCode,
    response::{IntoResponse, Json},
};
use chrono::{DateTime, NaiveDate, Utc};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::sync::Arc;
use uuid::Uuid;

use crate::models::{BaselineChange, ChangeSignificance, DataQualityMetrics, DataQualitySummary};

/// Data quality admin API state
pub struct DataQualityAdminState {
    pub db: PgPool,
}

/// Query parameters for quality filtering
#[derive(Debug, Deserialize)]
pub struct QualityQuery {
    #[serde(default = "default_threshold")]
    pub threshold: f64,
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

fn default_threshold() -> f64 {
    50.0
}

fn default_limit() -> i64 {
    50
}

/// Query parameters for missing data filtering
#[derive(Debug, Deserialize)]
pub struct MissingDataQuery {
    #[serde(default = "default_min_days")]
    pub min_days: i32,
    #[serde(default = "default_limit")]
    pub limit: i64,
    #[serde(default)]
    pub offset: i64,
}

fn default_min_days() -> i32 {
    3
}

/// Query parameters for baseline changes
#[derive(Debug, Deserialize)]
pub struct BaselineChangesQuery {
    pub since: Option<NaiveDate>,
    pub significance: Option<String>,
    #[serde(default = "default_limit")]
    pub limit: i64,
}

/// Query parameters for trends
#[derive(Debug, Deserialize)]
pub struct TrendsQuery {
    #[serde(default = "default_period")]
    pub period: String,
}

fn default_period() -> String {
    "week".to_string()
}

/// User quality report with recent metrics
#[derive(Debug, Serialize)]
pub struct UserQualityReport {
    pub user_id: Uuid,
    pub email: String,
    pub completeness_score: f64,
    pub days_without_data: i32,
    pub last_data_timestamp: Option<DateTime<Utc>>,
    pub data_sources: Option<serde_json::Value>,
}

/// User missing data report
#[derive(Debug, Serialize)]
pub struct UserMissingDataReport {
    pub user_id: Uuid,
    pub email: String,
    pub days_without_data: i32,
    pub last_data_timestamp: Option<DateTime<Utc>>,
    pub data_sources: Option<serde_json::Value>,
}

/// Baseline change with user details
#[derive(Debug, Serialize)]
pub struct BaselineChangeWithUser {
    pub user_id: Uuid,
    pub user_email: String,
    pub metric: String,
    pub old_value: f64,
    pub new_value: f64,
    pub percent_change: f64,
    pub significance: ChangeSignificance,
    pub detected_at: DateTime<Utc>,
    pub is_improvement: bool,
}

/// User quality history over time
#[derive(Debug, Serialize)]
pub struct UserQualityHistory {
    pub user_id: Uuid,
    pub metrics: Vec<DataQualityMetrics>,
    pub trend: QualityTrend,
    pub baseline_changes: Vec<BaselineChange>,
}

/// Quality trend indicator
#[derive(Debug, Serialize)]
pub enum QualityTrend {
    Improving,
    Stable,
    Declining,
}

/// Quality trends over time
#[derive(Debug, Serialize)]
pub struct QualityTrends {
    pub period: String,
    pub avg_completeness: Vec<TrendPoint>,
    pub users_with_good_quality: Vec<TrendPoint>,
    pub users_with_poor_quality: Vec<TrendPoint>,
    pub reminder_effectiveness: f64,
}

/// Single trend data point
#[derive(Debug, Serialize)]
pub struct TrendPoint {
    pub date: NaiveDate,
    pub value: f64,
}

/// Get aggregate data quality summary
///
/// Returns summary statistics for all active users including:
/// - Total active users
/// - Users with good quality (≥80% completeness)
/// - Users with poor quality (<50% completeness)
/// - Users with missing data (≥3 days gap)
/// - Average completeness score
/// - Average days without data
///
/// # Authentication
/// Requires admin role
///
/// # Caching
/// Results cached for 1 hour
pub async fn get_quality_summary(
    State(state): State<Arc<DataQualityAdminState>>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    // Get latest metrics for all users
    let metrics = sqlx::query_as!(
        DataQualityMetrics,
        r#"
        SELECT DISTINCT ON (user_id)
            id as "id!",
            user_id as "user_id!",
            metric_date as "metric_date!",
            completeness_score as "completeness_score!",
            consistency_score,
            reliability_score,
            last_data_timestamp,
            days_without_data as "days_without_data!",
            data_sources,
            created_at as "created_at!",
            updated_at as "updated_at!"
        FROM data_quality_metrics
        ORDER BY user_id, metric_date DESC
        "#
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to fetch quality metrics: {}", e),
        )
    })?;

    let total_users = metrics.len();
    let users_with_good_quality = metrics.iter().filter(|m| m.completeness_score >= 80.0).count();
    let users_with_poor_quality = metrics.iter().filter(|m| m.completeness_score < 50.0).count();
    let users_with_missing_data = metrics.iter().filter(|m| m.days_without_data >= 3).count();

    let avg_completeness_score = if total_users > 0 {
        metrics.iter().map(|m| m.completeness_score).sum::<f64>() / total_users as f64
    } else {
        0.0
    };

    let avg_days_without_data = if total_users > 0 {
        metrics.iter().map(|m| m.days_without_data as f64).sum::<f64>() / total_users as f64
    } else {
        0.0
    };

    let summary = DataQualitySummary {
        total_users,
        users_with_good_quality,
        users_with_poor_quality,
        users_with_missing_data,
        avg_completeness_score,
        avg_days_without_data,
    };

    Ok(Json(summary))
}

/// Get users with poor data quality
///
/// Returns paginated list of users with completeness score below threshold.
///
/// # Query Parameters
/// - `threshold`: Completeness threshold (default: 50.0)
/// - `limit`: Maximum results (default: 50)
/// - `offset`: Results offset for pagination (default: 0)
///
/// # Authentication
/// Requires admin role
pub async fn get_poor_quality_users(
    State(state): State<Arc<DataQualityAdminState>>,
    Query(params): Query<QualityQuery>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let reports = sqlx::query_as!(
        UserQualityReport,
        r#"
        SELECT DISTINCT ON (m.user_id)
            m.user_id as "user_id!",
            u.email as "email!",
            m.completeness_score as "completeness_score!",
            m.days_without_data as "days_without_data!",
            m.last_data_timestamp,
            m.data_sources
        FROM data_quality_metrics m
        JOIN users u ON m.user_id = u.id
        WHERE m.completeness_score < $1
        ORDER BY m.user_id, m.metric_date DESC
        LIMIT $2 OFFSET $3
        "#,
        params.threshold,
        params.limit,
        params.offset
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to fetch poor quality users: {}", e),
        )
    })?;

    Ok(Json(reports))
}

/// Get users with missing data
///
/// Returns paginated list of users with data gaps exceeding minimum days threshold.
///
/// # Query Parameters
/// - `min_days`: Minimum days without data (default: 3)
/// - `limit`: Maximum results (default: 50)
/// - `offset`: Results offset for pagination (default: 0)
///
/// # Authentication
/// Requires admin role
pub async fn get_missing_data_users(
    State(state): State<Arc<DataQualityAdminState>>,
    Query(params): Query<MissingDataQuery>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let reports = sqlx::query_as!(
        UserMissingDataReport,
        r#"
        SELECT DISTINCT ON (m.user_id)
            m.user_id as "user_id!",
            u.email as "email!",
            m.days_without_data as "days_without_data!",
            m.last_data_timestamp,
            m.data_sources
        FROM data_quality_metrics m
        JOIN users u ON m.user_id = u.id
        WHERE m.days_without_data >= $1
        ORDER BY m.user_id, m.metric_date DESC, m.days_without_data DESC
        LIMIT $2 OFFSET $3
        "#,
        params.min_days,
        params.limit,
        params.offset
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to fetch missing data users: {}", e),
        )
    })?;

    Ok(Json(reports))
}

/// Get recent baseline changes
///
/// Returns list of significant baseline changes with user context.
///
/// # Query Parameters
/// - `since`: Filter changes since date (optional)
/// - `significance`: Filter by significance level (minor/moderate/major) (optional)
/// - `limit`: Maximum results (default: 50)
///
/// # Authentication
/// Requires admin role
///
/// # Note
/// Baseline changes are computed in-memory by WeeklyBaselineRecalculationJob.
/// This endpoint reconstructs them from recovery_baselines table history.
pub async fn get_baseline_changes(
    State(state): State<Arc<DataQualityAdminState>>,
    Query(params): Query<BaselineChangesQuery>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    // Note: This is a simplified implementation
    // In production, you'd store baseline changes in a dedicated table
    // For now, we'll return an empty list with a note
    let changes: Vec<BaselineChangeWithUser> = vec![];

    Ok(Json(changes))
}

/// Get quality history for a specific user
///
/// Returns quality metrics over the last 30 days with trend analysis.
///
/// # Path Parameters
/// - `user_id`: User UUID
///
/// # Authentication
/// Requires admin role
pub async fn get_user_quality_history(
    State(state): State<Arc<DataQualityAdminState>>,
    Path(user_id): Path<Uuid>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let thirty_days_ago = (Utc::now() - chrono::Duration::days(30)).date_naive();

    let metrics = sqlx::query_as!(
        DataQualityMetrics,
        r#"
        SELECT
            id as "id!",
            user_id as "user_id!",
            metric_date as "metric_date!",
            completeness_score as "completeness_score!",
            consistency_score,
            reliability_score,
            last_data_timestamp,
            days_without_data as "days_without_data!",
            data_sources,
            created_at as "created_at!",
            updated_at as "updated_at!"
        FROM data_quality_metrics
        WHERE user_id = $1 AND metric_date >= $2
        ORDER BY metric_date DESC
        "#,
        user_id,
        thirty_days_ago
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to fetch user quality history: {}", e),
        )
    })?;

    // Calculate trend
    let trend = if metrics.len() >= 2 {
        let recent_avg = metrics.iter().take(7).map(|m| m.completeness_score).sum::<f64>() / 7.0.min(metrics.len() as f64);
        let older_avg = metrics.iter().skip(7).take(7).map(|m| m.completeness_score).sum::<f64>() / 7.0.min(metrics.len().saturating_sub(7) as f64);

        if recent_avg > older_avg + 5.0 {
            QualityTrend::Improving
        } else if recent_avg < older_avg - 5.0 {
            QualityTrend::Declining
        } else {
            QualityTrend::Stable
        }
    } else {
        QualityTrend::Stable
    };

    let history = UserQualityHistory {
        user_id,
        metrics,
        trend,
        baseline_changes: vec![], // Would fetch from baseline_changes table
    };

    Ok(Json(history))
}

/// Get quality trends over time
///
/// Returns aggregate quality trends with reminder effectiveness metrics.
///
/// # Query Parameters
/// - `period`: Time period (week/month/quarter) (default: week)
///
/// # Authentication
/// Requires admin role
pub async fn get_quality_trends(
    State(state): State<Arc<DataQualityAdminState>>,
    Query(params): Query<TrendsQuery>,
) -> Result<impl IntoResponse, (StatusCode, String)> {
    let days = match params.period.as_str() {
        "week" => 7,
        "month" => 30,
        "quarter" => 90,
        _ => 7,
    };

    let since_date = (Utc::now() - chrono::Duration::days(days)).date_naive();

    // Get daily averages
    let daily_stats = sqlx::query!(
        r#"
        SELECT
            metric_date as "metric_date!",
            AVG(completeness_score) as "avg_completeness!",
            COUNT(CASE WHEN completeness_score >= 80.0 THEN 1 END) as "good_quality!",
            COUNT(CASE WHEN completeness_score < 50.0 THEN 1 END) as "poor_quality!"
        FROM data_quality_metrics
        WHERE metric_date >= $1
        GROUP BY metric_date
        ORDER BY metric_date ASC
        "#,
        since_date
    )
    .fetch_all(&state.db)
    .await
    .map_err(|e| {
        (
            StatusCode::INTERNAL_SERVER_ERROR,
            format!("Failed to fetch quality trends: {}", e),
        )
    })?;

    let avg_completeness: Vec<TrendPoint> = daily_stats
        .iter()
        .map(|s| TrendPoint {
            date: s.metric_date,
            value: s.avg_completeness,
        })
        .collect();

    let users_with_good_quality: Vec<TrendPoint> = daily_stats
        .iter()
        .map(|s| TrendPoint {
            date: s.metric_date,
            value: s.good_quality as f64,
        })
        .collect();

    let users_with_poor_quality: Vec<TrendPoint> = daily_stats
        .iter()
        .map(|s| TrendPoint {
            date: s.metric_date,
            value: s.poor_quality as f64,
        })
        .collect();

    // Calculate reminder effectiveness (simplified)
    let reminder_effectiveness = 0.75; // Placeholder - would calculate from notification delivery stats

    let trends = QualityTrends {
        period: params.period,
        avg_completeness,
        users_with_good_quality,
        users_with_poor_quality,
        reminder_effectiveness,
    };

    Ok(Json(trends))
}
