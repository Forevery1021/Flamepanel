use axum::{middleware, Router};

pub mod ai;
pub mod alerts;
pub mod appstore;
pub mod auth;
pub mod backup;
pub mod plugins;
pub mod cleanup;
pub mod cron;
pub mod dashboard;
pub mod database;
pub mod docker;
pub mod file;
pub mod firewall;
pub mod grafana;
pub mod health;
pub mod logs;
pub mod metrics_endpoint;
pub mod nodes;
pub mod openapi;
pub mod roles;
pub mod settings;
pub mod system;
pub mod users;
pub mod waf;
pub mod website;

use crate::application::AppState;
use crate::middleware::auth::auth_middleware;

pub fn routes() -> Router<AppState> {
    let public = Router::new()
        .nest("/auth", auth::public_routes())
        .merge(health::routes())
        .merge(openapi::swagger_routes())
        .merge(nodes::public_routes())
        .merge(metrics_endpoint::routes())
        .merge(grafana::routes());

    let protected = Router::new()
        .nest("/auth", auth::protected_routes())
        .nest("/dashboard", dashboard::routes())
        .nest("/system", system::routes())
        .nest("/nodes", nodes::protected_routes())
        .nest("/docker", docker::routes())
        .nest("/file", file::routes())
        .nest("/website", website::routes())
        .nest("/waf", waf::routes())
        .nest("/users", users::routes())
        .nest("/logs", logs::routes())
        .nest("/cleanup", cleanup::routes())
        .nest("/settings", settings::routes())
        .nest("/cron", cron::routes())
        .nest("/databases", database::routes())
        .nest("/appstore", appstore::routes())
        .nest("/backup", backup::routes())
        .nest("/alerts", alerts::routes())
        .nest("/plugins", plugins::routes())
        .nest("/ai", ai::routes())
        .nest("/firewall", firewall::routes())
        .nest("/rbac", roles::routes())
        .layer(middleware::from_fn(auth_middleware));

    Router::new()
        .nest("/api", public.merge(protected))
}
