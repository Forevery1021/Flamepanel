//! 角色 / 权限领域服务（T8 拆分自原 `application/service.rs` 上帝文件）。
use crate::core::error::AppError;
use crate::domain::entity::*;
use crate::domain::repository::*;
use std::sync::Arc;

pub struct RoleService {
    pub role_repo: Arc<dyn RoleRepository>,
    pub perm_repo: Arc<dyn PermissionRepository>,
    /// 鉴权短缓存（Stage 2 / A4）：角色权限集合 cache-aside，写路径显式失效
    pub auth_cache: Arc<crate::utils::auth_cache::AuthCache>,
}

impl RoleService {
    pub fn new(
        role_repo: Arc<dyn RoleRepository>,
        perm_repo: Arc<dyn PermissionRepository>,
        auth_cache: Arc<crate::utils::auth_cache::AuthCache>,
    ) -> Self {
        Self {
            role_repo,
            perm_repo,
            auth_cache,
        }
    }

    /// 加载某角色的权限集合（cache-aside）。
    /// 缓存键为角色名；TTL 30s；写路径显式失效。
    async fn role_permission_set(
        &self,
        role_name: &str,
    ) -> Result<std::collections::HashSet<(String, String)>, AppError> {
        if let Some(set) = self.auth_cache.role_perms.get(role_name).await {
            return Ok(set);
        }
        let mut set = std::collections::HashSet::new();
        if let Some(role) = self.role_repo.find_by_name(role_name).await? {
            let pids = self.role_repo.get_role_permissions(role.id).await?;
            for pid in pids {
                if let Some(perm) = self.perm_repo.find_by_id(pid).await? {
                    set.insert((perm.resource, perm.action));
                }
            }
        }
        self.auth_cache
            .role_perms
            .insert(role_name.to_string(), set.clone())
            .await;
        Ok(set)
    }

    /// 使某角色的权限缓存失效（角色名）。
    async fn invalidate_role(&self, role_name: &str) {
        self.auth_cache.role_perms.invalidate(role_name).await;
    }

    pub async fn list_roles(&self) -> Result<Vec<Role>, AppError> {
        self.role_repo.list_all().await
    }

    pub async fn create_role(&self, role: &Role) -> Result<i64, AppError> {
        let id = self.role_repo.create(role).await?;
        // 新建角色可能复用旧名，失效旧缓存
        self.invalidate_role(&role.name).await;
        Ok(id)
    }

    pub async fn update_role(&self, role: &Role) -> Result<(), AppError> {
        self.role_repo.update(role).await?;
        // 角色名可能变更，新旧名均失效
        self.invalidate_role(&role.name).await;
        Ok(())
    }

    pub async fn delete_role(&self, id: i64) -> Result<(), AppError> {
        // 失效前先取角色名，删除后无法再查询
        let name = self
            .role_repo
            .find_by_id(id)
            .await?
            .map(|r| r.name)
            .unwrap_or_default();
        self.role_repo.delete(id).await?;
        self.invalidate_role(&name).await;
        Ok(())
    }

    pub async fn get_role_permissions(&self, role_id: i64) -> Result<Vec<i64>, AppError> {
        self.role_repo.get_role_permissions(role_id).await
    }

    pub async fn set_role_permissions(
        &self,
        role_id: i64,
        permission_ids: &[i64],
    ) -> Result<(), AppError> {
        // 失效前先取角色名
        let name = self
            .role_repo
            .find_by_id(role_id)
            .await?
            .map(|r| r.name)
            .unwrap_or_default();
        self.role_repo
            .set_role_permissions(role_id, permission_ids)
            .await?;
        // 权限变更后立即失效，下一次鉴权按新角色权限生效
        self.invalidate_role(&name).await;
        Ok(())
    }

    pub async fn check_permission(
        &self,
        user_role: &str,
        resource: &str,
        action: &str,
    ) -> Result<bool, AppError> {
        // Stage 2（A4）：命中角色权限集合缓存，减少鉴权路径 DB 访问
        let set = self.role_permission_set(user_role).await?;
        Ok(set.contains(&(resource.to_string(), action.to_string())))
    }
}

pub struct PermissionService {
    pub perm_repo: Arc<dyn PermissionRepository>,
}

impl PermissionService {
    pub fn new(perm_repo: Arc<dyn PermissionRepository>) -> Self {
        Self { perm_repo }
    }

    pub async fn list_permissions(&self) -> Result<Vec<Permission>, AppError> {
        self.perm_repo.list_all().await
    }
}
