use std::path::Path;
use serde::Deserialize;
use crate::core::error::AppError;
use crate::domain::entity::{AppFormat, AppMetadata, AppVersionInfo, FormField, InstallMode, SelectOption};
use super::{AppPackageAdapter, field_type_from_str};

/// 宝塔（aaPanel）Docker 应用格式解析器
///
/// 目录结构：
/// ```text
/// apphub/<app-name>/
/// ├── app.json             # 应用配置与表单定义
/// ├── icon.png
/// ├── latest/              # 默认最新版本
/// │   └── docker-compose.yml
/// └── <version>/
///     └── docker-compose.yml
/// ```
pub struct BaotaAdapter;

const PORT_VARS: &[&str] = &["HOST_IP", "CPUS", "MEMORY_LIMIT", "APP_PATH", "CONTAINER_NAME"];

#[derive(Debug, Deserialize)]
struct BaotaAppJson {
    #[serde(rename = "appname")]
    appname: Option<String>,
    #[serde(rename = "apptitle")]
    apptitle: Option<String>,
    #[serde(rename = "appdesc")]
    appdesc: Option<String>,
    #[serde(rename = "apptype")]
    apptype: Option<String>,
    field: Option<Vec<BaotaField>>,
}

#[derive(Debug, Deserialize)]
struct BaotaField {
    key: String,
    title: Option<String>,
    #[serde(rename = "type")]
    field_type: Option<String>,
    default: Option<serde_yaml::Value>,
    required: Option<bool>,
    placeholder: Option<String>,
    selects: Option<Vec<serde_yaml::Value>>,
}

impl BaotaAdapter {
    fn read_app_json(root: &Path) -> Result<BaotaAppJson, AppError> {
        let path = root.join("app.json");
        if !path.exists() {
            return Err(AppError::BadRequest(format!("缺少 app.json: {}", root.display())));
        }
        let content = std::fs::read_to_string(&path)
            .map_err(|e| AppError::BadRequest(format!("读取 app.json 失败: {}", e)))?;
        serde_json::from_str(&content)
            .map_err(|e| AppError::BadRequest(format!("解析 app.json 失败: {}", e)))
    }
}

impl AppPackageAdapter for BaotaAdapter {
    fn detect(&self, root: &Path) -> bool {
        if !root.join("app.json").exists() {
            return false;
        }
        // 与内置 Flame 格式区分：宝塔 app.json 含 appname/apptitle/apptype
        if let Ok(content) = std::fs::read_to_string(root.join("app.json")) {
            return content.contains("appname") || content.contains("apptitle") || content.contains("apptype");
        }
        false
    }

    fn parse_metadata(&self, root: &Path) -> Result<AppMetadata, AppError> {
        let json = Self::read_app_json(root)?;
        let key = json
            .appname
            .unwrap_or_else(|| root.file_name().and_then(|n| n.to_str()).unwrap_or("unknown").to_string());
        let versions = self.list_versions(root)?;
        let default_version = if versions.iter().any(|v| v == "latest") {
            "latest".into()
        } else {
            versions
                .first()
                .cloned()
                .unwrap_or_else(|| "latest".into())
        };

        Ok(AppMetadata {
            name: json.apptitle.unwrap_or_else(|| key.clone()),
            key: key.clone(),
            category: json.apptype.unwrap_or_default(),
            short_desc_zh: json.appdesc.unwrap_or_default(),
            short_desc_en: None,
            tags: vec![],
            format: AppFormat::Baota,
            modes: vec![InstallMode::Container],
            versions,
            default_version,
            logo: root.join("icon.png").exists().then(|| "icon.png".into()),
            min_memory_mb: None,
            architectures: vec![],
            readme: None,
        })
    }

    fn list_versions(&self, root: &Path) -> Result<Vec<String>, AppError> {
        let mut versions = Vec::new();
        if root.join("latest").is_dir() {
            versions.push("latest".into());
        }
        for entry in std::fs::read_dir(root)
            .map_err(|e| AppError::BadRequest(format!("读取目录失败: {}", e)))?
        {
            let entry = entry.map_err(|e| AppError::BadRequest(format!("读取目录失败: {}", e)))?;
            let name = entry.file_name().to_string_lossy().into_owned();
            if entry.path().is_dir() && super::is_version_dir(&name) {
                versions.push(name);
            }
        }
        versions.sort();
        versions.reverse();
        if versions.is_empty() {
            return Err(AppError::BadRequest("宝塔应用包中未找到版本目录".into()));
        }
        Ok(versions)
    }

