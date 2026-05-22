// src/api/mod.rs
//
// 所有 API 路由在此集中注册，统一挂载到 /api 前缀。
// 需要认证的路由组通过 axum::middleware::from_fn(auth_middleware) 保护。

pub mod auth;
pub mod system;
pub mod docker;
pub mod file;
pub mod website;

use axum::{middleware, Router};
use crate::middleware::auth::auth_middleware;

pub fn routes() -> Router {
    // 无需认证的公开路由
    let public = Router::new()
        .nest("/auth", auth::routes());

    // 需要 JWT 认证的路由（添加 auth_middleware 层）
    let protected = Router::new()
        .nest("/system", system::routes())
        .nest("/docker", docker::routes())
        .nest("/file",   file::routes())
        .nest("/website", website::routes())
        .layer(middleware::from_fn(auth_middleware));

    Router::new()
        .nest("/api", public.merge(protected))
}