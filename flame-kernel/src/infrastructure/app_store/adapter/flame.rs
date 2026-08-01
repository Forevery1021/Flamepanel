use std::path::Path;
use base64::Engine;
use serde::Deserialize;
use crate::core::error::AppError;
use crate::domain::entity::{AppFormat, AppManifest, AppMetadata, AppVersionInfo, FieldType, FormField, InstallMode};
use super::{AppPackageAdapter, field_type_from_str, form_field, is_version_dir};

/// 内置 Flame 格式：
/// - 内置应用目录：直接使用 `builtin_apps()` 的 compose 模板
/// - 文件应用目录：`app.json` + `<version>/docker-compose.yml | install.sh | app.wasm`
pub struct FlameAdapter;

#[derive(Debug, Deserialize)]
struct FlameAppJson {
    key: Option<String>,
    name: Option<String>,
    category: Option<String>,
    #[serde(rename = "short_desc_zh")]
    short_desc_zh: Option<String>,
    #[serde(rename = "short_desc_en")]
    short_desc_en: Option<String>,
    tags: Option<Vec<String>>,
    #[serde(rename = "mode")]
    mode: Option<String>,
    #[serde(rename = "default_port")]
    default_port: Option<i32>,
    icon: Option<String>,
    versions: Option<Vec<String>>,
    #[serde(rename = "form_fields")]
    form_fields: Option<Vec<FlameFormField>>,
    readme: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FlameFormField {
    #[serde(rename = "env_key")]
    env_key: Option<String>,
    #[serde(rename = "label_zh")]
    label_zh: Option<String>,
    #[serde(rename = "label_en")]
    label_en: Option<String>,
    #[serde(rename = "field_type")]
    field_type: Option<String>,
    default: Option<String>,
    required: Option<bool>,
    options: Option<Vec<FlameSelectOption>>,
    description: Option<String>,
}

#[derive(Debug, Deserialize)]
struct FlameSelectOption {
    label: String,
    value: String,
}

impl FlameAdapter {
    fn read_app_json(root: &Path) -> Result<FlameAppJson, AppError> {
        let path = root.join("app.json");
        if !path.exists() {
            return Err(AppError::BadRequest(format!("缺少 app.json: {}", root.display())));
        }
        let content = std::fs::read_to_string(&path)
            .map_err(|e| AppError::BadRequest(format!("读取 app.json 失败: {}", e)))?;
        serde_json::from_str(&content)
            .map_err(|e| AppError::BadRequest(format!("解析 app.json 失败: {}", e)))
    }

    pub fn find_builtin(key: &str) -> Option<AppManifest> {
        crate::domain::entity::builtin_apps()
            .into_iter()
            .find(|m| m.key == key)
    }

    /// 从内置 AppManifest 生成元数据
    pub fn builtin_metadata(manifest: &AppManifest) -> AppMetadata {
        AppMetadata {
            key: manifest.key.clone(),
            name: manifest.name.clone(),
            category: manifest.category.clone(),
            short_desc_zh: manifest.description.clone(),
            short_desc_en: None,
            tags: vec![],
            format: AppFormat::Flame,
            modes: vec![InstallMode::Container],
            versions: vec![manifest.version.clone()],
            default_version: manifest.version.clone(),
            logo: Some(manifest.icon.clone()),
            min_memory_mb: None,
            architectures: vec![],
            readme: None,
        }
    }

    /// 从内置 AppManifest 生成版本信息（含默认表单字段）
    pub fn builtin_version(manifest: &AppManifest) -> AppVersionInfo {
        AppVersionInfo {
            version: manifest.version.clone(),
            mode: InstallMode::Container,
            default_port: Some(manifest.default_port),
            form_fields: vec![
                FormField {
                    env_key: "PORT".into(),
                    label_zh: "服务端口".into(),
                    label_en: Some("Port".into()),
                    field_type: FieldType::Port,
                    default: Some(manifest.default_port.to_string()),
                    required: true,
                    pattern: None,
                    min: Some(1),
                    max: Some(65535),
                    min_length: None,
                    max_length: None,
                    options: vec![],
                    description: None,
                    group: Some("基础".into()),
                },
                FormField {
                    env_key: "NAME".into(),
                    label_zh: "实例名称".into(),
                    label_en: Some("Instance name".into()),
                    field_type: FieldType::Text,
                    default: Some(manifest.key.clone()),
                    required: true,
                    pattern: Some(r"^[a-zA-Z0-9_-]+$".into()),
                    min: None,
                    max: None,
                    min_length: Some(2),
                    max_length: Some(32),
                    options: vec![],
                    description: None,
                    group: Some("基础".into()),
                },
            ],
            compose_template: Some(manifest.compose.clone()),
            native_scripts: vec![],
            wasm_base64: None,
            min_memory_mb: None,
            architectures: vec![],
        }
    }
}

impl AppPackageAdapter for FlameAdapter {
    fn detect(&self, root: &Path) -> bool {
        if root.join("app.json").exists() {
            return true;
        }
        // 内置应用：以 key 作为根路径标记（目录名即 key）
        Self::find_builtin(&root.to_string_lossy().split('/').last().unwrap_or_default()).is_some()
    }

