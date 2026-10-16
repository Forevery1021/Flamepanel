use crate::api::extract::ApiJson;
use crate::api::types::{AppState, CreateWebsiteRequest, PaginatedResponse, PaginationParams};
use crate::application::website_service::SiteRewriteStatus;
use crate::core::error::AppError;
use crate::domain::entity::Website;
use crate::webserver::engine::WebServerEngine;
use axum::Router;
use axum::{
    extract::{Path, Query, State},
    Json,
};

/// 网站列表（分页）
#[utoipa::path(
    get,
    path = "/api/websites",
    tag = "websites",
    operation_id = "list_websites",
    params(PaginationParams),
    responses(
        (status = 200, description = "网站列表", body = PaginatedResponse<Website>),
        (status = 401, description = "未认证"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn list(
    State(state): State<AppState>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<PaginatedResponse<Website>>, AppError> {
    let result = state
        .website_service
        .list_websites_paginated(&params)
        .await?;
    Ok(Json(result))
}

/// 获取网站详情
#[utoipa::path(
    get,
    path = "/api/websites/{id}",
    tag = "websites",
    operation_id = "get_website",
    params(("id" = i64, Path, description = "网站 ID")),
    responses(
        (status = 200, description = "网站详情", body = Website),
        (status = 404, description = "网站不存在"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn get(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Website>, AppError> {
    let website = state.website_service.get_website(id).await?;
    Ok(Json(website))
}

/// 创建网站
#[utoipa::path(
    post,
    path = "/api/websites",
    tag = "websites",
    operation_id = "create_website",
    request_body = CreateWebsiteRequest,
    responses(
        (status = 200, description = "创建成功，返回网站 ID", body = i64),
        (status = 400, description = "参数错误"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn create(
    State(state): State<AppState>,
    ApiJson(payload): ApiJson<CreateWebsiteRequest>,
) -> Result<Json<i64>, AppError> {
    let id = state
        .website_service
        .create_website(&payload.website)
        .await?;
    Ok(Json(id))
}

/// 更新网站
#[utoipa::path(
    put,
    path = "/api/websites/{id}",
    tag = "websites",
    operation_id = "update_website",
    params(("id" = i64, Path, description = "网站 ID")),
    request_body = CreateWebsiteRequest,
    responses(
        (status = 200, description = "更新成功", body = Website),
        (status = 404, description = "网站不存在"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    ApiJson(payload): ApiJson<CreateWebsiteRequest>,
) -> Result<Json<Website>, AppError> {
    let mut website = payload.website;
    website.id = id;
    state.website_service.update_website(&website).await?;
    let updated = state.website_service.get_website(id).await?;
    Ok(Json(updated))
}

/// 删除网站
#[utoipa::path(
    delete,
    path = "/api/websites/{id}",
    tag = "websites",
    operation_id = "delete_website",
    params(("id" = i64, Path, description = "网站 ID")),
    responses(
        (status = 200, description = "删除成功"),
        (status = 404, description = "网站不存在"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn delete(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<&'static str>, AppError> {
    state.website_service.delete_website(id).await?;
    Ok(Json("deleted"))
}
/// 站点绑定请求（M-B 主线二 Stage A-6 · Doc/20 §5.5）
#[derive(Debug, serde::Deserialize, utoipa::ToSchema)]
pub struct BindRuntimeRequest {
    /// 站点类型：runtime_php | reverse_proxy | static | container_app
    pub site_type: String,
    /// 绑定的运行时 ID（runtime_php 场景）
    #[serde(default)]
    pub runtime_id: Option<i64>,
    /// 绑定的项目 ID（reverse_proxy + 项目场景）
    #[serde(default)]
    pub project_id: Option<i64>,
}

/// 伪静态模板套用请求
#[derive(Debug, serde::Deserialize, utoipa::ToSchema)]
pub struct ApplyRewriteRequest {
    /// 模板名：wordpress | discuz | typecho | laravel | thinkphp | springboot
    pub preset: String,
}

/// 绑定/解绑运行时（解绑传 site_type=static 且不传 id）
#[utoipa::path(
    post,
    path = "/api/websites/{id}/runtime",
    tag = "websites",
    operation_id = "bind_website_runtime",
    params(("id" = i64, Path, description = "网站 ID")),
    request_body = BindRuntimeRequest,
    responses(
        (status = 200, description = "绑定成功", body = Website),
        (status = 400, description = "参数错误/运行时或项目校验失败"),
        (status = 404, description = "网站不存在"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn bind_runtime(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    ApiJson(req): ApiJson<BindRuntimeRequest>,
) -> Result<Json<Website>, AppError> {
    let site = state
        .website_service
        .bind_runtime(id, &req.site_type, req.runtime_id, req.project_id)
        .await?;
    Ok(Json(site))
}

/// 一键套用伪静态规则模板——落库 + 写 vhost + 校验 + reload（「套用即生效」）。
///
/// 响应保持既有契约（生成的规则文本，按站点引擎语法）；写盘失败返回错误并回滚套用。
#[utoipa::path(
    post,
    path = "/api/websites/{id}/rewrite",
    tag = "websites",
    operation_id = "apply_website_rewrite",
    params(("id" = i64, Path, description = "网站 ID")),
    request_body = ApplyRewriteRequest,
    responses(
        (status = 200, description = "套用成功，返回规则文本", body = String),
        (status = 400, description = "未知模板"),
        (status = 404, description = "网站不存在"),
        (status = 500, description = "vhost 写盘/校验/reload 失败（套用已回滚）"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn apply_rewrite(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    ApiJson(req): ApiJson<ApplyRewriteRequest>,
) -> Result<Json<String>, AppError> {
    let result = state
        .website_service
        .apply_rewrite_preset(id, &req.preset)
        .await?;
    Ok(Json(result.rules))
}

/// 查询站点伪静态套用状态（当前模板 + 该引擎语法规则文本 + 可选模板清单）
#[utoipa::path(
    get,
    path = "/api/websites/{id}/rewrite",
    tag = "websites",
    operation_id = "get_website_rewrite",
    params(("id" = i64, Path, description = "网站 ID")),
    responses(
        (status = 200, description = "套用状态", body = SiteRewriteStatus),
        (status = 404, description = "网站不存在"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn get_rewrite(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<SiteRewriteStatus>, AppError> {
    let status = state.website_service.get_rewrite_preset(id).await?;
    Ok(Json(status))
}

/// 清除站点伪静态套用（回落引擎默认路由；同样落库 + 写 vhost）。
#[utoipa::path(
    delete,
    path = "/api/websites/{id}/rewrite",
    tag = "websites",
    operation_id = "clear_website_rewrite",
    params(("id" = i64, Path, description = "网站 ID")),
    responses(
        (status = 200, description = "已清除", body = SiteRewriteStatus),
        (status = 404, description = "网站不存在"),
        (status = 500, description = "vhost 写盘/校验/reload 失败（清除已回滚）"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn clear_rewrite(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<SiteRewriteStatus>, AppError> {
    state.website_service.clear_rewrite_preset(id).await?;
    let status = state.website_service.get_rewrite_preset(id).await?;
    Ok(Json(status))
}

#[derive(serde::Deserialize, utoipa::ToSchema)]
pub struct SwitchWebsiteEngineRequest {
    /// 目标引擎：`nginx` | `openresty` | `caddy` | `apache` | `openlitespeed`
    pub engine: String,
}

/// 切换网站的 Web 服务器引擎（重写配置并重载）
#[utoipa::path(
    post,
    path = "/api/websites/{id}/switch-engine",
    tag = "websites",
    params(("id" = i64, Path, description = "网站 ID")),
    request_body = SwitchWebsiteEngineRequest,
    responses(
        (status = 200, description = "切换后的网站", body = Website),
        (status = 400, description = "目标引擎不可用"),
        (status = 401, description = "未认证"),
        (status = 403, description = "无权限"),
        (status = 404, description = "网站不存在"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn switch_engine(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    ApiJson(req): ApiJson<SwitchWebsiteEngineRequest>,
) -> Result<Json<Website>, AppError> {
    let engine = WebServerEngine::from_name(&req.engine)
        .ok_or_else(|| AppError::BadRequest(format!("未知引擎: {}", req.engine)))?;
    let site = state.website_service.switch_engine(id, &engine).await?;
    Ok(Json(site))
}

/// 手动部署站点配置（Doc/20 §5.5 站点配置流水线）：按当前库中状态渲染完整配置
/// （含已绑定证书的 HTTPS），写入面板自有站点目录 → 配置校验 → reload，失败整体回滚。
///
/// 严格模式：引擎未安装 / 配置校验失败返回 400，同名非托管 OLS 虚拟主机返回 409。
#[utoipa::path(
    post,
    path = "/api/websites/{id}/deploy",
    tag = "websites",
    params(("id" = i64, Path, description = "网站 ID")),
    responses(
        (status = 200, description = "部署结果", body = crate::webserver::SiteDeployment),
        (status = 400, description = "引擎未安装或配置校验失败"),
        (status = 401, description = "未认证"),
        (status = 403, description = "无权限"),
        (status = 404, description = "网站不存在"),
        (status = 409, description = "存在同名非托管虚拟主机"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn deploy(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<crate::webserver::SiteDeployment>, AppError> {
    Ok(Json(state.website_service.deploy_website(id).await?))
}

/// 路由表（集中注册于 routes.rs 组合根）
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/websites", axum::routing::get(list))
        .route("/api/websites", axum::routing::post(create))
        .route("/api/websites/{id}", axum::routing::get(get))
        .route("/api/websites/{id}", axum::routing::put(update))
        .route("/api/websites/{id}", axum::routing::delete(delete))
        .route(
            "/api/websites/{id}/switch-engine",
            axum::routing::post(switch_engine),
        )
        .route("/api/websites/{id}/deploy", axum::routing::post(deploy))
        // M-B 主线二 Stage A-6 · Doc/20 §5.5 绑定闭环
        .route(
            "/api/websites/{id}/runtime",
            axum::routing::post(bind_runtime),
        )
        .route(
            "/api/websites/{id}/rewrite",
            axum::routing::post(apply_rewrite),
        )
        .route(
            "/api/websites/{id}/rewrite",
            axum::routing::get(get_rewrite),
        )
        .route(
            "/api/websites/{id}/rewrite",
            axum::routing::delete(clear_rewrite),
        )
}
