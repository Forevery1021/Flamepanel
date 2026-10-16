//! 用户项目 REST（M-B 主线二 Stage A-3 · Doc/20 §5.6）。
//!
//! 端点组：`/api/projects`（CRUD + deploy/start/stop/restart/status/logs）
//! 与 `/api/runtimes`（Stage A-4 一并落位：列表/可用版本/卸载/设默认；
//! 运行时安装走 `RuntimeService`，长耗时沿用其状态机）。

use crate::api::extract::ApiJson;
use crate::api::types::AppState;
use crate::application::runtime_service::RuntimeVersionInfo;
use crate::core::error::AppError;
use crate::domain::entity::Project;
use axum::{
    extract::{Path, Query, State},
    routing::{get, post},
    Json, Router,
};
use serde::Deserialize;
use utoipa::IntoParams;

fn svc(state: &AppState) -> &crate::application::project_service::ProjectService {
    &state.services.project_service
}

fn project_response(p: &Project) -> ProjectDto {
    ProjectDto {
        has_webhook_token: !p.webhook_token.is_empty(),
        has_procfile: p.procfile_json.is_some(),
        build_type: p.build_type.clone(),
        health_check_path: p.health_check_path.clone(),
        deploy_strategy: p.deploy_strategy.clone(),
        id: p.id,
        name: p.name.clone(),
        runtime_id: p.runtime_id,
        kind: p.kind.clone(),
        source: p.source.clone(),
        source_url: p.source_url.clone(),
        deploy_path: p.deploy_path.clone(),
        run_command: p.run_command.clone(),
        port: p.port,
        domain: p.domain.clone(),
        status: p.status.clone(),
        process_unit: p.process_unit.clone(),
        deploy_type: p.deploy_type.clone(),
        auto_deploy: p.auto_deploy,
        created_at: p.created_at,
        updated_at: p.updated_at,
    }
}

/// 项目响应 DTO。
#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct ProjectDto {
    pub id: i64,
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub runtime_id: Option<i64>,
    /// php | jar | war | node | python | go | static
    pub kind: String,
    /// local_path | git
    pub source: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_url: Option<String>,
    pub deploy_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub run_command: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub port: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
    /// building | running | stopped | error
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub process_unit: Option<String>,
    /// 部署方式：manual | webhook
    pub deploy_type: String,
    /// webhook 自动部署开关
    pub auto_deploy: bool,
    /// 是否已配置部署令牌（令牌明文不回显，enable 时一次性返回）
    pub has_webhook_token: bool,
    /// 是否声明了 Procfile 进程模型（Stage A-8 · Doc/20 §5.8 ③）
    pub has_procfile: bool,
    /// 构建器协商结果：dockerfile | buildpacks | runtime（Stage A-8 · Doc/20 §5.8 ②）
    pub build_type: String,
    /// 健康检查路径（空串 = 未启用，Stage A-9 · Doc/20 §5.8 ④）
    pub health_check_path: String,
    /// 切换策略：restart（直接重启）| rolling（健康检查零停机）
    pub deploy_strategy: String,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Debug, Deserialize, IntoParams, utoipa::ToSchema)]
pub struct AvailableQuery {
    /// 语言种类：php | java | node | python | go
    pub kind: String,
}

/// 开启 webhook 部署响应（令牌明文仅此一次返回）。
#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct WebhookDeployDto {
    pub project: ProjectDto,
    /// 部署端点：/api/projects/:id/webhook?token=<token>
    pub webhook_url: String,
    /// 部署令牌明文（仅本次响应返回，之后不可再获取）
    pub token: String,
}

/// 创建项目请求。
#[derive(Debug, serde::Deserialize, utoipa::ToSchema)]
pub struct CreateProjectRequest {
    pub name: String,
    #[serde(default)]
    pub runtime_id: Option<i64>,
    /// php | jar | war | node | python | go | static
    pub kind: String,
    /// local_path | git
    pub source: String,
    #[serde(default)]
    pub source_url: Option<String>,
    /// 留空自动取 /opt/flamepanel/projects/<name>
    #[serde(default)]
    pub deploy_path: Option<String>,
    /// 启动命令（模板变量 $RUNTIME / $DEPLOY 预留，当前原样使用）
    #[serde(default)]
    pub run_command: Option<String>,
    #[serde(default)]
    pub port: Option<i64>,
    #[serde(default)]
    pub domain: Option<String>,
    /// 健康检查路径（如 /health；空 = 未启用）
    #[serde(default)]
    pub health_check_path: Option<String>,
    /// 切换策略：restart | rolling（默认 restart）
    #[serde(default)]
    pub deploy_strategy: Option<String>,
}

