use axum::{
    extract::{Query, State},
    routing::{get, post, delete},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::application::AppState;
use crate::core::error::AppError;
use crate::domain::{CreateWebsiteRequest, Website};
use crate::middleware::auth::CurrentUser;

const WWW_ROOT: &str = "/www";

// ─── Engine paths ──────────────────────────────────────────────────────────────

struct EnginePaths {
    available: &'static str,
    enabled: &'static str,
    reload_cmd: &'static [&'static str],
    test_cmd: &'static [&'static str],
}

fn engine_paths(engine: &str) -> Result<EnginePaths, AppError> {
    match engine {
        "nginx" => Ok(EnginePaths {
            available: "/etc/nginx/sites-available",
            enabled: "/etc/nginx/sites-enabled",
            reload_cmd: &["nginx", "-s", "reload"],
            test_cmd: &["nginx", "-t"],
        }),
        "openresty" => Ok(EnginePaths {
            available: "/etc/nginx/sites-available",
            enabled: "/etc/nginx/sites-enabled",
            reload_cmd: &["nginx", "-s", "reload"],
            test_cmd: &["nginx", "-t"],
        }),
        "apache" => Ok(EnginePaths {
            available: "/etc/apache2/sites-available",
            enabled: "/etc/apache2/sites-enabled",
            reload_cmd: &["systemctl", "reload", "apache2"],
            test_cmd: &["apache2ctl", "configtest"],
        }),
        "lighttpd" => Ok(EnginePaths {
            available: "/etc/lighttpd/conf-available",
            enabled: "/etc/lighttpd/conf-enabled",
            reload_cmd: &["systemctl", "reload", "lighttpd"],
            test_cmd: &["lighttpd", "-t"],
        }),
        _ => Err(AppError::BadRequest(format!("不支持的引擎: {engine}"))),
    }
}

// ─── DTOs ─────────────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
pub struct CreateSiteRequest {
    pub domain: String,
    pub root_path: Option<String>,
    pub proxy_port: Option<i32>,
    pub enable_ssl: Option<bool>,
    pub engine: Option<String>,
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

// ─── Routes ───────────────────────────────────────────────────────────────────

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/list", get(list_sites))
        .route("/get", get(get_site))
        .route("/create", post(create_site))
        .route("/delete", delete(delete_site))
        .route("/ssl", post(enable_ssl))
        .route("/toggle", post(toggle_site))
        .route("/reload", post(reload_engine))
}

// ─── Handlers ─────────────────────────────────────────────────────────────────

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

    let engine = payload.engine.as_deref().unwrap_or("nginx");
    let paths = engine_paths(engine)?;

    if state.website_repo.find_by_domain(&payload.domain).await?.is_some() {
        return Err(AppError::BadRequest("该域名站点已存在".into()));
    }

    let root_path = payload.root_path.unwrap_or_else(|| format!("{}/{}", WWW_ROOT, payload.domain));
    tokio::fs::create_dir_all(&root_path)
        .await
        .map_err(|e| AppError::Internal(format!("创建网站目录失败: {e}")))?;

    let config_path = format!("{}/{}.conf", paths.available, payload.domain);
    let config = generate_config(engine, &payload.domain, &root_path, payload.proxy_port, false, None, None);
    tokio::fs::write(&config_path, &config)
        .await
        .map_err(|e| AppError::Internal(format!("写入配置失败: {e}")))?;

    // Enable the site
    let enabled_link = format!("{}/{}.conf", paths.enabled, payload.domain);
    let _ = tokio::fs::remove_file(&enabled_link).await;
    create_symlink(&config_path, &enabled_link)?;

    reload_engine_service(engine).await?;

    let req = CreateWebsiteRequest {
        domain: payload.domain.clone(),
        root_path,
        proxy_port: payload.proxy_port,
        enable_ssl: payload.enable_ssl.unwrap_or(false),
        engine: Some(engine.to_string()),
    };

    let site = state.website_repo.create(&req, &config_path).await?;

    tracing::info!("用户 '{}' 创建了站点 '{}' (engine={})", _claims.sub, payload.domain, engine);

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

    let paths = engine_paths(&site.engine)?;

    let _ = tokio::fs::remove_file(&site.config_path).await;
    let _ = tokio::fs::remove_file(format!("{}/{}.conf", paths.enabled, site.domain)).await;

    state.website_repo.delete(query.id).await?;
    reload_engine_service(&site.engine).await?;

    tracing::info!("用户 '{}' 删除了站点 '{}'", _claims.sub, site.domain);

    Ok(Json(MessageResponse { success: true, message: "站点删除成功".into() }))
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

    let ssl_config = generate_config(
        &site.engine, &site.domain, &site.root_path, site.proxy_port,
        true, Some(&payload.cert_path), Some(&payload.key_path),
    );
    tokio::fs::write(&site.config_path, &ssl_config)
        .await
        .map_err(|e| AppError::Internal(format!("更新SSL配置失败: {e}")))?;

    reload_engine_service(&site.engine).await?;

    Ok(Json(MessageResponse { success: true, message: "SSL 启用成功".into() }))
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

    let paths = engine_paths(&site.engine)?;
    let enabled_link = format!("{}/{}.conf", paths.enabled, site.domain);

    if enabled {
        create_symlink(&site.config_path, &enabled_link)?;
    } else {
        let _ = tokio::fs::remove_file(&enabled_link).await;
    }

    state.website_repo.toggle_enabled(id, enabled).await?;
    reload_engine_service(&site.engine).await?;

    tracing::info!("用户 '{}' 将站点 '{}' enabled={}", _claims.sub, site.domain, enabled);

    Ok(Json(MessageResponse {
        success: true,
        message: format!("站点已{}", if enabled { "启用" } else { "禁用" }),
    }))
}

