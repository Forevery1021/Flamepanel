use axum::{Json, extract::{State, Path}};
use crate::domain::entity::User;
use crate::api::types::{AppState, CreateUserRequest};
use crate::core::error::AppError;

pub async fn list(
    State(state): State<AppState>,
) -> Result<Json<Vec<User>>, AppError> {
    let users = state.user_service.list_users().await?;
    Ok(Json(users))
}

pub async fn create(
    State(state): State<AppState>,
    Json(payload): Json<CreateUserRequest>,
) -> Result<Json<User>, AppError> {
    let user = state.user_service
        .create_user(&payload.username, &payload.password_hash, &payload.role)
        .await?;
    Ok(Json(user))
}

pub async fn delete(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<&'static str>, AppError> {
    state.user_service.delete_user(id).await?;
    Ok(Json("deleted"))
}