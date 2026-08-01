use axum::{Json, extract::{State, Path, Query}};
use crate::domain::entity::Website;
use crate::api::types::{AppState, CreateWebsiteRequest, PaginationParams, PaginatedResponse};
use crate::core::error::AppError;
use crate::webserver::engine::WebServerEngine;

pub async fn list(
    State(state): State<AppState>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<PaginatedResponse<Website>>, AppError> {
    let result = state.website_service.list_websites_paginated(&params).await?;
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
    Json(payload): Json<CreateWebsiteRequest>,
) -> Result<Json<i64>, AppError> {
    let id = state.website_service.create_website(&payload.website).await?;
    Ok(Json(id))
}

pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    Json(payload): Json<CreateWebsiteRequest>,
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
    Json(req): Json<SwitchWebsiteEngineRequest>,
) -> Result<Json<Website>, AppError> {
    let engine = WebServerEngine::from_str(&req.engine)
        .ok_or_else(|| AppError::BadRequest(format!("未知引擎: {}", req.engine)))?;
    let site = state.website_service.switch_engine(id, &engine).await?;
    Ok(Json(site))
}
