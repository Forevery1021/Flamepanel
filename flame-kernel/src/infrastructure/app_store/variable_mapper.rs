use crate::application::app_store_ports::VariableMapper as VariableMapperPort;
use regex::Regex;
use std::collections::HashMap;

/// 变量映射引擎：支持 `${VAR}`、`$VAR` 与遗留 `{var}` 三种占位符。
/// 用户表单值优先级最高，未识别变量保留原样并收集警告。
/// 实现 application 层 `VariableMapper` 端口。
pub struct VariableMapper {
    builtins: HashMap<String, String>,
}

impl VariableMapper {
    pub fn new(values: HashMap<String, String>) -> Self {
        Self { builtins: values }
    }

    pub fn insert(&mut self, key: &str, value: impl Into<String>) {
        self.builtins.insert(key.to_uppercase(), value.into());
    }

    pub fn get(&self, key: &str) -> Option<&str> {
        self.builtins.get(&key.to_uppercase()).map(|s| s.as_str())
    }

    /// 支持 ${VAR} 与 $VAR；未识别变量保留原样并收集警告
    pub fn replace(&self, template: &str) -> (String, Vec<String>) {
        let var_re = Regex::new(r"\$\{([A-Za-z0-9_]+)\}|\$([A-Za-z0-9_]+)|\{([a-zA-Z0-9_]+)\}")
            .expect("valid regex");
        let mut warnings = Vec::new();

        let result = var_re.replace_all(template, |caps: &regex::Captures| {
            let name = caps
                .get(1)
                .or_else(|| caps.get(2))
                .or_else(|| caps.get(3))
                .map(|m| m.as_str())
                .unwrap_or_default();
            match self.builtins.get(&name.to_uppercase()) {
                Some(v) => v.clone(),
                None => {
                    warnings.push(format!("未识别的变量: ${{{}}}", name));
                    caps.get(0)
                        .map(|m| m.as_str().to_string())
                        .unwrap_or_default()
                }
            }
        });

        (result.into_owned(), warnings)
    }
}

impl VariableMapperPort for VariableMapper {
    fn insert(&mut self, key: &str, value: String) {
        VariableMapper::insert(self, key, value)
    }

    fn get(&self, key: &str) -> Option<&str> {
        VariableMapper::get(self, key)
    }

    fn replace(&self, template: &str) -> (String, Vec<String>) {
        VariableMapper::replace(self, template)
    }
}

/// 变量映射器工厂（实现 application `VariableMapperFactory` 端口）。
pub struct DefaultVariableMapperFactory;

impl DefaultVariableMapperFactory {
    pub fn new() -> Self {
        Self
    }
}

impl Default for DefaultVariableMapperFactory {
    fn default() -> Self {
        Self::new()
    }
}

impl crate::application::app_store_ports::VariableMapperFactory for DefaultVariableMapperFactory {
    fn create(&self, values: HashMap<String, String>) -> Box<dyn VariableMapperPort> {
        Box::new(VariableMapper::new(values))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mapper() -> VariableMapper {
        let mut m = VariableMapper::new(HashMap::new());
        m.insert("CONTAINER_NAME", "wordpress-a1b2c3d4");
        m.insert("PANEL_APP_PORT_HTTP", "8089");
        m.insert("APP_PATH", "/opt/flamepanel/apps/wordpress");
        m
    }

    #[test]
    fn replaces_braced_vars() {
        let m = mapper();
        let (out, warns) =
            m.replace("container_name: ${CONTAINER_NAME}\n  - \"${PANEL_APP_PORT_HTTP}:80\"");
        assert_eq!(out, "container_name: wordpress-a1b2c3d4\n  - \"8089:80\"");
        assert!(warns.is_empty());
    }

    #[test]
    fn replaces_unbraced_vars() {
        let m = mapper();
        let (out, warns) = m.replace("path=$APP_PATH");
        assert_eq!(out, "path=/opt/flamepanel/apps/wordpress");
        assert!(warns.is_empty());
    }

    #[test]
    fn replaces_legacy_brace_vars() {
        let m = mapper();
        let (out, _) = m.replace("ports: \"{PANEL_APP_PORT_HTTP}:80\"");
        assert_eq!(out, "ports: \"8089:80\"");
    }

    #[test]
    fn keeps_unknown_vars_and_warns() {
        let m = mapper();
        let (out, warns) = m.replace("env: ${UNKNOWN_VAR}");
        assert_eq!(out, "env: ${UNKNOWN_VAR}");
        assert_eq!(warns.len(), 1);
        assert!(warns[0].contains("UNKNOWN_VAR"));
    }

    #[test]
    fn case_insensitive_lookup() {
        let m = mapper();
        let (out, _) = m.replace("${container_name}");
        assert_eq!(out, "wordpress-a1b2c3d4");
    }

    #[test]
    fn user_values_override_builtins() {
        let mut m = VariableMapper::new(HashMap::new());
        m.insert("PORT", "8080");
        m.insert("PORT", "9090");
        let (out, _) = m.replace("${PORT}");
        assert_eq!(out, "9090");
    }

    #[test]
    fn every_builtin_manifest_renders_to_valid_safe_compose() {
        // 安装流程注入的系统变量（与 AppStoreService::install 一致的最小集合）
        for m in crate::domain::entity::builtin_apps() {
            let mut mapper = VariableMapper::new(HashMap::new());
            mapper.insert("NAME", m.key.clone());
            mapper.insert("PORT", m.default_port.to_string());
            mapper.insert(
                "DATA_DIR",
                format!("/opt/flamepanel/data/apps/{}/x/data", m.key),
            );
            let (rendered, warnings) = mapper.replace(&m.compose);
            assert!(
                warnings.is_empty(),
                "{}: 未解析的占位符 {:?}",
                m.key,
                warnings
            );
            serde_yaml::from_str::<serde_yaml::Value>(&rendered)
                .unwrap_or_else(|e| panic!("{}: 渲染后的 compose 无法解析: {e}", m.key));
            // 确认风险后不得有阻断项（内置应用可含 docker.sock 等需确认的挂载）
            let scan =
                crate::infrastructure::app_store::security_scanner::scan_compose(&rendered, true);
            assert!(
                !scan.has_blockers(),
                "{}: {:?}",
                m.key,
                scan.block_messages()
            );
        }
    }
}
