use axum::{
    extract::{State, Query},
    routing::get,
    Json, Router,
};
use serde::Deserialize;

use crate::application::AppState;
use crate::core::error::AppError;
use crate::domain::PageParams;
use crate::middleware::auth::CurrentUser;

#[derive(Debug, Deserialize)]
pub struct LogQuery {
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/list", get(list_logs))
}

async fn list_logs(
    State(state): State<AppState>,
    CurrentUser(_claims): CurrentUser,
    Query(query): Query<LogQuery>,
) -> Result<Json<serde_json::Value>, AppError> {
    let params = PageParams { page: query.page, page_size: query.page_size };
    let result = state.log_repo.list_paginated(&params).await?;

    Ok(Json(serde_json::json!({
        "items": result.items,
        "total": result.total,
        "page": result.page,
        "page_size": result.page_size,
    })))
}
