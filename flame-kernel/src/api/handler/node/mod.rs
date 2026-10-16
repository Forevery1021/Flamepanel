use crate::api::extract::ApiJson;
use crate::api::types::{AppState, CreateNodeRequest, PaginatedResponse, PaginationParams};
use crate::core::error::AppError;
use crate::domain::entity::ServerNode;
use axum::Router;
use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;
use utoipa::ToSchema;

const HEARTBEAT_TIMEOUT_SECS: i64 = 30;

/// Agent 心跳。前 4 项为旧 Agent 即有的字段；其余为 batch-12 起的完整快照字段
/// （缺省 0，旧 Agent 照常可用），并入 `MetricsHub` 统一模型后供仪表盘按节点展示。
#[derive(Debug, Deserialize, ToSchema)]
pub struct HeartbeatRequest {
    pub cpu_usage: f32,
    pub memory_usage_percent: f32,
    pub disk_usage_percent: f32,
    pub load_one: f32,
    #[serde(default)]
    pub load_five: f32,
    #[serde(default)]
    pub load_fifteen: f32,
    #[serde(default)]
    pub cpu_cores: u64,
    #[serde(default)]
    pub memory_total_mb: u64,
    #[serde(default)]
    pub memory_used_mb: u64,
    #[serde(default)]
    pub disk_total_gb: f64,
    #[serde(default)]
    pub disk_used_gb: f64,
    /// 网络接收速率（MB/s，Agent 按两次心跳间的字节差 / 实际间隔计算）
    #[serde(default)]
    pub network_rx_mbps: f64,
    /// 网络发送速率（MB/s）
    #[serde(default)]
    pub network_tx_mbps: f64,
}

impl HeartbeatRequest {
    fn to_metrics(&self) -> crate::infrastructure::node_metrics::HeartbeatMetrics {
        crate::infrastructure::node_metrics::HeartbeatMetrics {
            cpu_usage: f64::from(self.cpu_usage),
            memory_usage_percent: f64::from(self.memory_usage_percent),
            disk_usage_percent: f64::from(self.disk_usage_percent),
            load_one: f64::from(self.load_one),
            load_five: f64::from(self.load_five),
            load_fifteen: f64::from(self.load_fifteen),
            cpu_cores: self.cpu_cores,
            memory_total_mb: self.memory_total_mb,
            memory_used_mb: self.memory_used_mb,
            disk_total_gb: self.disk_total_gb,
            disk_used_gb: self.disk_used_gb,
            network_rx_mbps: self.network_rx_mbps,
            network_tx_mbps: self.network_tx_mbps,
        }
    }
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

/// Agent 心跳上报（免 JWT，Agent token 已在中间件集中校验——S15）
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
    ApiJson(req): ApiJson<HeartbeatRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    // S15：Agent token 鉴权已集中到中间件 `auth_middleware`（`is_agent_heartbeat_guarded_path`），
    // 与 register 的 bootstrap 守卫统一管理；此处不再重复校验。
    // 钳制后的统一快照：既落库为「最近一次指标」，也推入该节点的 MetricsHub
    let snapshot = crate::infrastructure::node_metrics::snapshot_from_heartbeat(
        &req.to_metrics(),
        chrono::Utc::now().timestamp_millis(),
    );
    let metrics = serde_json::to_value(&snapshot)
        .map_err(|e| AppError::internal(format!("serialize node metrics: {e}")))?;
    let node = state.node_service.record_heartbeat(id, &metrics).await?;
    state.metrics.nodes().publish(node.id, snapshot);
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
        (status = 200, description = "在线状态 `{id, status, edge: EdgeInfo}`"),
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
    // Doc/30 Agent Edge：反向通道连接状态（connected / terminal / version / connected_at）
    let edge = state.node_service.agent_client.edge().info(id);
    Ok(Json(
        serde_json::json!({ "id": id, "status": status, "edge": edge }),
    ))
}

/// 节点最近指标快照
#[utoipa::path(
    get,
    path = "/api/nodes/{id}/metrics",
    tag = "nodes",
    params(("id" = i64, Path, description = "ID")),
    responses(
        (status = 200, description = "最近一次心跳上报的指标（结构随 Agent 版本演进）"),
        (status = 401, description = "未认证"),
        (status = 403, description = "无权限"),
        (status = 404, description = "节点不存在"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn metrics(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, AppError> {
    let metrics = state.node_service.node_metrics(id).await?;
    Ok(Json(metrics))
}

/// 节点指标历史（内存窗口：Agent 10 s 一点、最多 120 点 ≈ 20 分钟；面板重启后重新积累）
#[utoipa::path(
    get,
    path = "/api/nodes/{id}/metrics/history",
    tag = "nodes",
    params(("id" = i64, Path, description = "节点 ID")),
    responses(
        (status = 200, description = "按时间升序的快照（未收到过心跳为空数组）", body = Vec<crate::domain::entity::MetricsSnapshot>),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 node:read 权限"),
        (status = 404, description = "节点不存在"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn metrics_history(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Vec<crate::domain::entity::MetricsSnapshot>>, AppError> {
    state.node_service.get_node(id).await?;
    Ok(Json(
        state
            .metrics
            .nodes()
            .hub(id)
            .map(|h| h.history())
            .unwrap_or_default(),
    ))
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
///
/// S4：统一走 Agent `/action`（`whitelisted_command`）。在 `execution_mode=agent` 下，
/// 仅执行白名单命令，非白名单命令将返回 `ACTION_NOT_ALLOWED`（功能降级）。
#[utoipa::path(
    post,
    path = "/api/nodes/{id}/execute",
    tag = "nodes",
    params(("id" = i64, Path, description = "节点 ID")),
    request_body = RemoteExecRequest,
    responses(
        (status = 200, description = "执行结果（agent 模式下受命令白名单约束）"),
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
///
/// S4：同样走 Agent `/action`（`whitelisted_command`），agent 模式下仅执行白名单命令。
#[utoipa::path(
    post,
    path = "/api/nodes/batch-execute",
    tag = "nodes",
    request_body = RemoteBatchExecRequest,
    responses(
        (status = 200, description = "各节点执行结果聚合（agent 模式下受命令白名单约束）"),
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

/// 更新节点
#[utoipa::path(
    put,
    path = "/api/nodes/{id}",
    tag = "nodes",
    params(("id" = i64, Path, description = "ID")),
    request_body = CreateNodeRequest,
    responses(
        (status = 200, description = "更新后的节点", body = ServerNode),
        (status = 401, description = "未认证"),
        (status = 403, description = "无权限"),
        (status = 404, description = "节点不存在"),
    ),
    security(("BearerAuth" = []))
)]
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

/// 删除节点
#[utoipa::path(
    delete,
    path = "/api/nodes/{id}",
    tag = "nodes",
    params(("id" = i64, Path, description = "ID")),
    responses(
        (status = 200, description = "删除成功", body = String),
        (status = 401, description = "未认证"),
        (status = 403, description = "无权限"),
        (status = 404, description = "节点不存在"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn delete(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<&'static str>, AppError> {
    state.node_service.delete_node(id).await?;
    state.metrics.nodes().remove(id);
    state.node_service.agent_client.edge().remove_node(id);
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
            "/api/nodes/{id}/metrics/history",
            axum::routing::get(metrics_history),
        )
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
