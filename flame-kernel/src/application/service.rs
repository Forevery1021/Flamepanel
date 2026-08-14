//! 兼容层：T8 拆分后各领域服务已移入独立文件（`user_service.rs` / `node_service.rs` /
//! `website_service.rs` / `docker_service.rs` / `role_service.rs` / `web_server_service.rs` /
//! `settings_service.rs` / `database_service.rs` / `misc_service.rs` / `firewall_service.rs`）。
//!
//! 本模块统一再导出全部服务，保持既有 `use crate::application::service::*` 兼容；
//! 同时保留跨服务分页 / 鉴权缓存 / Outbox 的集成测试。
//!
//! `FirewallManager`（OS 适配）已移至 `crate::infrastructure::firewall`。

// 以下 `use` 仅服务于本文件保留的 `#[cfg(test)]` 测试模块，故按测试编译门控，
// 避免非测试目标下产生 unused_imports 告警（clippy `-D warnings`）。
#[cfg(test)]
use crate::api::types::PaginationParams;
#[cfg(test)]
use crate::core::error::AppError;
#[cfg(test)]
use crate::domain::entity::*;
#[cfg(test)]
use crate::domain::repository::*;
#[cfg(test)]
use crate::event::EventBus;
#[cfg(test)]
use async_trait::async_trait;
#[cfg(test)]
use std::sync::Arc;

pub use crate::application::database_service::*;
pub use crate::application::docker_service::*;
pub use crate::application::firewall_service::*;
pub use crate::application::misc_service::*;
pub use crate::application::node_service::*;
pub use crate::application::role_service::*;
pub use crate::application::settings_service::*;
pub use crate::application::user_service::*;
pub use crate::application::web_server_service::*;
pub use crate::application::website_service::*;

#[cfg(test)]
mod pagination_tests {
    use super::*;
    use crate::infrastructure::db::InMemoryWebsiteRepository;
    use crate::infrastructure::db::{InMemoryNodeRepository, InMemoryUserRepository};

    fn params(page: i64, page_size: i64) -> PaginationParams {
        PaginationParams {
            page: Some(page),
            page_size: Some(page_size),
        }
    }

    #[tokio::test]
    async fn user_pagination_downshifts_to_repo() {
        let user_repo = Arc::new(InMemoryUserRepository::new());
        let svc = UserService::new(
            user_repo.clone(),
            EventBus::new(10),
            crate::utils::auth_cache::AuthCache::new(),
        );
        for i in 0..25 {
            user_repo
                .create(&format!("user{}", i), "hash", "operator")
                .await
                .unwrap();
        }
        let resp = svc.list_users_paginated(&params(2, 10)).await.unwrap();
        assert_eq!(resp.total, 25);
        assert_eq!(resp.data.len(), 10);
        assert_eq!(resp.total_pages, 3);
        // 分页下沉后 count 与列表一致
        assert_eq!(user_repo.count().await.unwrap(), 25);
        let page1 = svc.list_users_paginated(&params(1, 10)).await.unwrap();
        let page2 = svc.list_users_paginated(&params(2, 10)).await.unwrap();
        assert!(page1.data.iter().all(|u| u.id != page2.data[0].id));
    }

    #[tokio::test]
    async fn node_pagination_and_stale_heartbeat() {
        let node_repo = Arc::new(InMemoryNodeRepository::new());
        let svc = NodeService::new(node_repo.clone(), EventBus::new(10));
        for i in 0..35 {
            let node = ServerNode {
                id: 0,
                name: format!("node{}", i),
                hostname: format!("h{}", i),
                ip_address: format!("10.0.0.{}", i),
                status: "online".into(),
                created_at: chrono::Utc::now(),
                last_heartbeat_at: None,
                metrics_json: None,
                auth_token: None,
                agent_port: 8080,
            };
            node_repo.create(&node).await.unwrap();
        }
        let resp = svc.list_nodes_paginated(&params(3, 10)).await.unwrap();
        assert_eq!(resp.total, 35);
        assert_eq!(resp.data.len(), 10); // offset 20 → 10 items
                                         // 最后一页只剩 5 条
        let last = svc.list_nodes_paginated(&params(4, 10)).await.unwrap();
        assert_eq!(last.data.len(), 5);
        assert_eq!(node_repo.count().await.unwrap(), 35);

        // 离线扫描条件化：全部未心跳 → 全部 stale
        let stale = svc.list_stale_nodes(chrono::Utc::now()).await.unwrap();
        assert_eq!(stale.len(), 35);
    }

