use axum::{Json, extract::State};
use serde::{Deserialize, Serialize};
use crate::api::types::AppState;
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
) -> Result<Json<Vec<SettingEntry>>, AppError> {
    let settings = state.settings_service.list_all().await?;
    Ok(Json(settings.into_iter().map(|s| SettingEntry {
        key: s.key,
        value: s.value,
        description: s.description,
    }).collect()))
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
    Json(req): Json<UpdateSettingRequest>,
) -> Result<Json<()>, AppError> {
    state.settings_service.set(&req.key, &req.value).await?;
    Ok(Json(()))
}
