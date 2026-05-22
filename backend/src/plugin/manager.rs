use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use serde::Deserialize;
use crate::core::error::AppError;

#[derive(Deserialize, Debug)]
pub struct PluginManifest {
    pub name: String,
    pub version: String,
    pub author: String,
    pub entry: String,           // 相对路径
    pub api_prefix: String,
    pub permissions: Vec<String>,
}

pub struct Plugin {
    pub manifest: PluginManifest,
    pub handle: Option<tokio::process::Child>, // 子进程模式
}

pub type PluginManager = Arc<RwLock<PluginManagerInner>>;

pub struct PluginManagerInner {
    plugins: HashMap<String, Plugin>,
    base_dir: String,
}

impl PluginManagerInner {
    pub fn new(base_dir: String) -> Self {
        Self {
            plugins: HashMap::new(),
            base_dir,
        }
    }

    pub async fn load_all(&mut self) -> Result<(), AppError> {
        let plugins_dir = std::path::Path::new(&self.base_dir).join("plugins");
        if !plugins_dir.exists() {
            return Ok(());
        }

        for entry in std::fs::read_dir(plugins_dir)? {
            let entry = entry?;
            let manifest_path = entry.path().join("plugin.toml");
            if manifest_path.exists() {
                let content = std::fs::read_to_string(manifest_path)?;
                let manifest: PluginManifest = toml::from_str(&content)?;
                
                tracing::info!("加载插件: {}", manifest.name);
                self.plugins.insert(manifest.name.clone(), Plugin {
                    manifest,
                    handle: None,
                });
            }
        }
        Ok(())
    }
}