//! 插件市场 API（Doc/21 §5.8）：浏览内置 + 远程目录，管理员同意能力申请后一键安装。

use super::{install_plugin, plugin_config, PluginResponse};
use crate::api::extract::ApiJson;
use crate::api::types::AppState;
use crate::core::error::AppError;
use crate::domain::entity::Plugin;
use crate::plugin::market::{self, MarketPlugin, MarketSource};
use axum::{
    extract::{Path, State},
    Json,
};
use chrono::Utc;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Serialize, ToSchema)]
pub struct MarketPluginView {
    #[serde(flatten)]
    pub plugin: MarketPlugin,
    /// 已安装（同 ID 插件已注册）
    pub installed: bool,
    pub installed_version: Option<String>,
}

#[derive(Debug, Serialize, ToSchema)]
pub struct MarketListResponse {
    pub plugins: Vec<MarketPluginView>,
    /// 已配置的远程索引地址（设置项 `plugin_market_url`）
    pub remote_url: Option<String>,
    /// 远程索引拉取失败原因（内置目录仍可用）
    pub remote_error: Option<String>,
}

#[derive(Debug, Deserialize, ToSchema)]
pub struct MarketInstallRequest {
    /// 插件申请了宿主能力时必须为 true（管理员已查看并同意能力清单）
    #[serde(default)]
    pub accept_capabilities: bool,
}

async fn remote_url(state: &AppState) -> Option<String> {
    state
        .settings_service
        .get(market::MARKET_URL_SETTING)
        .await
        .ok()
        .flatten()
        .map(|u| u.trim().to_string())
        .filter(|u| !u.is_empty())
}

async fn catalog(state: &AppState) -> (Vec<MarketPlugin>, Option<String>, Option<String>) {
    let mut list = market::builtin_catalog();
    let url = remote_url(state).await;
    let mut err = None;
    if let Some(u) = &url {
        match market::fetch_remote_index(u).await {
            Ok(remote) => list.extend(remote),
            Err(e) => err = Some(e.to_string()),
        }
    }
    (list, url, err)
}

/// 插件市场目录
#[utoipa::path(
    get,
    path = "/api/plugins/market",
    tag = "plugins",
    responses(
        (status = 200, description = "市场目录（内置 + 远程）", body = MarketListResponse),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 plugin:read 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn list_market(
    State(state): State<AppState>,
) -> Result<Json<MarketListResponse>, AppError> {
    let (list, remote_url, remote_error) = catalog(&state).await;
    let plugins = list
        .into_iter()
        .map(|p| {
            let installed = state.plugin_registry.get(&p.id).ok();
            MarketPluginView {
                installed: installed.is_some(),
                installed_version: installed.map(|i| i.version),
                plugin: p,
            }
        })
        .collect();
    Ok(Json(MarketListResponse {
        plugins,
        remote_url,
        remote_error,
    }))
}

/// 从市场安装插件
#[utoipa::path(
    post,
    path = "/api/plugins/market/{id}/install",
    tag = "plugins",
    params(("id" = String, Path, description = "市场插件 ID")),
    request_body = MarketInstallRequest,
    responses(
        (status = 200, description = "已安装", body = PluginResponse),
        (status = 400, description = "未同意能力申请 / 包体哈希不符 / WASM 非法"),
        (status = 404, description = "市场中无此插件"),
        (status = 409, description = "同 ID 插件已安装"),
        (status = 503, description = "远程市场不可达或 WASM 支持未编译"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 plugin:create 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn install_market_plugin(
    State(state): State<AppState>,
    Path(id): Path<String>,
    ApiJson(req): ApiJson<MarketInstallRequest>,
) -> Result<Json<PluginResponse>, AppError> {
    let entry = match market::builtin_catalog().into_iter().find(|p| p.id == id) {
        Some(e) => e,
        None => {
            let url = remote_url(&state)
                .await
                .ok_or_else(|| AppError::NotFound(format!("market plugin {id} not found")))?;
            market::fetch_remote_index(&url)
                .await?
                .into_iter()
                .find(|p| p.id == id)
                .ok_or_else(|| AppError::NotFound(format!("market plugin {id} not found")))?
        }
    };
    if !entry.capabilities.is_empty() && !req.accept_capabilities {
        return Err(AppError::BadRequest(format!(
            "plugin {id} requests capabilities {:?}; set accept_capabilities=true to grant them",
            entry.capabilities
        )));
    }
    if state.plugin_registry.exists(&entry.id) {
        return Err(AppError::Conflict(format!("Plugin {id} already installed")));
    }
    let wasm = match entry.source {
        MarketSource::Builtin => market::builtin_wasm(&entry.id)?,
        MarketSource::Remote => market::fetch_remote_wasm(&entry).await?,
    };
    let now = Utc::now();
    let mut tags = entry.tags.clone();
    tags.push("market".into());
    let plugin = Plugin {
        id: entry.id,
        name: entry.name,
        version: entry.version,
        author: entry.author,
        description: entry.description,
        wasm_hash: String::new(),
        wasm_base64: String::new(),
        enabled: true,
        homepage: entry.homepage,
        license: entry.license,
        tags,
        config_schema: None,
        dependencies: vec![],
        capabilities: entry.capabilities,
        settings: Default::default(),
        created_at: now,
        updated_at: now,
    };
    let config = plugin_config(None, None)?;
    Ok(Json(install_plugin(&state, plugin, wasm, config).await?))
}
