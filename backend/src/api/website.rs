use axum::{Json, Router, routing::post};
use serde::{Deserialize, Serialize};
use crate::core::error::AppError;
use crate::middleware::auth::CurrentUser;

#[derive(Deserialize)]
pub struct WebsiteConfig {
    domain: String,
    root: String,
    port: u16,
    ssl: bool,
    ssl_cert: Option<String>,
    ssl_key: Option<String>,
}

#[derive(Serialize)]
pub struct WebsiteResponse {
    config_path: String,
    message: String,
}

pub fn routes() -> Router {
    Router::new().route("/create", post(create_website))
}

async fn create_website(
    CurrentUser(_claims): CurrentUser,
    Json(config): Json<WebsiteConfig>,
) -> Result<Json<WebsiteResponse>, AppError> {
    // TODO: 生成 nginx 配置文件
    let config_path = format!("/etc/nginx/sites-available/{}", config.domain);

    Ok(Json(WebsiteResponse {
        config_path,
        message: "站点配置创建成功".into(),
    }))
}