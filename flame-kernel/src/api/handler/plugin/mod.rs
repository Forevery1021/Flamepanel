use axum::{Json, extract::{State, Path}};
use serde::{Deserialize, Serialize};
use crate::api::types::{AppState, PluginSettingRequest, PluginMetricsResponse, PluginReloadRequest};
use crate::core::error::AppError;
use crate::domain::entity::Plugin;
use crate::plugin::sandbox::PluginConfig;
use base64::{Engine as _, engine::general_purpose};
use chrono::Utc;

#[derive(Debug, Deserialize)]
pub struct LoadPluginRequest {
    pub id: String,
    pub name: Option<String>,
    pub version: Option<String>,
    pub author: Option<String>,
    pub description: Option<String>,
    pub wasm_base64: String,
    pub memory_limit_bytes: Option<usize>,
    pub timeout_ms: Option<u64>,
    pub homepage: Option<String>,
    pub license: Option<String>,
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
pub struct ExecutePluginRequest {
    pub args: Option<Vec<i32>>,
}

#[derive(Debug, Serialize)]
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
    pub loaded_at: String,
    pub last_executed_at: Option<String>,
    pub exec_count: u64,
}

#[derive(Debug, Serialize)]
pub struct ExecutionResponse {
    pub output: Vec<u8>,
    pub output_hex: String,
    pub output_string: String,
    pub execution_ms: u64,
}

pub async fn list_plugins(
    State(state): State<AppState>,
) -> Result<Json<Vec<PluginResponse>>, AppError> {
    let plugins = state.plugin_registry.list_all();
    let sandbox_plugins = state.plugin_sandbox.list_plugins().await;

    let responses: Vec<PluginResponse> = plugins.into_iter().map(|p| {
        let sandbox_info = sandbox_plugins.iter().find(|s| s.id == p.id);
        PluginResponse {
            id: p.id,
            name: p.name,
            version: p.version,
            author: p.author,
            description: p.description,
            enabled: p.enabled,
            status: sandbox_info.map(|s| format!("{:?}", s.status)).unwrap_or_else(|| "Unloaded".into()),
            homepage: p.homepage,
            license: p.license,
            tags: p.tags,
            dependencies: p.dependencies.iter().map(|d| format!("{} ({})", d.plugin_id, d.version_requirement)).collect(),
            loaded_at: sandbox_info.map(|s| s.loaded_at.to_rfc3339()).unwrap_or_default(),
            last_executed_at: sandbox_info.and_then(|s| s.last_executed_at.map(|t| t.to_rfc3339())),
            exec_count: sandbox_info.map(|s| s.metrics.total_executions).unwrap_or(0),
        }
    }).collect();

    Ok(Json(responses))
}

