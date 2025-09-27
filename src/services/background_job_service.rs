use anyhow::{Result, anyhow};
use sqlx::PgPool;
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio_cron_scheduler::{Job, JobScheduler, JobSchedulerError};
use tracing::{info, warn, error};
use uuid::Uuid;

use crate::services::{TrainingAnalysisService, TrainingSessionService};
use crate::models::{TrainingSession, UpdateTrainingSession};

#[derive(Debug, Clone)]
pub enum JobType {
    ProcessTrainingFile {
        session_id: Uuid,
        user_id: Uuid,
        file_path: String,
    },
    CalculatePMC {
        user_id: Uuid,
        days: i32,
    },
    CleanupOldFiles {
        older_than_days: i32,
    },
}

#[derive(Debug, Clone)]
pub struct BackgroundJob {
    pub id: Uuid,
    pub job_type: JobType,
    pub status: JobStatus,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub started_at: Option<chrono::DateTime<chrono::Utc>>,
    pub completed_at: Option<chrono::DateTime<chrono::Utc>>,
    pub error_message: Option<String>,
    pub retries: i32,
}

#[derive(Debug, Clone, PartialEq)]
pub enum JobStatus {
    Pending,
    Running,
    Completed,
    Failed,
    Retrying,
}

pub struct BackgroundJobService {
    scheduler: Arc<RwLock<JobScheduler>>,
    training_analysis_service: TrainingAnalysisService,
    training_session_service: TrainingSessionService,
    db: PgPool,
    jobs: Arc<RwLock<Vec<BackgroundJob>>>,
}

impl BackgroundJobService {
    pub fn new(
        db: PgPool,
        redis_url: Option<String>,
    ) -> Result<Self> {
        let scheduler = tokio::runtime::Handle::current().block_on(async {
            JobScheduler::new().await
        }).map_err(|e| anyhow!("Failed to create job scheduler: {}", e))?;

        let training_analysis_service = TrainingAnalysisService::new(db.clone(), redis_url)?;
        let training_session_service = TrainingSessionService::new(db.clone());

        Ok(Self {
            scheduler: Arc::new(RwLock::new(scheduler)),
            training_analysis_service,
            training_session_service,
            db,
            jobs: Arc::new(RwLock::new(Vec::new())),
        })
    }

    /// Start the background job scheduler
    pub async fn start(&self) -> Result<()> {
        let scheduler = self.scheduler.read().await;
        scheduler.start()
            .await
            .map_err(|e| anyhow!("Failed to start job scheduler: {}", e))?;

        info!("Background job scheduler started");

        // Add periodic cleanup job
        self.add_cleanup_job().await?;

        Ok(())
    }

    /// Stop the background job scheduler
    pub async fn stop(&self) -> Result<()> {
        let scheduler = self.scheduler.read().await;
        scheduler.shutdown()
            .await
            .map_err(|e| anyhow!("Failed to stop job scheduler: {}", e))?;

        info!("Background job scheduler stopped");
        Ok(())
    }

    /// Queue a training file processing job
    pub async fn queue_training_file_processing(
        &self,
        session_id: Uuid,
        user_id: Uuid,
        file_path: String,
    ) -> Result<Uuid> {
        let job_id = Uuid::new_v4();

        let background_job = BackgroundJob {
            id: job_id,
            job_type: JobType::ProcessTrainingFile {
                session_id,
                user_id,
                file_path: file_path.clone(),
            },
            status: JobStatus::Pending,
            created_at: chrono::Utc::now(),
            started_at: None,
            completed_at: None,
            error_message: None,
            retries: 0,
        };

        // Add job to internal tracking
        {
            let mut jobs = self.jobs.write().await;
            jobs.push(background_job);
        }

        // Create and schedule the job
        let training_analysis_service = self.training_analysis_service.clone();
        let training_session_service = self.training_session_service.clone();
        let jobs_ref = Arc::clone(&self.jobs);

        let job = Job::new_async("0 0 * * * *", move |_uuid, _l| {
            let training_analysis_service = training_analysis_service.clone();
            let training_session_service = training_session_service.clone();
            let jobs_ref = Arc::clone(&jobs_ref);
            let file_path = file_path.clone();

            Box::pin(async move {
                Self::process_training_file_job(
                    job_id,
                    session_id,
                    user_id,
                    file_path,
                    training_analysis_service,
                    training_session_service,
                    jobs_ref,
                ).await;
            })
        })
        .map_err(|e| anyhow!("Failed to create training file processing job: {}", e))?;

        let mut scheduler = self.scheduler.write().await;
        scheduler.add(job)
            .await
            .map_err(|e| anyhow!("Failed to add job to scheduler: {}", e))?;

        info!("Queued training file processing job: {} for session: {}", job_id, session_id);
        Ok(job_id)
    }

