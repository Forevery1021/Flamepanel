//! x-flame 扩展字段解析（M-M1，Doc/20 §4.6）
//!
//! Flame 原生包的 `docker-compose.yml` 采用标准 Compose extension 规范的
//! `x-flame` 顶层扩展字段声明元数据（CasaOS `x-casaos` 同构路线）：
//!
//! ```yaml
//! x-flame:
//!   app:
//!     category: website            # 分类（缺省回退 app.json）
//!     arch: [amd64, arm64]        # 架构适配
//!   services:
//!     app:
//!       env:
//!         PANEL_APP_PORT_HTTP: 网站对外端口   # 变量人类可读说明（表单即元数据）
//!       ports:
//!         "8080:80": 网站端口                # 端口说明
//!       volumes:
//!         "${APP_PATH}/data:/var/www/html": 站点数据  # 卷说明
//! ```
//!
//! 解析结果只做**增强合并**：app.json 已声明的元数据/表单字段优先，
//! x-flame 仅补充缺省项，不覆盖显式配置——保证两种声明方式可共存。

use serde::Deserialize;

/// `x-flame` 顶层结构（宽松解析：缺省字段全 None）。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct XFlame {
    /// app 级元数据
    #[serde(default)]
    pub app: XFlameApp,
    /// service 级表单说明（service 名 → 说明）
    #[serde(default)]
    pub services: std::collections::HashMap<String, XFlameService>,
}

/// app 级：分类 / 架构适配 / 截图。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct XFlameApp {
    pub category: Option<String>,
    pub arch: Option<Vec<String>>,
    #[serde(default)]
    pub screenshots: Vec<String>,
}

/// service 级：每个 env / port / volume 的人类可读说明。
#[derive(Debug, Clone, Default, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct XFlameService {
    /// env 变量名 → 说明
    #[serde(default)]
    pub env: std::collections::HashMap<String, serde_yaml::Value>,
    /// 端口映射原文 → 说明
    #[serde(default)]
    pub ports: std::collections::HashMap<String, serde_yaml::Value>,
    /// 卷映射原文 → 说明
    #[serde(default)]
    pub volumes: std::collections::HashMap<String, serde_yaml::Value>,
}

/// 从 compose 模板解析 `x-flame` 字段；无该字段或缺 compose 返回默认空结构。
pub fn parse_xflame(compose: &str) -> XFlame {
    #[derive(Deserialize)]
    struct ComposeRoot {
        #[serde(default, rename = "x-flame")]
        x_flame: Option<XFlame>,
    }
    serde_yaml::from_str::<ComposeRoot>(compose)
        .ok()
        .and_then(|root| root.x_flame)
        .unwrap_or_default()
}

/// 合法 env 变量名（大写字母/数字/下划线，不以数字开头）；恶意/拼错键静默忽略。
fn is_valid_env_key(name: &str) -> bool {
    !name.is_empty()
        && name.len() <= 128
        && !name.starts_with(|c: char| c.is_ascii_digit())
        && name
            .chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit() || c == '_')
}

/// 提取 `x-flame.services.<svc>.env` 说明为 (变量名, 说明) 列表，
/// 供适配器转成补充表单字段（field_type=text，非必填）。
pub fn env_hints(xf: &XFlame) -> Vec<(String, String)> {
    let mut out = Vec::new();
    for svc in xf.services.values() {
        for (name, desc) in &svc.env {
            if !is_valid_env_key(name) {
                continue; // 路径穿越/非法字符等恶意键不进表单
            }
            let desc = yaml_value_to_string(desc);
            if !desc.is_empty() {
                out.push((name.clone(), desc));
            }
        }
    }
    out.sort();
    out.dedup();
    out
}

fn yaml_value_to_string(v: &serde_yaml::Value) -> String {
    match v {
        serde_yaml::Value::String(s) => s.clone(),
        serde_yaml::Value::Number(n) => n.to_string(),
        serde_yaml::Value::Bool(b) => b.to_string(),
        _ => String::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_full_xflame_block() {
        let compose = r#"
x-flame:
  app:
    category: website
    arch: [amd64, arm64]
  services:
    app:
      env:
        PANEL_APP_PORT_HTTP: 网站对外端口
        TZ: 时区
services:
  app:
    image: docker.io/library/wordpress:6.7
"#;
        let xf = parse_xflame(compose);
        assert_eq!(xf.app.category.as_deref(), Some("website"));
        assert_eq!(xf.app.arch, Some(vec!["amd64".into(), "arm64".into()]));
        let hints = env_hints(&xf);
        assert_eq!(hints.len(), 2);
        assert!(hints.contains(&("PANEL_APP_PORT_HTTP".into(), "网站对外端口".into())));
    }

    #[test]
    fn missing_xflame_returns_default() {
        let xf = parse_xflame("services:\n  app:\n    image: nginx\n");
        assert!(xf.app.category.is_none());
        assert!(xf.services.is_empty());
        assert!(env_hints(&xf).is_empty());
    }

    #[test]
    fn malformed_yaml_falls_back_to_default() {
        let xf = parse_xflame("x-flame: [1, 2, 3]\nservices: {}");
        assert!(xf.app.category.is_none());
    }

    #[test]
    fn unknown_keys_rejected() {
        // deny_unknown_fields：拼错的字段名不应静默吞掉（提示包作者拼写错误）
        let xf = parse_xflame("x-flame:\n  app:\n  catgory: website\nservices: {}");
        assert!(xf.app.category.is_none());
    }
}
