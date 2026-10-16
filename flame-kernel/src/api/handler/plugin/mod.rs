use crate::api::extract::ApiJson;
use crate::api::types::{
    AppState, PluginMetricsResponse, PluginReloadRequest, PluginSettingRequest,
};
use crate::core::error::AppError;
use crate::domain::entity::Plugin;
use crate::plugin::capability::Capabilities;
use crate::plugin::sandbox::{PluginConfig, SandboxedPlugin};
use axum::Router;
use axum::{
    extract::{Path, State},
    Json,
};
use base64::{engine::general_purpose, Engine as _};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

pub mod market;

/// 加载插件（WASM 以 base64 提交；请求体受全局 2 MiB 上限约束）
#[derive(Debug, Deserialize, ToSchema)]
pub struct LoadPluginRequest {
    /// 1–64 位 `[A-Za-z0-9._-]`，不以 `.` 开头
    pub id: String,
    pub name: Option<String>,
    pub version: Option<String>,
    pub author: Option<String>,
    pub description: Option<String>,
    pub wasm_base64: String,
    /// 1 MiB – 256 MiB，缺省 64 MiB
    pub memory_limit_bytes: Option<usize>,
    /// 1 – 300000 ms，缺省 30000
    pub timeout_ms: Option<u64>,
    pub homepage: Option<String>,
    pub license: Option<String>,
    pub tags: Option<Vec<String>>,
    /// 授予的宿主能力（Doc/21 §5.7）：`fs:read:<dir>` / `fs:write:<dir>` / `metrics:read` /
    /// `settings:write`；缺省 = 无能力
    pub capabilities: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct ExecutePluginRequest {
    /// 兼容旧客户端，已不使用
    pub args: Option<Vec<i32>>,
    /// 调用输入（插件经 `flame.input_read` 读取），≤ 1 MiB
    pub input: Option<String>,
}

/// 变更插件宿主能力授予（整体替换）
#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateCapabilitiesRequest {
    pub capabilities: Vec<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct PluginResponse {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    pub enabled: bool,
    pub status: String,
    pub homepage: Option<String>,
    pub license: Option<String>,
    pub tags: Vec<String>,
    pub dependencies: Vec<String>,
    /// 已授予的宿主能力
    pub capabilities: Vec<String>,
    pub loaded_at: String,
    pub last_executed_at: Option<String>,
    pub exec_count: u64,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct ExecutionResponse {
    pub output: Vec<u8>,
    pub output_hex: String,
    pub output_string: String,
    pub execution_ms: u64,
    /// 插件经 `flame.log` 输出的日志（≤ 100 条）
    pub logs: Vec<String>,
}

/// 插件 ID：1–64 位 `[A-Za-z0-9._-]`，不以 `.` 开头（ID 会出现在 URL 路径段、日志与审计中）。
pub(crate) fn validate_plugin_id(id: &str) -> Result<(), AppError> {
    // `market` 与 `/api/plugins/market` 静态路由冲突
    let ok = !id.is_empty()
        && id != "market"
        && id.len() <= 64
        && !id.starts_with('.')
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'));
    if ok {
        Ok(())
    } else {
        Err(AppError::ValidationError(format!(
            "id: plugin id must be 1-64 chars of [A-Za-z0-9._-] and not start with '.', got {id:?}"
        )))
    }
}

pub(crate) const PLUGIN_MEMORY_MIN: usize = 1024 * 1024;
pub(crate) const PLUGIN_MEMORY_MAX: usize = 256 * 1024 * 1024;
pub(crate) const PLUGIN_TIMEOUT_MAX_MS: u64 = 300_000;

/// 由请求构造沙箱配置。旧实现直接信任 `memory_limit_bytes`：传 `usize::MAX` 即可绕过
/// WASM 线性内存上限（wasm32 最多 4 GiB/实例），拖垮宿主。越界一律 400，不静默截断。
pub(crate) fn plugin_config(
    memory_limit_bytes: Option<usize>,
    timeout_ms: Option<u64>,
) -> Result<PluginConfig, AppError> {
    let mut config = PluginConfig::default();
    if let Some(mem) = memory_limit_bytes {
        if !(PLUGIN_MEMORY_MIN..=PLUGIN_MEMORY_MAX).contains(&mem) {
            return Err(AppError::ValidationError(format!(
                "memory_limit_bytes: must be between {PLUGIN_MEMORY_MIN} and {PLUGIN_MEMORY_MAX}"
            )));
        }
        config.memory_limit_bytes = mem;
    }
    if let Some(timeout) = timeout_ms {
        if !(1..=PLUGIN_TIMEOUT_MAX_MS).contains(&timeout) {
            return Err(AppError::ValidationError(format!(
                "timeout_ms: must be between 1 and {PLUGIN_TIMEOUT_MAX_MS}"
            )));
        }
        config.timeout_ms = timeout;
    }
    Ok(config)
}

fn validate_setting(key: &str, value: &str) -> Result<(), AppError> {
    let key_ok = !key.is_empty()
        && key.len() <= 128
        && key
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || matches!(b, b'.' | b'_' | b'-'));
    if !key_ok {
        return Err(AppError::ValidationError(
            "key: must be 1-128 chars of [A-Za-z0-9._-]".into(),
        ));
    }
    if value.len() > 4096 {
        return Err(AppError::ValidationError(
            "value: must not exceed 4096 bytes".into(),
        ));
    }
    Ok(())
}

pub(crate) fn to_response(p: Plugin, sandbox: Option<&SandboxedPlugin>) -> PluginResponse {
    PluginResponse {
        id: p.id,
        name: p.name,
        version: p.version,
        author: p.author,
        description: p.description,
        enabled: p.enabled,
        status: sandbox
            .map(|s| format!("{:?}", s.status))
            .unwrap_or_else(|| "Unloaded".into()),
        homepage: p.homepage,
        license: p.license,
        tags: p.tags,
        dependencies: p
            .dependencies
            .iter()
            .map(|d| format!("{} ({})", d.plugin_id, d.version_requirement))
            .collect(),
        capabilities: p.capabilities,
        loaded_at: sandbox
            .map(|s| s.loaded_at.to_rfc3339())
            .unwrap_or_default(),
        last_executed_at: sandbox.and_then(|s| s.last_executed_at.map(|t| t.to_rfc3339())),
        exec_count: sandbox.map(|s| s.metrics.total_executions).unwrap_or(0),
    }
}

/// 校验并规范化能力授予；非空时记录告警日志（审计中间件另记写操作）。
pub(crate) fn grant_capabilities(id: &str, raw: &[String]) -> Result<Vec<String>, AppError> {
    let caps = Capabilities::parse_all(raw)?.to_strings();
    if !caps.is_empty() {
        tracing::warn!(target: "plugin", plugin = %id, capabilities = ?caps, "授予插件宿主能力");
    }
    Ok(caps)
}

/// 安装一个 WASM 插件：能力校验 → 沙箱加载 → 注册 → 落库（`POST /api/plugins` 与市场安装共用）。
pub(crate) async fn install_plugin(
    state: &AppState,
    mut plugin: Plugin,
    wasm_bytes: Vec<u8>,
    mut config: PluginConfig,
) -> Result<PluginResponse, AppError> {
    validate_plugin_id(&plugin.id)?;
    if wasm_bytes.is_empty() {
        return Err(AppError::BadRequest("WASM module is empty".into()));
    }
    // 旧实现先覆盖沙箱再注册，注册因重复失败时沙箱已被替换；改为前置冲突检查
    if state.plugin_registry.exists(&plugin.id) {
        return Err(AppError::Conflict(format!(
            "Plugin {} already exists; unload it first or use reload",
            plugin.id
        )));
    }
    plugin.capabilities = grant_capabilities(&plugin.id, &plugin.capabilities)?;
    plugin.wasm_hash = crate::plugin::market::sha256_hex(&wasm_bytes);
    plugin.wasm_base64 = general_purpose::STANDARD.encode(&wasm_bytes);
    config.capabilities = plugin.capabilities.clone();
    config.settings = plugin.settings.clone();
    let sandbox_plugin = state
        .plugin_sandbox
        .load_plugin(&plugin.id, wasm_bytes, Some(config))
        .await?;
    if let Err(e) = state.plugin_registry.register(plugin.clone()) {
        let _ = state.plugin_sandbox.unload_plugin(&plugin.id).await;
        return Err(e);
    }
    // T10/A2：持久化失败向上返回错误，不在 handler 层吞掉。
    if let Err(e) = state.plugin_repo.save(&plugin).await {
        let _ = state.plugin_sandbox.unload_plugin(&plugin.id).await;
        let _ = state.plugin_registry.unregister(&plugin.id);
        return Err(e);
    }
    Ok(to_response(plugin, Some(&sandbox_plugin)))
}

/// 已加载插件列表
#[utoipa::path(
    get,
    path = "/api/plugins",
    tag = "plugins",
    responses(
        (status = 200, description = "插件列表", body = [PluginResponse]),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 plugin:read 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn list_plugins(
    State(state): State<AppState>,
) -> Result<Json<Vec<PluginResponse>>, AppError> {
    let plugins = state.plugin_registry.list_all();
    let sandbox_plugins = state.plugin_sandbox.list_plugins().await;

    let responses: Vec<PluginResponse> = plugins
        .into_iter()
        .map(|p| {
            let sandbox_info = sandbox_plugins.iter().find(|s| s.id == p.id);
            to_response(p, sandbox_info)
        })
        .collect();

    Ok(Json(responses))
}

/// 单个插件
#[utoipa::path(
    get,
    path = "/api/plugins/{id}",
    tag = "plugins",
    params(("id" = String, Path, description = "插件 ID")),
    responses(
        (status = 200, description = "插件", body = PluginResponse),
        (status = 404, description = "插件不存在"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 plugin:read 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn get_plugin(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<PluginResponse>, AppError> {
    let plugin = state.plugin_registry.get(&id)?;
    let sandbox_info = state.plugin_sandbox.get_plugin(&id).await.ok();
    Ok(Json(to_response(plugin, sandbox_info.as_ref())))
}

/// 加载 WASM 插件并持久化（同 ID 覆盖）
#[utoipa::path(
    post,
    path = "/api/plugins",
    tag = "plugins",
    request_body = LoadPluginRequest,
    responses(
        (status = 200, description = "插件", body = PluginResponse),
        (status = 400, description = "base64 / WASM 非法、为空；ID、内存或超时越界"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 plugin:create 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn load_plugin(
    State(state): State<AppState>,
    ApiJson(req): ApiJson<LoadPluginRequest>,
) -> Result<Json<PluginResponse>, AppError> {
    validate_plugin_id(&req.id)?;
    let wasm_bytes = general_purpose::STANDARD
        .decode(&req.wasm_base64)
        .map_err(|e| AppError::BadRequest(format!("Invalid base64: {}", e)))?;
    let config = plugin_config(req.memory_limit_bytes, req.timeout_ms)?;
    let now = Utc::now();
    let plugin = Plugin {
        id: req.id,
        name: req.name.unwrap_or_default(),
        version: req.version.unwrap_or_else(|| "0.1.0".into()),
        author: req.author.unwrap_or_default(),
        description: req.description.unwrap_or_default(),
        wasm_hash: String::new(),
        wasm_base64: String::new(),
        enabled: true,
        homepage: req.homepage,
        license: req.license,
        tags: req.tags.unwrap_or_default(),
        config_schema: None,
        dependencies: vec![],
        capabilities: req.capabilities.unwrap_or_default(),
        settings: Default::default(),
        created_at: now,
        updated_at: now,
    };
    Ok(Json(
        install_plugin(&state, plugin, wasm_bytes, config).await?,
    ))
}

/// 热重载插件字节码
#[utoipa::path(
    post,
    path = "/api/plugins/{id}/reload",
    tag = "plugins",
    params(("id" = String, Path, description = "插件 ID")),
    request_body = PluginReloadRequest,
    responses(
        (status = 200, description = "插件", body = PluginResponse),
        (status = 400, description = "base64 / WASM 非法；内存或超时越界"),
        (status = 404, description = "插件不存在"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 plugin:create 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn reload_plugin(
    State(state): State<AppState>,
    Path(id): Path<String>,
    ApiJson(req): ApiJson<PluginReloadRequest>,
) -> Result<Json<PluginResponse>, AppError> {
    let wasm_bytes = general_purpose::STANDARD
        .decode(&req.wasm_base64)
        .map_err(|e| AppError::BadRequest(format!("Invalid base64: {}", e)))?;

    let config = plugin_config(req.memory_limit_bytes, req.timeout_ms)?;

    let sandbox_plugin = state
        .plugin_sandbox
        .reload_plugin(&id, wasm_bytes, Some(config), None)
        .await?;
    let plugin = state.plugin_registry.get(&id)?;
    Ok(Json(to_response(plugin, Some(&sandbox_plugin))))
}

/// 执行插件导出函数（fuel 限额 + 内存上限）
#[utoipa::path(
    post,
    path = "/api/plugins/{id}/execute/{function}",
    tag = "plugins",
    params(("id" = String, Path, description = "插件 ID"),
        ("function" = String, Path, description = "导出函数名")),
    request_body = ExecutePluginRequest,
    responses(
        (status = 200, description = "执行结果", body = ExecutionResponse),
        (status = 400, description = "插件已禁用"),
        (status = 404, description = "插件不存在"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 plugin:execute 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn execute_plugin(
    State(state): State<AppState>,
    Path((id, function)): Path<(String, String)>,
    ApiJson(req): ApiJson<ExecutePluginRequest>,
) -> Result<Json<ExecutionResponse>, AppError> {
    let plugin = state.plugin_registry.get(&id)?;
    if !plugin.enabled {
        return Err(AppError::BadRequest(format!("Plugin {} is disabled", id)));
    }

    let input = req.input.map(String::into_bytes);
    if input
        .as_ref()
        .is_some_and(|i| i.len() > crate::plugin::host::MAX_IO_BYTES)
    {
        return Err(AppError::ValidationError(
            "input: must not exceed 1 MiB".into(),
        ));
    }
    let result = state
        .plugin_sandbox
        .execute_plugin(&id, &function, input)
        .await?;
    if let Some(settings) = &result.settings_changed {
        persist_settings(&state, &id, settings.clone()).await?;
    }

    Ok(Json(ExecutionResponse {
        output: result.output.clone(),
        output_hex: result.output.iter().map(|b| format!("{:02x}", b)).collect(),
        output_string: result.output_as_string(),
        execution_ms: result.execution_ms,
        logs: result.logs,
    }))
}

/// 插件设置写回注册表与数据库（重启后由 restore 带回沙箱）。
async fn persist_settings(
    state: &AppState,
    id: &str,
    settings: std::collections::HashMap<String, String>,
) -> Result<(), AppError> {
    let mut plugin = state.plugin_registry.get(id)?;
    plugin.settings = settings;
    plugin.updated_at = Utc::now();
    state.plugin_registry.replace(plugin.clone())?;
    state.plugin_repo.save(&plugin).await
}

/// 启用插件
#[utoipa::path(
    post,
    path = "/api/plugins/{id}/enable",
    tag = "plugins",
    params(("id" = String, Path, description = "插件 ID")),
    responses(
        (status = 200, description = "插件", body = PluginResponse),
        (status = 404, description = "插件不存在"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 plugin:create 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn enable_plugin(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<PluginResponse>, AppError> {
    let plugin = state.plugin_registry.enable(&id)?;
    let _ = state.plugin_sandbox.enable_plugin(&id).await;
    // 启停状态落库：restore 按 enabled 恢复（旧实现不持久化，重启后状态回退）
    state.plugin_repo.save(&plugin).await?;
    let sandbox_info = state.plugin_sandbox.get_plugin(&id).await.ok();
    Ok(Json(to_response(plugin, sandbox_info.as_ref())))
}

/// 禁用插件
#[utoipa::path(
    post,
    path = "/api/plugins/{id}/disable",
    tag = "plugins",
    params(("id" = String, Path, description = "插件 ID")),
    responses(
        (status = 200, description = "插件", body = PluginResponse),
        (status = 404, description = "插件不存在"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 plugin:create 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn disable_plugin(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<PluginResponse>, AppError> {
    let plugin = state.plugin_registry.disable(&id)?;
    let _ = state.plugin_sandbox.disable_plugin(&id).await;
    // 启停状态落库：restore 按 enabled 恢复（旧实现不持久化，重启后状态回退）
    state.plugin_repo.save(&plugin).await?;
    let sandbox_info = state.plugin_sandbox.get_plugin(&id).await.ok();
    Ok(Json(to_response(plugin, sandbox_info.as_ref())))
}

/// 卸载插件（返回 id / status=unloaded / exec_count）
#[utoipa::path(
    post,
    path = "/api/plugins/{id}",
    tag = "plugins",
    params(("id" = String, Path, description = "插件 ID")),
    responses(
        (status = 200, description = "已卸载", body = serde_json::Value),
        (status = 404, description = "插件不存在"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 plugin:delete 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn unload_plugin(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let sandbox_plugin = state.plugin_sandbox.unload_plugin(&id).await?;
    state.plugin_registry.unregister(&id)?;
    // 旧实现不删库：重启后被卸载的插件又被 restore 回来
    state.plugin_repo.delete(&id).await?;

    Ok(Json(serde_json::json!({
        "id": sandbox_plugin.id,
        "status": "unloaded",
        "exec_count": sandbox_plugin.metrics.total_executions,
    })))
}

/// 插件执行指标
#[utoipa::path(
    get,
    path = "/api/plugins/{id}/metrics",
    tag = "plugins",
    params(("id" = String, Path, description = "插件 ID")),
    responses(
        (status = 200, description = "指标", body = PluginMetricsResponse),
        (status = 404, description = "插件不存在"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 plugin:read 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn get_plugin_metrics(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<PluginMetricsResponse>, AppError> {
    let metrics = state.plugin_sandbox.get_plugin_metrics(&id).await?;
    Ok(Json(PluginMetricsResponse {
        total_executions: metrics.total_executions,
        successful_executions: metrics.successful_executions,
        failed_executions: metrics.failed_executions,
        avg_execution_ms: metrics.avg_execution_ms,
        max_execution_ms: metrics.max_execution_ms,
        min_execution_ms: if metrics.min_execution_ms == u64::MAX {
            0
        } else {
            metrics.min_execution_ms
        },
        last_execution_ms: metrics.last_execution_ms,
        peak_memory_bytes: metrics.peak_memory_bytes,
    }))
}

/// 重置插件执行指标
#[utoipa::path(
    delete,
    path = "/api/plugins/{id}/metrics",
    tag = "plugins",
    params(("id" = String, Path, description = "插件 ID")),
    responses(
        (status = 200, description = "已重置", body = serde_json::Value),
        (status = 404, description = "插件不存在"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 plugin:config 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn reset_plugin_metrics(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    state.plugin_sandbox.reset_plugin_metrics(&id).await?;
    Ok(Json(
        serde_json::json!({"message": "Metrics reset", "id": id}),
    ))
}

/// 插件设置（键值）
#[utoipa::path(
    get,
    path = "/api/plugins/{id}/settings",
    tag = "plugins",
    params(("id" = String, Path, description = "插件 ID")),
    responses(
        (status = 200, description = "设置", body = std::collections::HashMap<String, String>),
        (status = 404, description = "插件不存在"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 plugin:config 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn list_plugin_settings(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<std::collections::HashMap<String, String>>, AppError> {
    let settings = state.plugin_sandbox.list_plugin_settings(&id).await?;
    Ok(Json(settings))
}

/// 写入一项插件设置
#[utoipa::path(
    post,
    path = "/api/plugins/{id}/settings",
    tag = "plugins",
    params(("id" = String, Path, description = "插件 ID")),
    request_body = PluginSettingRequest,
    responses(
        (status = 200, description = "已保存（message / id / key）", body = serde_json::Value),
        (status = 400, description = "key 须 1–128 位 `[A-Za-z0-9._-]`；value ≤ 4096 字节"),
        (status = 404, description = "插件不存在"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 plugin:config 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn set_plugin_setting(
    State(state): State<AppState>,
    Path(id): Path<String>,
    ApiJson(req): ApiJson<PluginSettingRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    validate_setting(&req.key, &req.value)?;
    state
        .plugin_sandbox
        .set_plugin_setting(&id, &req.key, &req.value)
        .await?;
    let settings = state.plugin_sandbox.list_plugin_settings(&id).await?;
    persist_settings(&state, &id, settings).await?;
    Ok(Json(
        serde_json::json!({"message": "Setting saved", "id": id, "key": req.key}),
    ))
}

/// 读取一项插件设置（id / key / value）
#[utoipa::path(
    get,
    path = "/api/plugins/{id}/settings/{key}",
    tag = "plugins",
    params(("id" = String, Path, description = "插件 ID"),
        ("key" = String, Path, description = "设置键")),
    responses(
        (status = 200, description = "设置值", body = serde_json::Value),
        (status = 404, description = "插件不存在"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 plugin:config 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn get_plugin_setting(
    State(state): State<AppState>,
    Path((id, key)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, AppError> {
    let value = state.plugin_sandbox.get_plugin_setting(&id, &key).await?;
    Ok(Json(
        serde_json::json!({"id": id, "key": key, "value": value}),
    ))
}

/// 变更插件宿主能力授予（整体替换，立即生效并持久化）
#[utoipa::path(
    put,
    path = "/api/plugins/{id}/capabilities",
    tag = "plugins",
    params(("id" = String, Path, description = "插件 ID")),
    request_body = UpdateCapabilitiesRequest,
    responses(
        (status = 200, description = "插件", body = PluginResponse),
        (status = 400, description = "能力非法（未知能力 / 敏感目录 / 超过 16 项）"),
        (status = 404, description = "插件不存在"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 plugin:create 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn update_plugin_capabilities(
    State(state): State<AppState>,
    Path(id): Path<String>,
    ApiJson(req): ApiJson<UpdateCapabilitiesRequest>,
) -> Result<Json<PluginResponse>, AppError> {
    let mut plugin = state.plugin_registry.get(&id)?;
    let caps = grant_capabilities(&id, &req.capabilities)?;
    state
        .plugin_sandbox
        .set_plugin_capabilities(&id, caps.clone())
        .await?;
    plugin.capabilities = caps;
    plugin.updated_at = Utc::now();
    state.plugin_registry.replace(plugin.clone())?;
    state.plugin_repo.save(&plugin).await?;
    let sandbox_info = state.plugin_sandbox.get_plugin(&id).await.ok();
    Ok(Json(to_response(plugin, sandbox_info.as_ref())))
}

/// 路由表（集中注册于 routes.rs 组合根）
pub fn routes() -> Router<AppState> {
    Router::new()
        .route(
            "/api/plugins/market",
            axum::routing::get(market::list_market),
        )
        .route(
            "/api/plugins/market/{id}/install",
            axum::routing::post(market::install_market_plugin),
        )
        .route(
            "/api/plugins/{id}/capabilities",
            axum::routing::put(update_plugin_capabilities),
        )
        .route("/api/plugins", axum::routing::get(list_plugins))
        .route("/api/plugins", axum::routing::post(load_plugin))
        .route("/api/plugins/{id}", axum::routing::get(get_plugin))
        .route("/api/plugins/{id}", axum::routing::post(unload_plugin))
        .route(
            "/api/plugins/{id}/enable",
            axum::routing::post(enable_plugin),
        )
        .route(
            "/api/plugins/{id}/disable",
            axum::routing::post(disable_plugin),
        )
        .route(
            "/api/plugins/{id}/execute/{function}",
            axum::routing::post(execute_plugin),
        )
        .route(
            "/api/plugins/{id}/reload",
            axum::routing::post(reload_plugin),
        )
        .route(
            "/api/plugins/{id}/metrics",
            axum::routing::get(get_plugin_metrics),
        )
        .route(
            "/api/plugins/{id}/metrics",
            axum::routing::delete(reset_plugin_metrics),
        )
        .route(
            "/api/plugins/{id}/settings",
            axum::routing::get(list_plugin_settings),
        )
        .route(
            "/api/plugins/{id}/settings",
            axum::routing::post(set_plugin_setting),
        )
        .route(
            "/api/plugins/{id}/settings/{key}",
            axum::routing::get(get_plugin_setting),
        )
}
