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
    /// 最近一次失败/重置时间，用于空闲条目 TTL 清理。
    last_attempt: Option<Instant>,
}

impl LoginAttemptStore {
    pub fn new() -> Self {
        Self::default()
    }

    const MAX_FAILURES: u32 = 5;
    const LOCK_DURATION: Duration = Duration::from_secs(300); // 5 分钟
    /// 空闲条目超过该时长即视为可回收（防内存无界增长）。
    const IDLE_TTL: Duration = Duration::from_secs(3600);

    /// 检查是否已锁定；锁定中返回错误
    pub async fn check_locked(&self, username: &str) -> Result<(), crate::core::error::AppError> {
        let mut map = self.inner.lock().await;
        Self::cleanup_locked_locked(&mut map);
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
        entry.last_attempt = Some(Instant::now());
        if entry.failures >= Self::MAX_FAILURES {
            entry.locked_until = Some(Instant::now() + Self::LOCK_DURATION);
        }
    }

    /// 登录成功清除计数
    pub async fn reset(&self, username: &str) {
        let mut map = self.inner.lock().await;
        map.remove(username);
    }

    /// 在持锁状态下清理：回收所有空闲超时/锁已过期的条目。
    fn cleanup_locked_locked(map: &mut HashMap<String, LoginAttempts>) {
        let now = Instant::now();
        let idle_ttl = Self::IDLE_TTL;
        map.retain(|_k, e| {
            // 已锁定且未到期的条目保留（active 锁）
            if let Some(until) = e.locked_until {
                if until > now {
                    return true;
                }
            }
            // 空闲超过 TTL 的回收：返回 false 表示删除该条目
            !matches!(e.last_attempt, Some(t) if now.duration_since(t) > idle_ttl)
        });
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
