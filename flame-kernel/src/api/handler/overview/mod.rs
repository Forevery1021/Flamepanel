//! 顶栏概览聚合端点（S-06 · Doc/30 M-P）。
//!
//! 鉴权模型：路由本身为 auth-only（见 `middleware::is_auth_only_path`），但响应中的每个
//! 分组按调用者角色的 `node:read` / `docker:read` / `task:read` / `certificate:read`
//! 逐项裁剪——无权限的分组为 `null`，与前端 `v-permission` 口径一致。缺少角色上下文
//! 时按"全部不可见"处理（fail closed），绝不因聚合端点绕过资源级 RBAC。
use crate::api::types::{AppState, UserRole};
use crate::application::overview_service::{BadgeVisibility, OverviewBadges};
use crate::core::error::AppError;
use axum::{extract::State, routing::get, Extension, Json, Router};

/// `GET /api/overview/badges` — 顶栏徽标聚合（节点 / 容器 / 任务 / 告警一次返回）
///
/// 服务端 10 s TTL 缓存 + 单飞：多页签并发轮询只触发一次真实聚合；`cache_age_secs > 0` 表示命中缓存。
#[utoipa::path(
    get,
    path = "/api/overview/badges",
    tag = "overview",
    responses(
        (status = 200, description = "顶栏徽标聚合快照", body = OverviewBadges),
        (status = 401, description = "未认证"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn badges(
    State(state): State<AppState>,
    role: Option<Extension<UserRole>>,
) -> Result<Json<OverviewBadges>, AppError> {
    let visibility = match role {
        Some(Extension(UserRole(role))) => visibility_for_role(&state, &role).await?,
        None => BadgeVisibility::default(),
    };
    Ok(Json(state.overview_service.badges(visibility).await))
}

/// 角色 → 可见分组（每项一次角色权限集合查询，命中 `AuthCache` 不触达 DB）。
async fn visibility_for_role(state: &AppState, role: &str) -> Result<BadgeVisibility, AppError> {
    let roles = &state.role_service;
    Ok(BadgeVisibility {
        nodes: roles.check_permission(role, "node", "read").await?,
        containers: roles.check_permission(role, "docker", "read").await?,
        tasks: roles.check_permission(role, "task", "read").await?,
        certificates: roles.check_permission(role, "certificate", "read").await?,
    })
}

pub fn routes() -> Router<AppState> {
    Router::new().route("/api/overview/badges", get(badges))
}
