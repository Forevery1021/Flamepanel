use axum::{Json, extract::State};
use crate::domain::entity::User;
use crate::api::types::{AppState, CreateUserRequest};

pub async fn list(State(state): State<AppState>) -> Json<Vec<User>> {
    let users = state.user_service.list_users().await.unwrap();
    Json(users)
}

pub async fn create(
    State(state): State<AppState>,
    Json(payload): Json<CreateUserRequest>,
) -> Json<User> {
    let user = state.user_service
        .create_user(&payload.username, &payload.password_hash, &payload.role)
        .await
        .unwrap();
    Json(user)
}