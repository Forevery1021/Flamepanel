use std::time::Duration;
use crate::core::error::AppError;

#[derive(Debug, Clone)]
pub struct RetryConfig {
    pub max_retries: u32,
    pub initial_delay: Duration,
    pub max_delay: Duration,
    pub backoff_multiplier: f64,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(5),
            backoff_multiplier: 2.0,
        }
    }
}

pub async fn retry_with_backoff<F, Fut, T, E>(
    config: &RetryConfig,
    mut f: F,
) -> Result<T, AppError>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
    E: std::fmt::Display,
{
    let mut last_error = None;
    let mut delay = config.initial_delay;
    
    for attempt in 0..=config.max_retries {
        match f().await {
            Ok(result) => {
                return Ok(result);
            }
            Err(e) => {
                last_error = Some(e);
            }
        }
        
        if attempt < config.max_retries {
            tokio::time::sleep(delay).await;
            delay = std::cmp::min(
                Duration::from_secs_f64(delay.as_secs_f64() * config.backoff_multiplier),
                config.max_delay,
            );
        }
    }
    
    Err(AppError::internal(format!(
        "Operation failed after {} retries: {}",
        config.max_retries,
        last_error.map(|e: E| e.to_string()).unwrap_or_else(|| "Unknown error".into())
    )))
}

pub async fn retry_simple<F, Fut, T, E>(
    max_retries: u32,
    mut f: F,
) -> Result<T, AppError>
where
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
    E: std::fmt::Display,
{
    let config = RetryConfig {
        max_retries,
        ..Default::default()
    };
    retry_with_backoff(&config, &mut f).await
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU32, Ordering};
    use std::sync::Arc;

    #[tokio::test]
    async fn test_retry_success_on_first_attempt() {
        let config = RetryConfig::default();
        let result = retry_with_backoff(&config, || async { Ok::<i32, String>(42) }).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn test_retry_success_after_failures() {
        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = counter.clone();
        
        let config = RetryConfig {
            max_retries: 3,
            initial_delay: Duration::from_millis(10),
            ..Default::default()
        };
        
        let result = retry_with_backoff(&config, || {
            let count = counter_clone.fetch_add(1, Ordering::SeqCst);
            async move {
                if count < 2 {
                    Err::<i32, String>("Not ready".into())
                } else {
                    Ok(42)
                }
            }
        }).await;
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
        assert_eq!(counter.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn test_retry_exhausted() {
        let config = RetryConfig {
            max_retries: 2,
            initial_delay: Duration::from_millis(10),
            ..Default::default()
        };
        
        let result = retry_with_backoff(&config, || async {
            Err::<i32, String>("Permanent failure".into())
        }).await;
        
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_retry_simple() {
        let counter = Arc::new(AtomicU32::new(0));
        let counter_clone = counter.clone();
        
        let result = retry_simple(3, || {
            let count = counter_clone.fetch_add(1, Ordering::SeqCst);
            async move {
                if count < 2 {
                    Err::<i32, String>("Not ready".into())
                } else {
                    Ok(42)
                }
            }
        }).await;
        
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }
}
