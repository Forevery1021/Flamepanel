use super::xflame::{env_hints, parse_xflame};
use super::{field_type_from_str, form_field, is_version_dir, AppPackageAdapter};
use crate::core::error::AppError;
use crate::domain::entity::{
    AppFormat, AppManifest, AppMetadata, AppVersionInfo, FormField, InstallMode,
};
use base64::Engine;
use serde::Deserialize;
use std::path::Path;

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
    #[serde(default)]
    recommended: Option<bool>,
    /// 生命周期钩子声明（Doc/20 §4.3，路径相对包根/版本目录）
    #[serde(default)]
    hooks: Option<crate::domain::entity::AppHooks>,
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
            return Err(AppError::BadRequest(format!(
                "缺少 app.json: {}",
                root.display()
            )));
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

    /// 从内置 AppManifest 生成元数据（委托领域方法）
    pub fn builtin_metadata(manifest: &AppManifest) -> AppMetadata {
        manifest.to_metadata()
    }

    /// 从内置 AppManifest 生成版本信息（含默认表单字段）（委托领域方法）
    pub fn builtin_version(manifest: &AppManifest) -> AppVersionInfo {
        manifest.to_version()
    }

    /// 读取默认版本的 compose 并解析 x-flame 扩展字段（无 compose 返回默认空结构）
    fn read_xflame(&self, root: &Path, versions: &[String]) -> super::xflame::XFlame {
        versions
            .iter()
            .find(|v| *v != "latest")
            .or_else(|| versions.first())
            .and_then(|v| {
                let p = root.join(v).join("docker-compose.yml");
                std::fs::read_to_string(p).ok()
            })
            .map(|c| parse_xflame(&c))
            .unwrap_or_default()
    }
}

impl AppPackageAdapter for FlameAdapter {
    fn detect(&self, root: &Path) -> bool {
        if root.join("app.json").exists() {
            return true;
        }
        // 内置应用：以 key 作为根路径标记（目录名即 key）
        Self::find_builtin(
            root.to_string_lossy()
                .split('/')
                .next_back()
                .unwrap_or_default(),
        )
        .is_some()
    }

    fn parse_metadata(&self, root: &Path) -> Result<AppMetadata, AppError> {
        // 内置应用
        let key = root
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();
        if let Some(manifest) = Self::find_builtin(key) {
            return Ok(Self::builtin_metadata(&manifest));
        }

        let json = Self::read_app_json(root)?;
        let versions = json
            .versions
            .filter(|v| !v.is_empty())
            .unwrap_or_else(|| self.list_versions(root).unwrap_or_default());
        let default_version = versions
            .iter()
            .find(|v| *v != "latest")
            .or_else(|| versions.first())
            .cloned()
            .unwrap_or_else(|| "latest".into());
        let mode = InstallMode::from_name(json.mode.as_deref().unwrap_or("container"))
            .unwrap_or(InstallMode::Container);

        // M-M1（Doc/20 §4.6）：x-flame 扩展字段增强——compose 中声明的分类/架构
        // 仅在 app.json 未显式提供时补充（app.json 优先，两种声明方式可共存）。
        let xf = self.read_xflame(root, &versions);
        let category = if json.category.is_some() {
            json.category.unwrap_or_default()
        } else {
            xf.app.category.clone().unwrap_or_default()
        };
        let architectures = match &xf.app.arch {
            Some(arch) if !arch.is_empty() => arch.clone(),
            _ => vec![],
        };

        Ok(AppMetadata {
            key: json.key.unwrap_or_else(|| key.to_string()),
            name: json.name.unwrap_or_else(|| key.to_string()),
            category,
            short_desc_zh: json.short_desc_zh.unwrap_or_default(),
            short_desc_en: json.short_desc_en,
            tags: json.tags.unwrap_or_default(),
            format: AppFormat::Flame,
            modes: vec![mode],
            versions,
            default_version,
            logo: json.icon,
            min_memory_mb: None,
            architectures,
            readme: json.readme,
            recommended: json.recommended.unwrap_or(false),
        })
    }