async fn reload_engine(
    State(_state): State<AppState>,
    CurrentUser(_claims): CurrentUser,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<MessageResponse>, AppError> {
    let engine = params.get("engine").map(|s| s.as_str()).unwrap_or("nginx");
    reload_engine_service(engine).await?;

    Ok(Json(MessageResponse { success: true, message: format!("{engine} 已重载") }))
}

// ─── Engine helpers ───────────────────────────────────────────────────────────

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

async fn reload_engine_service(engine: &str) -> Result<(), AppError> {
    let paths = engine_paths(engine)?;

    if paths.test_cmd.len() > 1 || paths.test_cmd[0] != "systemctl" {
        let output = tokio::process::Command::new(paths.test_cmd[0])
            .args(&paths.test_cmd[1..])
            .output()
            .await
            .map_err(|_| AppError::Internal(format!("{engine} 不可用")))?;

        if !output.status.success() {
            let err = String::from_utf8_lossy(&output.stderr);
            return Err(AppError::Internal(format!("{engine} 配置错误: {err}")));
        }
    }

    tokio::process::Command::new(paths.reload_cmd[0])
        .args(&paths.reload_cmd[1..])
        .output()
        .await
        .map_err(|_| AppError::Internal(format!("{engine} 重载失败")))?;

    Ok(())
}

// ─── Config generators ────────────────────────────────────────────────────────

fn generate_config(
    engine: &str,
    domain: &str,
    root: &str,
    proxy_port: Option<i32>,
    ssl: bool,
    cert_path: Option<&str>,
    key_path: Option<&str>,
) -> String {
    match engine {
        "nginx" | "openresty" => nginx_config(domain, root, proxy_port, ssl, cert_path, key_path),
        "apache" => apache_config(domain, root, proxy_port, ssl, cert_path, key_path),
        "lighttpd" => lighttpd_config(domain, root, proxy_port, ssl, cert_path, key_path),
        _ => format!("# Unknown engine: {engine}"),
    }
}

fn nginx_config(
    domain: &str, root: &str, proxy_port: Option<i32>,
    ssl: bool, cert_path: Option<&str>, key_path: Option<&str>,
) -> String {
    if ssl {
        let cert = cert_path.unwrap_or("");
        let key = key_path.unwrap_or("");
        if let Some(port) = proxy_port {
            format!(
                "# {domain} - Flamepanel\n\
                 server {{\n\
                 \x20   listen 80;\n\
                 \x20   server_name {domain};\n\
                 \x20   return 301 https://$host$request_uri;\n\
                 }}\n\
                 server {{\n\
                 \x20   listen 443 ssl http2;\n\
                 \x20   server_name {domain};\n\
                 \x20   ssl_certificate {cert};\n\
                 \x20   ssl_certificate_key {key};\n\
                 \x20   ssl_protocols TLSv1.2 TLSv1.3;\n\
                 \x20   location / {{\n\
                 \x20       proxy_pass http://127.0.0.1:{port};\n\
                 \x20       proxy_set_header Host $host;\n\
                 \x20       proxy_set_header X-Real-IP $remote_addr;\n\
                 \x20       proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;\n\
                 \x20       proxy_set_header X-Forwarded-Proto $scheme;\n\
                 \x20   }}\n\
                 }}",
            )
        } else {
            format!(
                "# {domain} - Flamepanel\n\
                 server {{\n\
                 \x20   listen 80;\n\
                 \x20   server_name {domain};\n\
                 \x20   return 301 https://$host$request_uri;\n\
                 }}\n\
                 server {{\n\
                 \x20   listen 443 ssl http2;\n\
                 \x20   server_name {domain};\n\
                 \x20   ssl_certificate {cert};\n\
                 \x20   ssl_certificate_key {key};\n\
                 \x20   ssl_protocols TLSv1.2 TLSv1.3;\n\
                 \x20   root {root};\n\
                 \x20   index index.html index.htm index.php;\n\
                 \x20   location / {{\n\
                 \x20       try_files $uri $uri/ =404;\n\
                 \x20   }}\n\
                 \x20   location ~ \\.php$ {{\n\
                 \x20       include fastcgi_params;\n\
                 \x20       fastcgi_pass unix:/var/run/php/php8.1-fpm.sock;\n\
                 \x20       fastcgi_param SCRIPT_FILENAME $document_root$fastcgi_script_name;\n\
                 \x20   }}\n\
                 }}",
            )
        }
    } else if let Some(port) = proxy_port {
        format!(
            "# {domain} - Flamepanel\n\
             server {{\n\
             \x20   listen 80;\n\
             \x20   server_name {domain};\n\
             \x20   location / {{\n\
             \x20       proxy_pass http://127.0.0.1:{port};\n\
             \x20       proxy_set_header Host $host;\n\
             \x20       proxy_set_header X-Real-IP $remote_addr;\n\
             \x20       proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;\n\
             \x20       proxy_set_header X-Forwarded-Proto $scheme;\n\
             \x20   }}\n\
             }}",
        )
    } else {
        format!(
            "# {domain} - Flamepanel\n\
             server {{\n\
             \x20   listen 80;\n\
             \x20   server_name {domain};\n\
             \x20   root {root};\n\
             \x20   index index.html index.htm index.php;\n\
             \x20   location / {{\n\
             \x20       try_files $uri $uri/ =404;\n\
             \x20   }}\n\
             \x20   location ~ \\.php$ {{\n\
             \x20       include fastcgi_params;\n\
             \x20       fastcgi_pass unix:/var/run/php/php8.1-fpm.sock;\n\
             \x20       fastcgi_param SCRIPT_FILENAME $document_root$fastcgi_script_name;\n\
             \x20   }}\n\
             }}",
        )
    }
}

fn apache_config(
    domain: &str, root: &str, proxy_port: Option<i32>,
    ssl: bool, _cert: Option<&str>, _key: Option<&str>,
) -> String {
    let port = if ssl { 443 } else { 80 };
    if let Some(proxy) = proxy_port {
        format!(
            "# {domain} - Flamepanel\n\
             <VirtualHost *:{port}>\n\
             \x20   ServerName {domain}\n\
             \x20   ProxyPass / http://127.0.0.1:{proxy}/\n\
             \x20   ProxyPassReverse / http://127.0.0.1:{proxy}/\n\
             {ssl_block}\
             </VirtualHost>",
            ssl_block = if ssl {
                "   SSLEngine on\n   SSLCertificateFile /etc/ssl/certs/ssl-cert-snakeoil.pem\n   SSLCertificateKeyFile /etc/ssl/private/ssl-cert-snakeoil.key\n"
            } else { "" }
        )
    } else {
        format!(
            "# {domain} - Flamepanel\n\
             <VirtualHost *:{port}>\n\
             \x20   ServerName {domain}\n\
             \x20   DocumentRoot {root}\n\
             \x20   <Directory {root}>\n\
             \x20       Options Indexes FollowSymLinks\n\
             \x20       AllowOverride All\n\
             \x20       Require all granted\n\
             \x20   </Directory>\n\
             {ssl_block}\
             </VirtualHost>",
            ssl_block = if ssl {
                "   SSLEngine on\n   SSLCertificateFile /etc/ssl/certs/ssl-cert-snakeoil.pem\n   SSLCertificateKeyFile /etc/ssl/private/ssl-cert-snakeoil.key\n"
            } else { "" }
        )
    }
}

fn lighttpd_config(
    domain: &str, root: &str, proxy_port: Option<i32>,
    _ssl: bool, _cert: Option<&str>, _key: Option<&str>,
) -> String {
    if let Some(port) = proxy_port {
        format!(
            "# {domain} - Flamepanel\n\
             $HTTP[\"host\"] == \"{domain}\" {{\n\
             \x20   proxy.server = ( \"\" => ( ( \"host\" => \"127.0.0.1\", \"port\" => {port} ) ) )\n\
             }}"
        )
    } else {
        format!(
            "# {domain} - Flamepanel\n\
             $HTTP[\"host\"] == \"{domain}\" {{\n\
             \x20   server.document-root = \"{root}\"\n\
             \x20   index-file.names = ( \"index.html\", \"index.php\" )\n\
             }}"
        )
    }
}
