use crate::api::handler::{
    app_store, auth, backup, database, docker, file, firewall, health, log, memo, metrics, node,
    operation_log, plugin, scheduled_task, settings, user, web_server, website, ws,
};
use crate::api::types::AppState;
use crate::core::error::AppError;
use axum::{routing::get, Router};

/// 全局 fallback：未匹配路由 / 方法返回 JSON 错误而非纯文本 404
pub async fn fallback_handler(uri: axum::http::Uri) -> AppError {
    AppError::NotFound(format!("Route {} not found", uri.path()))
}

/// 组合根：汇聚各 handler 模块的路由表，附加全局中间件（见 middleware::add_middleware）
pub fn create_router(state: AppState) -> Router {
    Router::new()
        .route("/health", get(|| async { "OK" }))
        .merge(health::routes())
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
        .merge(backup::routes())
        .merge(scheduled_task::routes())
        .merge(log::routes())
        .merge(memo::routes())
        .merge(metrics::routes())
        .merge(ws::routes())
        .fallback(fallback_handler)
        .with_state(state)
}
