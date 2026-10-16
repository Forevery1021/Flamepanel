use crate::api::extract::ApiJson;
use crate::api::types::{
    AppState, CreateWebServerInstanceRequest, PaginatedResponse, PaginationParams,
    WebServerResponse,
};
use crate::core::error::AppError;
use crate::domain::entity::WebServerInstance;
use crate::webserver::engine::WebServerEngine;
use crate::webserver::native::NativeWebServerInfo;
use axum::Router;
use axum::{
    extract::{Path, Query, State},
    Json,
};
use chrono::Utc;
use serde::Serialize;

#[derive(Serialize, utoipa::ToSchema)]
pub struct EngineInfo {
    pub name: String,
    pub description: String,
    pub default_port: u16,
    pub default_ssl_port: u16,
    pub supports_ssl: bool,
    pub supports_rewrite: bool,
    pub supports_reverse_proxy: bool,
    pub supports_load_balancing: bool,
}

/// 支持的 Web 引擎及能力
#[utoipa::path(
    get,
    path = "/api/web-servers/engines",
    tag = "web-servers",
    responses(
        (status = 200, description = "引擎列表", body = [EngineInfo]),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 web_server:read 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn list_engines() -> Json<Vec<EngineInfo>> {
    let engines = vec![
        WebServerEngine::Nginx,
        WebServerEngine::Apache,
        WebServerEngine::OpenLiteSpeed,
        WebServerEngine::OpenResty,
        WebServerEngine::Caddy,
    ];
    Json(
        engines
            .into_iter()
            .map(|e| EngineInfo {
                name: e.as_str().to_string(),
                description: e.description().to_string(),
                default_port: e.default_port(),
                default_ssl_port: e.default_ssl_port(),
                supports_ssl: e.supports_ssl(),
                supports_rewrite: e.supports_rewrite(),
                supports_reverse_proxy: e.supports_reverse_proxy(),
                supports_load_balancing: e.supports_load_balancing(),
            })
            .collect(),
    )
}

/// 面板登记的 Web 服务器实例（分页）
#[utoipa::path(
    get,
    path = "/api/web-servers",
    tag = "web-servers",
    params(PaginationParams),
    responses(
        (status = 200, description = "实例分页", body = PaginatedResponse<WebServerResponse>),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 web_server:read 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn list(
    State(state): State<AppState>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<PaginatedResponse<WebServerResponse>>, AppError> {
    let result = state
        .web_server_service
        .list_servers_paginated(&params)
        .await?;
    let data = result.data.into_iter().map(to_response).collect();
    Ok(Json(PaginatedResponse::new(data, result.total, &params)))
}

/// 单个实例
#[utoipa::path(
    get,
    path = "/api/web-servers/{id}",
    tag = "web-servers",
    params(("id" = i64, Path, description = "实例 ID")),
    responses(
        (status = 200, description = "实例", body = WebServerResponse),
        (status = 404, description = "实例不存在"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 web_server:read 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn get(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<WebServerResponse>, AppError> {
    let server = state.web_server_service.get_server(id).await?;
    Ok(Json(to_response(server)))
}

/// 登记实例（不安装软件；安装见 native/install）
#[utoipa::path(
    post,
    path = "/api/web-servers",
    tag = "web-servers",
    request_body = CreateWebServerInstanceRequest,
    responses(
        (status = 200, description = "已登记", body = WebServerResponse),
        (status = 400, description = "未知引擎 / config_path 不在引擎配置根下 / binary_path 不是该引擎的已知二进制"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 web_server:create 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn create(
    State(state): State<AppState>,
    ApiJson(req): ApiJson<CreateWebServerInstanceRequest>,
) -> Result<Json<WebServerResponse>, AppError> {
    let engine = WebServerEngine::from_name(&req.engine)
        .ok_or_else(|| AppError::BadRequest(format!("Unknown engine: {}", req.engine)))?;

    let instance = WebServerInstance {
        id: 0,
        engine: engine.as_str().to_string(),
        version: req.version.clone(),
        status: "stopped".to_string(),
        config_path: req
            .config_path
            .unwrap_or_else(|| engine.default_config_path().to_string()),
        binary_path: req.binary_path.clone(),
        port: req.port.unwrap_or(engine.default_port() as i32),
        created_at: Utc::now(),
        resource_version: 0,
    };
    let id = state.web_server_service.create_server(&instance).await?;
    let created = state.web_server_service.get_server(id).await?;
    Ok(Json(to_response(created)))
}

/// 更新实例字段（`engine` / `version` / `config_path` / `binary_path` / `port`，可带 `resource_version` 做乐观并发）
#[utoipa::path(
    put,
    path = "/api/web-servers/{id}",
    tag = "web-servers",
    params(("id" = i64, Path, description = "实例 ID")),
    request_body = Object,
    responses(
        (status = 200, description = "已更新", body = WebServerResponse),
        (status = 400, description = "未知引擎 / config_path 不在引擎配置根下 / binary_path 不是该引擎的已知二进制"),
        (status = 404, description = "实例不存在"),
        (status = 409, description = "resource_version 冲突"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 web_server:update 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    ApiJson(req): ApiJson<serde_json::Value>,
) -> Result<Json<WebServerResponse>, AppError> {
    let mut server = state.web_server_service.get_server(id).await?;
    // 乐观并发控制：请求携带的 resource_version 作为客户端基准版本；
    // 未携带时使用当前版本（保持向后兼容），冲突返回稳定 `CONFLICT`。
    if let Some(version) = req.get("resource_version").and_then(|v| v.as_i64()) {
        server.resource_version = version;
    }
    if let Some(engine) = req.get("engine").and_then(|v| v.as_str()) {
        server.engine = engine.to_string();
    }
    if let Some(version) = req.get("version").and_then(|v| v.as_str()) {
        server.version = Some(version.to_string());
    }
    if let Some(config_path) = req.get("config_path").and_then(|v| v.as_str()) {
        server.config_path = config_path.to_string();
    }
    if let Some(binary_path) = req.get("binary_path").and_then(|v| v.as_str()) {
        server.binary_path = Some(binary_path.to_string());
    }
    if let Some(port) = req.get("port").and_then(|v| v.as_i64()) {
        server.port = port as i32;
    }
    state.web_server_service.update_server(&server).await?;
    let updated = state.web_server_service.get_server(id).await?;
    Ok(Json(to_response(updated)))
}

/// 删除实例登记（不卸载软件）
#[utoipa::path(
    delete,
    path = "/api/web-servers/{id}",
    tag = "web-servers",
    params(("id" = i64, Path, description = "实例 ID")),
    responses(
        (status = 200, description = "`{deleted: true, id}`", body = Object),
        (status = 404, description = "实例不存在"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 web_server:delete 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn delete(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, AppError> {
    state.web_server_service.delete_server(id).await?;
    Ok(Json(serde_json::json!({"deleted": true, "id": id})))
}

/// 启动引擎
#[utoipa::path(
    post,
    path = "/api/web-servers/{id}/start",
    tag = "web-servers",
    params(("id" = i64, Path, description = "实例 ID")),
    responses(
        (status = 200, description = "`{message, id}`", body = Object),
        (status = 404, description = "实例不存在"),
        (status = 403, description = "config_path 不在引擎配置根下（存量实例）"),
        (status = 500, description = "引擎命令执行失败"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 web_server:start 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn start(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, AppError> {
    let msg = state.web_server_service.start_server(id).await?;
    Ok(Json(serde_json::json!({"message": msg, "id": id})))
}

/// 停止引擎
#[utoipa::path(
    post,
    path = "/api/web-servers/{id}/stop",
    tag = "web-servers",
    params(("id" = i64, Path, description = "实例 ID")),
    responses(
        (status = 200, description = "`{message, id}`", body = Object),
        (status = 404, description = "实例不存在"),
        (status = 403, description = "config_path 不在引擎配置根下（存量实例）"),
        (status = 500, description = "引擎命令执行失败"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 web_server:stop 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn stop(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, AppError> {
    let msg = state.web_server_service.stop_server(id).await?;
    Ok(Json(serde_json::json!({"message": msg, "id": id})))
}

/// 重启引擎
#[utoipa::path(
    post,
    path = "/api/web-servers/{id}/restart",
    tag = "web-servers",
    params(("id" = i64, Path, description = "实例 ID")),
    responses(
        (status = 200, description = "`{message, id}`", body = Object),
        (status = 404, description = "实例不存在"),
        (status = 403, description = "config_path 不在引擎配置根下（存量实例）"),
        (status = 500, description = "引擎命令执行失败"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 web_server:start 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn restart(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, AppError> {
    let msg = state.web_server_service.restart_server(id).await?;
    Ok(Json(serde_json::json!({"message": msg, "id": id})))
}

/// 平滑重载配置
#[utoipa::path(
    post,
    path = "/api/web-servers/{id}/reload",
    tag = "web-servers",
    params(("id" = i64, Path, description = "实例 ID")),
    responses(
        (status = 200, description = "`{message, id}`", body = Object),
        (status = 404, description = "实例不存在"),
        (status = 403, description = "config_path 不在引擎配置根下（存量实例）"),
        (status = 500, description = "引擎命令执行失败"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 web_server:reload 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn reload(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, AppError> {
    let msg = state.web_server_service.reload_server(id).await?;
    Ok(Json(serde_json::json!({"message": msg, "id": id})))
}

/// 校验引擎配置（`nginx -t` 等）
#[utoipa::path(
    post,
    path = "/api/web-servers/{id}/configtest",
    tag = "web-servers",
    params(("id" = i64, Path, description = "实例 ID")),
    responses(
        (status = 200, description = "`{output, id}`", body = Object),
        (status = 404, description = "实例不存在"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 web_server:configtest 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn config_test(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, AppError> {
    let output = state.web_server_service.test_server_config(id).await?;
    Ok(Json(serde_json::json!({"output": output, "id": id})))
}

/// 读取实例主配置文件（仅限引擎配置根下；不存在时 content 为提示文本）
#[utoipa::path(
    get,
    path = "/api/web-servers/{id}/config",
    tag = "web-servers",
    params(("id" = i64, Path, description = "实例 ID")),
    responses(
        (status = 200, description = "`{id, config_path, content}`", body = Object),
        (status = 404, description = "实例不存在"),
        (status = 403, description = "config_path 不在引擎配置根下（存量实例）"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 web_server:read 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn get_config(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, AppError> {
    let server = state.web_server_service.get_server(id).await?;
    // 只读引擎配置根下的文件（存量行可能早于 create/update 的路径校验）
    crate::webserver::helper::policy::check_path(&server.config_path)?;
    let content = tokio::fs::read_to_string(&server.config_path)
        .await
        .unwrap_or_else(|_| "Config file not found".to_string());
    Ok(Json(serde_json::json!({
        "id": id,
        "config_path": server.config_path,
        "content": content
    })))
}

fn to_response(instance: WebServerInstance) -> WebServerResponse {
    WebServerResponse {
        id: instance.id,
        engine: instance.engine,
        version: instance.version,
        status: instance.status,
        config_path: instance.config_path,
        binary_path: instance.binary_path,
        port: instance.port,
        created_at: instance.created_at.to_rfc3339(),
        resource_version: instance.resource_version,
    }
}

#[derive(serde::Deserialize, utoipa::ToSchema)]
pub struct SwitchEngineRequest {
    pub engine: String,
}

#[derive(serde::Deserialize, utoipa::ToSchema)]
pub struct ApplyPresetRequest {
    pub preset: String,
}

#[derive(serde::Deserialize, utoipa::ToSchema)]
pub struct NativeInstallRequest {
    pub engine: String,
    pub version: Option<String>,
}

#[derive(serde::Deserialize, utoipa::ToSchema)]
pub struct NativeUninstallRequest {
    pub engine: String,
}

#[derive(serde::Deserialize, utoipa::ToSchema)]
pub struct AutostartRequest {
    pub enabled: bool,
}

#[derive(serde::Deserialize, utoipa::ToSchema)]
pub struct NativeAutostartRequest {
    pub engine: String,
    pub enabled: bool,
}

// ── 原生控制（检测 / 安装 / 卸载 / 开机自启） ────────────────────

/// 探测系统已安装的 Web 引擎（包管理器 / PATH / 候选路径 + systemd 状态 + 监听端口）
#[utoipa::path(
    get,
    path = "/api/web-servers/native/detect",
    tag = "web-servers",
    responses(
        (status = 200, description = "各引擎检测结果", body = [NativeWebServerInfo]),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 web_server:read 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn native_detect(
    State(state): State<AppState>,
) -> Result<Json<Vec<NativeWebServerInfo>>, AppError> {
    let result = state.web_server_service.native_detect().await;
    Ok(Json(result))
}

/// 经系统包管理器安装引擎并登记实例
#[utoipa::path(
    post,
    path = "/api/web-servers/native/install",
    tag = "web-servers",
    request_body = NativeInstallRequest,
    responses(
        (status = 200, description = "已安装并登记", body = WebServerResponse),
        (status = 400, description = "未知引擎"),
        (status = 500, description = "包管理器安装失败"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 web_server:create 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn native_install(
    State(state): State<AppState>,
    ApiJson(req): ApiJson<NativeInstallRequest>,
) -> Result<Json<WebServerResponse>, AppError> {
    let engine = WebServerEngine::from_name(&req.engine)
        .ok_or_else(|| AppError::BadRequest(format!("未知引擎: {}", req.engine)))?;
    let instance = state
        .web_server_service
        .native_install(&engine, req.version.as_deref())
        .await?;
    Ok(Json(to_response(instance)))
}

/// 卸载系统引擎并删除对应实例登记
#[utoipa::path(
    post,
    path = "/api/web-servers/native/uninstall",
    tag = "web-servers",
    request_body = NativeUninstallRequest,
    responses(
        (status = 200, description = "`{deleted: true, engine}`", body = Object),
        (status = 400, description = "未知引擎"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 web_server:delete 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn native_uninstall(
    State(state): State<AppState>,
    ApiJson(req): ApiJson<NativeUninstallRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let engine = WebServerEngine::from_name(&req.engine)
        .ok_or_else(|| AppError::BadRequest(format!("未知引擎: {}", req.engine)))?;
    state
        .web_server_service
        .native_uninstall_by_engine(&engine)
        .await?;
    Ok(Json(
        serde_json::json!({"deleted": true, "engine": engine.as_str()}),
    ))
}

/// 设置实例对应 systemd 服务的开机自启
#[utoipa::path(
    post,
    path = "/api/web-servers/{id}/autostart",
    tag = "web-servers",
    params(("id" = i64, Path, description = "实例 ID")),
    request_body = AutostartRequest,
    responses(
        (status = 200, description = "`{id, engine, autostart}`", body = Object),
        (status = 404, description = "实例不存在"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 web_server:update 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn set_autostart(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    ApiJson(req): ApiJson<AutostartRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let instance = state
        .web_server_service
        .set_autostart(id, req.enabled)
        .await?;
    Ok(Json(serde_json::json!({
        "id": instance.id,
        "engine": instance.engine,
        "autostart": req.enabled,
    })))
}

/// 按引擎设置 systemd 开机自启（未登记实例也可用）
#[utoipa::path(
    post,
    path = "/api/web-servers/native/autostart",
    tag = "web-servers",
    request_body = NativeAutostartRequest,
    responses(
        (status = 200, description = "`{engine, autostart}`", body = Object),
        (status = 400, description = "未知引擎"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 web_server:update 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn native_autostart(
    State(state): State<AppState>,
    ApiJson(req): ApiJson<NativeAutostartRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let engine = WebServerEngine::from_name(&req.engine)
        .ok_or_else(|| AppError::BadRequest(format!("未知引擎: {}", req.engine)))?;
    state
        .web_server_service
        .set_autostart_by_engine(&engine, req.enabled)
        .await?;
    Ok(Json(serde_json::json!({
        "engine": engine.as_str(),
        "autostart": req.enabled,
    })))
}

/// 实例对应引擎的原生状态
#[utoipa::path(
    get,
    path = "/api/web-servers/{id}/native-status",
    tag = "web-servers",
    params(("id" = i64, Path, description = "实例 ID")),
    responses(
        (status = 200, description = "检测结果", body = NativeWebServerInfo),
        (status = 404, description = "实例不存在"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 web_server:read 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn native_status(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<NativeWebServerInfo>, AppError> {
    let info = state.web_server_service.native_status(id).await?;
    Ok(Json(info))
}

/// 切换实例引擎（按新引擎默认路径重写登记）
#[utoipa::path(
    post,
    path = "/api/web-servers/{id}/switch-engine",
    tag = "web-servers",
    params(("id" = i64, Path, description = "实例 ID")),
    request_body = SwitchEngineRequest,
    responses(
        (status = 200, description = "切换后的实例", body = WebServerInstance),
        (status = 400, description = "未知引擎"),
        (status = 404, description = "实例不存在"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 web_server:update 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn switch_engine(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    ApiJson(req): ApiJson<SwitchEngineRequest>,
) -> Result<Json<WebServerInstance>, AppError> {
    let engine = WebServerEngine::from_name(&req.engine)
        .ok_or_else(|| AppError::BadRequest(format!("未知引擎: {}", req.engine)))?;
    let instance = state.web_server_service.switch_engine(id, &engine).await?;
    Ok(Json(instance))
}

/// 应用性能预设：生成全局配置 → 原子写 → 校验 → reload，失败回滚
#[utoipa::path(
    post,
    path = "/api/web-servers/{id}/preset",
    tag = "web-servers",
    params(("id" = i64, Path, description = "实例 ID")),
    request_body = ApplyPresetRequest,
    responses(
        (status = 200, description = "`{id, engine, preset, worker_processes, keepalive_timeout}`", body = Object),
        (status = 400, description = "未知预设 / 配置校验失败"),
        (status = 404, description = "实例不存在"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 web_server:update 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn apply_preset(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    ApiJson(req): ApiJson<ApplyPresetRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let preset = crate::webserver::preset::PerformancePreset::from_name(&req.preset)
        .ok_or_else(|| AppError::BadRequest(format!("未知预设: {}", req.preset)))?;
    let instance = state.web_server_service.apply_preset(id, &preset).await?;
    Ok(Json(serde_json::json!({
        "id": instance.id,
        "engine": instance.engine,
        "preset": preset.as_str(),
        "worker_processes": preset.worker_processes(&instance.engine),
        "keepalive_timeout": preset.keepalive_timeout(),
    })))
}

/// 性能预设列表（按本机资源标注推荐项）
#[utoipa::path(
    get,
    path = "/api/web-servers/presets",
    tag = "web-servers",
    responses(
        (status = 200, description = "`[{name, description, recommended, worker_processes}]`", body = [Object]),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 web_server:read 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn list_presets() -> Json<Vec<serde_json::Value>> {
    let resources = crate::application::app_store_service::SystemResources::detect();
    let recommended = crate::webserver::preset::PerformancePreset::recommend(&resources);
    let presets = [
        crate::webserver::preset::PerformancePreset::Low,
        crate::webserver::preset::PerformancePreset::Medium,
        crate::webserver::preset::PerformancePreset::High,
        crate::webserver::preset::PerformancePreset::Ultra,
    ];
    Json(
        presets
            .into_iter()
            .map(|p| {
                serde_json::json!({
                    "name": p.as_str(),
                    "description": p.description_zh(),
                    "recommended": p == recommended,
                    "worker_processes": p.worker_processes("nginx"),
                })
            })
            .collect(),
    )
}

/// 引擎配套能力请求：查询某引擎的差异化依赖/配套清单。
#[derive(serde::Deserialize, utoipa::IntoParams)]
#[into_params(parameter_in = Query)]
pub struct CapabilitiesQuery {
    pub engine: String,
}

/// 批量安装配套请求：`{engine, capabilities: ["php", "cache", ...]}`。
#[derive(serde::Deserialize, utoipa::ToSchema)]
pub struct InstallCapabilitiesRequest {
    pub engine: String,
    pub capabilities: Vec<String>,
}

// ── 配套能力（安装时"各取所需"，Doc/13 §5.4 阶段 3） ─────────────────

/// 查询引擎支持的配套能力（PHP-FPM / Lua / Redis / 扩展模块 / 自动 HTTPS）。
/// 引擎可选配套能力（PHP-FPM / Lua / 缓存 / 扩展模块 / 自动 HTTPS）及推荐系统包
#[utoipa::path(
    get,
    path = "/api/web-servers/capabilities",
    tag = "web-servers",
    params(CapabilitiesQuery),
    responses(
        (status = 200, description = "`{engine, capabilities: CapabilityInfo[]}`", body = Object),
        (status = 400, description = "未知引擎"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 web_server:read 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn engine_capabilities(
    State(state): State<AppState>,
    Query(q): Query<CapabilitiesQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let engine = WebServerEngine::from_name(&q.engine)
        .ok_or_else(|| AppError::BadRequest(format!("未知引擎: {}", q.engine)))?;
    let caps = state.web_server_service.engine_capabilities(&engine).await;
    Ok(Json(serde_json::json!({
        "engine": engine.as_str(),
        "capabilities": caps,
    })))
}

/// 批量安装引擎的配套依赖。
/// 批量安装配套能力
#[utoipa::path(
    post,
    path = "/api/web-servers/capabilities/install",
    tag = "web-servers",
    request_body = InstallCapabilitiesRequest,
    responses(
        (status = 200, description = "`{success, installed[], failed[]}`", body = Object),
        (status = 400, description = "未知引擎 / 未知能力（php / lua / cache / extra_modules / auto_https）"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 web_server:update 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn install_capabilities(
    State(state): State<AppState>,
    ApiJson(req): ApiJson<InstallCapabilitiesRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let engine = WebServerEngine::from_name(&req.engine)
        .ok_or_else(|| AppError::BadRequest(format!("未知引擎: {}", req.engine)))?;
    let mut caps = Vec::new();
    for key in &req.capabilities {
        let cap = match key.as_str() {
            "php" => crate::webserver::capability::WebServerCapability::Php,
            "lua" => crate::webserver::capability::WebServerCapability::Lua,
            "cache" => crate::webserver::capability::WebServerCapability::Cache,
            "extra_modules" => crate::webserver::capability::WebServerCapability::ExtraModules,
            "auto_https" => crate::webserver::capability::WebServerCapability::AutoHttps,
            _ => {
                return Err(AppError::BadRequest(format!("未知配套能力: {}", key)));
            }
        };
        caps.push(cap);
    }
    let result = state
        .web_server_service
        .install_capabilities(&engine, &caps)
        .await?;
    Ok(Json(result))
}

/// 路由表（集中注册于 routes.rs 组合根）
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/web-servers/engines", axum::routing::get(list_engines))
        .route("/api/web-servers", axum::routing::get(list))
        .route("/api/web-servers", axum::routing::post(create))
        .route("/api/web-servers/{id}", axum::routing::get(get))
        .route("/api/web-servers/{id}", axum::routing::put(update))
        .route("/api/web-servers/{id}", axum::routing::delete(delete))
        .route("/api/web-servers/{id}/start", axum::routing::post(start))
        .route("/api/web-servers/{id}/stop", axum::routing::post(stop))
        .route(
            "/api/web-servers/{id}/restart",
            axum::routing::post(restart),
        )
        .route("/api/web-servers/{id}/reload", axum::routing::post(reload))
        .route(
            "/api/web-servers/{id}/configtest",
            axum::routing::post(config_test),
        )
        .route(
            "/api/web-servers/{id}/config",
            axum::routing::get(get_config),
        )
        .route(
            "/api/web-servers/{id}/switch-engine",
            axum::routing::post(switch_engine),
        )
        .route(
            "/api/web-servers/{id}/preset",
            axum::routing::post(apply_preset),
        )
        .route("/api/web-servers/presets", axum::routing::get(list_presets))
        .route(
            "/api/web-servers/native/detect",
            axum::routing::get(native_detect),
        )
        .route(
            "/api/web-servers/native/install",
            axum::routing::post(native_install),
        )
        .route(
            "/api/web-servers/native/uninstall",
            axum::routing::post(native_uninstall),
        )
        .route(
            "/api/web-servers/native/autostart",
            axum::routing::post(native_autostart),
        )
        .route(
            "/api/web-servers/{id}/autostart",
            axum::routing::post(set_autostart),
        )
        .route(
            "/api/web-servers/{id}/native-status",
            axum::routing::get(native_status),
        )
        .route(
            "/api/web-servers/capabilities",
            axum::routing::get(engine_capabilities),
        )
        .route(
            "/api/web-servers/capabilities/install",
            axum::routing::post(install_capabilities),
        )
}
