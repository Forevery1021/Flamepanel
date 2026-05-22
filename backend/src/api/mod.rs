use axum::Router;

pub mod auth;
pub mod system;
pub mod docker;
pub mod file;
pub mod website;

use crate::middleware::auth::auth_middleware;

pub fn routes() -> Router {
    Router::new()
        // 公开接口
        .nest("/api/auth", auth::routes())

        // 需要认证的接口
        .nest("/api", protected_routes())
}

fn protected_routes() -> Router {
    Router::new()
        .nest("/system", system::routes())
        .nest("/docker", docker::routes())
        .nest("/file", file::routes())
        .nest("/website", website::routes())
        .layer(axum::middleware::from_fn(auth_middleware))
}