    /// Queue a PMC calculation job
    pub async fn queue_pmc_calculation(&self, user_id: Uuid, days: i32) -> Result<Uuid> {
        let job_id = Uuid::new_v4();

        let background_job = BackgroundJob {
            id: job_id,
            job_type: JobType::CalculatePMC { user_id, days },
            status: JobStatus::Pending,
            created_at: chrono::Utc::now(),
            started_at: None,
            completed_at: None,
            error_message: None,
            retries: 0,
        };

        {
            let mut jobs = self.jobs.write().await;
            jobs.push(background_job);
        }

        let training_analysis_service = self.training_analysis_service.clone();
        let jobs_ref = Arc::clone(&self.jobs);

        let job = Job::new_async("0 0 * * * *", move |_uuid, _l| {
            let training_analysis_service = training_analysis_service.clone();
            let jobs_ref = Arc::clone(&jobs_ref);

            Box::pin(async move {
                Self::calculate_pmc_job(
                    job_id,
                    user_id,
                    days,
                    training_analysis_service,
                    jobs_ref,
                ).await;
            })
        })
        .map_err(|e| anyhow!("Failed to create PMC calculation job: {}", e))?;

        let mut scheduler = self.scheduler.write().await;
        scheduler.add(job)
            .await
            .map_err(|e| anyhow!("Failed to add job to scheduler: {}", e))?;

        info!("Queued PMC calculation job: {} for user: {}", job_id, user_id);
        Ok(job_id)
    }

    /// Get job status
    pub async fn get_job_status(&self, job_id: Uuid) -> Option<BackgroundJob> {
        let jobs = self.jobs.read().await;
        jobs.iter().find(|job| job.id == job_id).cloned()
    }

    /// Get all jobs for a user
    pub async fn get_user_jobs(&self, user_id: Uuid) -> Vec<BackgroundJob> {
        let jobs = self.jobs.read().await;
        jobs.iter()
            .filter(|job| match &job.job_type {
                JobType::ProcessTrainingFile { user_id: uid, .. } => *uid == user_id,
                JobType::CalculatePMC { user_id: uid, .. } => *uid == user_id,
                JobType::CleanupOldFiles { .. } => false, // System job
            })
            .cloned()
            .collect()
    }

    /// Add periodic cleanup job
    async fn add_cleanup_job(&self) -> Result<()> {
        let jobs_ref = Arc::clone(&self.jobs);

        // Run cleanup daily at 2 AM
        let job = Job::new_async("0 0 2 * * *", move |_uuid, _l| {
            let jobs_ref = Arc::clone(&jobs_ref);

            Box::pin(async move {
                Self::cleanup_old_files_job(jobs_ref).await;
            })
        })
        .map_err(|e| anyhow!("Failed to create cleanup job: {}", e))?;

        let mut scheduler = self.scheduler.write().await;
        scheduler.add(job)
            .await
            .map_err(|e| anyhow!("Failed to add cleanup job to scheduler: {}", e))?;

        info!("Added periodic cleanup job");
        Ok(())
    }

    // Job execution methods

