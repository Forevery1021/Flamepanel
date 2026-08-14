use crate::api::types::{AppState, PaginatedResponse, PaginationParams};
use crate::core::error::AppError;
use crate::domain::entity::OperationLog;
use axum::Router;
use axum::{
    extract::{Path, Query, State},
    http::header,
    response::IntoResponse,
    Json,
};
use serde::Deserialize;
use utoipa::{IntoParams, ToSchema};

#[derive(Debug, Deserialize, Default)]
pub struct LogListQuery {
    pub page: Option<i64>,
    pub page_size: Option<i64>,
    /// 按 action 前缀过滤（如 `action=LOGIN` 匹配 LOGIN_SUCCESS/LOGIN_FAILED）
    pub action: Option<String>,
}

/// 导出查询参数：`format=csv|json`（默认 csv）
#[derive(Debug, Deserialize, Default, IntoParams)]
pub struct ExportQuery {
    pub format: Option<String>,
}

/// 导出条目（DTO，避免暴露内部字段）
#[derive(Debug, serde::Serialize, ToSchema)]
struct ExportRow {
    id: i64,
    username: String,
    action: String,
    target: Option<String>,
    ip: Option<String>,
    created_at: String,
}

impl From<&OperationLog> for ExportRow {
    fn from(log: &OperationLog) -> Self {
        Self {
            id: log.id,
            username: log.username.clone(),
            action: log.action.clone(),
            target: log.target.clone(),
            ip: log.ip.clone(),
            created_at: log.created_at.to_rfc3339(),
        }
    }
}

pub async fn list(
    State(state): State<AppState>,
    Query(query): Query<LogListQuery>,
) -> Result<Json<PaginatedResponse<OperationLog>>, AppError> {
    let params = PaginationParams {
        page: query.page,
        page_size: query.page_size,
    };
    let result = state
        .operation_log_service
        .list_paginated(&params, query.action.as_deref())
        .await?;
    Ok(Json(result))
}

pub async fn delete(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<&'static str>, AppError> {
    state.operation_log_service.delete_log(id).await?;
    Ok(Json("deleted"))
}

/// `GET /api/operation-logs/export?format=csv|json` — 导出全部审计日志
///
/// Stage4.4：支持 CSV / JSON 两种格式，便于归档与 SIEM 对接。
/// CSV 带 BOM（UTF-8）以兼容 Excel；JSON 为 UTF-8 数组。
#[utoipa::path(
    get,
    path = "/api/operation-logs/export",
    tag = "operation-logs",
    params(ExportQuery),
    responses(
        (status = 200, description = "导出文件"),
        (status = 401, description = "未认证"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn export(
    State(state): State<AppState>,
    Query(query): Query<ExportQuery>,
) -> Result<impl IntoResponse, AppError> {
    let logs = state.operation_log_service.list().await?;
    let fmt = query.format.as_deref().unwrap_or("csv").to_lowercase();

    match fmt.as_str() {
        "json" => {
            let rows: Vec<ExportRow> = logs.iter().map(ExportRow::from).collect();
            let body = serde_json::to_vec_pretty(&rows)?;
            Ok((
                [
                    (header::CONTENT_TYPE, "application/json; charset=utf-8"),
                    (
                        header::CONTENT_DISPOSITION,
                        "attachment; filename=\"operation-logs.json\"",
                    ),
                ],
                body,
            ))
        }
        "csv" => {
            let mut out = String::new();
            // BOM 供 Excel 正确识别 UTF-8
            out.push('\u{feff}');
            out.push_str("id,username,action,target,ip,created_at\n");
            for log in &logs {
                out.push_str(&format!(
                    "{},{},{},{},{},{}\n",
                    log.id,
                    csv_escape(&log.username),
                    csv_escape(&log.action),
                    csv_escape(log.target.as_deref().unwrap_or("")),
                    csv_escape(log.ip.as_deref().unwrap_or("")),
                    log.created_at.to_rfc3339(),
                ));
            }
            Ok((
                [
                    (header::CONTENT_TYPE, "text/csv; charset=utf-8"),
                    (
                        header::CONTENT_DISPOSITION,
                        "attachment; filename=\"operation-logs.csv\"",
                    ),
                ],
                out.into_bytes(),
            ))
        }
        _ => Err(AppError::BadRequest(format!(
            "Unsupported export format: {}",
            fmt
        ))),
    }
}

/// CSV 字段转义：包含逗号/引号/换行时用双引号包裹并转义内部引号
fn csv_escape(field: &str) -> String {
    if field.contains(',') || field.contains('"') || field.contains('\n') || field.contains('\r') {
        format!("\"{}\"", field.replace('"', "\"\""))
    } else {
        field.to_string()
    }
}

/// 路由表（集中注册于 routes.rs 组合根）
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/operation-logs", axum::routing::get(list))
        .route("/api/operation-logs/export", axum::routing::get(export))
        .route("/api/operation-logs/{id}", axum::routing::delete(delete))
}
