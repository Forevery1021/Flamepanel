use axum::Router;
use std::collections::HashMap;

use axum::{
    extract::{Path, State},
    Json,
};
use serde::{Deserialize, Serialize};

use crate::api::extract::ApiJson;
use crate::api::types::AppState;
use crate::application::app_store_service::InstallRequest;
use crate::core::error::AppError;
use crate::domain::entity::{AppMetadata, InstalledApp};
use utoipa::ToSchema;

#[derive(Serialize, ToSchema)]
pub struct AppStoreListResponse {
    pub packages: Vec<AppMetadata>,
    pub total: i64,
}

#[derive(Deserialize, ToSchema)]
pub struct InstallAppRequest {
    pub package_key: String,
    pub version: Option<String>,
    pub mode: Option<String>,
    pub name: Option<String>,
    pub port: Option<i32>,
    pub container_name: Option<String>,
    pub values: Option<HashMap<String, String>>,
    #[serde(default)]
    pub confirm_risky: bool,
    /// 安装包含原生脚本时需显式确认执行第三方脚本。
    #[serde(default)]
    pub acknowledge_scripts: bool,
}

#[derive(Deserialize, ToSchema)]
pub struct ImportPackageRequest {
    pub path: String,
}

#[derive(Deserialize, ToSchema)]
pub struct BatchImportPackagesRequest {
    pub paths: Vec<String>,
}

#[derive(Serialize, ToSchema)]
pub struct BatchImportPackagesResponse {
    pub imported: Vec<AppMetadata>,
    pub count: usize,
}

#[derive(Serialize, ToSchema)]
pub struct InstalledAppResponse {
    pub app: InstalledApp,
    pub logs: Option<String>,
}

