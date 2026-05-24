use axum::{
    extract::{Path, State},
    Json, Router,
    routing::{get, post},
};

use crate::application::AppState;
use crate::core::error::AppError;
use crate::plugin::manager::PluginInfo;
use crate::plugin::wasm_runtime::PluginMeta;
use serde::Deserialize;

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/", get(list_plugins))
        .route("/{name}/start", get(start_plugin))
        .route("/{name}/stop", get(stop_plugin))
        .route("/wasm", get(list_wasm_plugins))
        .route("/wasm/execute", post(execute_wasm_plugin))
        .route("/wasm/reload", post(reload_wasm_plugins))
}

async fn list_plugins(
    State(state): State<AppState>,
) -> Result<Json<Vec<PluginInfo>>, AppError> {
    let manager = state.plugin_manager.read().await;
    Ok(Json(manager.list()))
}

async fn start_plugin(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let mut manager = state.plugin_manager.write().await;
    manager.start(&name).await?;
    Ok(Json(serde_json::json!({"message": "插件已启动", "name": name})))
}

async fn stop_plugin(
    State(state): State<AppState>,
    Path(name): Path<String>,
) -> Result<Json<serde_json::Value>, AppError> {
    let mut manager = state.plugin_manager.write().await;
    manager.stop(&name).await?;
    Ok(Json(serde_json::json!({"message": "插件已停止", "name": name})))
}

// ─── WASM Plugin Handlers ────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct ExecuteWasmRequest {
    pub name: String,
    pub input: String,
}

async fn list_wasm_plugins(
    State(state): State<AppState>,
) -> Json<Vec<PluginMeta>> {
    Json(state.wasm_runtime.list().await)
}

async fn execute_wasm_plugin(
    State(state): State<AppState>,
    Json(payload): Json<ExecuteWasmRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let output = state.wasm_runtime.execute(&payload.name, &payload.input).await?;
    Ok(Json(serde_json::json!({
        "plugin": payload.name,
        "output": output,
    })))
}

async fn reload_wasm_plugins(
    State(state): State<AppState>,
) -> Result<Json<serde_json::Value>, AppError> {
    let loaded = state.wasm_runtime.load_all().await?;
    Ok(Json(serde_json::json!({
        "message": "WASM 插件重新加载完成",
        "count": loaded.len(),
        "plugins": loaded,
    })))
}
