use axum::{Json, extract::{State, Path, Query}};
use axum::Router;
use crate::api::extract::ApiJson;
use crate::domain::entity::User;
use crate::api::types::{AppState, CreateUserRequest, UpdateUserRequest, PaginationParams, PaginatedResponse};
use crate::core::error::AppError;

pub async fn list(
    State(state): State<AppState>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<PaginatedResponse<User>>, AppError> {
    let result = state.user_service.list_users_paginated(&params).await?;
    Ok(Json(result))
}

pub async fn create(
    State(state): State<AppState>,
    ApiJson(payload): ApiJson<CreateUserRequest>,
) -> Result<Json<User>, AppError> {
    let user = state.user_service
        .create_user(&payload.username, &payload.password_hash, &payload.role)
        .await?;
    Ok(Json(user))
}

pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    ApiJson(payload): ApiJson<UpdateUserRequest>,
) -> Result<Json<User>, AppError> {
    let mut user = state.user_service.get_user(id).await?;
    user.username = payload.username;
    if let Some(password_hash) = payload.password_hash {
        user.password_hash = password_hash;
    }
    user.role = payload.role;
    state.user_service.update_user(&user).await?;
    Ok(Json(user))
}

pub async fn delete(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<&'static str>, AppError> {
    state.user_service.delete_user(id).await?;
    Ok(Json("deleted"))
}


/// 路由表（集中注册于 routes.rs 组合根）
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/users", axum::routing::get(list))
        .route("/api/users", axum::routing::post(create))
        .route("/api/users/:id", axum::routing::put(update))
        .route("/api/users/:id", axum::routing::delete(delete))
}