    #[tokio::test]
    async fn website_pagination_count_matches() {
        let website_repo = Arc::new(InMemoryWebsiteRepository::new());
        let svc = WebsiteService::new(website_repo.clone(), EventBus::new(10));
        for i in 0..12 {
            let ws = Website {
                id: 0,
                name: format!("site{}", i),
                domain: format!("{}.example.com", i),
                root_path: "/srv/www".into(),
                status: "running".into(),
                node_id: 1,
                engine: "nginx".into(),
                ssl_enabled: false,
                proxy_enabled: false,
                proxy_pass: None,
                created_at: chrono::Utc::now(),
                resource_version: 0,
            };
            website_repo.create(&ws).await.unwrap();
        }
        let resp = svc.list_websites_paginated(&params(1, 5)).await.unwrap();
        assert_eq!(resp.total, 12);
        assert_eq!(resp.data.len(), 5);
    }
}

#[cfg(test)]
mod auth_cache_tests {
    use super::*;
    use crate::domain::repository::UserRepository;
    use crate::infrastructure::db::{
        InMemoryPermissionRepository, InMemoryRoleRepository, InMemoryUserRepository,
    };
    use crate::utils::auth_cache::AuthCache;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    /// 记录型 UserRepository：包装 InMemory 实现，统计 find_by_id 调用次数，
    /// 用于验证鉴权短缓存命中后不再打仓储。
    struct CountingUserRepo {
        inner: InMemoryUserRepository,
        find_calls: AtomicUsize,
    }

    impl CountingUserRepo {
        fn new() -> Self {
            Self {
                inner: InMemoryUserRepository::new(),
                find_calls: AtomicUsize::new(0),
            }
        }
        fn find_calls(&self) -> usize {
            self.find_calls.load(Ordering::SeqCst)
        }
    }

    #[async_trait::async_trait]
    impl UserRepository for CountingUserRepo {
        async fn find_by_id(&self, id: i64) -> Result<Option<User>, AppError> {
            self.find_calls.fetch_add(1, Ordering::SeqCst);
            self.inner.find_by_id(id).await
        }
        async fn find_by_username(&self, username: &str) -> Result<Option<User>, AppError> {
            self.inner.find_by_username(username).await
        }
        async fn create(
            &self,
            username: &str,
            password_hash: &str,
            role: &str,
        ) -> Result<User, AppError> {
            self.inner.create(username, password_hash, role).await
        }
        async fn update(&self, user: &User) -> Result<(), AppError> {
            self.inner.update(user).await
        }
        async fn list(&self) -> Result<Vec<User>, AppError> {
            self.inner.list().await
        }
        async fn update_password(&self, id: i64, new_password_hash: &str) -> Result<(), AppError> {
            self.inner.update_password(id, new_password_hash).await
        }
        async fn delete(&self, id: i64) -> Result<(), AppError> {
            self.inner.delete(id).await
        }
        async fn list_page(&self, limit: i64, offset: i64) -> Result<Vec<User>, AppError> {
            self.inner.list_page(limit, offset).await
        }
        async fn count(&self) -> Result<i64, AppError> {
            self.inner.count().await
        }
    }

