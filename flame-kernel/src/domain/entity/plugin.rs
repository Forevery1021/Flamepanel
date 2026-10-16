//! 插件与依赖。

use super::*;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Plugin {
    pub id: String,
    pub name: String,
    pub version: String,
    pub author: String,
    pub description: String,
    pub wasm_hash: String,
    /// WASM 字节码（base64 编码），用于持久化与恢复
    pub wasm_base64: String,
    pub enabled: bool,
    pub homepage: Option<String>,
    pub license: Option<String>,
    pub tags: Vec<String>,
    pub config_schema: Option<String>,
    pub dependencies: Vec<PluginDependency>,
    /// 管理员授予的宿主能力（Doc/21 §5.7，如 `fs:read:/var/log`、`metrics:read`）；
    /// 空 = 仅基础宿主函数（日志 / 输入输出 / 读自身设置）
    #[serde(default)]
    pub capabilities: Vec<String>,
    /// 插件 KV 设置（API 与 `flame.setting_set` 写入，持久化，重启后恢复）
    #[serde(default)]
    pub settings: std::collections::HashMap<String, String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PluginDependency {
    pub plugin_id: String,
    pub version_requirement: String,
    pub optional: bool,
}
