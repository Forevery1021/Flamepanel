use crate::api::extract::ApiJson;
use crate::api::types::{AppState, CreateWebsiteRequest, PaginatedResponse, PaginationParams};
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
#[derive(serde::Deserialize)]
pub struct SwitchWebsiteEngineRequest {
    pub engine: String,
}

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
}
