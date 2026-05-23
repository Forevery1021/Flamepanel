use axum::{middleware, Router};

pub mod auth;
pub mod dashboard;
pub mod docker;
pub mod file;
pub mod system;
pub mod waf;
pub mod website;

use crate::application::AppState;
use crate::middleware::auth::auth_middleware;

pub fn routes() -> Router<AppState> {
    let public = Router::new()
        .nest("/auth", auth::public_routes());

    let protected = Router::new()
        .nest("/auth", auth::protected_routes())
        .nest("/dashboard", dashboard::routes())
        .nest("/system", system::routes())
        .nest("/docker", docker::routes())
        .nest("/file", file::routes())
        .nest("/website", website::routes())
        .nest("/waf", waf::routes())
        .layer(middleware::from_fn(auth_middleware));

    Router::new()
        .nest("/api", public.merge(protected))
}
