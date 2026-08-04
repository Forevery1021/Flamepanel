use super::{field_type_from_str, is_version_dir, AppPackageAdapter};
use crate::core::error::AppError;
use crate::domain::entity::{
    AppFormat, AppMetadata, AppVersionInfo, FormField, InstallMode, SelectOption,
};
use serde::Deserialize;
use std::path::Path;

/// 1Panel 应用商店格式解析器
///
/// 目录结构：
/// ```text
/// apps/<app-key>/
/// ├── data.yml          # 应用元数据
/// ├── logo.png
/// └── <version>/        # 如 2.21.0（不加 v）
///     ├── data.yml      # 安装表单字段
///     ├── docker-compose.yml
///     └── scripts/      # 可选 init/upgrade/uninstall
/// ```
pub struct OnePanelAdapter;

const PORT_VARS: &[&str] = &[
    "PANEL_APP_PORT_HTTP",
    "PANEL_APP_PORT_HTTPS",
    "PANEL_APP_PORT_API",
    "PANEL_APP_PORT_ADMIN",
    "PANEL_APP_PORT_PROXY",
    "PANEL_APP_PORT_DB",
];

#[derive(Debug, Deserialize)]
struct OnePanelMetaYaml {
    key: Option<String>,
    name: Option<String>,
    tags: Option<Vec<String>>,
    #[serde(rename = "shortDescZh")]
    short_desc_zh: Option<String>,
    #[serde(rename = "shortDescEn")]
    short_desc_en: Option<String>,
    #[serde(rename = "type")]
    app_type: Option<String>,
    icon: Option<String>,
}

#[derive(Debug, Deserialize)]
struct OnePanelVersionYaml {
    #[serde(rename = "additionalProperties")]
    additional_properties: Option<OnePanelAdditional>,
}

#[derive(Debug, Deserialize)]
struct OnePanelAdditional {
    #[serde(rename = "formFields")]
    form_fields: Option<Vec<OnePanelFormField>>,
}

#[derive(Debug, Deserialize)]
struct OnePanelFormField {
    #[serde(rename = "type")]
    field_type: String,
    label: Option<String>,
    #[serde(rename = "envKey")]
    env_key: Option<String>,
    default: Option<String>,
    required: Option<bool>,
    regex: Option<String>,
    #[serde(rename = "selectValue")]
    select_value: Option<Vec<String>>,
}

impl OnePanelAdapter {
    fn meta_path(root: &Path) -> std::path::PathBuf {
        let yml = root.join("data.yml");
        if yml.exists() {
            yml
        } else {
            root.join("data.yaml")
        }
    }

    fn read_meta(root: &Path) -> Result<OnePanelMetaYaml, AppError> {
        let path = Self::meta_path(root);
        if !path.exists() {
            return Err(AppError::BadRequest(format!(
                "缺少 data.yml: {}",
                root.display()
            )));
        }
        let content = std::fs::read_to_string(&path)
            .map_err(|e| AppError::BadRequest(format!("读取 data.yml 失败: {}", e)))?;
        serde_yaml::from_str(&content)
            .map_err(|e| AppError::BadRequest(format!("解析 data.yml 失败: {}", e)))
    }
}

impl AppPackageAdapter for OnePanelAdapter {
    fn detect(&self, root: &Path) -> bool {
        root.join("data.yml").exists() || root.join("data.yaml").exists()
    }

    fn parse_metadata(&self, root: &Path) -> Result<AppMetadata, AppError> {
        let meta = Self::read_meta(root)?;
        let versions = self.list_versions(root)?;
        let key = meta.key.unwrap_or_else(|| {
            root.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or("unknown")
                .to_string()
        });
        let default_version = versions
            .iter()
            .find(|v| *v != "latest")
            .or_else(|| versions.first())
            .cloned()
            .unwrap_or_else(|| "latest".into());

        Ok(AppMetadata {
            key,
            name: meta.name.unwrap_or_default(),
            category: meta.app_type.unwrap_or_default(),
            short_desc_zh: meta.short_desc_zh.unwrap_or_default(),
            short_desc_en: meta.short_desc_en,
            tags: meta.tags.unwrap_or_default(),
            format: AppFormat::OnePanel,
            modes: vec![InstallMode::Container],
            versions,
            default_version,
            logo: meta.icon,
            min_memory_mb: None,
            architectures: vec![],
            readme: None,
            recommended: false,
        })
    }

