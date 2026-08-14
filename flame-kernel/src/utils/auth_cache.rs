use crate::domain::entity::User;
use moka::future::Cache;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::Duration;

/// 鉴权路径短缓存（Stage 2 / A4）
///
/// 目的：中间件每次请求都要 `find_by_id` 查用户并做 RBAC 鉴权，
/// 高频下会给仓储（尤其 SQLite）带来不必要的压力。这里用 moka 短缓存
/// 缓存热点用户与角色权限集合，显著降低鉴权路径的 DB 访问。
///
/// 设计要点：
/// - `users`：用户对象，TTL 15s，容量 10_000（对应登录用户规模）。
/// - `role_perms`：角色名 → 权限集合 `(resource, action)`，TTL 30s，容量 256。
/// - 写路径（改用户/角色/权限）显式 `invalidate`，保证无缓存导致越权/残留。
/// - 中间件不直接依赖本缓存，仅通过 Service 的 `find_by_id` / `check_permission` 间接命中。
#[derive(Clone, Debug)]
pub struct AuthCache {
    /// 用户短缓存：user_id → User
    pub users: Cache<i64, User>,
    /// 角色权限集合短缓存：role_name → {(resource, action)}
    pub role_perms: Cache<String, HashSet<(String, String)>>,
}

impl AuthCache {
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            users: Cache::builder()
                .time_to_live(Duration::from_secs(15))
                .max_capacity(10_000)
                .build(),
            role_perms: Cache::builder()
                .time_to_live(Duration::from_secs(30))
                .max_capacity(256)
                .build(),
        })
    }
}
