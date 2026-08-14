pub mod baota;
pub mod flame;
pub mod onepanel;

use crate::core::error::AppError;
use crate::domain::entity::FormField;
use async_trait::async_trait;
use std::path::Path;
use std::sync::Arc;

// 应用包适配器 trait 定义位于 application 层端口（六边形），此处重新导出。
pub use crate::application::app_store_ports::AppPackageAdapter;

/// 供适配器复用：排除非版本目录名
pub fn is_version_dir(name: &str) -> bool {
    if name.starts_with('.') || name.starts_with('v') {
        return false;
    }
    let lower = name.to_lowercase();
    let excluded = [
        "logo",
        "logo.png",
        "readme",
        "readme.md",
        "readme_en",
        "readme_en.md",
        "data",
        "scripts",
        "app.yml",
        "app.yaml",
        "data.yml",
        "data.yaml",
        "app.json",
        "icon",
        "icon.png",
        "latest",
        "docs",
        "assets",
        ".git",
        ".github",
    ];
    if excluded.contains(&lower.as_str()) {
        return false;
    }
    // 版本目录需含版本特征（数字或带版本分隔符）
    name.chars().any(|c| c.is_ascii_digit()) || lower.contains("beta") || lower.contains("rc")
}

/// 根据目录结构自动选择适配器（默认实现，供 `AppAdapterProvider` 使用）
pub fn select_adapter(root: &Path) -> Result<Arc<dyn AppPackageAdapter>, AppError> {
    let adapters: Vec<Arc<dyn AppPackageAdapter>> = vec![
        Arc::new(flame::FlameAdapter),
        Arc::new(onepanel::OnePanelAdapter),
        Arc::new(baota::BaotaAdapter),
    ];
    adapters
        .into_iter()
        .find(|a| a.detect(root))
        .ok_or_else(|| {
            AppError::BadRequest("无法识别的应用包格式（需包含 app.json 或 data.yml）".into())
        })
}

/// 适配器选择端口实现：将 `select_adapter` 封装为可注入的端口。
pub struct DefaultAdapterProvider;

#[async_trait]
impl crate::application::app_store_ports::AppAdapterProvider for DefaultAdapterProvider {
    fn select(&self, root: &Path) -> Result<Arc<dyn AppPackageAdapter>, AppError> {
        select_adapter(root)
    }
}

pub(crate) fn field_type_from_str(s: &str) -> crate::domain::entity::FieldType {
    crate::domain::entity::FieldType::from_name(s).unwrap_or(crate::domain::entity::FieldType::Text)
}

pub(crate) fn clean_label(s: &str) -> String {
    if s.is_empty() {
        return "参数".into();
    }
    s.to_string()
}

pub(crate) fn form_field(env_key: &str, label: &str) -> FormField {
    FormField {
        env_key: env_key.into(),
        label_zh: clean_label(label),
        label_en: None,
        field_type: crate::domain::entity::FieldType::Text,
        default: None,
        required: false,
        pattern: None,
        min: None,
        max: None,
        min_length: None,
        max_length: None,
        options: vec![],
        description: None,
        group: None,
    }
}
