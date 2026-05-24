use axum::{extract::State, Json};
use serde_json::json;

use crate::application::AppState;
use crate::core::error::AppError;
use crate::domain::{PanelSettings, UpdateSettingsRequest};
use crate::middleware::auth::CurrentUser;

pub async fn get_settings(
    State(state): State<AppState>,
    _user: CurrentUser,
) -> Result<Json<PanelSettings>, AppError> {
    state.settings_repo.get_all().await.map(Json)
}

pub async fn update_settings(
    State(state): State<AppState>,
    _user: CurrentUser,
    Json(req): Json<UpdateSettingsRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    if let Some(theme) = &req.theme {
        state.settings_repo.set("theme", theme).await?;
    }
    if let Some(language) = &req.language {
        state.settings_repo.set("language", language).await?;
    }
    if let Some(theme_color) = &req.theme_color {
        state.settings_repo.set("theme_color", theme_color).await?;
    }
    if let Some(bg) = &req.background_image {
        state.settings_repo.set("background_image", bg).await?;
    }
    if let Some(opacity) = req.background_opacity {
        state.settings_repo.set("background_opacity", &opacity.to_string()).await?;
    }
    let settings = state.settings_repo.get_all().await?;
    Ok(Json(json!({
        "message": "设置已更新",
        "settings": settings,
    })))
}

pub fn routes() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/", axum::routing::get(get_settings))
        .route("/", axum::routing::put(update_settings))
}