pub async fn get_plugin(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<PluginResponse>, AppError> {
    let plugin = state.plugin_registry.get(&id)?;
    let sandbox_info = state.plugin_sandbox.get_plugin(&id).await.ok();
    let status = sandbox_info.as_ref().map(|s| format!("{:?}", s.status)).unwrap_or_else(|| "Unloaded".into());
    let loaded_at = sandbox_info.as_ref().map(|s| s.loaded_at.to_rfc3339()).unwrap_or_default();
    let last_executed_at = sandbox_info.as_ref().and_then(|s| s.last_executed_at.map(|t| t.to_rfc3339()));
    let exec_count = sandbox_info.as_ref().map(|s| s.metrics.total_executions).unwrap_or(0);

    Ok(Json(PluginResponse {
        id: plugin.id,
        name: plugin.name,
        version: plugin.version,
        author: plugin.author,
        description: plugin.description,
        enabled: plugin.enabled,
        status,
        homepage: plugin.homepage,
        license: plugin.license,
        tags: plugin.tags,
        dependencies: plugin.dependencies.iter().map(|d| format!("{} ({})", d.plugin_id, d.version_requirement)).collect(),
        loaded_at,
        last_executed_at,
        exec_count,
    }))
}

pub async fn load_plugin(
    State(state): State<AppState>,
    Json(req): Json<LoadPluginRequest>,
) -> Result<Json<PluginResponse>, AppError> {
    let wasm_bytes = general_purpose::STANDARD.decode(&req.wasm_base64)
        .map_err(|e| AppError::BadRequest(format!("Invalid base64: {}", e)))?;

    if wasm_bytes.is_empty() {
        return Err(AppError::BadRequest("WASM module is empty".into()));
    }

    use sha2::{Sha256, Digest};
    let mut hasher = Sha256::new();
    hasher.update(&wasm_bytes);
    let wasm_hash = format!("{:x}", hasher.finalize());

    let mut config = PluginConfig::default();
    if let Some(mem) = req.memory_limit_bytes {
        config.memory_limit_bytes = mem;
    }
    if let Some(timeout) = req.timeout_ms {
        config.timeout_ms = timeout;
    }

    let sandbox_plugin = state.plugin_sandbox.load_plugin(&req.id, wasm_bytes, Some(config)).await?;

    let now = Utc::now();
    let plugin = Plugin {
        id: req.id,
        name: req.name.unwrap_or_default(),
        version: req.version.unwrap_or_else(|| "0.1.0".into()),
        author: req.author.unwrap_or_default(),
        description: req.description.unwrap_or_default(),
        wasm_hash,
        enabled: true,
        homepage: req.homepage,
        license: req.license,
        tags: req.tags.unwrap_or_default(),
        config_schema: None,
        dependencies: vec![],
        created_at: now,
        updated_at: now,
    };
    state.plugin_registry.register(plugin.clone())?;

    Ok(Json(PluginResponse {
        id: plugin.id,
        name: plugin.name,
        version: plugin.version,
        author: plugin.author,
        description: plugin.description,
        enabled: plugin.enabled,
        status: format!("{:?}", sandbox_plugin.status),
        homepage: plugin.homepage,
        license: plugin.license,
        tags: plugin.tags,
        dependencies: vec![],
        loaded_at: sandbox_plugin.loaded_at.to_rfc3339(),
        last_executed_at: None,
        exec_count: 0,
    }))
}

pub async fn reload_plugin(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<PluginReloadRequest>,
) -> Result<Json<PluginResponse>, AppError> {
    let wasm_bytes = general_purpose::STANDARD.decode(&req.wasm_base64)
        .map_err(|e| AppError::BadRequest(format!("Invalid base64: {}", e)))?;

    let mut config = PluginConfig::default();
    if let Some(mem) = req.memory_limit_bytes {
        config.memory_limit_bytes = mem;
    }
    if let Some(timeout) = req.timeout_ms {
        config.timeout_ms = timeout;
    }

    let sandbox_plugin = state.plugin_sandbox.reload_plugin(&id, wasm_bytes, Some(config)).await?;
    let plugin = state.plugin_registry.get(&id)?;

    Ok(Json(PluginResponse {
        id: plugin.id,
        name: plugin.name,
        version: plugin.version,
        author: plugin.author,
        description: plugin.description,
        enabled: plugin.enabled,
        status: format!("{:?}", sandbox_plugin.status),
        homepage: plugin.homepage,
        license: plugin.license,
        tags: plugin.tags,
        dependencies: plugin.dependencies.iter().map(|d| format!("{} ({})", d.plugin_id, d.version_requirement)).collect(),
        loaded_at: sandbox_plugin.loaded_at.to_rfc3339(),
        last_executed_at: sandbox_plugin.last_executed_at.map(|t| t.to_rfc3339()),
        exec_count: sandbox_plugin.metrics.total_executions,
    }))
}

pub async fn execute_plugin(
    State(state): State<AppState>,
    Path((id, function)): Path<(String, String)>,
    Json(req): Json<ExecutePluginRequest>,
) -> Result<Json<ExecutionResponse>, AppError> {
    let plugin = state.plugin_registry.get(&id)?;
    if !plugin.enabled {
        return Err(AppError::BadRequest(format!("Plugin {} is disabled", id)));
    }

    let result = state.plugin_sandbox.execute_plugin(&id, &function, req.args).await?;

    Ok(Json(ExecutionResponse {
        output: result.output.clone(),
        output_hex: result.output.iter().map(|b| format!("{:02x}", b)).collect(),
        output_string: result.output_as_string(),
        execution_ms: result.execution_ms,
    }))
}

pub async fn enable_plugin(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<PluginResponse>, AppError> {
    let plugin = state.plugin_registry.enable(&id)?;
    let _ = state.plugin_sandbox.enable_plugin(&id).await;

    let sandbox_info = state.plugin_sandbox.get_plugin(&id).await.ok();
    let status = sandbox_info.as_ref().map(|s| format!("{:?}", s.status)).unwrap_or_else(|| "Unloaded".into());
    let loaded_at = sandbox_info.as_ref().map(|s| s.loaded_at.to_rfc3339()).unwrap_or_default();
    let last_executed_at = sandbox_info.as_ref().and_then(|s| s.last_executed_at.map(|t| t.to_rfc3339()));
    let exec_count = sandbox_info.as_ref().map(|s| s.metrics.total_executions).unwrap_or(0);

    Ok(Json(PluginResponse {
        id: plugin.id,
        name: plugin.name,
        version: plugin.version,
        author: plugin.author,
        description: plugin.description,
        enabled: plugin.enabled,
        status,
        homepage: plugin.homepage,
        license: plugin.license,
        tags: plugin.tags,
        dependencies: plugin.dependencies.iter().map(|d| format!("{} ({})", d.plugin_id, d.version_requirement)).collect(),
        loaded_at,
        last_executed_at,
        exec_count,
    }))
}

pub async fn disable_plugin(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<PluginResponse>, AppError> {
    let plugin = state.plugin_registry.disable(&id)?;
    let _ = state.plugin_sandbox.disable_plugin(&id).await;

    let sandbox_info = state.plugin_sandbox.get_plugin(&id).await.ok();
    let status = sandbox_info.as_ref().map(|s| format!("{:?}", s.status)).unwrap_or_else(|| "Unloaded".into());
    let loaded_at = sandbox_info.as_ref().map(|s| s.loaded_at.to_rfc3339()).unwrap_or_default();
    let last_executed_at = sandbox_info.as_ref().and_then(|s| s.last_executed_at.map(|t| t.to_rfc3339()));
    let exec_count = sandbox_info.as_ref().map(|s| s.metrics.total_executions).unwrap_or(0);

    Ok(Json(PluginResponse {
        id: plugin.id,
        name: plugin.name,
        version: plugin.version,
        author: plugin.author,
        description: plugin.description,
        enabled: plugin.enabled,
        status,
        homepage: plugin.homepage,
        license: plugin.license,
        tags: plugin.tags,
        dependencies: plugin.dependencies.iter().map(|d| format!("{} ({})", d.plugin_id, d.version_requirement)).collect(),
        loaded_at,
        last_executed_at,
        exec_count,
    }))
}

pub async fn unload_plugin(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let sandbox_plugin = state.plugin_sandbox.unload_plugin(&id).await?;
    state.plugin_registry.unregister(&id)?;

    Ok(Json(serde_json::json!({
        "id": sandbox_plugin.id,
        "status": "unloaded",
        "exec_count": sandbox_plugin.metrics.total_executions,
    })))
}

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
        min_execution_ms: if metrics.min_execution_ms == u64::MAX { 0 } else { metrics.min_execution_ms },
        last_execution_ms: metrics.last_execution_ms,
        peak_memory_bytes: metrics.peak_memory_bytes,
    }))
}

pub async fn reset_plugin_metrics(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    state.plugin_sandbox.reset_plugin_metrics(&id).await?;
    Ok(Json(serde_json::json!({"message": "Metrics reset", "id": id})))
}

pub async fn list_plugin_settings(
    State(state): State<AppState>,
    Path(id): Path<String>,
) -> Result<Json<std::collections::HashMap<String, String>>, AppError> {
    let settings = state.plugin_sandbox.list_plugin_settings(&id).await?;
    Ok(Json(settings))
}

pub async fn set_plugin_setting(
    State(state): State<AppState>,
    Path(id): Path<String>,
    Json(req): Json<PluginSettingRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    state.plugin_sandbox.set_plugin_setting(&id, &req.key, &req.value).await?;
    Ok(Json(serde_json::json!({"message": "Setting saved", "id": id, "key": req.key})))
}

pub async fn get_plugin_setting(
    State(state): State<AppState>,
    Path((id, key)): Path<(String, String)>,
) -> Result<Json<serde_json::Value>, AppError> {
    let value = state.plugin_sandbox.get_plugin_setting(&id, &key).await?;
    Ok(Json(serde_json::json!({"id": id, "key": key, "value": value})))
}
