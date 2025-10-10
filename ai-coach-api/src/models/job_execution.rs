use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

/// Job execution status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, sqlx::Type)]
#[sqlx(type_name = "text")]
pub enum JobExecutionStatus {
    #[serde(rename = "running")]
    #[sqlx(rename = "running")]
    Running,
    #[serde(rename = "success")]
    #[sqlx(rename = "success")]
    Success,
    #[serde(rename = "failed")]
    #[sqlx(rename = "failed")]
    Failed,
    #[serde(rename = "cancelled")]
    #[sqlx(rename = "cancelled")]
    Cancelled,
}

/// Job execution log record
#[derive(Debug, Clone, Serialize, Deserialize, FromRow)]
pub struct JobExecutionLog {
    pub id: Uuid,
    pub job_name: String,
    pub started_at: DateTime<Utc>,
    pub completed_at: Option<DateTime<Utc>>,
    pub status: JobExecutionStatus,
    pub records_processed: i32,
    pub records_failed: i32,
    pub error_message: Option<String>,
    pub execution_time_ms: Option<i64>,
    pub metadata: serde_json::Value,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

/// Request to start a job execution
#[derive(Debug, Serialize, Deserialize)]
pub struct StartJobExecutionRequest {
    pub job_name: String,
    pub metadata: Option<serde_json::Value>,
}

/// Job execution statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobExecutionStats {
    pub records_processed: i32,
    pub records_failed: i32,
    pub execution_time_ms: i64,
    pub error_message: Option<String>,
    pub metadata: Option<serde_json::Value>,
}

/// Job history filter for queries
#[derive(Debug, Serialize, Deserialize)]
pub struct JobHistoryFilter {
    pub job_name: Option<String>,
    pub status: Option<JobExecutionStatus>,
    pub from_date: Option<DateTime<Utc>>,
    pub to_date: Option<DateTime<Utc>>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

impl Default for JobHistoryFilter {
    fn default() -> Self {
        Self {
            job_name: None,
            status: None,
            from_date: None,
            to_date: None,
            page: Some(1),
            page_size: Some(50),
        }
    }
}

/// Job status summary
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobStatusSummary {
    pub job_name: String,
    pub latest_execution: Option<JobExecutionLog>,
    pub total_executions: i64,
    pub success_count: i64,
    pub failed_count: i64,
    pub average_execution_time_ms: Option<f64>,
    pub last_success_at: Option<DateTime<Utc>>,
    pub last_failure_at: Option<DateTime<Utc>>,
}

/// Job health status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum JobHealthStatus {
    #[serde(rename = "healthy")]
    Healthy,
    #[serde(rename = "degraded")]
    Degraded,
    #[serde(rename = "unhealthy")]
    Unhealthy,
}

/// Job health check result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobHealthCheck {
    pub job_name: String,
    pub status: JobHealthStatus,
    pub message: String,
    pub last_execution_at: Option<DateTime<Utc>>,
    pub consecutive_failures: i64,
    pub checked_at: DateTime<Utc>,
}

/// Overall jobs health status
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JobsHealthStatus {
    pub overall_status: JobHealthStatus,
    pub total_jobs: usize,
    pub healthy_jobs: usize,
    pub degraded_jobs: usize,
    pub unhealthy_jobs: usize,
    pub job_checks: Vec<JobHealthCheck>,
    pub checked_at: DateTime<Utc>,
}
