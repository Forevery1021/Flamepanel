use axum::{Json, Router, routing::get};
use utoipa::OpenApi;

use crate::application::AppState;

#[derive(OpenApi)]
#[openapi(
    info(
        title = "Flamepanel API",
        version = "0.1.0",
        description = "Flamepanel 运维面板 REST API 文档",
    ),
    // Components removed: domain types may not implement utoipa::ToSchema here.
    // Ensure domain structs derive ToSchema in their modules instead.
    tags(
        (name = "auth", description = "认证接口"),
        (name = "dashboard", description = "仪表盘"),
        (name = "system", description = "系统信息"),
        (name = "website", description = "网站管理"),
        (name = "waf", description = "WAF 防火墙"),
        (name = "users", description = "用户管理"),
        (name = "logs", description = "操作日志"),
        (name = "health", description = "健康检查"),
    )
)]
pub struct ApiDoc;

pub fn swagger_routes() -> Router<AppState> {
    Router::new()
        .route("/api-docs/openapi.json", get(openapi_json))
}

async fn openapi_json() -> Json<utoipa::openapi::OpenApi> {
    Json(ApiDoc::openapi())
}