    async fn process_training_file_job(
        job_id: Uuid,
        session_id: Uuid,
        user_id: Uuid,
        file_path: String,
        training_analysis_service: TrainingAnalysisService,
        training_session_service: TrainingSessionService,
        jobs_ref: Arc<RwLock<Vec<BackgroundJob>>>,
    ) {
        info!("Starting training file processing job: {}", job_id);

        // Update job status to running
        Self::update_job_status(&jobs_ref, job_id, JobStatus::Running, None).await;

        // Process the file
        let result = async {
            // Read the file
            let file_data = tokio::fs::read(&file_path).await
                .map_err(|e| anyhow!("Failed to read file: {}", e))?;

            let filename = std::path::Path::new(&file_path)
                .file_name()
                .and_then(|name| name.to_str())
                .unwrap_or("unknown");

            // Process the file
            let metrics = training_analysis_service
                .process_training_file(
                    bytes::Bytes::from(file_data),
                    filename,
                    user_id,
                    None,
                )
                .await?;

            // Update the training session with metrics
            let metrics_json = serde_json::to_value(&metrics)
                .map_err(|e| anyhow!("Failed to serialize metrics: {}", e))?;

            let update_data = UpdateTrainingSession {
                date: None,
                trainrs_data: Some(metrics_json),
                uploaded_file_path: None,
                session_type: None,
                duration_seconds: metrics.duration_seconds,
                distance_meters: metrics.distance_meters,
            };

            training_session_service
                .update_session(session_id, update_data)
                .await?;

            Ok::<(), anyhow::Error>(())
        }.await;

        // Update job status based on result
        match result {
            Ok(()) => {
                Self::update_job_status(&jobs_ref, job_id, JobStatus::Completed, None).await;
                info!("Completed training file processing job: {}", job_id);
            }
            Err(e) => {
                let error_msg = format!("Training file processing failed: {}", e);
                Self::update_job_status(&jobs_ref, job_id, JobStatus::Failed, Some(error_msg.clone())).await;
                error!("Failed training file processing job {}: {}", job_id, error_msg);
            }
        }
    }

    async fn calculate_pmc_job(
        job_id: Uuid,
        user_id: Uuid,
        days: i32,
        training_analysis_service: TrainingAnalysisService,
        jobs_ref: Arc<RwLock<Vec<BackgroundJob>>>,
    ) {
        info!("Starting PMC calculation job: {}", job_id);

        Self::update_job_status(&jobs_ref, job_id, JobStatus::Running, None).await;

        let result = training_analysis_service
            .calculate_pmc(user_id, days)
            .await;

        match result {
            Ok(_) => {
                Self::update_job_status(&jobs_ref, job_id, JobStatus::Completed, None).await;
                info!("Completed PMC calculation job: {}", job_id);
            }
            Err(e) => {
                let error_msg = format!("PMC calculation failed: {}", e);
                Self::update_job_status(&jobs_ref, job_id, JobStatus::Failed, Some(error_msg.clone())).await;
                error!("Failed PMC calculation job {}: {}", job_id, error_msg);
            }
        }
    }

    async fn cleanup_old_files_job(jobs_ref: Arc<RwLock<Vec<BackgroundJob>>>) {
        info!("Starting cleanup job");

        // Clean up completed jobs older than 30 days
        let cutoff_date = chrono::Utc::now() - chrono::Duration::days(30);

        let mut jobs = jobs_ref.write().await;
        let initial_count = jobs.len();

        jobs.retain(|job| {
            job.status != JobStatus::Completed || job.created_at > cutoff_date
        });

        let cleaned_count = initial_count - jobs.len();

        if cleaned_count > 0 {
            info!("Cleaned up {} old completed jobs", cleaned_count);
        }
    }

    async fn update_job_status(
        jobs_ref: &Arc<RwLock<Vec<BackgroundJob>>>,
        job_id: Uuid,
        status: JobStatus,
        error_message: Option<String>,
    ) {
        let mut jobs = jobs_ref.write().await;

        if let Some(job) = jobs.iter_mut().find(|job| job.id == job_id) {
            job.status = status.clone();
            job.error_message = error_message;

            match status {
                JobStatus::Running => {
                    job.started_at = Some(chrono::Utc::now());
                }
                JobStatus::Completed | JobStatus::Failed => {
                    job.completed_at = Some(chrono::Utc::now());
                }
                _ => {}
            }
        }
    }
}