    fn parse_version(&self, root: &Path, version: &str) -> Result<AppVersionInfo, AppError> {
        let version_dir = root.join(version);
        if !version_dir.is_dir() {
            return Err(AppError::NotFound(format!("版本目录不存在: {}", version)));
        }

        let json = Self::read_app_json(root)?;
        let mut form_fields: Vec<FormField> = Vec::new();
        for f in json.field.unwrap_or_default() {
            let field_type = f
                .field_type
                .as_deref()
                .and_then(|t| field_type_from_str(t).eq(&crate::domain::entity::FieldType::Text).then(|| {
                    match t.to_lowercase().as_str() {
                        "select" => crate::domain::entity::FieldType::Select,
                        "number" => crate::domain::entity::FieldType::Number,
                        "password" => crate::domain::entity::FieldType::Password,
                        _ => field_type_from_str(t),
                    }
                }))
                .unwrap_or_else(|| field_type_from_str(f.field_type.as_deref().unwrap_or("text")));

            let options = f
                .selects
                .unwrap_or_default()
                .into_iter()
                .filter_map(|v| v.as_str().map(|s| SelectOption { label: s.to_string(), value: s.to_string() }))
                .collect();

            form_fields.push(FormField {
                env_key: f.key,
                label_zh: f.title.unwrap_or_default(),
                label_en: None,
                field_type,
                default: f.default.as_ref().and_then(|v| match v {
                    serde_yaml::Value::String(s) => Some(s.clone()),
                    serde_yaml::Value::Number(n) => Some(n.to_string()),
                    serde_yaml::Value::Bool(b) => Some(b.to_string()),
                    _ => None,
                }),
                required: f.required.unwrap_or(false),
                pattern: None,
                min: None,
                max: None,
                min_length: None,
                max_length: None,
                options,
                description: f.placeholder,
                group: None,
            });
        }

        let compose_path = version_dir.join("docker-compose.yml");
        let compose_template = if compose_path.exists() {
            std::fs::read_to_string(&compose_path)
                .ok()
                .map(|c| c.replace("baota_net", "flamepanel-network"))
        } else {
            None
        };

        Ok(AppVersionInfo {
            version: version.into(),
            mode: InstallMode::Container,
            default_port: None,
            form_fields,
            compose_template,
            native_scripts: vec![],
            wasm_base64: None,
            min_memory_mb: None,
            architectures: vec![],
        })
    }

    fn known_port_vars(&self) -> &'static [&'static str] {
        PORT_VARS
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("baota_test_{}_{}", name, std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn detects_baota_package() {
        let dir = temp_dir("detect");
        std::fs::write(
            dir.join("app.json"),
            r#"{"appname": "gitea", "apptitle": "Gitea", "apptype": "git"}"#,
        )
        .unwrap();
        std::fs::write(dir.join("icon.png"), "png").unwrap();
        let adapter = BaotaAdapter;
        assert!(adapter.detect(&dir));
    }

    #[test]
    fn does_not_detect_flame_package() {
        let dir = temp_dir("flame");
        std::fs::write(
            dir.join("app.json"),
            r#"{"key": "gitea", "name": "Gitea", "category": "devops"}"#,
        )
        .unwrap();
        let adapter = BaotaAdapter;
        assert!(!adapter.detect(&dir));
    }

    #[test]
    fn parses_metadata_and_versions() {
        let dir = temp_dir("meta");
        std::fs::write(
            dir.join("app.json"),
            r#"{"appname": "gitea", "apptitle": "Gitea", "appdesc": "轻量 Git", "apptype": "git"}"#,
        )
        .unwrap();
        std::fs::create_dir_all(dir.join("latest")).unwrap();
        std::fs::create_dir_all(dir.join("1.21.0")).unwrap();
        std::fs::create_dir_all(dir.join("README.md")).unwrap();

        let adapter = BaotaAdapter;
        let meta = adapter.parse_metadata(&dir).unwrap();
        assert_eq!(meta.key, "gitea");
        assert_eq!(meta.format, AppFormat::Baota);
        assert_eq!(meta.default_version, "latest");
        assert!(meta.versions.contains(&"latest".to_string()));
        assert!(meta.versions.contains(&"1.21.0".to_string()));
    }

    #[test]
    fn parses_version_with_fields_and_network() {
        let dir = temp_dir("ver");
        std::fs::write(
            dir.join("app.json"),
            r#"{
                "appname": "gitea",
                "apptitle": "Gitea",
                "field": [
                    {"key": "HOST_IP", "title": "监听地址", "type": "input", "default": "0.0.0.0"},
                    {"key": "PORT", "title": "端口", "type": "number", "default": 3000},
                    {"key": "DOMAIN", "title": "域名", "type": "input", "required": true}
                ]
            }"#,
        )
        .unwrap();
        let vdir = dir.join("latest");
        std::fs::create_dir_all(&vdir).unwrap();
        std::fs::write(
            vdir.join("docker-compose.yml"),
            "services:\n  app:\n    image: gitea/gitea\n    networks:\n      - baota_net\n    ports:\n      - \"${HOST_IP}:${PORT}:3000\"\n",
        )
        .unwrap();

        let adapter = BaotaAdapter;
        let version = adapter.parse_version(&dir, "latest").unwrap();
        assert_eq!(version.form_fields.len(), 3);
        assert_eq!(version.form_fields[1].env_key, "PORT");
        assert_eq!(version.form_fields[1].field_type, crate::domain::entity::FieldType::Number);
        let compose = version.compose_template.unwrap();
        assert!(compose.contains("flamepanel-network"));
        assert!(!compose.contains("baota_net"));
    }
}
