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
use utoipa::{IntoParams, ToSchema};

#[derive(Debug, Deserialize, IntoParams)]
pub struct ListQuery {
    /// `memo` | `todo`
    pub kind: Option<String>,
    pub done: Option<bool>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct CreateMemoRequest {
    pub content: String,
    #[serde(default)]
    pub kind: String,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct UpdateMemoRequest {
    pub content: Option<String>,
    pub done: Option<bool>,
}

/// 备忘录列表（仪表盘便签与待办）
#[utoipa::path(
    get,
    path = "/api/memos",
    tag = "memos",
    params(ListQuery),
    responses(
        (status = 200, description = "备忘 / 待办列表", body = Vec<Memo>),
        (status = 401, description = "未认证"),
    ),
    security(("BearerAuth" = []))
)]
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

/// 新建备忘 / 待办
#[utoipa::path(
    post,
    path = "/api/memos",
    tag = "memos",
    request_body = CreateMemoRequest,
    responses(
        (status = 200, description = "新建的条目", body = Memo),
        (status = 400, description = "内容为空"),
        (status = 401, description = "未认证"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn create(
    State(state): State<AppState>,
    ApiJson(req): ApiJson<CreateMemoRequest>,
) -> Result<Json<Memo>, AppError> {
    let memo = state.memo_service.create(&req.content, &req.kind).await?;
    Ok(Json(memo))
}

/// 更新备忘内容或完成状态
#[utoipa::path(
    put,
    path = "/api/memos/{id}",
    tag = "memos",
    params(("id" = i64, Path, description = "ID")),
    request_body = UpdateMemoRequest,
    responses(
        (status = 200, description = "更新后的条目", body = Memo),
        (status = 401, description = "未认证"),
        (status = 404, description = "条目不存在"),
    ),
    security(("BearerAuth" = []))
)]
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

/// 删除备忘
#[utoipa::path(
    delete,
    path = "/api/memos/{id}",
    tag = "memos",
    params(("id" = i64, Path, description = "ID")),
    responses(
        (status = 200, description = "删除成功", body = String),
        (status = 401, description = "未认证"),
        (status = 404, description = "条目不存在"),
    ),
    security(("BearerAuth" = []))
)]
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
        .route("/api/memos/{id}", axum::routing::put(update))
        .route("/api/memos/{id}", axum::routing::delete(delete))
}
