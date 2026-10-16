//! 内置 WASM 工具与重启后插件恢复。

use super::*;

impl AppStoreService {
    /// 内置 WASM 工具列表
    pub fn wasm_builtins(&self) -> Vec<AppMetadata> {
        vec![AppMetadata {
            key: "wasm-hello".into(),
            name: "Hello WASM".into(),
            category: "wasm".into(),
            short_desc_zh: "内置 WASM 演示工具：导出 run() 函数返回 42，验证沙箱可用".into(),
            short_desc_en: Some("Builtin WASM demo tool: exports run() returning 42".into()),
            tags: vec!["wasm".into(), "demo".into()],
            format: AppFormat::Flame,
            modes: vec![InstallMode::Wasm],
            versions: vec!["1.0.0".into()],
            default_version: "1.0.0".into(),
            logo: Some("wasm".into()),
            min_memory_mb: None,
            architectures: vec![],
            readme: None,
            recommended: false,
        }]
    }

    /// 内置 WASM 工具版本信息
    pub fn wasm_builtin_version(&self, key: &str, version: &str) -> Option<AppVersionInfo> {
        if key == "wasm-hello" && version == "1.0.0" {
            Some(AppVersionInfo {
                version: "1.0.0".into(),
                mode: InstallMode::Wasm,
                default_port: None,
                form_fields: vec![FormField {
                    env_key: "name".into(),
                    label_zh: "插件名称".into(),
                    label_en: Some("Plugin name".into()),
                    field_type: crate::domain::entity::FieldType::Text,
                    default: Some("wasm-hello".into()),
                    required: true,
                    pattern: Some(r"^[a-zA-Z0-9_-]+$".into()),
                    min: None,
                    max: None,
                    min_length: Some(2),
                    max_length: Some(64),
                    options: vec![],
                    description: None,
                    group: Some("基础".into()),
                }],
                compose_template: None,
                native_scripts: vec![],
                wasm_base64: Some(WASM_HELLO_B64.into()),
                hooks: crate::domain::entity::AppHooks::default(),
                min_memory_mb: None,
                architectures: vec![],
            })
        } else {
            None
        }
    }

    /// 从持久化恢复 WASM 插件到沙箱（启动时调用）
    pub async fn restore_wasm_plugins(&self) -> Result<usize, AppError> {
        let plugins = self.plugin_repo.list().await?;
        let mut count = 0;
        for plugin in plugins {
            if self.plugin_sandbox.get_plugin(&plugin.id).await.is_ok() {
                if !self.plugin_registry.exists(&plugin.id) {
                    let _ = self.plugin_registry.register(plugin.clone());
                }
                count += 1;
                continue;
            }
            match base64::engine::general_purpose::STANDARD.decode(&plugin.wasm_base64) {
                Ok(bytes) => {
                    // 恢复时强制校验哈希，防止磁盘上插件被篡改
                    if let Err(e) = crate::plugin::verify_wasm_hash(&bytes, Some(&plugin.wasm_hash))
                    {
                        tracing::warn!("恢复 WASM 插件 {} 完整性校验失败: {}", plugin.id, e);
                        continue;
                    }
                    // 能力授予与设置随插件持久化，恢复时原样带回沙箱
                    let config = PluginConfig {
                        capabilities: plugin.capabilities.clone(),
                        settings: plugin.settings.clone(),
                        ..PluginConfig::default()
                    };
                    if let Err(e) = self
                        .plugin_sandbox
                        .load_plugin(&plugin.id, bytes, Some(config))
                        .await
                    {
                        tracing::warn!("恢复 WASM 插件 {} 失败: {}", plugin.id, e);
                        continue;
                    }
                    if !plugin.enabled {
                        let _ = self.plugin_sandbox.disable_plugin(&plugin.id).await;
                    }
                    // 旧实现只恢复到沙箱、不进注册表：重启后 GET /api/plugins 为空、执行 404
                    if !self.plugin_registry.exists(&plugin.id) {
                        if let Err(e) = self.plugin_registry.register(plugin.clone()) {
                            tracing::warn!("恢复 WASM 插件 {} 注册失败: {}", plugin.id, e);
                        }
                    }
                    count += 1;
                }
                Err(e) => {
                    tracing::warn!("WASM 插件 {} base64 解码失败: {}", plugin.id, e);
                }
            }
        }
        Ok(count)
    }
}
