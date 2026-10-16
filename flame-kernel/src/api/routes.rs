use crate::api::handler::{
    app_store, assistant, auth, backup, backup_account, backup_job, certificate, database,
    dns_provider, docker, file, firewall, health, iac, inspection, kernel_logs, log, mcp, memo,
    metrics, node, operation_log, outbox, overview, plugin, project, scheduled_task, settings,
    setup, task, toolbox, user, waf, web_server, website, ws,
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
    let mut app = Router::new()
        .route("/health", get(crate::api::handler::health::liveness))
        .merge(health::routes())
        .merge(auth::routes())
        .merge(user::routes())
        .merge(node::routes())
        .merge(website::routes())
        .merge(docker::routes())
        .merge(plugin::routes())
        .merge(web_server::routes())
        .merge(waf::routes())
        .merge(settings::routes())
        .merge(setup::routes())
        .merge(database::routes())
        .merge(app_store::routes())
        .merge(certificate::routes())
        .merge(dns_provider::routes())
        .merge(file::routes())
        .merge(firewall::routes())
        .merge(operation_log::routes())
        .merge(outbox::routes())
        .merge(overview::routes())
        .merge(iac::routes())
        .merge(inspection::routes())
        .merge(toolbox::routes())
        .merge(kernel_logs::routes())
        .merge(mcp::routes())
        .merge(assistant::routes())
        .merge(backup::routes())
        .merge(backup_account::routes())
        .merge(backup_job::routes())
        .merge(scheduled_task::routes())
        .merge(project::routes())
        .merge(task::routes())
        .merge(log::routes())
        .merge(memo::routes())
        .merge(metrics::routes())
        .merge(ws::routes())
        .fallback(fallback_handler);

    // Stage3.3：OpenAPI 文档 + Swagger UI（feature 控制）
    app = app
        .merge(crate::openapi::openapi_router())
        .merge(crate::openapi::swagger_ui_router());

    app.with_state(state)
}