/// 更新项目请求（全量可编辑字段）。
#[derive(Debug, serde::Deserialize, utoipa::ToSchema)]
pub struct UpdateProjectRequest {
    #[serde(default)]
    pub runtime_id: Option<i64>,
    pub kind: String,
    pub source: String,
    #[serde(default)]
    pub source_url: Option<String>,
    #[serde(default)]
    pub deploy_path: Option<String>,
    #[serde(default)]
    pub run_command: Option<String>,
    #[serde(default)]
    pub port: Option<i64>,
    #[serde(default)]
    pub domain: Option<String>,
    #[serde(default)]
    pub health_check_path: Option<String>,
    #[serde(default)]
    pub deploy_strategy: Option<String>,
}

/// 运行时响应 DTO。
#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct RuntimeDto {
    pub id: i64,
    /// php | java | node | python | go
    pub kind: String,
    pub version: String,
    pub install_path: String,
    /// not_installed | installing | installed | running | error
    pub status: String,
    pub is_default: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub metadata_json: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
}

fn runtime_response(r: &crate::domain::entity::Runtime) -> RuntimeDto {
    RuntimeDto {
        id: r.id,
        kind: r.kind.clone(),
        version: r.version.clone(),
        install_path: r.install_path.clone(),
        status: r.status.clone(),
        is_default: r.is_default,
        metadata_json: r.metadata_json.clone(),
        created_at: r.created_at,
        updated_at: r.updated_at,
    }
}

/// 创建运行时安装请求。
#[derive(Debug, serde::Deserialize, utoipa::ToSchema)]
pub struct InstallRuntimeRequest {
    /// php | java | node | python | go
    pub kind: String,
    /// 版本号（如 8.2 / 21 / 20）。Node 可写主版本（`22`）或次版本（`22.11`），
    /// 安装时按镜像索引解析到最新补丁版（解析结果见 `metadata_json.version`）
    pub version: String,
    /// 仅 Node：安装后同时执行 `npm install -g pm2`（其余 kind 传 true 返回 400）
    #[serde(default)]
    pub pm2: Option<bool>,
}

// ─── 项目端点 ───────────────────────────────────────────────────────────