    fn list_versions(&self, root: &Path) -> Result<Vec<String>, AppError> {
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
            return Err(AppError::BadRequest("1Panel 应用包中未找到版本目录".into()));
        }
        Ok(versions)
    }

    fn parse_version(&self, root: &Path, version: &str) -> Result<AppVersionInfo, AppError> {
        let version_dir = root.join(version);
        if !version_dir.is_dir() {
            return Err(AppError::NotFound(format!("版本目录不存在: {}", version)));
        }

        // 表单字段
        let mut form_fields: Vec<FormField> = Vec::new();
        let version_yaml_path = version_dir.join("data.yml");
        if version_yaml_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&version_yaml_path) {
                if let Ok(vy) = serde_yaml::from_str::<OnePanelVersionYaml>(&content) {
                    if let Some(props) = vy.additional_properties {
                        for f in props.form_fields.unwrap_or_default() {
                            let field_type = field_type_from_str(&f.field_type);
                            let env_key = f.env_key.unwrap_or_default();
                            let options = f
                                .select_value
                                .unwrap_or_default()
                                .into_iter()
                                .map(|v| SelectOption {
                                    label: v.clone(),
                                    value: v,
                                })
                                .collect();
                            form_fields.push(FormField {
                                env_key,
                                label_zh: f.label.unwrap_or_default(),
                                label_en: None,
                                field_type,
                                default: f.default,
                                required: f.required.unwrap_or(false),
                                pattern: f.regex,
                                min: None,
                                max: None,
                                min_length: None,
                                max_length: None,
                                options,
                                description: None,
                                group: None,
                            });
                        }
                    }
                }
            }
        }

        // Compose 模板（替换网络名）
        let compose_path = version_dir.join("docker-compose.yml");
        let compose_template = if compose_path.exists() {
            std::fs::read_to_string(&compose_path)
                .ok()
                .map(|c| c.replace("1panel-network", "flamepanel-network"))
        } else {
            None
        };

        // 生命周期脚本
        let scripts_dir = version_dir.join("scripts");
        let mut native_scripts = Vec::new();
        for script in ["init.sh", "upgrade.sh", "uninstall.sh"] {
            let p = scripts_dir.join(script);
            if p.exists() {
                native_scripts.push(script.into());
            }
        }

        Ok(AppVersionInfo {
            version: version.into(),
            mode: InstallMode::Container,
            default_port: None,
            form_fields,
            compose_template,
            native_scripts,
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
        let dir =
            std::env::temp_dir().join(format!("onepanel_test_{}_{}", name, std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn detects_onepanel_package() {
        let dir = temp_dir("detect");
        std::fs::write(dir.join("data.yml"), "key: wordpress\n").unwrap();
        let adapter = OnePanelAdapter;
        assert!(adapter.detect(&dir));
    }

    #[test]
    fn parses_metadata() {
        let dir = temp_dir("meta");
        std::fs::write(
            dir.join("data.yml"),
            "key: wordpress\nname: WordPress\ntags: [php, mysql]\nshortDescZh: 博客\nshortDescEn: Blog\ntype: website\n",
        )
        .unwrap();
        std::fs::create_dir_all(dir.join("6.7")).unwrap();
        std::fs::create_dir_all(dir.join("6.8")).unwrap();
        std::fs::create_dir_all(dir.join("logo")).unwrap();
        std::fs::create_dir_all(dir.join("README.md")).unwrap();

        let adapter = OnePanelAdapter;
        let meta = adapter.parse_metadata(&dir).unwrap();
        assert_eq!(meta.key, "wordpress");
        assert_eq!(meta.name, "WordPress");
        assert_eq!(meta.category, "website");
        assert_eq!(meta.versions, vec!["6.8", "6.7"]);
        assert_eq!(meta.format, AppFormat::OnePanel);
    }

    #[test]
    fn parses_version_with_form_fields() {
        let dir = temp_dir("ver");
        std::fs::write(dir.join("data.yml"), "key: wordpress\n").unwrap();
        let vdir = dir.join("6.7");
        std::fs::create_dir_all(&vdir).unwrap();
        std::fs::write(
            vdir.join("data.yml"),
            r#"Version: "6.7"
additionalProperties:
  formFields:
    - type: input
      label: 服务端口
      envKey: PANEL_APP_PORT_HTTP
      default: "8089"
      required: true
    - type: password
      label: 数据库密码
      envKey: DB_PASSWORD
      required: true
    - type: select
      label: 版本
      envKey: WP_VERSION
      selectValue:
        - "6.7"
        - "6.6"
"#,
        )
        .unwrap();
        std::fs::write(
            vdir.join("docker-compose.yml"),
            "services:\n  app:\n    image: wordpress:6.7\n    network: 1panel-network\n",
        )
        .unwrap();
        std::fs::create_dir_all(vdir.join("scripts")).unwrap();
        std::fs::write(vdir.join("scripts/init.sh"), "#!/bin/sh\necho init").unwrap();

        let adapter = OnePanelAdapter;
        let version = adapter.parse_version(&dir, "6.7").unwrap();
        assert_eq!(version.form_fields.len(), 3);
        assert_eq!(version.form_fields[0].env_key, "PANEL_APP_PORT_HTTP");
        assert!(version.form_fields[0].required);
        assert_eq!(
            version.form_fields[1].field_type,
            crate::domain::entity::FieldType::Password
        );
        assert_eq!(version.form_fields[2].options.len(), 2);
        assert_eq!(
            version.form_fields[2].field_type,
            crate::domain::entity::FieldType::Select
        );
        let compose = version.compose_template.unwrap();
        assert!(compose.contains("flamepanel-network"));
        assert!(!compose.contains("1panel-network"));
        assert_eq!(version.native_scripts, vec!["init.sh"]);
    }

    #[test]
    fn version_not_found() {
        let dir = temp_dir("missing");
        std::fs::write(dir.join("data.yml"), "key: wordpress\n").unwrap();
        let adapter = OnePanelAdapter;
        let err = adapter.parse_version(&dir, "9.9").unwrap_err();
        assert!(err.to_string().contains("不存在"));
    }
}
