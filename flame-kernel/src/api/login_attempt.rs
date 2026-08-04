use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::sync::Mutex;

/// 登录失败锁定：滑动窗口计数，超过阈值锁定一段时间
#[derive(Default)]
pub struct LoginAttemptStore {
    inner: Mutex<HashMap<String, LoginAttempts>>,
}

#[derive(Default, Clone)]
struct LoginAttempts {
    failures: u32,
    locked_until: Option<Instant>,
}

impl LoginAttemptStore {
    pub fn new() -> Self {
        Self::default()
    }

    const MAX_FAILURES: u32 = 5;
    const LOCK_DURATION: Duration = Duration::from_secs(300); // 5 分钟

    /// 检查是否已锁定；锁定中返回错误
    pub async fn check_locked(&self, username: &str) -> Result<(), crate::core::error::AppError> {
        let mut map = self.inner.lock().await;
        if let Some(entry) = map.get(username) {
            if let Some(until) = entry.locked_until {
                if until > Instant::now() {
                    let remaining = until.duration_since(Instant::now()).as_secs();
                    return Err(crate::core::error::AppError::Forbidden(format!(
                        "Too many failed attempts; locked for {} more seconds",
                        remaining
                    )));
                }
                // 锁定到期，清除
                map.remove(username);
            }
        }
        Ok(())
    }

    /// 记录一次失败；达到阈值则锁定
    pub async fn record_failure(&self, username: &str) {
        let mut map = self.inner.lock().await;
        let entry = map.entry(username.to_string()).or_default();
        entry.failures += 1;
        if entry.failures >= Self::MAX_FAILURES {
            entry.locked_until = Some(Instant::now() + Self::LOCK_DURATION);
        }
    }

    /// 登录成功清除计数
    pub async fn reset(&self, username: &str) {
        let mut map = self.inner.lock().await;
        map.remove(username);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn lock_after_five_failures() {
        let store = LoginAttemptStore::new();
        for _ in 0..5 {
            store.record_failure("bob").await;
        }
        let err = store.check_locked("bob").await;
        assert!(err.is_err(), "should be locked after 5 failures");
    }

    #[tokio::test]
    async fn reset_clears_lock() {
        let store = LoginAttemptStore::new();
        for _ in 0..5 {
            store.record_failure("bob").await;
        }
        store.reset("bob").await;
        assert!(store.check_locked("bob").await.is_ok());
    }

    #[tokio::test]
    async fn below_threshold_allowed() {
        let store = LoginAttemptStore::new();
        store.record_failure("bob").await;
        store.record_failure("bob").await;
        assert!(store.check_locked("bob").await.is_ok());
    }
}