    fn parse_metadata(&self, root: &Path) -> Result<AppMetadata, AppError> {
        // 内置应用
        let key = root.file_name().and_then(|n| n.to_str()).unwrap_or_default();
        if let Some(manifest) = Self::find_builtin(key) {
            return Ok(Self::builtin_metadata(&manifest));
        }

        let json = Self::read_app_json(root)?;
        let versions = json
            .versions
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| {
                self.list_versions(root).unwrap_or_default()
            });
        let default_version = versions
            .iter()
            .filter(|v| *v != "latest")
            .next()
            .or_else(|| versions.first())
            .cloned()
            .unwrap_or_else(|| "latest".into());
        let mode = InstallMode::from_str(json.mode.as_deref().unwrap_or("container"))
            .unwrap_or(InstallMode::Container);

        Ok(AppMetadata {
            key: json.key.unwrap_or_else(|| key.to_string()),
            name: json.name.unwrap_or_else(|| key.to_string()),
            category: json.category.unwrap_or_default(),
            short_desc_zh: json.short_desc_zh.unwrap_or_default(),
            short_desc_en: json.short_desc_en,
            tags: json.tags.unwrap_or_default(),
            format: AppFormat::Flame,
            modes: vec![mode],
            versions,
            default_version,
            logo: json.icon,
            min_memory_mb: None,
            architectures: vec![],
            readme: json.readme,
        })
    }

    fn list_versions(&self, root: &Path) -> Result<Vec<String>, AppError> {
        if let Some(manifest) = Self::find_builtin(root.file_name().and_then(|n| n.to_str()).unwrap_or_default()) {
            return Ok(vec![manifest.version]);
        }
        let mut versions = Vec::new();
        for entry in std::fs::read_dir(root)
            .map_err(|e| AppError::BadRequest(format!("读取目录失败: {}", e)))?
        {
            let entry = entry.map_err(|e| AppError::BadRequest(format!("读取目录失败: {}", e)))?;
            let name = entry.file_name().to_string_lossy().into_owned();
            if entry.path().is_dir() && is_version_dir(&name) {
                versions.push(name);
            }
        }
        versions.sort();
        versions.reverse();
        if versions.is_empty() {
            return Err(AppError::BadRequest("应用包中未找到版本目录".into()));
        }
        Ok(versions)
    }

    fn parse_version(&self, root: &Path, version: &str) -> Result<AppVersionInfo, AppError> {
        // 内置应用
        let key = root.file_name().and_then(|n| n.to_str()).unwrap_or_default();
        if let Some(manifest) = Self::find_builtin(key) {
            if version != manifest.version {
                return Err(AppError::NotFound(format!("版本不存在: {}", version)));
            }
            return Ok(Self::builtin_version(&manifest));
        }

        let json = Self::read_app_json(root)?;
        let version_dir = root.join(version);
        if !version_dir.is_dir() {
            return Err(AppError::NotFound(format!("版本目录不存在: {}", version)));
        }

        let mode = InstallMode::from_str(json.mode.as_deref().unwrap_or("container"))
            .unwrap_or(InstallMode::Container);

        let mut form_fields: Vec<FormField> = json
            .form_fields
            .unwrap_or_default()
            .into_iter()
            .map(|f| FormField {
                env_key: f.env_key.clone().unwrap_or_default(),
                label_zh: f.label_zh.unwrap_or_else(|| f.env_key.clone().unwrap_or_default()),
                label_en: f.label_en,
                field_type: field_type_from_str(f.field_type.as_deref().unwrap_or("text")),
                default: f.default,
                required: f.required.unwrap_or(false),
                pattern: None,
                min: None,
                max: None,
                min_length: None,
                max_length: None,
                options: f
                    .options
                    .unwrap_or_default()
                    .into_iter()
                    .map(|o| crate::domain::entity::SelectOption { label: o.label, value: o.value })
                    .collect(),
                description: f.description,
                group: None,
            })
            .collect();

        if form_fields.is_empty() && mode == InstallMode::Container {
            form_fields.push(form_field("PORT", "服务端口"));
        }

        let compose_template = version_dir
            .join("docker-compose.yml")
            .is_file()
            .then(|| std::fs::read_to_string(version_dir.join("docker-compose.yml")).ok())
            .flatten();
        let native_scripts = if version_dir.join("install.sh").is_file() {
            std::fs::read_to_string(version_dir.join("install.sh"))
                .map(|s| s.lines().filter(|l| !l.trim().is_empty() && !l.trim().starts_with('#')).map(|l| l.to_string()).collect())
                .unwrap_or_default()
        } else {
            vec![]
        };
        let wasm_base64 = if version_dir.join("app.wasm").is_file() {
            std::fs::read(version_dir.join("app.wasm"))
                .ok()
                .map(|bytes| base64::engine::general_purpose::STANDARD.encode(bytes))
        } else {
            None
        };

        Ok(AppVersionInfo {
            version: version.into(),
            mode,
            default_port: json.default_port,
            form_fields,
            compose_template,
            native_scripts,
            wasm_base64,
            min_memory_mb: None,
            architectures: vec![],
        })
    }

