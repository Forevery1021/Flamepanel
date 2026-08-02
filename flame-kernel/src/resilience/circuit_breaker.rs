use std::sync::Arc;
use tokio::sync::Mutex;
use std::time::{Duration, Instant};
use crate::core::error::AppError;

#[derive(Debug, Clone, PartialEq)]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

#[derive(Debug, Clone)]
pub struct CircuitBreakerConfig {
    pub failure_threshold: u32,
    pub success_threshold: u32,
    pub timeout: Duration,
    pub half_open_max_calls: u32,
}

impl Default for CircuitBreakerConfig {
    fn default() -> Self {
        Self {
            failure_threshold: 5,
            success_threshold: 3,
            timeout: Duration::from_secs(30),
            half_open_max_calls: 3,
        }
    }
}

struct CircuitBreakerInner {
    state: CircuitState,
    failure_count: u32,
    success_count: u32,
    last_failure_time: Option<Instant>,
    half_open_calls: u32,
    config: CircuitBreakerConfig,
}

#[derive(Clone)]
pub struct CircuitBreaker {
    inner: Arc<Mutex<CircuitBreakerInner>>,
}

impl CircuitBreaker {
    pub fn new(config: CircuitBreakerConfig) -> Self {
        Self {
            inner: Arc::new(Mutex::new(CircuitBreakerInner {
                state: CircuitState::Closed,
                failure_count: 0,
                success_count: 0,
                last_failure_time: None,
                half_open_calls: 0,
                config,
            })),
        }
    }

    pub async fn call<F, Fut, T, E>(&self, f: F) -> Result<T, AppError>
    where
        F: FnOnce() -> Fut,
        Fut: std::future::Future<Output = Result<T, E>>,
        E: std::fmt::Display,
    {
        let mut inner = self.inner.lock().await;
        
        match inner.state {
            CircuitState::Open => {
                if let Some(last_failure) = inner.last_failure_time {
                    if last_failure.elapsed() >= inner.config.timeout {
                        inner.state = CircuitState::HalfOpen;
                        inner.half_open_calls = 0;
                        inner.success_count = 0;
                    } else {
                        return Err(AppError::internal("Circuit breaker is open"));
                    }
                } else {
                    return Err(AppError::internal("Circuit breaker is open"));
                }
            }
            CircuitState::HalfOpen => {
                if inner.half_open_calls >= inner.config.half_open_max_calls {
                    return Err(AppError::internal("Circuit breaker is open"));
                }
                inner.half_open_calls += 1;
            }
            CircuitState::Closed => {}
        }
        
        drop(inner);
        
        let result = f().await;
        
        let mut inner = self.inner.lock().await;
        match result {
            Ok(_) => {
                match inner.state {
                    CircuitState::HalfOpen => {
                        inner.success_count += 1;
                        if inner.success_count >= inner.config.success_threshold {
                            inner.state = CircuitState::Closed;
                            inner.failure_count = 0;
                            inner.success_count = 0;
                        }
                    }
                    CircuitState::Closed => {
                        inner.failure_count = 0;
                    }
                    _ => {}
                }
                result.map_err(|e| AppError::internal(e.to_string()))
            }
            Err(e) => {
                inner.failure_count += 1;
                inner.last_failure_time = Some(Instant::now());
                
                if inner.failure_count >= inner.config.failure_threshold {
                    inner.state = CircuitState::Open;
                }
                
                Err(AppError::internal(e.to_string()))
            }
        }
    }

    pub async fn state(&self) -> CircuitState {
        self.inner.lock().await.state.clone()
    }

    pub async fn reset(&self) {
        let mut inner = self.inner.lock().await;
        inner.state = CircuitState::Closed;
        inner.failure_count = 0;
        inner.success_count = 0;
        inner.last_failure_time = None;
        inner.half_open_calls = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;


    #[tokio::test]
    async fn test_circuit_breaker_closed_state() {
        let cb = CircuitBreaker::new(CircuitBreakerConfig::default());
        assert_eq!(cb.state().await, CircuitState::Closed);
        
        let result = cb.call(|| async { Ok::<i32, String>(42) }).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn test_circuit_breaker_opens_on_failures() {
        let config = CircuitBreakerConfig {
            failure_threshold: 3,
            ..Default::default()
        };
        let cb = CircuitBreaker::new(config);
        
        for _ in 0..3 {
            let _ = cb.call(|| async { Err::<i32, String>("error".into()) }).await;
        }
        
        assert_eq!(cb.state().await, CircuitState::Open);
        
        let result = cb.call(|| async { Ok::<i32, String>(42) }).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_circuit_breaker_half_open_recovery() {
        let config = CircuitBreakerConfig {
            failure_threshold: 2,
            success_threshold: 2,
            timeout: Duration::from_millis(50),
            half_open_max_calls: 3,
        };
        let cb = CircuitBreaker::new(config);
        
        for _ in 0..2 {
            let _ = cb.call(|| async { Err::<i32, String>("error".into()) }).await;
        }
        assert_eq!(cb.state().await, CircuitState::Open);
        
        tokio::time::sleep(Duration::from_millis(100)).await;
        
        for _ in 0..2 {
            let _ = cb.call(|| async { Ok::<i32, String>(42) }).await;
        }
        assert_eq!(cb.state().await, CircuitState::Closed);
    }

    #[tokio::test]
    async fn test_circuit_breaker_reset() {
        let cb = CircuitBreaker::new(CircuitBreakerConfig::default());
        
        for _ in 0..5 {
            let _ = cb.call(|| async { Err::<i32, String>("error".into()) }).await;
        }
        assert_eq!(cb.state().await, CircuitState::Open);
        
        cb.reset().await;
        assert_eq!(cb.state().await, CircuitState::Closed);
    }
}
