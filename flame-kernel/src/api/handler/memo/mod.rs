use crate::api::extract::ApiJson;
use crate::api::types::AppState;
use crate::core::error::AppError;
use crate::domain::entity::Memo;
use axum::Router;
use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct ListQuery {
    pub kind: Option<String>,
    pub done: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct CreateMemoRequest {
    pub content: String,
    #[serde(default)]
    pub kind: String,
}

#[derive(Debug, Deserialize)]
pub struct UpdateMemoRequest {
    pub content: Option<String>,
    pub done: Option<bool>,
}

pub async fn list(
    State(state): State<AppState>,
    Query(query): Query<ListQuery>,
) -> Result<Json<Vec<Memo>>, AppError> {
    let memos = state
        .memo_service
        .list(query.kind.as_deref(), query.done)
        .await?;
    Ok(Json(memos))
}

pub async fn create(
    State(state): State<AppState>,
    ApiJson(req): ApiJson<CreateMemoRequest>,
) -> Result<Json<Memo>, AppError> {
    let memo = state.memo_service.create(&req.content, &req.kind).await?;
    Ok(Json(memo))
}

pub async fn update(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    ApiJson(req): ApiJson<UpdateMemoRequest>,
) -> Result<Json<Memo>, AppError> {
    let memo = state
        .memo_service
        .update(id, req.content.as_deref(), req.done)
        .await?;
    Ok(Json(memo))
}

pub async fn delete(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<&'static str>, AppError> {
    state.memo_service.delete(id).await?;
    Ok(Json("deleted"))
}

/// 路由表
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/memos", axum::routing::get(list))
        .route("/api/memos", axum::routing::post(create))
        .route("/api/memos/:id", axum::routing::put(update))
        .route("/api/memos/:id", axum::routing::delete(delete))
}
