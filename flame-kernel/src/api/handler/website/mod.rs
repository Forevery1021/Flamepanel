use axum::{Json, extract::State};
use crate::domain::entity::Website;
use crate::api::types::{AppState, CreateWebsiteRequest};

pub async fn list(State(state): State<AppState>) -> Json<Vec<Website>> {
    let websites = state.website_service.list_websites().await.unwrap();
    Json(websites)
}

pub async fn create(
    State(state): State<AppState>,
    Json(payload): Json<CreateWebsiteRequest>,
) -> Json<i64> {
    let id = state.website_service.create_website(&payload.website).await.unwrap();
    Json(id)
}