/// 应用商店包列表
#[utoipa::path(
    get,
    path = "/api/app-store/packages",
    tag = "app_store",
    params(("category" = Option<String>, Query, description = "分类过滤")),
    responses(
        (status = 200, description = "包列表", body = AppStoreListResponse),
        (status = 401, description = "未认证"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn list_packages(
    State(state): State<AppState>,
    axum::extract::Query(params): axum::extract::Query<HashMap<String, String>>,
) -> Result<Json<AppStoreListResponse>, AppError> {
    let category = params.get("category").map(|s| s.as_str());
    let packages = state.app_store_service.list_packages(category).await?;
    Ok(Json(AppStoreListResponse {
        total: packages.len() as i64,
        packages,
    }))
}

pub async fn get_package(
    State(state): State<AppState>,
    Path(key): Path<String>,
) -> Result<Json<AppMetadata>, AppError> {
    let meta = state.app_store_service.get_metadata(&key).await?;
    Ok(Json(meta))
}

pub async fn list_versions(
    State(state): State<AppState>,
    Path((key, version)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, AppError> {
    let info = state.app_store_service.get_version(&key, &version).await?;
    Ok(Json(
        serde_json::to_value(&info).map_err(|e| AppError::internal(e.to_string()))?,
    ))
}

pub async fn install(
    State(state): State<AppState>,
    ApiJson(req): ApiJson<InstallAppRequest>,
) -> Result<Json<InstalledApp>, AppError> {
    let values = req.values.unwrap_or_default();
    let request = InstallRequest {
        package_key: req.package_key,
        version: req.version,
        mode: req.mode,
        name: req.name,
        port: req.port,
        container_name: req.container_name,
        values,
        confirm_risky: req.confirm_risky,
        acknowledge_scripts: req.acknowledge_scripts,
    };
    let app = state.app_store_service.install(&request).await?;
    Ok(Json(app))
}

pub async fn import_package(
    State(state): State<AppState>,
    ApiJson(req): ApiJson<ImportPackageRequest>,
) -> Result<Json<AppMetadata>, AppError> {
    let meta = state.app_store_service.import_package(&req.path).await?;
    Ok(Json(meta))
}

/// 批量导入本地目录应用包（Phase A2：`create_many` 事务语义，全成或全败）
#[utoipa::path(
    post,
    path = "/api/app-store/packages/batch-import",
    tag = "app_store",
    request_body = BatchImportPackagesRequest,
    responses(
        (status = 200, description = "批量导入结果", body = BatchImportPackagesResponse),
        (status = 400, description = "非法路径 / 空列表 / 已存在"),
        (status = 401, description = "未认证"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn batch_import_packages(
    State(state): State<AppState>,
    ApiJson(req): ApiJson<BatchImportPackagesRequest>,
) -> Result<Json<BatchImportPackagesResponse>, AppError> {
    let imported = state
        .app_store_service
        .batch_import_packages(&req.paths)
        .await?;
    let count = imported.len();
    Ok(Json(BatchImportPackagesResponse { imported, count }))
}

/// 已安装应用列表
#[utoipa::path(
    get,
    path = "/api/app-store/installed",
    tag = "app_store",
    responses(
        (status = 200, description = "已安装应用列表", body = Vec<InstalledApp>),
        (status = 401, description = "未认证"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn list_installed(
    State(state): State<AppState>,
) -> Result<Json<Vec<InstalledApp>>, AppError> {
    let apps = state.app_store_service.list_installed().await?;
    Ok(Json(apps))
}

pub async fn get_installed(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<InstalledAppResponse>, AppError> {
    let app = state.app_store_service.get_installed(id).await?;
    Ok(Json(InstalledAppResponse { app, logs: None }))
}

/// 记录应用启动次数（常用应用排序）
pub async fn launch(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<InstalledApp>, AppError> {
    let app = state.app_store_service.record_launch(id).await?;
    Ok(Json(app))
}

pub async fn uninstall(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, AppError> {
    state.app_store_service.uninstall(id).await?;
    Ok(Json(serde_json::json!({ "success": true })))
}

pub async fn upgrade(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    axum::extract::Query(params): axum::extract::Query<HashMap<String, String>>,
) -> Result<Json<InstalledApp>, AppError> {
    let target = params
        .get("target_version")
        .map(|s| s.as_str())
        .unwrap_or("latest");
    let app = state.app_store_service.upgrade(id, target).await?;
    Ok(Json(app))
}

pub async fn get_logs(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    axum::extract::Query(params): axum::extract::Query<HashMap<String, String>>,
) -> Result<Json<serde_json::Value>, AppError> {
    let tail = params
        .get("tail")
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(200);
    let logs = state.app_store_service.get_logs(id, tail).await?;
    Ok(Json(serde_json::json!({ "logs": logs })))
}

pub async fn list_wasm_builtins(
    State(state): State<AppState>,
) -> Result<Json<Vec<AppMetadata>>, AppError> {
    let metas = state.app_store_service.wasm_builtins();
    Ok(Json(metas))
}

/// 路由表（集中注册于 routes.rs 组合根）
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/app-store/packages", axum::routing::get(list_packages))
        .route(
            "/api/app-store/packages/import",
            axum::routing::post(import_package),
        )
        .route(
            "/api/app-store/packages/batch-import",
            axum::routing::post(batch_import_packages),
        )
        .route(
            "/api/app-store/packages/{key}",
            axum::routing::get(get_package),
        )
        .route(
            "/api/app-store/packages/{key}/versions/{version}",
            axum::routing::get(list_versions),
        )
        .route(
            "/api/app-store/packages/{key}/install",
            axum::routing::post(install),
        )
        .route(
            "/api/app-store/installed",
            axum::routing::get(list_installed),
        )
        .route(
            "/api/app-store/installed/{id}",
            axum::routing::get(get_installed),
        )
        .route(
            "/api/app-store/installed/{id}/upgrade",
            axum::routing::post(upgrade),
        )
        .route(
            "/api/app-store/installed/{id}/uninstall",
            axum::routing::post(uninstall),
        )
        .route(
            "/api/app-store/installed/{id}/launch",
            axum::routing::post(launch),
        )
        .route(
            "/api/app-store/installed/{id}/logs",
            axum::routing::get(get_logs),
        )
        .route(
            "/api/app-store/wasm-builtins",
            axum::routing::get(list_wasm_builtins),
        )
}