    /// 热路径命中：首次 find_by_id 打一次仓储并回填缓存，
    /// 第二次起命中缓存、不再打仓储。
    #[tokio::test]
    async fn user_find_by_id_cache_hits_after_first_lookup() {
        let repo = Arc::new(CountingUserRepo::new());
        repo.create("alice", "hash", "admin").await.unwrap();
        let svc = UserService::new(repo.clone(), EventBus::new(10), AuthCache::new());

        let first = svc.find_by_id(1).await.unwrap().expect("user exists");
        assert_eq!(first.username, "alice");
        assert_eq!(repo.find_calls(), 1);

        // 第二次命中缓存，不再访问仓储
        let second = svc.find_by_id(1).await.unwrap().expect("user exists");
        assert_eq!(second.username, "alice");
        assert_eq!(repo.find_calls(), 1, "第二次应命中缓存，不打仓储");
    }

    /// 用户写路径失效：改角色后旧缓存立即失效，下一次读取到最新数据。
    #[tokio::test]
    async fn user_update_invalidates_cache() {
        let repo = Arc::new(InMemoryUserRepository::new());
        repo.create("bob", "hash", "viewer").await.unwrap();
        let svc = UserService::new(repo.clone(), EventBus::new(10), AuthCache::new());

        let cached = svc.find_by_id(1).await.unwrap().unwrap();
        assert_eq!(cached.role, "viewer");
        // 缓存已回填
        assert!(svc.auth_cache.users.contains_key(&1));

        // 改角色为 admin
        let mut user = cached;
        user.role = "admin".into();
        svc.update_user(&user).await.unwrap();

        // 失效后读取到最新角色，而不是缓存的旧值
        let fresh = svc.find_by_id(1).await.unwrap().unwrap();
        assert_eq!(fresh.role, "admin");
    }

    /// 改密码后旧缓存失效。
    #[tokio::test]
    async fn update_password_invalidates_user_cache() {
        let repo = Arc::new(InMemoryUserRepository::new());
        repo.create("carol", "old-hash", "operator").await.unwrap();
        let svc = UserService::new(repo.clone(), EventBus::new(10), AuthCache::new());

        svc.find_by_id(1).await.unwrap();
        assert!(svc.auth_cache.users.contains_key(&1));

        svc.update_password(1, "new-hash").await.unwrap();
        assert!(!svc.auth_cache.users.contains_key(&1), "改密码后缓存应失效");
    }

    /// 角色权限集合缓存：首次 check 回填，set_role_permissions 后立即失效并按新权限生效。
    #[tokio::test]
    async fn role_permission_set_invalidates_on_change() {
        let role_repo = Arc::new(InMemoryRoleRepository::new());
        let perm_repo = Arc::new(InMemoryPermissionRepository::new());
        let svc = RoleService::new(role_repo.clone(), perm_repo.clone(), AuthCache::new());

        // admin 默认拥有全部权限，先触发缓存回填
        assert!(svc
            .check_permission("admin", "user", "create")
            .await
            .unwrap());
        assert!(svc.auth_cache.role_perms.contains_key("admin"));

        // 收回 admin 的 user:create 权限 → 立即失效并按新权限生效
        // 找到 user:create 权限 id
        let perm = perm_repo
            .find_by_resource_action("user", "create")
            .await
            .unwrap()
            .unwrap();
        // admin role id = 1；先取全部现有 pids 再移除该项
        let mut pids = role_repo.get_role_permissions(1).await.unwrap();
        pids.retain(|pid| *pid != perm.id);
        svc.set_role_permissions(1, &pids).await.unwrap();

        assert!(
            !svc.check_permission("admin", "user", "create")
                .await
                .unwrap(),
            "权限收回后应立即生效（缓存已失效）"
        );
    }

    /// 无越权：未授予的权限返回 false；授予后返回 true。
    #[tokio::test]
    async fn no_permission_escalation_with_cache() {
        let role_repo = Arc::new(InMemoryRoleRepository::new());
        let perm_repo = Arc::new(InMemoryPermissionRepository::new());
        let svc = RoleService::new(role_repo.clone(), perm_repo.clone(), AuthCache::new());

        // viewer 仅 read 权限，create 应为 false
        assert!(!svc
            .check_permission("viewer", "user", "create")
            .await
            .unwrap());
        assert!(svc
            .check_permission("viewer", "user", "read")
            .await
            .unwrap());

        // 授予 viewer user:create 后，缓存失效，立即按新权限生效
        let perm = perm_repo
            .find_by_resource_action("user", "create")
            .await
            .unwrap()
            .unwrap();
        let mut pids = role_repo.get_role_permissions(3).await.unwrap();
        pids.push(perm.id);
        svc.set_role_permissions(3, &pids).await.unwrap();

        assert!(svc
            .check_permission("viewer", "user", "create")
            .await
            .unwrap());
    }
}

