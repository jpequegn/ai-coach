use anyhow::Result;
use std::time::Duration;
use tokio::time::sleep;

/// Retry configuration with exponential backoff
pub struct RetryConfig {
    pub max_retries: u32,
    pub initial_delay_ms: u64,
    pub max_delay_ms: u64,
    pub backoff_factor: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay_ms: 100,
            max_delay_ms: 5000,
            backoff_factor: 2.0,
        }
    }
}

impl RetryConfig {
    /// Execute a function with retries and exponential backoff
    pub async fn execute<F, Fut, T>(&self, mut func: F) -> Result<T>
    where
        F: FnMut() -> Fut,
        Fut: std::future::Future<Output = Result<T>>,
    {
        let mut attempt = 0;
        let mut delay_ms = self.initial_delay_ms;

        loop {
            match func().await {
                Ok(result) => return Ok(result),
                Err(e) => {
                    attempt += 1;

                    if attempt >= self.max_retries {
                        tracing::warn!("Max retries ({}) exceeded", self.max_retries);
                        return Err(e);
                    }

                    tracing::debug!(
                        "Attempt {} failed, retrying in {}ms: {}",
                        attempt,
                        delay_ms,
                        e
                    );

                    sleep(Duration::from_millis(delay_ms)).await;

                    // Exponential backoff with cap
                    delay_ms = ((delay_ms as f64 * self.backoff_factor) as u64).min(self.max_delay_ms);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    #[tokio::test]
    async fn test_retry_success_on_first_try() {
        let config = RetryConfig::default();
        let attempts = Arc::new(AtomicU32::new(0));
        let attempts_clone = attempts.clone();

        let result = config
            .execute(|| {
                let attempts = attempts_clone.clone();
                async move {
                    attempts.fetch_add(1, Ordering::SeqCst);
                    Ok::<_, anyhow::Error>(42)
                }
            })
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
        assert_eq!(attempts.load(Ordering::SeqCst), 1);
    }

    #[tokio::test]
    async fn test_retry_success_after_failures() {
        let config = RetryConfig::default();
        let attempts = Arc::new(AtomicU32::new(0));
        let attempts_clone = attempts.clone();

        let result = config
            .execute(|| {
                let attempts = attempts_clone.clone();
                async move {
                    let count = attempts.fetch_add(1, Ordering::SeqCst) + 1;
                    if count < 3 {
                        Err(anyhow::anyhow!("Simulated failure"))
                    } else {
                        Ok::<_, anyhow::Error>(42)
                    }
                }
            })
            .await;

        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
        assert_eq!(attempts.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_retry_max_retries_exceeded() {
        let config = RetryConfig {
            max_retries: 2,
            ..Default::default()
        };
        let attempts = Arc::new(AtomicU32::new(0));
        let attempts_clone = attempts.clone();

        let result = config
            .execute(|| {
                let attempts = attempts_clone.clone();
                async move {
                    attempts.fetch_add(1, Ordering::SeqCst);
                    Err::<i32, _>(anyhow::anyhow!("Always fails"))
                }
            })
            .await;

        assert!(result.is_err());
        assert_eq!(attempts.load(Ordering::SeqCst), 2);
    }
}
