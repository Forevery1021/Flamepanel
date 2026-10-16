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
    /// 达到该条目数后触发一次全表清理（S8：避免每次请求都 O(n) retain）。
    const CLEANUP_THRESHOLD: usize = 1024;

    /// 组合维度 key（M3）：锁定按「用户名+IP」隔离，避免攻击者用同一用户名刷爆后
    /// 将真实管理员锁定（账号 DoS）；单个 IP 的失败也只在自身维度计数。
    fn key(username: &str, ip: &str) -> String {
        format!("{}|{}", username, ip)
    }

    /// 检查是否已锁定；锁定中返回错误（返回体与密码错误不可区分，防信息泄露）
    pub async fn check_locked(
        &self,
        username: &str,
        ip: &str,
    ) -> Result<(), crate::core::error::AppError> {
        let k = Self::key(username, ip);
        let mut map = self.inner.lock().await;
        // 仅在条目达到阈值时做一次全表清理，避免每次请求都 O(n) retain（S8）。
        if map.len() >= Self::CLEANUP_THRESHOLD {
            Self::cleanup_locked_locked(&mut map);
        }
        if let Some(entry) = map.get(&k) {
            if let Some(until) = entry.locked_until {
                if until > Instant::now() {
                    return Err(crate::core::error::AppError::Forbidden(
                        "Invalid credentials".to_string(),
                    ));
                }
                // 锁定到期，清除
                map.remove(&k);
            }
        }
        Ok(())
    }

    /// 记录一次失败；达到阈值则锁定（维度：用户名+IP），使用内置缺省阈值
    pub async fn record_failure(&self, username: &str, ip: &str) {
        self.record_failure_with(username, ip, Self::MAX_FAILURES, Self::LOCK_DURATION)
            .await
    }

    /// W4-a：按面板设置（`login_max_failures` / `login_lockout_minutes`）记录失败。
    pub async fn record_failure_with(
        &self,
        username: &str,
        ip: &str,
        max_failures: u32,
        lock: Duration,
    ) {
        let k = Self::key(username, ip);
        let mut map = self.inner.lock().await;
        let entry = map.entry(k).or_default();
        entry.failures += 1;
        entry.last_attempt = Some(Instant::now());
        if entry.failures >= max_failures.max(1) {
            entry.locked_until = Some(Instant::now() + lock);
        }
    }

    /// 登录成功清除计数（维度：用户名+IP）
    pub async fn reset(&self, username: &str, ip: &str) {
        let k = Self::key(username, ip);
        let mut map = self.inner.lock().await;
        map.remove(&k);
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
            store.record_failure("bob", "1.2.3.4").await;
        }
        let err = store.check_locked("bob", "1.2.3.4").await;
        assert!(err.is_err(), "should be locked after 5 failures");
    }

    #[tokio::test]
    async fn reset_clears_lock() {
        let store = LoginAttemptStore::new();
        for _ in 0..5 {
            store.record_failure("bob", "1.2.3.4").await;
        }
        store.reset("bob", "1.2.3.4").await;
        assert!(store.check_locked("bob", "1.2.3.4").await.is_ok());
    }

    #[tokio::test]
    async fn below_threshold_allowed() {
        let store = LoginAttemptStore::new();
        store.record_failure("bob", "1.2.3.4").await;
        store.record_failure("bob", "1.2.3.4").await;
        assert!(store.check_locked("bob", "1.2.3.4").await.is_ok());
    }

    #[tokio::test]
    async fn configured_threshold_locks_earlier() {
        let s = LoginAttemptStore::new();
        for _ in 0..3 {
            s.record_failure_with("u", "1.1.1.1", 3, Duration::from_secs(60))
                .await;
        }
        assert!(s.check_locked("u", "1.1.1.1").await.is_err());
    }

    #[tokio::test]
    async fn different_ip_not_locked() {
        let store = LoginAttemptStore::new();
        for _ in 0..5 {
            store.record_failure("bob", "1.2.3.4").await;
        }
        // 不同 IP 不应被锁定（M3：组合维度隔离）
        assert!(store.check_locked("bob", "5.6.7.8").await.is_ok());
    }
}
