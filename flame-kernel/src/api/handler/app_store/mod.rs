use axum::Router;
use std::collections::HashMap;

use axum::{
    extract::{Path, State},
    Json,
};
use serde::{Deserialize, Serialize};

use crate::api::extract::ApiJson;
use crate::api::types::AppState;
use crate::application::app_store_service::{
    ConflictStrategy, InstallRequest, StoreSecurityConfig, UpgradePreview,
};
use crate::core::error::AppError;
use crate::domain::entity::{AppMetadata, AppSource, InstalledApp};
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

/// 日志尾行上限（`docker logs --tail`；过大值会把整段日志读进内存）
pub const MAX_LOG_TAIL: usize = 5000;

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

/// 应用包元数据
#[utoipa::path(
    get,
    path = "/api/app-store/packages/{key}",
    tag = "app_store",
    params(("key" = String, Path, description = "应用包 key")),
    responses(
        (status = 200, description = "元数据", body = AppMetadata),
        (status = 404, description = "应用包不存在"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 app_store:read 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn get_package(
    State(state): State<AppState>,
    Path(key): Path<String>,
) -> Result<Json<AppMetadata>, AppError> {
    let meta = state.app_store_service.get_metadata(&key).await?;
    Ok(Json(meta))
}

/// 指定版本详情（compose 模板、表单字段、钩子、原生安装参数）
#[utoipa::path(
    get,
    path = "/api/app-store/packages/{key}/versions/{version}",
    tag = "app_store",
    params(("key" = String, Path, description = "应用包 key"),
        ("version" = String, Path, description = "版本号")),
    responses(
        (status = 200, description = "版本详情（AppVersionInfo）", body = serde_json::Value),
        (status = 404, description = "应用包或版本不存在"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 app_store:read 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn list_versions(
    State(state): State<AppState>,
    Path((key, version)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, AppError> {
    let info = state.app_store_service.get_version(&key, &version).await?;
    Ok(Json(
        serde_json::to_value(&info).map_err(|e| AppError::internal(e.to_string()))?,
    ))
}

/// 安装应用（容器 / 原生 / WASM；高风险或含宿主机脚本需显式确认）
#[utoipa::path(
    post,
    path = "/api/app-store/packages/{key}/install",
    tag = "app_store",
    params(("key" = String, Path, description = "应用包 key")),
    request_body = InstallAppRequest,
    responses(
        (status = 200, description = "已安装应用", body = InstalledApp),
        (status = 400, description = "package_key 与路径不一致 / 端口或变量非法 / 未确认风险或脚本 / 验签失败"),
        (status = 404, description = "应用包不存在"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 app_store:create 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn install(
    State(state): State<AppState>,
    Path(key): Path<String>,
    ApiJson(req): ApiJson<InstallAppRequest>,
) -> Result<Json<InstalledApp>, AppError> {
    // 路径 key 与请求体须一致：旧实现忽略路径段，审计日志记录的包与实际安装的包可以不同
    if req.package_key != key {
        return Err(AppError::ValidationError(format!(
            "package_key: body {:?} does not match path {:?}",
            req.package_key, key
        )));
    }
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

/// 导入服务器本地目录为应用包（Flame / 1Panel / 宝塔格式自动识别；key 须为安全路径段，不跟随符号链接）
#[utoipa::path(
    post,
    path = "/api/app-store/packages/import",
    tag = "app_store",
    request_body = ImportPackageRequest,
    responses(
        (status = 200, description = "导入的元数据", body = AppMetadata),
        (status = 400, description = "不是目录 / 格式无法识别 / key 非法 / 已存在 / 验签失败 / 源与目标目录互相包含"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 app_store:create 权限"),
    ),
    security(("BearerAuth" = []))
)]
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

/// 已安装应用详情
#[utoipa::path(
    get,
    path = "/api/app-store/installed/{id}",
    tag = "app_store",
    params(("id" = i64, Path, description = "已安装应用 ID")),
    responses(
        (status = 200, description = "详情", body = InstalledAppResponse),
        (status = 404, description = "已安装应用不存在"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 app_store:read 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn get_installed(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<InstalledAppResponse>, AppError> {
    let app = state.app_store_service.get_installed(id).await?;
    Ok(Json(InstalledAppResponse { app, logs: None }))
}

/// 记录应用启动次数（常用应用排序）
/// 记录一次启动（常用应用排序）
#[utoipa::path(
    post,
    path = "/api/app-store/installed/{id}/launch",
    tag = "app_store",
    params(("id" = i64, Path, description = "已安装应用 ID")),
    responses(
        (status = 200, description = "更新后的应用", body = InstalledApp),
        (status = 404, description = "已安装应用不存在"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 app_store:read 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn launch(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<InstalledApp>, AppError> {
    let app = state.app_store_service.record_launch(id).await?;
    Ok(Json(app))
}

/// 卸载应用（`{success: true}`）
#[utoipa::path(
    post,
    path = "/api/app-store/installed/{id}/uninstall",
    tag = "app_store",
    params(("id" = i64, Path, description = "已安装应用 ID")),
    responses(
        (status = 200, description = "已卸载", body = serde_json::Value),
        (status = 404, description = "已安装应用不存在"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 app_store:delete 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn uninstall(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, AppError> {
    state.app_store_service.uninstall(id).await?;
    Ok(Json(serde_json::json!({ "success": true })))
}

/// 升级应用（拒绝降级；同版本幂等）。容器模式对 compose 做三方合并保留用户修改，
/// 冲突时默认 409，可用 `conflict_strategy=local|upstream` 指定处理方式。
#[utoipa::path(
    post,
    path = "/api/app-store/installed/{id}/upgrade",
    tag = "app_store",
    params(("id" = i64, Path, description = "已安装应用 ID"),
        ("target_version" = Option<String>, Query, description = "目标版本，缺省 latest"),
        ("conflict_strategy" = Option<String>, Query, description = "compose 冲突处理：abort（默认，409）/ local（保留本地）/ upstream（采用新版本）")),
    responses(
        (status = 200, description = "升级后的应用", body = InstalledApp),
        (status = 400, description = "降级 / 新版本无 compose 模板 / 未知冲突策略"),
        (status = 404, description = "已安装应用不存在"),
        (status = 409, description = "本地 compose 与新版本冲突（abort 策略）"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 app_store:update 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn upgrade(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    axum::extract::Query(params): axum::extract::Query<HashMap<String, String>>,
) -> Result<Json<InstalledApp>, AppError> {
    let target = params
        .get("target_version")
        .map(|s| s.as_str())
        .unwrap_or("latest");
    let strategy = ConflictStrategy::parse(
        params
            .get("conflict_strategy")
            .map(|s| s.as_str())
            .unwrap_or(""),
    )?;
    let app = state
        .app_store_service
        .upgrade_with(id, target, strategy)
        .await?;
    Ok(Json(app))
}

/// 升级预览：渲染目标版本 compose 并与本地做三方合并，返回冲突 / 保留的本地修改 /
/// 生效的上游修改与合并结果；不改动任何文件。
#[utoipa::path(
    get,
    path = "/api/app-store/installed/{id}/upgrade-preview",
    tag = "app_store",
    params(("id" = i64, Path, description = "已安装应用 ID"),
        ("target_version" = Option<String>, Query, description = "目标版本，缺省 latest")),
    responses(
        (status = 200, description = "升级预览", body = UpgradePreview),
        (status = 400, description = "降级 / compose 不是合法 YAML"),
        (status = 404, description = "已安装应用或版本不存在"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 app_store:read 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn upgrade_preview(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    axum::extract::Query(params): axum::extract::Query<HashMap<String, String>>,
) -> Result<Json<UpgradePreview>, AppError> {
    let target = params
        .get("target_version")
        .map(|s| s.as_str())
        .unwrap_or("latest");
    Ok(Json(
        state.app_store_service.upgrade_preview(id, target).await?,
    ))
}

/// 容器日志尾部（`{logs}`；原生模式为空）
#[utoipa::path(
    get,
    path = "/api/app-store/installed/{id}/logs",
    tag = "app_store",
    params(("id" = i64, Path, description = "已安装应用 ID"),
        ("tail" = Option<usize>, Query, description = "尾行数，缺省 200，钳制到 1–5000")),
    responses(
        (status = 200, description = "日志", body = serde_json::Value),
        (status = 404, description = "已安装应用不存在"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 app_store:read 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn get_logs(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    axum::extract::Query(params): axum::extract::Query<HashMap<String, String>>,
) -> Result<Json<serde_json::Value>, AppError> {
    let tail = params
        .get("tail")
        .and_then(|s| s.parse::<usize>().ok())
        .unwrap_or(200)
        .clamp(1, MAX_LOG_TAIL);
    let logs = state.app_store_service.get_logs(id, tail).await?;
    Ok(Json(serde_json::json!({ "logs": logs })))
}

/// 原生已安装应用状态（running/stopped）
/// 运行状态（`{id, status}`，running / stopped）
#[utoipa::path(
    get,
    path = "/api/app-store/installed/{id}/native-status",
    tag = "app_store",
    params(("id" = i64, Path, description = "已安装应用 ID")),
    responses(
        (status = 200, description = "状态", body = serde_json::Value),
        (status = 404, description = "已安装应用不存在"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 app_store:read 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn native_status(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, AppError> {
    let status = state.app_store_service.native_app_status(id).await?;
    Ok(Json(serde_json::json!({ "id": id, "status": status })))
}

/// 启动原生已安装应用服务
/// 启动已安装应用服务（`{success, id, status}`）
#[utoipa::path(
    post,
    path = "/api/app-store/installed/{id}/native-start",
    tag = "app_store",
    params(("id" = i64, Path, description = "已安装应用 ID")),
    responses(
        (status = 200, description = "已执行", body = serde_json::Value),
        (status = 404, description = "已安装应用不存在"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 app_store:update 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn native_start(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, AppError> {
    state.app_store_service.native_app_start(id).await?;
    Ok(Json(
        serde_json::json!({ "success": true, "id": id, "status": "running" }),
    ))
}

/// 停止原生已安装应用服务
/// 停止已安装应用服务（`{success, id, status}`）
#[utoipa::path(
    post,
    path = "/api/app-store/installed/{id}/native-stop",
    tag = "app_store",
    params(("id" = i64, Path, description = "已安装应用 ID")),
    responses(
        (status = 200, description = "已执行", body = serde_json::Value),
        (status = 404, description = "已安装应用不存在"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 app_store:update 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn native_stop(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, AppError> {
    state.app_store_service.native_app_stop(id).await?;
    Ok(Json(
        serde_json::json!({ "success": true, "id": id, "status": "stopped" }),
    ))
}

/// 重启原生已安装应用服务
/// 重启已安装应用服务（`{success, id, status}`）
#[utoipa::path(
    post,
    path = "/api/app-store/installed/{id}/native-restart",
    tag = "app_store",
    params(("id" = i64, Path, description = "已安装应用 ID")),
    responses(
        (status = 200, description = "已执行", body = serde_json::Value),
        (status = 404, description = "已安装应用不存在"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 app_store:update 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn native_restart(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, AppError> {
    state.app_store_service.native_app_restart(id).await?;
    Ok(Json(
        serde_json::json!({ "success": true, "id": id, "status": "running" }),
    ))
}

/// 内置 WASM 应用
#[utoipa::path(
    get,
    path = "/api/app-store/wasm-builtins",
    tag = "app_store",
    responses(
        (status = 200, description = "元数据列表", body = [AppMetadata]),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 app_store:read 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn list_wasm_builtins(
    State(state): State<AppState>,
) -> Result<Json<Vec<AppMetadata>>, AppError> {
    let metas = state.app_store_service.wasm_builtins();
    Ok(Json(metas))
}

// ─── 远程应用源（M-A Issue 1.2，Doc/20 §4.1）────────────────────────────────

#[derive(Deserialize, ToSchema)]
pub struct CreateSourceRequest {
    pub name: String,
    /// git | http
    pub source_type: String,
    pub url: String,
    /// Ed25519 公钥（可选，验签用）
    pub public_key: Option<String>,
    /// 未配置公钥的源须显式确认（Doc/20 §4.2：无签名第三方源二次确认）
    #[serde(default)]
    pub confirm_unsigned: bool,
}

#[derive(Deserialize, ToSchema)]
pub struct UpdateSourceRequest {
    pub name: Option<String>,
    pub url: Option<String>,
    pub enabled: Option<bool>,
    pub public_key: Option<Option<String>>,
    /// 清空公钥（签名源降级为未签名源）须显式确认
    #[serde(default)]
    pub confirm_unsigned: bool,
}

#[derive(Serialize, ToSchema)]
pub struct SourcesResponse {
    pub sources: Vec<AppSource>,
    pub total: i64,
}

#[derive(Serialize, ToSchema)]
pub struct SyncSourceResponse {
    pub added: usize,
    pub skipped: usize,
    pub packages: Vec<String>,
}

/// 添加远程源
#[utoipa::path(
    post,
    path = "/api/app-store/sources",
    tag = "app_store",
    request_body = CreateSourceRequest,
    responses(
        (status = 200, description = "新建的远程源", body = AppSource),
        (status = 400, description = "参数非法 / url 重复"),
        (status = 401, description = "未认证"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn create_source(
    State(state): State<AppState>,
    ApiJson(req): ApiJson<CreateSourceRequest>,
) -> Result<Json<AppSource>, AppError> {
    let source = state
        .app_store_service
        .create_source(
            &req.name,
            &req.source_type,
            &req.url,
            req.public_key,
            req.confirm_unsigned,
        )
        .await?;
    Ok(Json(source))
}

/// 远程源列表
#[utoipa::path(
    get,
    path = "/api/app-store/sources",
    tag = "app_store",
    responses(
        (status = 200, description = "远程源列表", body = SourcesResponse),
        (status = 401, description = "未认证"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn list_sources(
    State(state): State<AppState>,
) -> Result<Json<SourcesResponse>, AppError> {
    let sources = state.app_store_service.list_sources().await?;
    Ok(Json(SourcesResponse {
        total: sources.len() as i64,
        sources,
    }))
}

/// 更新远程源（name / url / enabled / public_key）
#[utoipa::path(
    put,
    path = "/api/app-store/sources/{id}",
    tag = "app_store",
    request_body = UpdateSourceRequest,
    params(("id" = i64, Path, description = "源 id")),
    responses(
        (status = 200, description = "更新后的远程源", body = AppSource),
        (status = 404, description = "源不存在"),
        (status = 401, description = "未认证"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn update_source(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    ApiJson(req): ApiJson<UpdateSourceRequest>,
) -> Result<Json<AppSource>, AppError> {
    let source = state
        .app_store_service
        .update_source(
            id,
            req.name,
            req.url,
            req.enabled,
            req.public_key,
            req.confirm_unsigned,
        )
        .await?;
    Ok(Json(source))
}

/// 删除远程源
#[utoipa::path(
    delete,
    path = "/api/app-store/sources/{id}",
    tag = "app_store",
    params(("id" = i64, Path, description = "源 id")),
    responses(
        (status = 200, description = "删除成功"),
        (status = 404, description = "源不存在"),
        (status = 401, description = "未认证"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn delete_source(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, AppError> {
    state.app_store_service.delete_source(id).await?;
    Ok(Json(serde_json::json!({ "success": true })))
}

/// 手动同步远程源索引（同步任务经统一任务中心跟踪）
#[utoipa::path(
    post,
    path = "/api/app-store/sources/{id}/sync",
    tag = "app_store",
    params(("id" = i64, Path, description = "源 id")),
    responses(
        (status = 200, description = "同步结果", body = SyncSourceResponse),
        (status = 404, description = "源不存在"),
        (status = 409, description = "该源正在同步中"),
        (status = 401, description = "未认证"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn sync_source(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<SyncSourceResponse>, AppError> {
    let report = state.app_store_service.sync_source(id).await?;
    Ok(Json(SyncSourceResponse {
        added: report.added,
        skipped: report.skipped,
        packages: report.packages,
    }))
}

/// 按源过滤应用包（复用现有包列表，返回包元数据）
#[utoipa::path(
    get,
    path = "/api/app-store/sources/{id}/packages",
    tag = "app_store",
    params(("id" = i64, Path, description = "源 id")),
    responses(
        (status = 200, description = "该源下的应用包列表", body = AppStoreListResponse),
        (status = 404, description = "源不存在"),
        (status = 401, description = "未认证"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn list_source_packages(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<AppStoreListResponse>, AppError> {
    let packages = state.app_store_service.list_packages_by_source(id).await?;
    let metas: Vec<AppMetadata> = packages
        .into_iter()
        .filter_map(|p| serde_json::from_str(&p.metadata_json).ok())
        .collect();
    Ok(Json(AppStoreListResponse {
        total: metas.len() as i64,
        packages: metas,
    }))
}

// ─── 商店安全配置（Doc/20 §4.2 收口）─────────────────────────────────

#[derive(Deserialize, ToSchema)]
pub struct UpdateStoreSecurityRequest {
    /// 验签策略：optional（源配公钥才验签）| required（远程包强制验签）
    pub signature_policy: Option<String>,
    /// 镜像仓库白名单追加项（全量替换；与内置白名单合并去重）
    pub registry_whitelist: Option<Vec<String>>,
}

/// 商店安全配置（验签策略 + 镜像仓库白名单）
#[utoipa::path(
    get,
    path = "/api/app-store/security",
    tag = "app_store",
    responses(
        (status = 200, description = "商店安全配置", body = StoreSecurityConfig),
        (status = 401, description = "未认证"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn get_store_security(
    State(state): State<AppState>,
) -> Result<Json<StoreSecurityConfig>, AppError> {
    let config = state.app_store_service.security_config().await?;
    Ok(Json(config))
}

/// 更新商店安全配置（验签策略 / 镜像仓库白名单）
#[utoipa::path(
    put,
    path = "/api/app-store/security",
    tag = "app_store",
    request_body = UpdateStoreSecurityRequest,
    responses(
        (status = 200, description = "更新后的商店安全配置", body = StoreSecurityConfig),
        (status = 400, description = "无更新项 / 参数非法"),
        (status = 401, description = "未认证"),
        (status = 503, description = "设置服务未装配"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn update_store_security(
    State(state): State<AppState>,
    ApiJson(req): ApiJson<UpdateStoreSecurityRequest>,
) -> Result<Json<StoreSecurityConfig>, AppError> {
    let config = state
        .app_store_service
        .update_security_config(req.signature_policy, req.registry_whitelist)
        .await?;
    Ok(Json(config))
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
            "/api/app-store/installed/{id}/upgrade-preview",
            axum::routing::get(upgrade_preview),
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
            "/api/app-store/installed/{id}/native-status",
            axum::routing::get(native_status),
        )
        .route(
            "/api/app-store/installed/{id}/native-start",
            axum::routing::post(native_start),
        )
        .route(
            "/api/app-store/installed/{id}/native-stop",
            axum::routing::post(native_stop),
        )
        .route(
            "/api/app-store/installed/{id}/native-restart",
            axum::routing::post(native_restart),
        )
        .route(
            "/api/app-store/wasm-builtins",
            axum::routing::get(list_wasm_builtins),
        )
        // 远程应用源（M-A Issue 1.2，Doc/20 §4.1）
        .route(
            "/api/app-store/sources",
            axum::routing::get(list_sources).post(create_source),
        )
        .route(
            "/api/app-store/sources/{id}",
            axum::routing::put(update_source).delete(delete_source),
        )
        .route(
            "/api/app-store/sources/{id}/sync",
            axum::routing::post(sync_source),
        )
        .route(
            "/api/app-store/sources/{id}/packages",
            axum::routing::get(list_source_packages),
        )
        // 商店安全配置（Doc/20 §4.2 收口）
        .route(
            "/api/app-store/security",
            axum::routing::get(get_store_security).put(update_store_security),
        )
}
