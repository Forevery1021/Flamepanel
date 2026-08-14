use crate::api::extract::ApiJson;
use crate::api::types::{AppState, CreateNodeRequest, PaginatedResponse, PaginationParams};
use crate::core::error::AppError;
use crate::domain::entity::ServerNode;
use axum::Router;
use axum::{
    extract::{Path, Query, State},
    http::HeaderMap,
    Json,
};
use serde::Deserialize;
use utoipa::ToSchema;

const HEARTBEAT_TIMEOUT_SECS: i64 = 30;

#[derive(Debug, Deserialize, ToSchema)]
pub struct HeartbeatRequest {
    pub cpu_usage: f32,
    pub memory_usage_percent: f32,
    pub disk_usage_percent: f32,
    pub load_one: f32,
}

/// 节点列表（分页）
#[utoipa::path(
    get,
    path = "/api/nodes",
    tag = "nodes",
    operation_id = "list_nodes",
    params(PaginationParams),
    responses(
        (status = 200, description = "节点列表", body = PaginatedResponse<ServerNode>),
        (status = 401, description = "未认证"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn list(
    State(state): State<AppState>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<PaginatedResponse<ServerNode>>, AppError> {
    let result = state.node_service.list_nodes_paginated(&params).await?;
    Ok(Json(result))
}

/// 注册节点
#[utoipa::path(
    post,
    path = "/api/nodes",
    tag = "nodes",
    operation_id = "create_node",
    request_body = CreateNodeRequest,
    responses(
        (status = 200, description = "注册成功，返回节点 ID", body = i64),
        (status = 400, description = "参数错误"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn create(
    State(state): State<AppState>,
    ApiJson(payload): ApiJson<CreateNodeRequest>,
) -> Result<Json<i64>, AppError> {
    let id = state.node_service.register_node(&payload.to_node()).await?;
    Ok(Json(id))
}

/// Agent 注册（白名单免 JWT，供 Agent 启动时自动注册）
/// 请求体：`{name, host, agent_port, auth_token}`，返回 `{"id": <node_id>}`
#[utoipa::path(
    post,
    path = "/api/nodes/register",
    tag = "nodes",
    request_body = CreateNodeRequest,
    responses(
        (status = 200, description = "注册成功，返回节点 ID", body = serde_json::Value),
        (status = 400, description = "参数错误"),
    )
)]
pub async fn register_agent(
    State(state): State<AppState>,
    ApiJson(payload): ApiJson<CreateNodeRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let node = payload.to_node();
    let id = state.node_service.register_node(&node).await?;
    Ok(Json(serde_json::json!({ "id": id })))
}

/// Agent 心跳上报（白名单免 JWT，校验 Agent token）
#[utoipa::path(
    post,
    path = "/api/nodes/heartbeat/{id}",
    tag = "nodes",
    params(("id" = i64, Path, description = "节点 ID")),
    request_body = HeartbeatRequest,
    responses(
        (status = 200, description = "心跳记录成功"),
        (status = 401, description = "Agent token 无效"),
    )
)]
pub async fn heartbeat(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    headers: HeaderMap,
    ApiJson(req): ApiJson<HeartbeatRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    // Agent token 校验（兼容旧 Agent：库中无 token 时放行）
    let provided = headers
        .get(axum::http::header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .map(|s| s.strip_prefix("Bearer ").unwrap_or(s).to_string());
    let valid = state
        .node_service
        .verify_agent_token(id, provided.as_deref())
        .await?;
    if !valid {
        return Err(AppError::Unauthorized("Invalid agent token".into()));
    }

    let metrics = serde_json::json!({
        "cpu_usage": req.cpu_usage,
        "memory_usage_percent": req.memory_usage_percent,
        "disk_usage_percent": req.disk_usage_percent,
        "load_one": req.load_one,
    });
    let node = state.node_service.record_heartbeat(id, &metrics).await?;
    Ok(Json(serde_json::json!({
        "id": node.id,
        "status": "ok",
        "last_heartbeat_at": node.last_heartbeat_at,
    })))
}

/// 节点在线状态（惰性判定）
#[utoipa::path(
    get,
    path = "/api/nodes/{id}/status",
    tag = "nodes",
    params(("id" = i64, Path, description = "节点 ID")),
    responses(
        (status = 200, description = "在线状态"),
        (status = 404, description = "节点不存在"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn status(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, AppError> {
    let status = state
        .node_service
        .node_status(id, HEARTBEAT_TIMEOUT_SECS)
        .await?;
    Ok(Json(serde_json::json!({ "id": id, "status": status })))
}

/// 节点最近指标快照
pub async fn metrics(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, AppError> {
    let metrics = state.node_service.node_metrics(id).await?;
    Ok(Json(metrics))
}

// ── Stage5 多节点远程调用 ────────────────────────────────────────────────

#[derive(Debug, Deserialize, ToSchema)]
pub struct RemoteExecRequest {
    pub command: String,
    #[serde(default)]
    pub timeout_secs: Option<u64>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct RemoteBatchExecRequest {
    pub node_ids: Vec<i64>,
    pub command: String,
    #[serde(default)]
    pub timeout_secs: Option<u64>,
}

/// Phase A1：Agent 动作枚举请求（动作 + 参数）
#[derive(Debug, Deserialize, ToSchema)]
pub struct RemoteActionRequest {
    pub action: String,
    #[serde(default)]
    pub params: serde_json::Value,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct RemoteListQuery {
    #[serde(default = "default_remote_path")]
    pub path: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct RemoteUploadRequest {
    pub path: String,
    /// base64 编码的文件内容
    pub content_base64: String,
}

fn default_remote_path() -> String {
    ".".into()
}

/// 在远程节点执行命令
#[utoipa::path(
    post,
    path = "/api/nodes/{id}/execute",
    tag = "nodes",
    params(("id" = i64, Path, description = "节点 ID")),
    request_body = RemoteExecRequest,
    responses(
        (status = 200, description = "执行结果"),
        (status = 401, description = "未认证"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn remote_execute(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    ApiJson(req): ApiJson<RemoteExecRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let result = state
        .node_service
        .remote_execute(id, &req.command, req.timeout_secs)
        .await?;
    Ok(Json(serde_json::json!({
        "node_id": id,
        "output": result.output,
        "exit_code": result.exit_code,
        "duration_ms": result.duration_ms,
    })))
}

/// 调用 Agent 动作枚举（Phase A1：安全动作白名单）
#[utoipa::path(
    post,
    path = "/api/nodes/{id}/action",
    tag = "nodes",
    params(("id" = i64, Path, description = "节点 ID")),
    request_body = RemoteActionRequest,
    responses(
        (status = 200, description = "动作结果"),
        (status = 401, description = "未认证"),
        (status = 400, description = "非法动作或参数"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn remote_action(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    ApiJson(req): ApiJson<RemoteActionRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let action = crate::infrastructure::agent_client::AgentActionRequest {
        action: req.action,
        params: req.params,
    };
    let result = state.node_service.remote_action(id, &action).await?;
    Ok(Json(result))
}

/// 批量在多个节点并行执行命令
#[utoipa::path(
    post,
    path = "/api/nodes/batch-execute",
    tag = "nodes",
    request_body = RemoteBatchExecRequest,
    responses(
        (status = 200, description = "各节点执行结果聚合"),
        (status = 401, description = "未认证"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn batch_execute(
    State(state): State<AppState>,
    ApiJson(req): ApiJson<RemoteBatchExecRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    if req.node_ids.is_empty() {
        return Err(AppError::BadRequest("node_ids must not be empty".into()));
    }
    let results = state
        .node_service
        .batch_execute(&req.node_ids, &req.command, req.timeout_secs)
        .await?;
    let items: Vec<serde_json::Value> = results
        .into_iter()
        .map(|(id, name, result)| {
            serde_json::json!({
                "node_id": id,
                "node_name": name,
                "success": result.is_ok(),
                "result": match result {
                    Ok(r) => serde_json::json!({
                        "output": r.output,
                        "exit_code": r.exit_code,
                        "duration_ms": r.duration_ms,
                    }),
                    Err(e) => serde_json::json!({
                        "error": e.to_string(),
                    }),
                },
            })
        })
        .collect();
    Ok(Json(serde_json::json!({"items": items})))
}

/// 列出远程节点目录
#[utoipa::path(
    get,
    path = "/api/nodes/{id}/files",
    tag = "nodes",
    params(("id" = i64, Path, description = "节点 ID")),
    responses(
        (status = 200, description = "远程目录列表"),
        (status = 401, description = "未认证"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn remote_list_files(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Query(q): Query<RemoteListQuery>,
) -> Result<Json<Vec<serde_json::Value>>, AppError> {
    let entries = state.node_service.remote_list_files(id, &q.path).await?;
    Ok(Json(
        entries
            .into_iter()
            .map(|e| {
                serde_json::json!({
                    "name": e.name,
                    "is_dir": e.is_dir,
                    "size": e.size,
                    "modified": e.modified,
                })
            })
            .collect(),
    ))
}

/// 下载远程节点文件
#[utoipa::path(
    get,
    path = "/api/nodes/{id}/files/download",
    tag = "nodes",
    params(("id" = i64, Path, description = "节点 ID")),
    responses(
        (status = 200, description = "文件内容（base64）"),
        (status = 401, description = "未认证"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn remote_download_file(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Query(q): Query<RemoteListQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let bytes = state.node_service.remote_download_file(id, &q.path).await?;
    Ok(Json(serde_json::json!({
        "node_id": id,
        "path": q.path,
        "size": bytes.len(),
        "content_base64": base64::Engine::encode(&base64::engine::general_purpose::STANDARD, &bytes),
    })))
}

/// 上传文件到远程节点
#[utoipa::path(
    post,
    path = "/api/nodes/{id}/files/upload",
    tag = "nodes",
    params(("id" = i64, Path, description = "节点 ID")),
    request_body = RemoteUploadRequest,
    responses(
        (status = 200, description = "上传结果"),
        (status = 401, description = "未认证"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn remote_upload_file(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    ApiJson(req): ApiJson<RemoteUploadRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    use base64::Engine;
    let content = base64::engine::general_purpose::STANDARD
        .decode(&req.content_base64)
        .map_err(|e| AppError::BadRequest(format!("Invalid base64: {}", e)))?;
    let size = state
        .node_service
        .remote_upload_file(id, &req.path, content)
        .await?;
    Ok(Json(serde_json::json!({
        "node_id": id,
        "path": req.path,
        "size": size,
    })))
}

pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    ApiJson(payload): ApiJson<CreateNodeRequest>,
) -> Result<Json<ServerNode>, AppError> {
    let mut node = payload.to_node();
    node.id = id;
    state.node_service.update_node(&node).await?;
    let updated = state.node_service.get_node(id).await?;
    Ok(Json(updated))
}

pub async fn delete(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<&'static str>, AppError> {
    state.node_service.delete_node(id).await?;
    Ok(Json("deleted"))
}

/// 路由表（集中注册于 routes.rs 组合根）
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/nodes", axum::routing::get(list))
        .route("/api/nodes", axum::routing::post(create))
        .route("/api/nodes/register", axum::routing::post(register_agent))
        .route("/api/nodes/{id}", axum::routing::put(update))
        .route("/api/nodes/{id}", axum::routing::delete(delete))
        .route("/api/nodes/heartbeat/{id}", axum::routing::post(heartbeat))
        .route("/api/nodes/{id}/status", axum::routing::get(status))
        .route("/api/nodes/{id}/metrics", axum::routing::get(metrics))
        .route(
            "/api/nodes/{id}/execute",
            axum::routing::post(remote_execute),
        )
        .route("/api/nodes/{id}/action", axum::routing::post(remote_action))
        .route(
            "/api/nodes/batch-execute",
            axum::routing::post(batch_execute),
        )
        .route(
            "/api/nodes/{id}/files",
            axum::routing::get(remote_list_files),
        )
        .route(
            "/api/nodes/{id}/files/download",
            axum::routing::get(remote_download_file),
        )
        .route(
            "/api/nodes/{id}/files/upload",
            axum::routing::post(remote_upload_file),
        )
}