    fn known_port_vars(&self) -> &'static [&'static str] {
        &["PORT"]
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    fn temp_dir(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("flame_app_test_{}_{}", name, std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn detects_flame_dir() {
        let dir = temp_dir("detect");
        std::fs::write(dir.join("app.json"), "{}").unwrap();
        let adapter = FlameAdapter;
        assert!(adapter.detect(&dir));
    }

    #[test]
    fn parses_flame_dir_metadata() {
        let dir = temp_dir("meta");
        let json = r#"{
            "key": "gitea",
            "name": "Gitea",
            "category": "devops",
            "short_desc_zh": "轻量 Git 服务",
            "tags": ["git"],
            "mode": "container",
            "default_port": 3000,
            "versions": ["1.21.0"]
        }"#;
        std::fs::write(dir.join("app.json"), json).unwrap();
        std::fs::create_dir_all(dir.join("1.21.0")).unwrap();

        let adapter = FlameAdapter;
        let meta = adapter.parse_metadata(&dir).unwrap();
        assert_eq!(meta.key, "gitea");
        assert_eq!(meta.category, "devops");
        assert_eq!(meta.versions, vec!["1.21.0"]);
        assert_eq!(meta.format, AppFormat::Flame);
        assert_eq!(meta.modes, vec![InstallMode::Container]);
    }

    #[test]
    fn parses_flame_dir_version_compose() {
        let dir = temp_dir("ver");
        let json = r#"{"key": "gitea", "name": "Gitea", "versions": ["1.21.0"]}"#;
        std::fs::write(dir.join("app.json"), json).unwrap();
        let vdir = dir.join("1.21.0");
        std::fs::create_dir_all(&vdir).unwrap();
        std::fs::write(vdir.join("docker-compose.yml"), "services:\n  gitea:\n    image: gitea/gitea\n").unwrap();

        let adapter = FlameAdapter;
        let version = adapter.parse_version(&dir, "1.21.0").unwrap();
        assert_eq!(version.mode, InstallMode::Container);
        assert!(version.compose_template.unwrap().contains("gitea/gitea"));
        assert_eq!(version.form_fields.len(), 1);
        assert_eq!(version.form_fields[0].env_key, "PORT");
    }

    #[test]
    fn parses_native_and_wasm_versions() {
        let dir = temp_dir("native");
        let json = r#"{"key": "php-fpm", "name": "PHP-FPM", "mode": "native", "versions": ["8.3"]}"#;
        std::fs::write(dir.join("app.json"), json).unwrap();
        let vdir = dir.join("8.3");
        std::fs::create_dir_all(&vdir).unwrap();
        std::fs::write(vdir.join("install.sh"), "apt-get install -y php-fpm\nsystemctl enable php-fpm").unwrap();

        let adapter = FlameAdapter;
        let version = adapter.parse_version(&dir, "8.3").unwrap();
        assert_eq!(version.mode, InstallMode::Native);
        assert_eq!(version.native_scripts.len(), 2);
        assert!(version.native_scripts[0].contains("apt-get"));
    }

    #[test]
    fn builtin_apps_are_served() {
        let manifest = FlameAdapter::find_builtin("wordpress").expect("wordpress builtin");
        let meta = FlameAdapter::builtin_metadata(&manifest);
        assert_eq!(meta.key, "wordpress");
        let version = FlameAdapter::builtin_version(&manifest);
        assert!(version.compose_template.as_ref().unwrap().contains("wordpress:"));
        assert_eq!(version.form_fields[0].env_key, "PORT");
        assert_eq!(version.default_port, Some(8081));
    }

    #[test]
    fn skips_non_version_dirs() {
        assert!(!is_version_dir("logo"));
        assert!(!is_version_dir("README.md"));
        assert!(!is_version_dir("data.yml"));
        assert!(!is_version_dir("v2.1"));
        assert!(is_version_dir("2.21.0"));
        assert!(is_version_dir("latest_2"));
    }
}
