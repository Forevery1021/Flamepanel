use axum::{Json, extract::{State, Path, Query}};
use serde::Serialize;
use crate::api::types::{AppState, WebServerResponse, CreateWebServerInstanceRequest, PaginationParams, PaginatedResponse};
use crate::core::error::AppError;
use crate::domain::entity::WebServerInstance;
use crate::webserver::engine::WebServerEngine;
use chrono::Utc;

#[derive(Serialize)]
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

pub async fn list_engines() -> Json<Vec<EngineInfo>> {
    let engines = vec![
        WebServerEngine::Nginx,
        WebServerEngine::Apache,
        WebServerEngine::OpenLiteSpeed,
        WebServerEngine::OpenResty,
        WebServerEngine::Caddy,
    ];
    Json(engines.into_iter().map(|e| EngineInfo {
        name: e.as_str().to_string(),
        description: e.description().to_string(),
        default_port: e.default_port(),
        default_ssl_port: e.default_ssl_port(),
        supports_ssl: e.supports_ssl(),
        supports_rewrite: e.supports_rewrite(),
        supports_reverse_proxy: e.supports_reverse_proxy(),
        supports_load_balancing: e.supports_load_balancing(),
    }).collect())
}

pub async fn list(
    State(state): State<AppState>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<PaginatedResponse<WebServerResponse>>, AppError> {
    let result = state.web_server_service.list_servers_paginated(&params).await?;
    let data = result.data.into_iter().map(to_response).collect();
    Ok(Json(PaginatedResponse::new(data, result.total, &params)))
}

pub async fn get(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<WebServerResponse>, AppError> {
    let server = state.web_server_service.get_server(id).await?;
    Ok(Json(to_response(server)))
}

pub async fn create(
    State(state): State<AppState>,
    Json(req): Json<CreateWebServerInstanceRequest>,
) -> Result<Json<WebServerResponse>, AppError> {
    let engine = WebServerEngine::from_str(&req.engine)
        .ok_or_else(|| AppError::BadRequest(format!("Unknown engine: {}", req.engine)))?;

    let instance = WebServerInstance {
        id: 0,
        engine: engine.as_str().to_string(),
        version: req.version.clone(),
        status: "stopped".to_string(),
        config_path: req.config_path.unwrap_or_else(|| engine.default_config_path().to_string()),
        binary_path: req.binary_path.clone(),
        port: req.port.unwrap_or(engine.default_port() as i32),
        created_at: Utc::now(),
    };
    let id = state.web_server_service.create_server(&instance).await?;
    let created = state.web_server_service.get_server(id).await?;
    Ok(Json(to_response(created)))
}

pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(req): Json<serde_json::Value>,
) -> Result<Json<WebServerResponse>, AppError> {
    let mut server = state.web_server_service.get_server(id).await?;
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

pub async fn delete(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, AppError> {
    state.web_server_service.delete_server(id).await?;
    Ok(Json(serde_json::json!({"deleted": true, "id": id})))
}

pub async fn start(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, AppError> {
    let msg = state.web_server_service.start_server(id).await?;
    Ok(Json(serde_json::json!({"message": msg, "id": id})))
}

pub async fn stop(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, AppError> {
    let msg = state.web_server_service.stop_server(id).await?;
    Ok(Json(serde_json::json!({"message": msg, "id": id})))
}

pub async fn restart(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, AppError> {
    let msg = state.web_server_service.restart_server(id).await?;
    Ok(Json(serde_json::json!({"message": msg, "id": id})))
}

pub async fn reload(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, AppError> {
    let msg = state.web_server_service.reload_server(id).await?;
    Ok(Json(serde_json::json!({"message": msg, "id": id})))
}

pub async fn config_test(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, AppError> {
    let output = state.web_server_service.test_server_config(id).await?;
    Ok(Json(serde_json::json!({"output": output, "id": id})))
}

pub async fn get_config(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, AppError> {
    let server = state.web_server_service.get_server(id).await?;
    let content = tokio::fs::read_to_string(&server.config_path).await
        .unwrap_or_else(|_| "Config file not found".to_string());
    Ok(Json(serde_json::json!({
        "id": id,
        "config_path": server.config_path,
        "content": content
    })))
}

pub async fn update_config(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(req): Json<serde_json::Value>,
) -> Result<Json<serde_json::Value>, AppError> {
    let server = state.web_server_service.get_server(id).await?;
    let content = req.get("content").and_then(|v| v.as_str())
        .ok_or_else(|| AppError::BadRequest("Missing 'content' field".into()))?;
    tokio::fs::write(&server.config_path, content)
        .await
        .map_err(|e| AppError::Internal(format!("Failed to write config: {}", e)))?;
    Ok(Json(serde_json::json!({"message": "Config updated", "id": id})))
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
    }
}

#[derive(serde::Deserialize)]
pub struct SwitchEngineRequest {
    pub engine: String,
}

#[derive(serde::Deserialize)]
pub struct ApplyPresetRequest {
    pub preset: String,
}

pub async fn switch_engine(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(req): Json<SwitchEngineRequest>,
) -> Result<Json<WebServerInstance>, AppError> {
    let engine = WebServerEngine::from_str(&req.engine)
        .ok_or_else(|| AppError::BadRequest(format!("未知引擎: {}", req.engine)))?;
    let instance = state.web_server_service.switch_engine(id, &engine).await?;
    Ok(Json(instance))
}

pub async fn apply_preset(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(req): Json<ApplyPresetRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let preset = crate::webserver::preset::PerformancePreset::from_str(&req.preset)
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

pub async fn list_presets() -> Json<Vec<serde_json::Value>> {
    let resources = crate::application::app_store_service::SystemResources::detect();
    let recommended = crate::webserver::preset::PerformancePreset::recommend(&resources);
    let presets = [
        crate::webserver::preset::PerformancePreset::Low,
        crate::webserver::preset::PerformancePreset::Medium,
        crate::webserver::preset::PerformancePreset::High,
        crate::webserver::preset::PerformancePreset::Ultra,
    ];
    Json(presets.into_iter().map(|p| serde_json::json!({
        "name": p.as_str(),
        "description": p.description_zh(),
        "recommended": p == recommended,
        "worker_processes": p.worker_processes("nginx"),
    })).collect())
}
