use crate::api::extract::ApiJson;
use crate::api::types::{AppState, PaginatedResponse, PaginationParams};
use crate::core::error::AppError;
use axum::Router;
use axum::{
    extract::{Query, State},
    Json,
};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Serialize, ToSchema)]
pub struct SettingEntry {
    pub key: String,
    pub value: String,
    pub description: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateSettingRequest {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateSettingsBatchRequest {
    /// 批量设置项（key -> value），在一次事务内原子写入。
    pub settings: Vec<(String, String)>,
}

/// 设置列表（分页）
#[utoipa::path(
    get,
    path = "/api/settings",
    tag = "settings",
    params(PaginationParams),
    responses(
        (status = 200, description = "设置列表", body = PaginatedResponse<SettingEntry>),
        (status = 401, description = "未认证"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn list_settings(
    State(state): State<AppState>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<PaginatedResponse<SettingEntry>>, AppError> {
    let result = state.settings_service.list_all_paginated(&params).await?;
    let data = result
        .data
        .into_iter()
        .map(|s| SettingEntry {
            key: s.key,
            value: s.value,
            description: s.description,
        })
        .collect();
    Ok(Json(PaginatedResponse::new(data, result.total, &params)))
}

pub async fn get_setting(
    State(state): State<AppState>,
    axum::extract::Path(key): axum::extract::Path<String>,
) -> Result<Json<SettingEntry>, AppError> {
    let all = state.settings_service.list_all().await?;
    let setting = all
        .into_iter()
        .find(|s| s.key == key)
        .ok_or_else(|| AppError::NotFound(format!("Setting '{}' not found", key)))?;
    Ok(Json(SettingEntry {
        key: setting.key,
        value: setting.value,
        description: setting.description,
    }))
}

/// 更新设置
#[utoipa::path(
    put,
    path = "/api/settings",
    tag = "settings",
    request_body = UpdateSettingRequest,
    responses(
        (status = 200, description = "更新成功"),
        (status = 401, description = "未认证"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn update_setting(
    State(state): State<AppState>,
    ApiJson(req): ApiJson<UpdateSettingRequest>,
) -> Result<Json<()>, AppError> {
    state.settings_service.set(&req.key, &req.value).await?;
    Ok(Json(()))
}

/// 批量更新设置（一次事务原子写入多键）
#[utoipa::path(
    patch,
    path = "/api/settings/batch",
    tag = "settings",
    request_body = UpdateSettingsBatchRequest,
    responses(
        (status = 200, description = "批量更新成功"),
        (status = 400, description = "请求体为空"),
        (status = 401, description = "未认证"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn update_settings_batch(
    State(state): State<AppState>,
    ApiJson(req): ApiJson<UpdateSettingsBatchRequest>,
) -> Result<Json<()>, AppError> {
    if req.settings.is_empty() {
        return Err(AppError::BadRequest("settings must not be empty".into()));
    }
    state.settings_service.set_many(&req.settings).await?;
    Ok(Json(()))
}

/// 路由表（集中注册于 routes.rs 组合根）
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/settings", axum::routing::get(list_settings))
        .route("/api/settings/{key}", axum::routing::get(get_setting))
        .route("/api/settings", axum::routing::put(update_setting))
        .route(
            "/api/settings/batch",
            axum::routing::patch(update_settings_batch),
        )
}