#[cfg(test)]
mod outbox_tests {
    use super::*;
    use crate::infrastructure::db::InMemoryOutboxRepository;
    use std::sync::atomic::{AtomicUsize, Ordering};
    use std::sync::Arc;

    /// 记录型 OutboxRepository：首次 append 失败一次，随后成功，
    /// 用于验证 `record_event` 的失败重试语义（Stage 9：不丢关键审计）。
    struct FlakyOutboxRepo {
        inner: InMemoryOutboxRepository,
        fails_before_success: AtomicUsize,
        attempts: AtomicUsize,
    }

    impl FlakyOutboxRepo {
        fn new(failures: usize) -> Self {
            Self {
                inner: InMemoryOutboxRepository::new(),
                fails_before_success: AtomicUsize::new(failures),
                attempts: AtomicUsize::new(0),
            }
        }
        fn attempts(&self) -> usize {
            self.attempts.load(Ordering::SeqCst)
        }
    }

    #[async_trait]
    impl OutboxRepository for FlakyOutboxRepo {
        async fn append(
            &self,
            event_type: &str,
            payload: &str,
            published: bool,
        ) -> Result<OutboxEvent, AppError> {
            self.attempts.fetch_add(1, Ordering::SeqCst);
            let remaining = self.fails_before_success.load(Ordering::SeqCst);
            if remaining > 0 {
                self.fails_before_success
                    .store(remaining - 1, Ordering::SeqCst);
                return Err(AppError::internal("transient db failure"));
            }
            self.inner.append(event_type, payload, published).await
        }
        async fn list_page(
            &self,
            limit: i64,
            offset: i64,
            event_type: Option<&str>,
        ) -> Result<Vec<OutboxEvent>, AppError> {
            self.inner.list_page(limit, offset, event_type).await
        }
        async fn count(&self, event_type: Option<&str>) -> Result<i64, AppError> {
            self.inner.count(event_type).await
        }
    }

    /// 失败一次后重试成功，事件不丢失（审计不丢）。
    #[tokio::test]
    async fn record_event_retries_transient_failure() {
        let repo = Arc::new(FlakyOutboxRepo::new(1));
        let svc = OutboxService::new(repo.clone());
        let event = DomainEvent::BackupCreated {
            filename: "test.tar.gz".into(),
        };

        let outbox_event = svc
            .record_event(&event)
            .await
            .expect("retry should succeed");
        assert_eq!(outbox_event.event_type, "BackupCreated");
        assert!(outbox_event.published);
        // 首次失败 + 重试成功 = 至少 2 次尝试
        assert!(
            repo.attempts() >= 2,
            "expected retry, got {}",
            repo.attempts()
        );
        // 事件确已持久化（未丢失）
        let total = svc
            .list_paginated(
                &PaginationParams {
                    page: None,
                    page_size: None,
                },
                Some("BackupCreated"),
            )
            .await
            .unwrap()
            .total;
        assert_eq!(total, 1);
    }

    /// 连续失败超过最大重试次数后返回错误（不无限重试）。
    #[tokio::test]
    async fn record_event_gives_up_after_max_retries() {
        let repo = Arc::new(FlakyOutboxRepo::new(10)); // 永远失败
        let svc = OutboxService::new(repo.clone());
        let event = DomainEvent::NodeOffline {
            node_id: 1,
            node_name: "n".into(),
        };
        let err = svc.record_event(&event).await.expect_err("should fail");
        assert!(err.to_string().contains("transient"));
        // 最多重试 3 次（首次 + 2 次重试）
        assert_eq!(repo.attempts(), 3);
    }
}
