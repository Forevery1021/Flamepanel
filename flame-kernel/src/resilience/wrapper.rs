use super::circuit_breaker::{CircuitBreaker, CircuitBreakerConfig, CircuitState};
use crate::core::error::AppError;

#[derive(Clone)]
pub struct ResilientWrapper<T: Clone> {
    inner: T,
    circuit_breaker: CircuitBreaker,
}

impl<T: Clone> ResilientWrapper<T> {
    pub fn new(inner: T, circuit_breaker_config: CircuitBreakerConfig) -> Self {
        Self {
            inner,
            circuit_breaker: CircuitBreaker::new(circuit_breaker_config),
        }
    }

    pub fn with_defaults(inner: T) -> Self {
        Self {
            inner,
            circuit_breaker: CircuitBreaker::new(CircuitBreakerConfig::default()),
        }
    }

    pub async fn call<F, Fut, R>(&self, f: F) -> Result<R, AppError>
    where
        F: Fn(T) -> Fut,
        Fut: std::future::Future<Output = Result<R, AppError>>,
    {
        let inner = self.inner.clone();
        let cb = self.circuit_breaker.clone();
        cb.call(|| f(inner)).await
    }

    pub async fn circuit_state(&self) -> CircuitState {
        self.circuit_breaker.state().await
    }

    pub async fn reset_circuit(&self) {
        self.circuit_breaker.reset().await;
    }
}

#[derive(Clone)]
pub struct ResilientRepoFactory {
    circuit_breaker_config: CircuitBreakerConfig,
}

impl ResilientRepoFactory {
    pub fn new(circuit_breaker_config: CircuitBreakerConfig) -> Self {
        Self {
            circuit_breaker_config,
        }
    }

    pub fn with_defaults() -> Self {
        Self {
            circuit_breaker_config: CircuitBreakerConfig::default(),
        }
    }

    pub fn wrap<T: Clone>(&self, inner: T) -> ResilientWrapper<T> {
        ResilientWrapper::new(inner, self.circuit_breaker_config.clone())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_resilient_wrapper_success() {
        let wrapper = ResilientWrapper::with_defaults(());

        let result = wrapper.call(|_| async { Ok::<i32, AppError>(42) }).await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), 42);
    }

    #[tokio::test]
    async fn test_resilient_wrapper_circuit_breaker() {
        let config = CircuitBreakerConfig {
            failure_threshold: 2,
            ..Default::default()
        };

        let wrapper = ResilientWrapper::new((), config);

        for _ in 0..2 {
            let _: Result<i32, AppError> = wrapper
                .call(|_| async { Err(AppError::internal("Permanent failure")) })
                .await;
        }

        assert_eq!(wrapper.circuit_state().await, CircuitState::Open);

        let result = wrapper.call(|_| async { Ok::<i32, AppError>(42) }).await;
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_resilient_repo_factory() {
        let factory = ResilientRepoFactory::with_defaults();
        let wrapper = factory.wrap(());

        let result = wrapper.call(|_| async { Ok::<i32, AppError>(42) }).await;
        assert!(result.is_ok());
    }
}