/// 项目列表
#[utoipa::path(
    get,
    path = "/api/projects",
    tag = "projects",
    responses(
        (status = 200, description = "项目列表", body = Vec<ProjectDto>),
        (status = 401, description = "未认证"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn list_projects(
    State(state): State<AppState>,
) -> Result<Json<Vec<ProjectDto>>, AppError> {
    let projects = svc(&state).list_projects().await?;
    Ok(Json(projects.iter().map(project_response).collect()))
}

/// 项目详情
#[utoipa::path(
    get,
    path = "/api/projects/{id}",
    tag = "projects",
    params(("id" = i64, Path, description = "项目 ID")),
    responses(
        (status = 200, description = "项目详情", body = ProjectDto),
        (status = 404, description = "项目不存在"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn get_project(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<ProjectDto>, AppError> {
    let p = svc(&state).get_project(id).await?;
    Ok(Json(project_response(&p)))
}

/// 创建项目
#[utoipa::path(
    post,
    path = "/api/projects",
    tag = "projects",
    request_body = CreateProjectRequest,
    responses(
        (status = 200, description = "创建成功", body = ProjectDto),
        (status = 400, description = "参数校验失败"),
        (status = 409, description = "项目重名"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn create_project(
    State(state): State<AppState>,
    ApiJson(req): ApiJson<CreateProjectRequest>,
) -> Result<Json<ProjectDto>, AppError> {
    let p = Project {
        name: req.name,
        runtime_id: req.runtime_id,
        kind: req.kind,
        source: req.source,
        source_url: req.source_url,
        deploy_path: req.deploy_path.unwrap_or_default(),
        run_command: req.run_command,
        port: req.port,
        domain: req.domain,
        health_check_path: req.health_check_path.unwrap_or_default(),
        deploy_strategy: req.deploy_strategy.unwrap_or_else(|| "restart".into()),
        ..Project::default()
    };
    let created = svc(&state).create_project(p).await?;
    Ok(Json(project_response(&created)))
}

/// 更新项目
#[utoipa::path(
    put,
    path = "/api/projects/{id}",
    tag = "projects",
    params(("id" = i64, Path, description = "项目 ID")),
    request_body = UpdateProjectRequest,
    responses(
        (status = 200, description = "更新成功", body = ProjectDto),
        (status = 404, description = "项目不存在"),
        (status = 409, description = "运行中禁止修改启动命令"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn update_project(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    ApiJson(req): ApiJson<UpdateProjectRequest>,
) -> Result<Json<ProjectDto>, AppError> {
    let existing = svc(&state).get_project(id).await?;
    let mut p = existing.clone();
    p.runtime_id = req.runtime_id;
    p.kind = req.kind;
    p.source = req.source;
    p.source_url = req.source_url;
    if let Some(dp) = req.deploy_path {
        p.deploy_path = dp;
    }
    p.run_command = req.run_command;
    p.port = req.port;
    p.domain = req.domain;
    if let Some(hp) = req.health_check_path {
        p.health_check_path = hp;
    }
    if let Some(ds) = req.deploy_strategy {
        p.deploy_strategy = ds;
    }
    let updated = svc(&state).update_project(&p).await?;
    Ok(Json(project_response(&updated)))
}

/// 删除项目（运行中自动停止；部署目录保留——零锁定纪律）
#[utoipa::path(
    delete,
    path = "/api/projects/{id}",
    tag = "projects",
    params(("id" = i64, Path, description = "项目 ID")),
    responses(
        (status = 200, description = "删除成功"),
        (status = 404, description = "项目不存在"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn delete_project(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, AppError> {
    svc(&state).delete_project(id).await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

/// 部署项目（git clone / local_path 校验；部署 ≠ 启动）
#[utoipa::path(
    post,
    path = "/api/projects/{id}/deploy",
    tag = "projects",
    params(("id" = i64, Path, description = "项目 ID")),
    responses(
        (status = 200, description = "部署成功", body = ProjectDto),
        (status = 400, description = "来源校验失败"),
        (status = 409, description = "项目操作互斥"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn deploy_project(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<ProjectDto>, AppError> {
    let p = svc(&state).deploy(id).await?;
    Ok(Json(project_response(&p)))
}

/// 启动项目
#[utoipa::path(
    post,
    path = "/api/projects/{id}/start",
    tag = "projects",
    params(("id" = i64, Path, description = "项目 ID")),
    responses(
        (status = 200, description = "启动成功", body = ProjectDto),
        (status = 400, description = "未配置启动命令"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn start_project(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<ProjectDto>, AppError> {
    let p = svc(&state).start(id).await?;
    Ok(Json(project_response(&p)))
}

/// 停止项目
#[utoipa::path(
    post,
    path = "/api/projects/{id}/stop",
    tag = "projects",
    params(("id" = i64, Path, description = "项目 ID")),
    responses((status = 200, description = "停止成功", body = ProjectDto)),
    security(("BearerAuth" = []))
)]
pub async fn stop_project(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<ProjectDto>, AppError> {
    let p = svc(&state).stop(id).await?;
    Ok(Json(project_response(&p)))
}

/// 重启项目
#[utoipa::path(
    post,
    path = "/api/projects/{id}/restart",
    tag = "projects",
    params(("id" = i64, Path, description = "项目 ID")),
    responses((status = 200, description = "重启成功", body = ProjectDto)),
    security(("BearerAuth" = []))
)]
pub async fn restart_project(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<ProjectDto>, AppError> {
    let p = svc(&state).restart(id).await?;
    Ok(Json(project_response(&p)))
}

/// 实时进程状态（按执行面分流：容器模式 docker inspect / 原生模式守护器探测）
#[utoipa::path(
    get,
    path = "/api/projects/{id}/status",
    tag = "projects",
    params(("id" = i64, Path, description = "项目 ID")),
    responses(
        (status = 200, description = "进程状态（容器模式自容器探测；两路同构）", body = crate::application::project_ports::ProcessStatus),
        (status = 404, description = "项目不存在"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn project_process_status(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<crate::application::project_ports::ProcessStatus>, AppError> {
    Ok(Json(svc(&state).process_status(id).await?))
}

/// 最近日志（按执行面分流：容器模式 docker logs / 原生模式守护器日志）
#[utoipa::path(
    get,
    path = "/api/projects/{id}/logs",
    tag = "projects",
    params(("id" = i64, Path, description = "项目 ID")),
    responses((status = 200, description = "日志列表（容器模式取 docker logs）", body = Vec<crate::application::project_ports::LogLine>)),
    security(("BearerAuth" = []))
)]
pub async fn project_logs(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Vec<crate::application::project_ports::LogLine>>, AppError> {
    Ok(Json(svc(&state).logs(id, 200).await?))
}

// ─── 运行时端点（Stage A-4 一并落位） ────────────────────────────────────

/// 运行时列表
#[utoipa::path(
    get,
    path = "/api/runtimes",
    tag = "runtimes",
    responses((status = 200, description = "运行时列表", body = Vec<RuntimeDto>)),
    security(("BearerAuth" = []))
)]
pub async fn list_runtimes(
    State(state): State<AppState>,
) -> Result<Json<Vec<RuntimeDto>>, AppError> {
    let list = state.services.runtime_service.list_runtimes().await?;
    Ok(Json(list.iter().map(runtime_response).collect()))
}

/// 按语言过滤运行时
#[utoipa::path(
    get,
    path = "/api/runtimes/kind/{kind}",
    tag = "runtimes",
    params(("kind" = String, Path, description = "语言种类")),
    responses((status = 200, description = "该语言运行时列表", body = Vec<RuntimeDto>)),
    security(("BearerAuth" = []))
)]
pub async fn list_runtimes_by_kind(
    State(state): State<AppState>,
    Path(kind): Path<String>,
) -> Result<Json<Vec<RuntimeDto>>, AppError> {
    let list = state.services.runtime_service.list_by_kind(&kind).await?;
    Ok(Json(list.iter().map(runtime_response).collect()))
}

/// 可安装版本清单（声明版本 × 系统探测）
#[utoipa::path(
    get,
    path = "/api/runtimes/available",
    tag = "runtimes",
    params(AvailableQuery),
    responses(
        (status = 200, description = "版本清单", body = Vec<RuntimeVersionInfo>),
        (status = 400, description = "不支持的语言"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn available_runtimes(
    State(state): State<AppState>,
    Query(q): Query<AvailableQuery>,
) -> Result<Json<Vec<RuntimeVersionInfo>>, AppError> {
    // 声明版本清单（与 LanguageRuntimeSpec 包名映射对齐；Stage A-2 先落 PHP）
    let declared: &[&str] = match q.kind.as_str() {
        "php" => &["8.1", "8.2", "8.3"],
        // 按大 / 次版本声明，安装时解析到最新补丁版（M-C 收尾：Java 为 Temurin LTS）
        "java" => &["8", "11", "17", "21", "25"],
        "node" => &["20", "22", "24", "26"],
        "python" => &["3.10", "3.11", "3.12", "3.13", "3.14"],
        "go" => &["1.25", "1.26", "1.27"],
        "tomcat" => &["9.0.98", "10.1.34", "11.0.2"],
        _ => {
            return Err(AppError::ValidationError(format!(
                "不支持的语言运行时: {:?}",
                q.kind
            )))
        }
    };
    let list = state
        .services
        .runtime_service
        .available(&q.kind, declared)
        .await?;
    Ok(Json(list))
}

/// 安装运行时
#[utoipa::path(
    post,
    path = "/api/runtimes/install",
    tag = "runtimes",
    request_body = InstallRuntimeRequest,
    responses(
        (status = 200, description = "安装成功（或已登记并安装中）", body = RuntimeDto),
        (status = 400, description = "参数校验失败"),
        (status = 409, description = "已安装"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn install_runtime(
    State(state): State<AppState>,
    ApiJson(req): ApiJson<InstallRuntimeRequest>,
) -> Result<Json<RuntimeDto>, AppError> {
    let r = state
        .services
        .runtime_service
        .install_runtime_with_options(
            &req.kind,
            &req.version,
            crate::application::runtime_service::InstallOptions {
                pm2: req.pm2.unwrap_or(false),
            },
        )
        .await?;
    Ok(Json(runtime_response(&r)))
}

/// 卸载运行时
#[utoipa::path(
    delete,
    path = "/api/runtimes/{id}",
    tag = "runtimes",
    params(("id" = i64, Path, description = "运行时 ID")),
    responses((status = 200, description = "卸载成功")),
    security(("BearerAuth" = []))
)]
pub async fn uninstall_runtime(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, AppError> {
    state.services.runtime_service.uninstall_runtime(id).await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

/// 设为同语言默认版本
#[utoipa::path(
    post,
    path = "/api/runtimes/{id}/default",
    tag = "runtimes",
    params(("id" = i64, Path, description = "运行时 ID")),
    responses(
        (status = 200, description = "设置成功", body = RuntimeDto),
        (status = 400, description = "未安装"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn set_default_runtime(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<RuntimeDto>, AppError> {
    state.services.runtime_service.set_default(id).await?;
    let r = state.services.runtime_service.get_runtime(id).await?;
    Ok(Json(runtime_response(&r)))
}

/// 按运行时列出绑定站点（M-B 主线二 Stage A-6 · Doc/20 §5.5）
#[utoipa::path(
    get,
    path = "/api/runtimes/{id}/sites",
    tag = "runtimes",
    operation_id = "list_runtime_sites",
    params(("id" = i64, Path, description = "运行时 ID")),
    responses(
        (status = 200, description = "绑定该运行时的站点列表", body = [WebsiteDto]),
        (status = 404, description = "运行时不存在"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn list_runtime_sites(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Vec<WebsiteDto>>, AppError> {
    // 运行时存在性校验
    state
        .services
        .runtime_service
        .get_runtime(id)
        .await
        .map_err(|_| AppError::NotFound(format!("运行时不存在: {}", id)))?;
    let sites = state
        .services
        .website_service
        .list_sites_by_runtime(id)
        .await?;
    Ok(Json(sites.iter().map(website_response).collect()))
}

/// 开启/重置项目 webhook 部署（返回一次性部署令牌）
#[utoipa::path(
    post,
    path = "/api/projects/{id}/webhook/enable",
    tag = "projects",
    params(("id" = i64, Path, description = "项目 ID")),
    responses(
        (status = 200, description = "已开启（重复调用轮换令牌）", body = WebhookDeployDto),
        (status = 400, description = "非 git 来源项目"),
        (status = 404, description = "项目不存在"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn enable_project_webhook(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<WebhookDeployDto>, AppError> {
    let p = svc(&state).enable_webhook(id).await?;
    let dto = project_response(&p);
    let token = p.webhook_token.clone();
    Ok(Json(WebhookDeployDto {
        webhook_url: format!("/api/projects/{}/webhook?token={}", id, token),
        token,
        project: dto,
    }))
}

/// 关闭项目 webhook 部署
#[utoipa::path(
    post,
    path = "/api/projects/{id}/webhook/disable",
    tag = "projects",
    params(("id" = i64, Path, description = "项目 ID")),
    responses(
        (status = 200, description = "已关闭", body = ProjectDto),
        (status = 404, description = "项目不存在"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn disable_project_webhook(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<ProjectDto>, AppError> {
    let p = svc(&state).disable_webhook(id).await?;
    Ok(Json(project_response(&p)))
}

/// Webhook push 触发部署（免 JWT，令牌经中间件校验；
/// 兼容 GitHub / GitLab / Gitee / Gitea / 通用 JSON——服务端不解析平台语义，
/// 任何 POST 命中即视为一次 push 部署请求）
#[utoipa::path(
    post,
    path = "/api/projects/{id}/webhook",
    tag = "projects",
    params(("id" = i64, Path, description = "项目 ID")),
    request_body = Option<serde_json::Value>,
    responses(
        (status = 200, description = "部署已触发", body = ProjectDto),
        (status = 401, description = "令牌缺失/无效"),
        (status = 404, description = "项目未启用 webhook"),
        (status = 409, description = "同项目操作互斥"),
    ),
)]
pub async fn project_webhook_deploy(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<ProjectDto>, AppError> {
    // 令牌校验已在 auth_middleware 的 webhook 守卫内完成
    let p = svc(&state).deploy_by_webhook(id).await?;
    Ok(Json(project_response(&p)))
}

/// Procfile 进程声明（Stage A-8 · Doc/20 §5.8 ③，ps:scale 伸缩面板数据源）。
#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct ProcfileDto {
    pub project_id: i64,
    /// 进程类型 → 启动命令（web/worker/clock；无 Procfile 为空表）
    pub processes: Vec<ProcEntry>,
}

/// 单个进程类型条目。
#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct ProcEntry {
    /// 进程类型：web | worker | clock
    pub proc_type: String,
    /// 启动命令
    pub command: String,
}

/// 项目构建器决策（Stage A-8 · Doc/20 §5.8 ②）。
#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct BuilderDecisionDto {
    pub project_id: i64,
    /// 当前协商出的构建器：dockerfile | buildpacks | runtime
    pub build_type: String,
    /// 容器模式本地镜像名（其余构建器为 None）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub image: Option<String>,
}

/// 查看项目构建器决策（部署时自动协商结果，Doc/20 §5.8 ②）
#[utoipa::path(
    get,
    path = "/api/projects/{id}/builder",
    tag = "projects",
    params(("id" = i64, Path, description = "项目 ID")),
    responses(
        (status = 200, description = "构建器决策", body = BuilderDecisionDto),
        (status = 404, description = "项目不存在"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn get_project_builder(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<BuilderDecisionDto>, AppError> {
    let decision = svc(&state).builder_decision(id).await?;
    Ok(Json(BuilderDecisionDto {
        project_id: id,
        build_type: decision.build_type,
        image: decision.image,
    }))
}

/// 查看项目 Procfile 进程声明
#[utoipa::path(
    get,
    path = "/api/projects/{id}/procfile",
    tag = "projects",
    params(("id" = i64, Path, description = "项目 ID")),
    responses(
        (status = 200, description = "进程声明（无 Procfile 为空表）", body = ProcfileDto),
        (status = 404, description = "项目不存在"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn get_project_procfile(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<ProcfileDto>, AppError> {
    let p = svc(&state).get_project(id).await?;
    Ok(Json(ProcfileDto {
        project_id: p.id,
        processes: p
            .procfile_processes()
            .into_iter()
            .map(|(proc_type, command)| ProcEntry { proc_type, command })
            .collect(),
    }))
}

#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct WebsiteDto {
    pub id: i64,
    pub name: String,
    pub domain: String,
    pub root_path: String,
    pub status: String,
    pub node_id: i64,
    pub engine: String,
    pub ssl_enabled: bool,
    pub proxy_enabled: bool,
    pub proxy_pass: Option<String>,
    /// 站点类型：runtime_php | reverse_proxy | static | container_app
    pub site_type: String,
    pub runtime_id: Option<i64>,
    pub project_id: Option<i64>,
    pub created_at: String,
}

fn website_response(w: &crate::domain::entity::Website) -> WebsiteDto {
    WebsiteDto {
        id: w.id,
        name: w.name.clone(),
        domain: w.domain.clone(),
        root_path: w.root_path.clone(),
        status: w.status.clone(),
        node_id: w.node_id,
        engine: w.engine.clone(),
        ssl_enabled: w.ssl_enabled,
        proxy_enabled: w.proxy_enabled,
        proxy_pass: w.proxy_pass.clone(),
        site_type: w.site_type.clone(),
        runtime_id: w.runtime_id,
        project_id: w.project_id,
        created_at: w.created_at.to_rfc3339(),
    }
}

/// 手动健康探测（Stage A-9 · Doc/20 §5.8 ④）
#[utoipa::path(
    post,
    path = "/api/projects/{id}/health-check",
    tag = "projects",
    params(("id" = i64, Path, description = "项目 ID")),
    responses(
        (status = 200, description = "探测结果", body = HealthCheckDto),
        (status = 400, description = "未配置健康检查路径或端口"),
        (status = 503, description = "健康探测器不可用"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn project_health_check(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<HealthCheckDto>, AppError> {
    let healthy = svc(&state).check_health(id).await?;
    Ok(Json(HealthCheckDto {
        project_id: id,
        healthy,
    }))
}

/// 健康探测结果 DTO。
#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct HealthCheckDto {
    pub project_id: i64,
    /// 探测窗口内是否健康
    pub healthy: bool,
}

// ─── 服务联链（Stage A-10 · Doc/20 §5.8 ⑤，Dokku link 模式）──────────────

/// 可关联的数据库实例（联链弹窗数据源）。
#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct LinkableDatabaseDto {
    pub id: i64,
    pub db_type: String,
    pub name: String,
    pub port: i32,
    pub status: String,
    /// 推荐注入的环境变量名（DATABASE_URL / MYSQL_URL / REDIS_URL）
    pub default_env_name: String,
}

/// 联链摘要（不含连接串）。
#[derive(Debug, serde::Serialize, utoipa::ToSchema)]
pub struct ProjectDbLinkDto {
    pub id: i64,
    pub project_id: i64,
    pub db_instance_id: i64,
    pub env_name: String,
    pub db_type: String,
    pub db_instance_name: String,
    pub port: i32,
}

/// 关联数据库请求。
#[derive(Debug, serde::Deserialize, utoipa::ToSchema)]
pub struct LinkDatabaseRequest {
    /// 数据库实例 ID（/api/databases 返回的实例）
    pub db_instance_id: i64,
    /// 注入的环境变量名（留空按 db_type 推导默认值）
    #[serde(default)]
    pub env_name: Option<String>,
    /// 业务库名（留空取项目名；redis 忽略）
    #[serde(default)]
    pub db_name: Option<String>,
}

fn link_summary_response(l: crate::application::project_service::LinkSummary) -> ProjectDbLinkDto {
    ProjectDbLinkDto {
        id: l.id,
        project_id: l.project_id,
        db_instance_id: l.db_instance_id,
        env_name: l.env_name,
        db_type: l.db_type,
        db_instance_name: l.db_instance_name,
        port: l.port,
    }
}

/// 项目的联链列表
#[utoipa::path(
    get,
    path = "/api/projects/{id}/links",
    tag = "projects",
    params(("id" = i64, Path, description = "项目 ID")),
    responses(
        (status = 200, description = "联链列表（不含连接串）", body = Vec<ProjectDbLinkDto>),
        (status = 404, description = "项目不存在"),
        (status = 503, description = "联链组件未启用"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn list_project_links(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Vec<ProjectDbLinkDto>>, AppError> {
    let links = svc(&state).list_links(id).await?;
    Ok(Json(links.into_iter().map(link_summary_response).collect()))
}

/// 可关联的数据库实例清单（联链弹窗数据源；仅返回 running 实例）。
#[utoipa::path(
    get,
    path = "/api/projects/{id}/links/available",
    tag = "projects",
    params(("id" = i64, Path, description = "项目 ID")),
    responses(
        (status = 200, description = "可关联实例清单", body = Vec<LinkableDatabaseDto>),
        (status = 404, description = "项目不存在"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn list_linkable_databases(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Vec<LinkableDatabaseDto>>, AppError> {
    svc(&state).get_project(id).await?;
    let instances = state.services.database_service.list_instances().await?;
    Ok(Json(
        instances
            .into_iter()
            .filter(|i| i.status == "running")
            .map(|i| LinkableDatabaseDto {
                default_env_name: match i.db_type.as_str() {
                    "mysql" | "mariadb" => "MYSQL_URL".into(),
                    "postgres" => "DATABASE_URL".into(),
                    "redis" => "REDIS_URL".into(),
                    "mongodb" => "MONGODB_URI".into(),
                    _ => "DATABASE_URL".into(),
                },
                id: i.id,
                db_type: i.db_type,
                name: i.name,
                port: i.port,
                status: i.status,
            })
            .collect(),
    ))
}

/// 关联数据库（联链成功即在建库侧幂等创建业务库；连接串在启动时注入）。
#[utoipa::path(
    post,
    path = "/api/projects/{id}/links",
    tag = "projects",
    params(("id" = i64, Path, description = "项目 ID")),
    request_body = LinkDatabaseRequest,
    responses(
        (status = 200, description = "关联成功", body = ProjectDbLinkDto),
        (status = 400, description = "参数校验失败 / 实例未运行"),
        (status = 404, description = "项目或数据库实例不存在"),
        (status = 409, description = "环境变量名冲突或已关联该实例"),
        (status = 503, description = "联链组件未启用"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn link_project_database(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    ApiJson(req): ApiJson<LinkDatabaseRequest>,
) -> Result<Json<ProjectDbLinkDto>, AppError> {
    let link = svc(&state)
        .link_database(id, req.db_instance_id, req.env_name, req.db_name)
        .await?;
    // 复用 list_links 的摘要组装（回查实例元数据）
    let links = svc(&state).list_links(id).await?;
    let summary = links
        .into_iter()
        .find(|l| l.id == link.id)
        .ok_or_else(|| AppError::internal("联链创建后回查失败"))?;
    Ok(Json(link_summary_response(summary)))
}

/// 解除联链
#[utoipa::path(
    delete,
    path = "/api/projects/{id}/links/{link_id}",
    tag = "projects",
    params(
        ("id" = i64, Path, description = "项目 ID"),
        ("link_id" = i64, Path, description = "联链 ID"),
    ),
    responses(
        (status = 200, description = "解除成功", body = str),
        (status = 404, description = "项目或联链不存在"),
        (status = 503, description = "联链组件未启用"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn unlink_project_database(
    State(state): State<AppState>,
    Path((id, link_id)): Path<(i64, i64)>,
) -> Result<Json<&'static str>, AppError> {
    svc(&state).unlink_database(id, link_id).await?;
    Ok(Json("unlinked"))
}

/// 路由表（集中注册于 routes.rs 组合根）
pub fn routes() -> Router<AppState> {
    Router::new()
        // projects
        .route("/api/projects", get(list_projects).post(create_project))
        .route(
            "/api/projects/{id}",
            get(get_project).put(update_project).delete(delete_project),
        )
        .route("/api/projects/{id}/deploy", post(deploy_project))
        .route("/api/projects/{id}/start", post(start_project))
        .route("/api/projects/{id}/stop", post(stop_project))
        .route("/api/projects/{id}/restart", post(restart_project))
        .route("/api/projects/{id}/status", get(project_process_status))
        .route("/api/projects/{id}/procfile", get(get_project_procfile))
        .route(
            "/api/projects/{id}/health-check",
            post(project_health_check),
        )
        .route("/api/projects/{id}/builder", get(get_project_builder))
        // 服务联链（Stage A-10 · Doc/20 §5.8 ⑤）
        .route(
            "/api/projects/{id}/links",
            get(list_project_links).post(link_project_database),
        )
        .route(
            "/api/projects/{id}/links/available",
            get(list_linkable_databases),
        )
        .route(
            "/api/projects/{id}/links/{link_id}",
            axum::routing::delete(unlink_project_database),
        )
        .route("/api/projects/{id}/logs", get(project_logs))
        // webhook 部署通道（Stage A-7 · Doc/20 §5.8）
        .route("/api/projects/{id}/webhook", post(project_webhook_deploy))
        .route(
            "/api/projects/{id}/webhook/enable",
            post(enable_project_webhook),
        )
        .route(
            "/api/projects/{id}/webhook/disable",
            post(disable_project_webhook),
        )
        // runtimes
        .route("/api/runtimes", get(list_runtimes))
        .route("/api/runtimes/available", get(available_runtimes))
        .route("/api/runtimes/install", post(install_runtime))
        .route(
            "/api/runtimes/{id}",
            get(crate::api::handler::project::get_runtime).delete(uninstall_runtime),
        )
        .route("/api/runtimes/{id}/default", post(set_default_runtime))
        .route(
            "/api/runtimes/{id}/sites",
            get(crate::api::handler::project::list_runtime_sites),
        )
        .route("/api/runtimes/kind/{kind}", get(list_runtimes_by_kind))
}

/// 运行时详情（GET /api/runtimes/{id}）
#[utoipa::path(
    get,
    path = "/api/runtimes/{id}",
    tag = "runtimes",
    params(("id" = i64, Path, description = "运行时 ID")),
    responses(
        (status = 200, description = "运行时详情", body = RuntimeDto),
        (status = 404, description = "运行时不存在"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn get_runtime(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<RuntimeDto>, AppError> {
    let r = state.services.runtime_service.get_runtime(id).await?;
    Ok(Json(runtime_response(&r)))
}
