use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::{Deserialize, Serialize};
use crate::core::error::AppError;

// ─── 插件清单 ─────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginManifest {
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: Option<String>,
    pub entry: String,
    pub api_prefix: String,
    pub permissions: Vec<String>,
}

// ─── 插件运行时 ───────────────────────────────────────────────────────────────

#[derive(Debug)]
pub enum PluginState {
    Loaded,
    Running,
    Stopped,
    Error(String),
}

#[derive(Debug)]
pub struct Plugin {
    pub manifest: PluginManifest,
    pub state: PluginState,
    pub process: Option<tokio::process::Child>,
}

// ─── Plugin Manager ───────────────────────────────────────────────────────────

pub type PluginManagerRef = Arc<RwLock<PluginManager>>;

pub struct PluginManager {
    plugins: HashMap<String, Plugin>,
    base_dir: String,
}

impl PluginManager {
    pub fn new(base_dir: String) -> Self {
        Self {
            plugins: HashMap::new(),
            base_dir,
        }
    }

    /// 扫描并加载所有插件清单（不启动）
    pub async fn load_all(&mut self) -> Result<usize, AppError> {
        let plugins_dir = std::path::Path::new(&self.base_dir).join("plugins");
        if !plugins_dir.exists() {
            tracing::info!("插件目录不存在: {:?}", plugins_dir);
            return Ok(0);
        }

        let mut count = 0;
        let entries = std::fs::read_dir(&plugins_dir)
            .map_err(|e| AppError::Internal(format!("读取插件目录失败: {e}")))?;

        for entry in entries {
            let entry = entry.map_err(|e| AppError::Internal(e.to_string()))?;
            let manifest_path = entry.path().join("plugin.toml");

            if !manifest_path.exists() {
                continue;
            }

            let content = std::fs::read_to_string(&manifest_path)
                .map_err(|e| AppError::Internal(format!("读取插件清单失败: {e}")))?;

            let manifest: PluginManifest = toml::from_str(&content)
                .map_err(|e| AppError::Internal(format!("解析插件清单失败: {e}")))?;

            tracing::info!("发现插件: {} v{} by {}", manifest.name, manifest.version, manifest.author);

            self.plugins.insert(manifest.name.clone(), Plugin {
                manifest,
                state: PluginState::Loaded,
                process: None,
            });
            count += 1;
        }

        Ok(count)
    }

    /// 启动指定插件（作为子进程）
    pub async fn start(&mut self, name: &str) -> Result<(), AppError> {
        let plugin = self.plugins.get_mut(name)
            .ok_or(AppError::NotFound(format!("插件 '{}' 不存在", name)))?;

        let entry_path = std::path::Path::new(&self.base_dir)
            .join("plugins")
            .join(&plugin.manifest.name)
            .join(&plugin.manifest.entry);

        if !entry_path.exists() {
            return Err(AppError::Internal(format!("插件入口文件不存在: {:?}", entry_path)));
        }

        let child = tokio::process::Command::new(&entry_path)
            .env("PLUGIN_API_PREFIX", &plugin.manifest.api_prefix)
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .map_err(|e| AppError::Internal(format!("启动插件失败: {e}")))?;

        plugin.process = Some(child);
        plugin.state = PluginState::Running;

        tracing::info!("插件 '{}' 已启动", name);
        Ok(())
    }

    /// 停止指定插件
    pub async fn stop(&mut self, name: &str) -> Result<(), AppError> {
        let plugin = self.plugins.get_mut(name)
            .ok_or(AppError::NotFound(format!("插件 '{}' 不存在", name)))?;

        if let Some(ref mut child) = plugin.process {
            child.kill().await
                .map_err(|e| AppError::Internal(format!("停止插件失败: {e}")))?;
            child.wait().await
                .map_err(|e| AppError::Internal(format!("等待插件退出失败: {e}")))?;
        }

        plugin.process = None;
        plugin.state = PluginState::Stopped;

        tracing::info!("插件 '{}' 已停止", name);
        Ok(())
    }

    /// 获取所有插件列表
    pub fn list(&self) -> Vec<PluginInfo> {
        self.plugins.iter().map(|(name, p)| {
            PluginInfo {
                name: name.clone(),
                version: p.manifest.version.clone(),
                author: p.manifest.author.clone(),
                description: p.manifest.description.clone(),
                state: format!("{:?}", p.state),
                api_prefix: p.manifest.api_prefix.clone(),
            }
        }).collect()
    }
}

#[derive(Debug, Serialize)]
pub struct PluginInfo {
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: Option<String>,
    pub state: String,
    pub api_prefix: String,
}
