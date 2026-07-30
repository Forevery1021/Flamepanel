use axum::{Json, extract::State};
use crate::domain::entity::Website;
use crate::api::types::{AppState, CreateWebsiteRequest};
use crate::core::error::AppError;

pub async fn list(
    State(state): State<AppState>,
) -> Result<Json<Vec<Website>>, AppError> {
    let websites = state.website_service.list_websites().await?;
    Ok(Json(websites))
}

pub async fn create(
    State(state): State<AppState>,
    Json(payload): Json<CreateWebsiteRequest>,
) -> Result<Json<i64>, AppError> {
    let id = state.website_service.create_website(&payload.website).await?;
    Ok(Json(id))
}