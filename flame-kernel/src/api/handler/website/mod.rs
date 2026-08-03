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

pub async fn get(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Website>, AppError> {
    let website = state.website_service.get_website(id).await?;
    Ok(Json(website))
}

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
        .route("/api/websites/:id", axum::routing::get(get))
        .route("/api/websites/:id", axum::routing::put(update))
        .route("/api/websites/:id", axum::routing::delete(delete))
        .route(
            "/api/websites/:id/switch-engine",
            axum::routing::post(switch_engine),
        )
}
