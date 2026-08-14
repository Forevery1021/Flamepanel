use crate::api::extract::ApiJson;
use crate::api::types::{
    AppState, CreateUserRequest, PaginatedResponse, PaginationParams, UpdateUserRequest,
};
use crate::core::error::AppError;
use crate::domain::entity::User;
use axum::Router;
use axum::{
    extract::{Path, Query, State},
    Json,
};

/// 用户列表（分页）
#[utoipa::path(
    get,
    path = "/api/users",
    tag = "users",
    operation_id = "list_users",
    params(PaginationParams),
    responses(
        (status = 200, description = "用户列表", body = PaginatedResponse<User>),
        (status = 401, description = "未认证"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn list(
    State(state): State<AppState>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<PaginatedResponse<User>>, AppError> {
    let result = state.user_service.list_users_paginated(&params).await?;
    Ok(Json(result))
}

/// 创建用户
#[utoipa::path(
    post,
    path = "/api/users",
    tag = "users",
    operation_id = "create_user",
    request_body = CreateUserRequest,
    responses(
        (status = 200, description = "创建成功", body = User),
        (status = 400, description = "参数错误"),
        (status = 401, description = "未认证"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn create(
    State(state): State<AppState>,
    ApiJson(payload): ApiJson<CreateUserRequest>,
) -> Result<Json<User>, AppError> {
    let user = state
        .user_service
        .create_user(&payload.username, &payload.password_hash, &payload.role)
        .await?;
    Ok(Json(user))
}

/// 更新用户
#[utoipa::path(
    put,
    path = "/api/users/{id}",
    tag = "users",
    operation_id = "update_user",
    params(("id" = i64, Path, description = "用户 ID")),
    request_body = UpdateUserRequest,
    responses(
        (status = 200, description = "更新成功", body = User),
        (status = 404, description = "用户不存在"),
    ),
    security(("BearerAuth" = []))
)]
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

/// 删除用户
#[utoipa::path(
    delete,
    path = "/api/users/{id}",
    tag = "users",
    operation_id = "delete_user",
    params(("id" = i64, Path, description = "用户 ID")),
    responses(
        (status = 200, description = "删除成功"),
        (status = 404, description = "用户不存在"),
    ),
    security(("BearerAuth" = []))
)]
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
        .route("/api/users/{id}", axum::routing::put(update))
        .route("/api/users/{id}", axum::routing::delete(delete))
}