    fn list_versions(&self, root: &Path) -> Result<Vec<String>, AppError> {
        if let Some(manifest) = Self::find_builtin(
            root.file_name()
                .and_then(|n| n.to_str())
                .unwrap_or_default(),
        ) {
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
        let key = root
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or_default();
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

        let mode = InstallMode::from_name(json.mode.as_deref().unwrap_or("container"))
            .unwrap_or(InstallMode::Container);

        // Compose 模板（先读出供 x-flame 解析；M-M1 Doc/20 §4.6）
        let compose_template = version_dir
            .join("docker-compose.yml")
            .is_file()
            .then(|| std::fs::read_to_string(version_dir.join("docker-compose.yml")).ok())
            .flatten();

        let mut form_fields: Vec<FormField> = json
            .form_fields
            .unwrap_or_default()
            .into_iter()
            .map(|f| FormField {
                env_key: f.env_key.clone().unwrap_or_default(),
                label_zh: f
                    .label_zh
                    .unwrap_or_else(|| f.env_key.clone().unwrap_or_default()),
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
                    .map(|o| crate::domain::entity::SelectOption {
                        label: o.label,
                        value: o.value,
                    })
                    .collect(),
                description: f.description,
                group: None,
            })
            .collect();

        if form_fields.is_empty() && mode == InstallMode::Container {
            form_fields.push(form_field("PORT", "服务端口"));
        }

        // x-flame service 级 env 说明 → 补充表单字段（表单即元数据，Doc/20 §4.6）。
        // 仅追加 app.json / 默认 PORT 未覆盖的变量，不覆盖显式声明。
        if mode == InstallMode::Container {
            if let Some(compose) = &compose_template {
                let xf = parse_xflame(compose);
                let declared: std::collections::HashSet<String> =
                    form_fields.iter().map(|f| f.env_key.clone()).collect();
                for (env_key, hint) in env_hints(&xf) {
                    if !declared.contains(&env_key) {
                        let mut f = form_field(&env_key, &hint);
                        f.description = Some(hint);
                        form_fields.push(f);
                    }
                }
            }
        }

        let native_scripts = if version_dir.join("install.sh").is_file() {
            std::fs::read_to_string(version_dir.join("install.sh"))
                .map(|s| {
                    s.lines()
                        .filter(|l| !l.trim().is_empty() && !l.trim().starts_with('#'))
                        .map(|l| l.to_string())
                        .collect()
                })
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

        // 生命周期钩子（Doc/20 §4.3）：app.json "hooks" 显式声明优先，
        // 否则按 scripts/ 目录标准命名探测（pre_install.sh / post_install.sh / ...）。
        let hooks = json.hooks.clone().unwrap_or_else(|| {
            let hook = |name: &str| -> Option<String> {
                let p = version_dir.join("scripts").join(name);
                p.is_file().then(|| format!("scripts/{}", name))
            };
            crate::domain::entity::AppHooks {
                pre_install: hook("pre_install.sh"),
                post_install: hook("post_install.sh"),
                pre_upgrade: hook("pre_upgrade.sh"),
                post_upgrade: hook("post_upgrade.sh"),
                pre_uninstall: hook("pre_uninstall.sh"),
            }
        });

        Ok(AppVersionInfo {
            version: version.into(),
            mode,
            default_port: json.default_port,
            form_fields,
            compose_template,
            native_scripts,
            wasm_base64,
            hooks,
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

    fn temp_dir(name: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("flame_app_test_{}_{}", name, std::process::id()));
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
        std::fs::write(
            vdir.join("docker-compose.yml"),
            "services:\n  gitea:\n    image: gitea/gitea\n",
        )
        .unwrap();

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
        let json =
            r#"{"key": "php-fpm", "name": "PHP-FPM", "mode": "native", "versions": ["8.3"]}"#;
        std::fs::write(dir.join("app.json"), json).unwrap();
        let vdir = dir.join("8.3");
        std::fs::create_dir_all(&vdir).unwrap();
        std::fs::write(
            vdir.join("install.sh"),
            "apt-get install -y php-fpm\nsystemctl enable php-fpm",
        )
        .unwrap();

        let adapter = FlameAdapter;
        let version = adapter.parse_version(&dir, "8.3").unwrap();
        assert_eq!(version.mode, InstallMode::Native);
        assert_eq!(version.native_scripts.len(), 2);
        assert!(version.native_scripts[0].contains("apt-get"));
        // Doc/20 §4.3：无 hooks 声明且无 scripts/ → 默认空
        assert!(version.hooks.is_empty());
    }

    #[test]
    fn parses_explicit_hooks_from_app_json() {
        // Doc/20 §4.3：app.json "hooks" 显式声明优先于 scripts/ 目录探测
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("app.json"),
            r#"{
                "key": "myapp", "name": "MyApp",
                "mode": "container",
                "versions": ["1.0"],
                "hooks": {
                    "pre_install": "scripts/pre.sh",
                    "post_install": "scripts/post.sh",
                    "pre_uninstall": "scripts/bye.sh"
                }
            }"#,
        )
        .unwrap();
        let vdir = dir.path().join("1.0");
        std::fs::create_dir_all(&vdir).unwrap();
        std::fs::write(
            vdir.join("docker-compose.yml"),
            "services:\n  app:\n    image: busybox\n",
        )
        .unwrap();
        std::fs::create_dir_all(vdir.join("scripts")).unwrap();
        std::fs::write(vdir.join("scripts/pre.sh"), "echo pre").unwrap();

        let version = FlameAdapter.parse_version(dir.path(), "1.0").unwrap();
        let h = &version.hooks;
        assert!(!h.is_empty());
        assert_eq!(h.pre_install.as_deref(), Some("scripts/pre.sh"));
        assert_eq!(h.post_install.as_deref(), Some("scripts/post.sh"));
        assert_eq!(h.pre_uninstall.as_deref(), Some("scripts/bye.sh"));
        assert!(h.pre_upgrade.is_none());
    }

    #[test]
    fn probes_hooks_from_scripts_dir_when_not_declared() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(
            dir.path().join("app.json"),
            r#"{ "key": "myapp", "name": "MyApp", "mode": "container", "versions": ["1.0"] }"#,
        )
        .unwrap();
        let vdir = dir.path().join("1.0");
        std::fs::create_dir_all(vdir.join("scripts")).unwrap();
        std::fs::write(vdir.join("scripts/pre_install.sh"), "echo pre").unwrap();
        std::fs::write(vdir.join("scripts/post_install.sh"), "echo post").unwrap();

        let version = FlameAdapter.parse_version(dir.path(), "1.0").unwrap();
        assert_eq!(
            version.hooks.pre_install.as_deref(),
            Some("scripts/pre_install.sh")
        );
        assert_eq!(
            version.hooks.post_install.as_deref(),
            Some("scripts/post_install.sh")
        );
        assert!(version.hooks.pre_uninstall.is_none());
    }

    #[test]
    fn builtin_apps_are_served() {
        let manifest = FlameAdapter::find_builtin("wordpress").expect("wordpress builtin");
        let meta = FlameAdapter::builtin_metadata(&manifest);
        assert_eq!(meta.key, "wordpress");
        let version = FlameAdapter::builtin_version(&manifest);
        assert!(version
            .compose_template
            .as_ref()
            .unwrap()
            .contains("wordpress:"));
        assert_eq!(version.form_fields[0].env_key, "PORT");
        assert_eq!(version.default_port, Some(8081));
    }

    #[test]
    fn merges_xflame_category_and_arch_from_compose() {
        // M-M1 Doc/20 §4.6：compose x-flame 声明分类/架构，app.json 未提供时补充
        let dir = temp_dir("xflame_meta");
        std::fs::write(
            dir.join("app.json"),
            r#"{ "key": "wp", "name": "WP", "mode": "container", "versions": ["1.0"] }"#,
        )
        .unwrap();
        let vdir = dir.join("1.0");
        std::fs::create_dir_all(&vdir).unwrap();
        std::fs::write(
            vdir.join("docker-compose.yml"),
            "x-flame:\n  app:\n    category: website\n    arch: [amd64, arm64]\nservices:\n  app:\n    image: docker.io/library/wordpress:6.7\n",
        )
        .unwrap();

        let meta = FlameAdapter.parse_metadata(&dir).unwrap();
        assert_eq!(meta.category, "website");
        assert_eq!(meta.architectures, vec!["amd64", "arm64"]);
    }

    #[test]
    fn app_json_category_overrides_xflame() {
        // app.json 显式声明优先，x-flame 不覆盖
        let dir = temp_dir("xflame_prio");
        std::fs::write(
            dir.join("app.json"),
            r#"{ "key": "wp", "name": "WP", "category": "cms", "mode": "container", "versions": ["1.0"] }"#,
        )
        .unwrap();
        let vdir = dir.join("1.0");
        std::fs::create_dir_all(&vdir).unwrap();
        std::fs::write(
            vdir.join("docker-compose.yml"),
            "x-flame:\n  app:\n    category: website\nservices:\n  app:\n    image: docker.io/library/wordpress:6.7\n",
        )
        .unwrap();

        let meta = FlameAdapter.parse_metadata(&dir).unwrap();
        assert_eq!(meta.category, "cms");
    }

    #[test]
    fn xflame_env_hints_become_supplementary_form_fields() {
        // service 级 env 说明 → 补充表单字段（不覆盖 app.json 显式声明）
        let dir = temp_dir("xflame_env");
        std::fs::write(
            dir.join("app.json"),
            r#"{
                "key": "wp", "name": "WP", "mode": "container", "versions": ["1.0"],
                "form_fields": [{"env_key": "PORT", "label_zh": "端口", "field_type": "port"}]
            }"#,
        )
        .unwrap();
        let vdir = dir.join("1.0");
        std::fs::create_dir_all(&vdir).unwrap();
        std::fs::write(
            vdir.join("docker-compose.yml"),
            "x-flame:\n  services:\n    app:\n      env:\n        PANEL_APP_PORT_HTTP: 网站对外端口\n        PORT: 不应覆盖\nservices:\n  app:\n    image: docker.io/library/wordpress:6.7\n",
        )
        .unwrap();

        let version = FlameAdapter.parse_version(&dir, "1.0").unwrap();
        let port_field = version
            .form_fields
            .iter()
            .find(|f| f.env_key == "PORT")
            .unwrap();
        assert_eq!(port_field.label_zh, "端口"); // 显式声明不被覆盖
        let extra = version
            .form_fields
            .iter()
            .find(|f| f.env_key == "PANEL_APP_PORT_HTTP")
            .expect("x-flame env hint 应生成补充字段");
        assert_eq!(extra.label_zh, "网站对外端口");
        assert_eq!(extra.description.as_deref(), Some("网站对外端口"));
        assert!(!extra.required); // 补充字段非必填
    }

    #[test]
    fn xflame_env_hint_rejects_path_traversal_key() {
        // 恶意输入：env 变量名含路径穿越/非法字符时忽略该提示
        let dir = temp_dir("xflame_mal");
        std::fs::write(
            dir.join("app.json"),
            r#"{ "key": "wp", "name": "WP", "mode": "container", "versions": ["1.0"] }"#,
        )
        .unwrap();
        let vdir = dir.join("1.0");
        std::fs::create_dir_all(&vdir).unwrap();
        std::fs::write(
            vdir.join("docker-compose.yml"),
            "x-flame:\n  services:\n    app:\n      env:\n        \"../evil\": 穿越\n        GOOD_VAR: 正常\nservices:\n  app:\n    image: docker.io/library/wordpress:6.7\n",
        )
        .unwrap();

        let version = FlameAdapter.parse_version(&dir, "1.0").unwrap();
        assert!(version.form_fields.iter().all(|f| f.env_key != "../evil"));
        assert!(version.form_fields.iter().any(|f| f.env_key == "GOOD_VAR"));
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
