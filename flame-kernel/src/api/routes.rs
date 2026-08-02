use axum::{Router, routing::get};
use crate::api::handler::{auth, user, node, website, docker, plugin, ws, operation_log, log, web_server, settings, database, file, firewall, app_store};
use crate::api::types::AppState;
use crate::core::error::AppError;

/// 全局 fallback：未匹配路由 / 方法返回 JSON 错误而非纯文本 404
pub async fn fallback_handler(uri: axum::http::Uri) -> AppError {
    AppError::NotFound(format!("Route {} not found", uri.path()))
}

/// 组合根：汇聚各 handler 模块的路由表，附加全局中间件（见 middleware::add_middleware）
pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(|| async { "OK" }))
        .merge(auth::routes())
        .merge(user::routes())
        .merge(node::routes())
        .merge(website::routes())
        .merge(docker::routes())
        .merge(plugin::routes())
        .merge(web_server::routes())
        .merge(settings::routes())
        .merge(database::routes())
        .merge(app_store::routes())
        .merge(file::routes())
        .merge(firewall::routes())
        .merge(operation_log::routes())
        .merge(log::routes())
        .merge(ws::routes())
        .fallback(fallback_handler)
        .with_state(state)
}
