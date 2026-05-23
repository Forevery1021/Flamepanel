use axum::{middleware, Router};

pub mod appstore;
pub mod auth;
pub mod cleanup;
pub mod cron;
pub mod dashboard;
pub mod database;
pub mod docker;
pub mod file;
pub mod health;
pub mod logs;
pub mod openapi;
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
        .merge(openapi::swagger_routes());

    let protected = Router::new()
        .nest("/auth", auth::protected_routes())
        .nest("/dashboard", dashboard::routes())
        .nest("/system", system::routes())
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
        .layer(middleware::from_fn(auth_middleware));

    Router::new()
        .nest("/api", public.merge(protected))
}
