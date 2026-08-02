use axum::{Json, extract::{State, Query}};
use axum::Router;
use serde::{Deserialize, Serialize};
use crate::api::extract::ApiJson;
use crate::api::types::{AppState, PaginationParams, PaginatedResponse};
use crate::core::error::AppError;


#[derive(Debug, Serialize)]
pub struct SettingEntry {
    pub key: String,
    pub value: String,
    pub description: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateSettingRequest {
    pub key: String,
    pub value: String,
}

pub async fn list_settings(
    State(state): State<AppState>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<PaginatedResponse<SettingEntry>>, AppError> {
    let result = state.settings_service.list_all_paginated(&params).await?;
    let data = result.data.into_iter().map(|s| SettingEntry {
        key: s.key,
        value: s.value,
        description: s.description,
    }).collect();
    Ok(Json(PaginatedResponse::new(data, result.total, &params)))
}

pub async fn get_setting(
    State(state): State<AppState>,
    axum::extract::Path(key): axum::extract::Path<String>,
) -> Result<Json<SettingEntry>, AppError> {
    let all = state.settings_service.list_all().await?;
    let setting = all.into_iter().find(|s| s.key == key)
        .ok_or_else(|| AppError::NotFound(format!("Setting '{}' not found", key)))?;
    Ok(Json(SettingEntry {
        key: setting.key,
        value: setting.value,
        description: setting.description,
    }))
}

pub async fn update_setting(
    State(state): State<AppState>,
    ApiJson(req): ApiJson<UpdateSettingRequest>,
) -> Result<Json<()>, AppError> {
    state.settings_service.set(&req.key, &req.value).await?;
    Ok(Json(()))
}



/// 路由表（集中注册于 routes.rs 组合根）
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/settings", axum::routing::get(list_settings))
        .route("/api/settings/:key", axum::routing::get(get_setting))
        .route("/api/settings", axum::routing::put(update_setting))
}
