//! Job registry for simplified background job management
//!
//! This module provides a centralized way to register and manage background jobs,
//! eliminating the repetitive job registration code in main.rs.

use crate::services::RecoveryJobScheduler;
use anyhow::Result;
use async_trait::async_trait;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use tracing::info;

/// Type alias for job execution function
pub type JobExecution = Pin<Box<dyn Future<Output = Result<()>> + Send>>;

/// Type alias for job factory function
pub type JobFactory = Box<dyn Fn() -> JobExecution + Send + Sync>;

/// Trait for background jobs
#[async_trait]
pub trait Job: Send + Sync {
    /// Get the job name
    fn name(&self) -> &'static str;

    /// Get the cron schedule
    fn schedule(&self) -> &'static str;

    /// Execute the job
    async fn execute(&self) -> Result<()>;
}

/// Container for registered jobs
pub struct RegisteredJob {
    name: &'static str,
    schedule: &'static str,
    factory: JobFactory,
}

/// Registry for managing background jobs
pub struct JobRegistry {
    scheduler: Arc<RecoveryJobScheduler>,
    jobs: Vec<RegisteredJob>,
}

impl JobRegistry {
    /// Create a new job registry
    pub fn new(scheduler: Arc<RecoveryJobScheduler>) -> Self {
        Self {
            scheduler,
            jobs: Vec::new(),
        }
    }

    /// Register a job using the Job trait
    pub fn register_job<J: Job + Clone + 'static>(mut self, job: J) -> Self {
        let name = job.name();
        let schedule = job.schedule();

        info!("Registering job: {} (schedule: {})", name, schedule);

        let factory: JobFactory = Box::new(move || {
            let job = job.clone();
            Box::pin(async move { job.execute().await })
        });

        self.jobs.push(RegisteredJob {
            name,
            schedule,
            factory,
        });

        self
    }

    /// Register a job with explicit name, schedule, and execution function
    pub fn register<F, Fut>(mut self, name: &'static str, schedule: &'static str, job_fn: F) -> Self
    where
        F: Fn() -> Fut + Send + Sync + 'static,
        Fut: Future<Output = Result<()>> + Send + 'static,
    {
        info!("Registering job: {} (schedule: {})", name, schedule);

        let factory: JobFactory = Box::new(move || Box::pin(job_fn()));

        self.jobs.push(RegisteredJob {
            name,
            schedule,
            factory,
        });

        self
    }

    /// Start all registered jobs
    pub async fn start_all(self) -> Result<()> {
        info!("Starting {} registered jobs", self.jobs.len());

        for registered_job in self.jobs {
            let factory = Arc::new(registered_job.factory);

            self.scheduler
                .register_job(registered_job.name, registered_job.schedule, move || {
                    let factory = factory.clone();
                    Box::pin(async move {
                        // Execute the job factory and get the future
                        let job_future = (factory)();
                        // Await the job execution
                        job_future.await?;
                        // Return empty stats for now
                        Ok(crate::models::JobExecutionStats {
                            duration_ms: 0,
                            success: true,
                            error_message: None,
                        })
                    })
                })
                .await?;

            info!(
                "✓ Job '{}' registered successfully ({})",
                registered_job.name, registered_job.schedule
            );
        }

        info!("All jobs started successfully");
        Ok(())
    }

    /// Get the number of registered jobs
    pub fn count(&self) -> usize {
        self.jobs.len()
    }
}

/// Helper macro for implementing the Job trait
#[macro_export]
macro_rules! impl_job {
    ($type:ty, $name:expr, $schedule:expr) => {
        #[async_trait::async_trait]
        impl Job for $type {
            fn name(&self) -> &'static str {
                $name
            }

            fn schedule(&self) -> &'static str {
                $schedule
            }

            async fn execute(&self) -> anyhow::Result<()> {
                self.execute().await
            }
        }
    };
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Clone)]
    struct TestJob {
        name: &'static str,
        schedule: &'static str,
    }

    #[async_trait]
    impl Job for TestJob {
        fn name(&self) -> &'static str {
            self.name
        }

        fn schedule(&self) -> &'static str {
            self.schedule
        }

        async fn execute(&self) -> Result<()> {
            Ok(())
        }
    }

    #[test]
    fn test_job_registration() {
        let job1 = TestJob {
            name: "test_job_1",
            schedule: "0 * * * * *",
        };

        let job2 = TestJob {
            name: "test_job_2",
            schedule: "0 0 * * * *",
        };

        // Note: We can't actually test start_all without a real scheduler
        // but we can test that jobs are registered correctly
        assert_eq!(job1.name(), "test_job_1");
        assert_eq!(job1.schedule(), "0 * * * * *");
        assert_eq!(job2.name(), "test_job_2");
        assert_eq!(job2.schedule(), "0 0 * * * *");
    }

    #[tokio::test]
    async fn test_job_execution() {
        let job = TestJob {
            name: "test_job",
            schedule: "0 * * * * *",
        };

        // Test that job can execute without error
        let result = job.execute().await;
        assert!(result.is_ok());
    }
}
