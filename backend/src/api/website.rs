use axum::{
    extract::{Query, State},
    routing::{get, post, delete},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::application::AppState;
use crate::core::error::AppError;
use crate::domain::{CreateWebsiteRequest, Website};
use crate::infrastructure::WebsiteRepository;
use crate::middleware::auth::CurrentUser;

const NGINX_AVAILABLE: &str = "/etc/nginx/sites-available";
const NGINX_ENABLED: &str = "/etc/nginx/sites-enabled";
const WWW_ROOT: &str = "/www";

#[derive(Debug, Deserialize)]
pub struct CreateSiteRequest {
    pub domain: String,
    pub root_path: Option<String>,
    pub proxy_port: Option<i32>,
    pub enable_ssl: Option<bool>,
}

#[derive(Debug, Deserialize)]
pub struct SslRequest {
    pub id: i64,
    pub cert_path: String,
    pub key_path: String,
}

#[derive(Debug, Deserialize)]
pub struct IdQuery {
    pub id: i64,
}

#[derive(Debug, Serialize)]
pub struct SiteListResponse {
    pub sites: Vec<Website>,
    pub total: usize,
}

#[derive(Debug, Serialize)]
pub struct MessageResponse {
    pub success: bool,
    pub message: String,
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/list", get(list_sites))
        .route("/get", get(get_site))
        .route("/create", post(create_site))
        .route("/delete", delete(delete_site))
        .route("/ssl", post(enable_ssl))
        .route("/toggle", post(toggle_site))
        .route("/reload-nginx", post(reload_nginx))
}

async fn list_sites(
    State(state): State<AppState>,
    CurrentUser(_claims): CurrentUser,
) -> Result<Json<SiteListResponse>, AppError> {
    let sites = state.website_repo.list().await?;
    let total = sites.len();
    Ok(Json(SiteListResponse { sites, total }))
}

async fn get_site(
    State(state): State<AppState>,
    CurrentUser(_claims): CurrentUser,
    Query(query): Query<IdQuery>,
) -> Result<Json<Website>, AppError> {
    let site = state.website_repo.find_by_id(query.id)
        .await?
        .ok_or(AppError::NotFound("站点不存在".into()))?;
    Ok(Json(site))
}

async fn create_site(
    State(state): State<AppState>,
    CurrentUser(_claims): CurrentUser,
    Json(payload): Json<CreateSiteRequest>,
) -> Result<Json<Website>, AppError> {
    if payload.domain.is_empty() {
        return Err(AppError::BadRequest("域名不能为空".into()));
    }

    if state.website_repo.find_by_domain(&payload.domain).await?.is_some() {
        return Err(AppError::BadRequest("该域名站点已存在".into()));
    }

    // 生成 Nginx 配置文件
    let config_path = format!("{}/{}", NGINX_AVAILABLE, payload.domain);
    let root_path = payload.root_path.unwrap_or_else(|| format!("{}/{}", WWW_ROOT, payload.domain));

    // 确保站点根目录存在
    tokio::fs::create_dir_all(&root_path)
        .await
        .map_err(|e| AppError::Internal(format!("创建网站目录失败: {e}")))?;

    // 生成 Nginx 配置
    let nginx_config = generate_nginx_config(&payload.domain, &root_path, payload.proxy_port);
    tokio::fs::write(&config_path, &nginx_config)
        .await
        .map_err(|e| AppError::Internal(format!("写入Nginx配置失败: {e}")))?;

    // 创建软链接到 sites-enabled
    let enabled_link = format!("{}/{}", NGINX_ENABLED, payload.domain);
    let _ = tokio::fs::remove_file(&enabled_link).await; // 清理旧链接
    let src = config_path.clone();
    let dst = enabled_link.clone();
    create_symlink(&src, &dst)?;

    // 重载 Nginx
    reload_nginx_service().await?;

    let req = CreateWebsiteRequest {
        domain: payload.domain.clone(),
        root_path,
        proxy_port: payload.proxy_port,
        enable_ssl: payload.enable_ssl.unwrap_or(false),
    };

    let site = state.website_repo.create(&req, &config_path).await?;

    tracing::info!("用户 '{}' 创建了站点 '{}'", _claims.sub, payload.domain);

    Ok(Json(site))
}

async fn delete_site(
    State(state): State<AppState>,
    CurrentUser(_claims): CurrentUser,
    Query(query): Query<IdQuery>,
) -> Result<Json<MessageResponse>, AppError> {
    let site = state.website_repo.find_by_id(query.id)
        .await?
        .ok_or(AppError::NotFound("站点不存在".into()))?;

    // 删除 Nginx 配置
    let _ = tokio::fs::remove_file(&site.config_path).await;
    let _ = tokio::fs::remove_file(format!("{}/{}", NGINX_ENABLED, site.domain)).await;

    state.website_repo.delete(query.id).await?;
    reload_nginx_service().await?;

    tracing::info!("用户 '{}' 删除了站点 '{}'", _claims.sub, site.domain);

    Ok(Json(MessageResponse {
        success: true,
        message: "站点删除成功".into(),
    }))
}

async fn enable_ssl(
    State(state): State<AppState>,
    CurrentUser(_claims): CurrentUser,
    Json(payload): Json<SslRequest>,
) -> Result<Json<MessageResponse>, AppError> {
    let site = state.website_repo.find_by_id(payload.id)
        .await?
        .ok_or(AppError::NotFound("站点不存在".into()))?;

    state.website_repo.update_ssl(payload.id, &payload.cert_path, &payload.key_path).await?;

    // 更新 Nginx 配置以启用 SSL
    let ssl_config = generate_nginx_ssl_config(&site.domain, &site.root_path, site.proxy_port, &payload.cert_path, &payload.key_path);
    tokio::fs::write(&site.config_path, &ssl_config)
        .await
        .map_err(|e| AppError::Internal(format!("更新SSL配置失败: {e}")))?;

    reload_nginx_service().await?;

    Ok(Json(MessageResponse {
        success: true,
        message: "SSL 启用成功".into(),
    }))
}

async fn toggle_site(
    State(state): State<AppState>,
    CurrentUser(_claims): CurrentUser,
    Json(payload): Json<serde_json::Value>,
) -> Result<Json<MessageResponse>, AppError> {
    let id = payload["id"].as_i64().ok_or(AppError::BadRequest("id 参数必填".into()))?;
    let enabled = payload["enabled"].as_bool().ok_or(AppError::BadRequest("enabled 参数必填".into()))?;

    let site = state.website_repo.find_by_id(id)
        .await?
        .ok_or(AppError::NotFound("站点不存在".into()))?;

    let enabled_link = format!("{}/{}", NGINX_ENABLED, site.domain);

    if enabled {
        let src = site.config_path.clone();
        let dst = enabled_link.clone();
        create_symlink(&src, &dst)?;
    } else {
        let _ = tokio::fs::remove_file(&enabled_link).await;
    }

    state.website_repo.toggle_enabled(id, enabled).await?;
    reload_nginx_service().await?;

    tracing::info!("用户 '{}' 将站点 '{}' enabled={}", _claims.sub, site.domain, enabled);

    Ok(Json(MessageResponse {
        success: true,
        message: format!("站点已{}", if enabled { "启用" } else { "禁用" }),
    }))
}

async fn reload_nginx(
    State(_state): State<AppState>,
    CurrentUser(_claims): CurrentUser,
) -> Result<Json<MessageResponse>, AppError> {
    reload_nginx_service().await?;

    Ok(Json(MessageResponse {
        success: true,
        message: "Nginx 已重载".into(),
    }))
}

#[cfg(unix)]
fn create_symlink(src: &str, dst: &str) -> Result<(), AppError> {
    std::os::unix::fs::symlink(src, dst)
        .map_err(|e| AppError::Internal(format!("创建软链接失败: {e}")))
}

#[cfg(not(unix))]
fn create_symlink(src: &str, dst: &str) -> Result<(), AppError> {
    std::fs::copy(src, dst)
        .map_err(|e| AppError::Internal(format!("复制文件失败: {e}")))?;
    Ok(())
}

async fn reload_nginx_service() -> Result<(), AppError> {
    let output = tokio::process::Command::new("nginx")
        .args(["-t"])
        .output()
        .await
        .map_err(|_| AppError::Internal("Nginx 不可用".into()))?;

    if !output.status.success() {
        let err = String::from_utf8_lossy(&output.stderr);
        return Err(AppError::Internal(format!("Nginx 配置错误: {err}")));
    }

    tokio::process::Command::new("nginx")
        .args(["-s", "reload"])
        .output()
        .await
        .map_err(|_| AppError::Internal("Nginx 重载失败".into()))?;

    Ok(())
}

fn generate_nginx_config(domain: &str, root: &str, proxy_port: Option<i32>) -> String {
    if let Some(port) = proxy_port {
        format!(
            r#"# {} - Generated by Flamepanel
server {{
    listen 80;
    server_name {};

    location / {{
        proxy_pass http://127.0.0.1:{};
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }}
}}
"#,
            domain, domain, port
        )
    } else {
        format!(
            r#"# {} - Generated by Flamepanel
server {{
    listen 80;
    server_name {};
    root {};
    index index.html index.htm index.php;

    location / {{
        try_files $uri $uri/ =404;
    }}

    location ~ \.php$ {{
        include fastcgi_params;
        fastcgi_pass unix:/var/run/php/php8.1-fpm.sock;
        fastcgi_param SCRIPT_FILENAME $document_root$fastcgi_script_name;
    }}
}}
"#,
            domain, domain, root
        )
    }
}

fn generate_nginx_ssl_config(domain: &str, root: &str, proxy_port: Option<i32>, cert_path: &str, key_path: &str) -> String {
    let location_block = if let Some(port) = proxy_port {
        format!(
            r#"    location / {{
        proxy_pass http://127.0.0.1:{};
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }}"#,
            port
        )
    } else {
        format!(
            r#"    root {};
    index index.html index.htm index.php;

    location / {{
        try_files $uri $uri/ =404;
    }}

    location ~ \.php$ {{
        include fastcgi_params;
        fastcgi_pass unix:/var/run/php/php8.1-fpm.sock;
        fastcgi_param SCRIPT_FILENAME $document_root$fastcgi_script_name;
    }}"#,
            root
        )
    };

    format!(
        r#"# {} - Generated by Flamepanel (SSL)
server {{
    listen 80;
    server_name {};
    return 301 https://$host$request_uri;
}}

server {{
    listen 443 ssl http2;
    server_name {};

    ssl_certificate {};
    ssl_certificate_key {};
    ssl_protocols TLSv1.2 TLSv1.3;
    ssl_ciphers HIGH:!aNULL:!MD5;

{}
}}
"#,
        domain, domain, domain, cert_path, key_path, location_block
    )
}
