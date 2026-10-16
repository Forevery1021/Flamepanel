//! 只读 IaC 导出：入口 settings:read，每个区块额外检查对应资源的 read 权限。
use axum::{
    extract::State,
    http::header,
    response::{IntoResponse, Response},
    routing::get,
    Extension, Router,
};

use crate::api::types::{AppState, UserRole};
use crate::application::iac_service::{IacManifest, IacVisibility};
use crate::core::error::AppError;

#[utoipa::path(
    get,
    path = "/api/iac/export",
    operation_id = "exportIacConfiguration",
    tag = "iac",
    responses(
        (status = 200, description = "无凭据配置清单；无读权限的资源区块省略，不是可直接执行的部署声明", body = IacManifest, content_type = "application/yaml"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 settings:read"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn export(
    State(state): State<AppState>,
    role: Option<Extension<UserRole>>,
) -> Result<Response, AppError> {
    let visibility = match role {
        Some(Extension(UserRole(role))) => IacVisibility {
            websites: state
                .role_service
                .check_permission(&role, "website", "read")
                .await?,
            databases: state
                .role_service
                .check_permission(&role, "database", "read")
                .await?,
            applications: state
                .role_service
                .check_permission(&role, "app_store", "read")
                .await?,
        },
        None => IacVisibility::default(),
    };
    let manifest = state.iac_service.export(visibility).await?;
    let yaml = serde_yaml::to_string(&manifest).map_err(|error| {
        AppError::internal_with_source("Cannot serialize configuration export", error)
    })?;
    Ok((
        [
            (header::CONTENT_TYPE, "application/yaml; charset=utf-8"),
            (
                header::CONTENT_DISPOSITION,
                "attachment; filename=\"flamepanel-config.yaml\"",
            ),
            (header::CACHE_CONTROL, "no-store"),
        ],
        yaml,
    )
        .into_response())
}

pub fn routes() -> Router<AppState> {
    Router::new().route("/api/iac/export", get(export))
}
