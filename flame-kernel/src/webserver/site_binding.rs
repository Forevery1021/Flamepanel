//! 站点绑定配置生成（M-B 主线二 Stage A-6 · Doc/20 §5.5）。
//!
//! 网站 ⇄ 运行时 ⇄ 项目绑定的配置生成器分支：
//! - **PHP 站点**（`runtime_php`）：按运行时版本生成 php-fpm fastcgi upstream
//!   （`fastcgi_pass unix:/run/php/php<ver>-fpm-<site>.sock`）；OLS 场景走 lsphp handler；
//! - **项目站点**（`reverse_proxy` + 项目端口）：`proxy_pass http://127.0.0.1:<port>`
//!   并附 WebSocket upgrade 头；
//! - **静态 / 容器应用**：沿用既有 root_path / proxy_pass 生成逻辑，本模块不介入。
//!
//! 伪静态规则库：内置 WordPress/Discuz/Typecho/Laravel/ThinkPHP/SpringBoot
//! 常用 rewrite 模板（对齐宝塔），按站点一键套用。
//!
//! **套用即生效**（Doc/20 §5.5 收口）：站点套用的模板名持久化在 `websites.rewrite_preset`，
//! 各引擎的配置生成器（`webserver::config`）在生成 vhost 时按该名解析模板并注入对应
//! 语法的规则块——套用后立即写盘 + reload，后续任何一次站点配置重生成（证书绑定、
//! 引擎切换等）同样带着规则，不会丢。

use crate::domain::entity::Website;

/// 站点类型：与 `Website.site_type` 白名单对齐（`is_valid_site_type`）。
pub const SITE_TYPES: [&str; 4] = ["runtime_php", "reverse_proxy", "static", "container_app"];

/// php-fpm pool socket 路径（每站点独立 pool，版本并存）。
pub fn php_fpm_socket(runtime_version: &str, site_domain: &str) -> String {
    format!(
        "/run/php/php{version}-fpm-{site}.sock",
        version = runtime_version,
        site = site_domain
    )
}

/// 项目站点反代目标（面板本机回环端口）。
pub fn project_proxy_target(port: i64) -> String {
    format!("http://127.0.0.1:{port}")
}

/// 伪静态规则模板：按场景一键套用（对齐宝塔伪静态库）。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RewritePreset {
    WordPress,
    Discuz,
    Typecho,
    Laravel,
    ThinkPhp,
    SpringBoot,
}

impl RewritePreset {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::WordPress => "wordpress",
            Self::Discuz => "discuz",
            Self::Typecho => "typecho",
            Self::Laravel => "laravel",
            Self::ThinkPhp => "thinkphp",
            Self::SpringBoot => "springboot",
        }
    }

    pub fn from_name(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "wordpress" => Some(Self::WordPress),
            "discuz" => Some(Self::Discuz),
            "typecho" => Some(Self::Typecho),
            "laravel" => Some(Self::Laravel),
            "thinkphp" => Some(Self::ThinkPhp),
            "springboot" => Some(Self::SpringBoot),
            _ => None,
        }
    }

    /// 全部模板名（前端下拉 / 校验共用），顺序与 `from_name` 一致。
    pub const ALL: [&'static str; 6] = [
        "wordpress",
        "discuz",
        "typecho",
        "laravel",
        "thinkphp",
        "springboot",
    ];

    /// Caddyfile 语法规则（指令集，注入站点块）。
    ///
    /// `file_server` 由调用方替换为本返回值——SpringBoot 场景整块换为反向代理。
    pub fn caddy_rules(&self) -> &'static str {
        match self {
            Self::WordPress => "try_files {path} {path}/ /index.php?{query}",
            Self::Discuz => {
                r#"rewrite * /index.php?{query}
try_files {path} {path}/ /index.php?{query}"#
            }
            Self::Typecho => "try_files {path} {path}/ /index.php?{query}",
            Self::Laravel => {
                r#"try_files {path} {path}/ /index.php?{query}
handle /storage/* {
    respond 403
}
handle /bootstrap/cache/* {
    respond 403
}"#
            }
            Self::ThinkPhp => "try_files {path} {path}/ /index.php?s={uri}",
            Self::SpringBoot => "reverse_proxy 127.0.0.1:8080",
        }
    }

    /// OpenLiteSpeed vhost 语法规则块（业界习惯：OLS 兼容 Apache mod_rewrite 语法）。
    ///
    /// SpringBoot 属于反代场景，OLS 走 `context` proxy（不支持 ProxyPass 指令）。
    pub fn ols_rules(&self) -> String {
        match self {
            Self::SpringBoot => r#"context / {
    type proxy
    handler 127.0.0.1:8080
    addDefaultCharset off
}"#
            .to_string(),
            other => format!(
                "rewrite {{
    enable 1
    rules <<<END_REWRITE_RULES
{}
END_REWRITE_RULES
}}",
                other.apache_rules()
            ),
        }
    }
    /// nginx 语法 rewrite 规则（其他引擎由调用方经 `for_engine` 适配）。
    pub fn nginx_rules(&self) -> &'static str {
        match self {
            Self::WordPress => {
                r#"location / {
    try_files $uri $uri/ /index.php?$args;
}"#
            }
            Self::Discuz => {
                r#"location / {
    rewrite ^([^\.]*)/topic-(.+)\.html$ $1/forum.php?mod=topic&topic=$2 last;
    rewrite ^([^\.]*)/article-([0-9]+)-([0-9]+)\.html$ $1/forum.php?mod=viewthread&tid=$2&page=$3 last;
    rewrite ^([^\.]*)/forum-(\w+)-([0-9]+)\.html$ $1/forum.php?mod=forumdisplay&fid=$2&page=$3 last;
}"#
            }
            Self::Typecho => {
                r#"location / {
    if (!-e $request_filename) {
        rewrite ^(.*)$ /index.php$1 last;
    }
}"#
            }
            Self::Laravel => {
                r#"location / {
    try_files $uri $uri/ /index.php?$query_string;
}
location ~ ^/(storage|bootstrap/cache)/ {
    deny all;
}"#
            }
            Self::ThinkPhp => {
                r#"location / {
    if (!-e $request_filename) {
        rewrite ^(.*)$ /index.php?s=$1 last;
    }
}"#
            }
            Self::SpringBoot => {
                r#"location / {
    proxy_pass http://127.0.0.1:8080;
    proxy_set_header Host $host;
    proxy_set_header X-Real-IP $remote_addr;
}"#
            }
        }
    }

    /// 是否反代类模板（Apache 侧须放在 VirtualHost 级而非 `<Directory>` 内）。
    pub fn is_proxy(&self) -> bool {
        matches!(self, Self::SpringBoot)
    }

    /// OpenLiteSpeed / Apache 语法（htaccess 兼容；OLS 支持 htaccess 语义）。
    pub fn apache_rules(&self) -> &'static str {
        match self {
            Self::WordPress => {
                r#"<IfModule mod_rewrite.c>
RewriteEngine On
RewriteBase /
RewriteRule ^index\.php$ - [L]
RewriteCond %{REQUEST_FILENAME} !-f
RewriteCond %{REQUEST_FILENAME} !-d
RewriteRule . /index.php [L]
</IfModule>"#
            }
            Self::Discuz => {
                r#"RewriteEngine On
RewriteRule ^topic-(.+)\.html$ forum.php?mod=topic&topic=$1 [L]
RewriteRule ^article-([0-9]+)-([0-9]+)\.html$ forum.php?mod=viewthread&tid=$1&page=$2 [L]"#
            }
            Self::Typecho => {
                r#"RewriteEngine On
RewriteCond %{REQUEST_FILENAME} !-f
RewriteCond %{REQUEST_FILENAME} !-d
RewriteRule ^(.*)$ /index.php/$1 [L]"#
            }
            Self::Laravel => {
                r#"RewriteEngine On
RewriteRule ^ index.php [L]"#
            }
            Self::ThinkPhp => {
                r#"RewriteEngine On
RewriteCond %{REQUEST_FILENAME} !-f
RewriteCond %{REQUEST_FILENAME} !-d
RewriteRule ^(.*)$ index.php?s=$1 [QSA,PT,L]"#
            }
            // SpringBoot 属于反代场景，Apache 走 ProxyPass
            Self::SpringBoot => {
                r#"ProxyPreserveHost On
ProxyPass / http://127.0.0.1:8080/
ProxyPassReverse / http://127.0.0.1:8080/"#
            }
        }
    }
}

/// 站点已套用的模板（`rewrite_preset` 名 → 枚举；未套用或名字非法返回 `None`）。
///
/// 非法名静默降级为「未套用」：配置生成是热路径，脏数据不应让 vhost 生成失败。
pub fn site_preset(site: &Website) -> Option<RewritePreset> {
    site.rewrite_preset
        .as_deref()
        .filter(|s| !s.is_empty())
        .and_then(RewritePreset::from_name)
}

/// nginx 站点的规则块（整块替换默认 `location /`）。
pub fn nginx_site_rules(site: &Website) -> Option<&'static str> {
    site_preset(site).map(|p| p.nginx_rules())
}

/// Apache 站点的规则指令（追加进 VirtualHost；SpringBoot 为 ProxyPass 组）。
pub fn apache_site_rules(site: &Website) -> Option<&'static str> {
    site_preset(site).map(|p| p.apache_rules())
}

/// Caddy 站点路由指令块（替换默认 `file_server browse`；带缩进由调用方处理）。
///
/// - 未套用模板 → `file_server browse`（目录浏览，既有行为不变）；
/// - 套用静态类模板 → 模板 `try_files` 指令 + `file_server`（改写后仍服务静态资源）；
/// - 套用 SpringBoot → `reverse_proxy 127.0.0.1:8080`——代理与静态服务互斥，
///   此时不再追加 `file_server`（否则两套 handler 语义冲突）。
pub fn caddy_route_block(site: &Website) -> String {
    match site_preset(site) {
        Some(RewritePreset::SpringBoot) => "reverse_proxy 127.0.0.1:8080".to_string(),
        Some(p) => format!("{}\nfile_server", p.caddy_rules()),
        None => "file_server browse".to_string(),
    }
}

/// OLS 站点规则块（追加进 virtualHost）。
pub fn ols_site_rules(site: &Website) -> Option<String> {
    site_preset(site).map(|p| p.ols_rules())
}

/// 按引擎取模板规则文本（前端预览 / 套用接口出参共用）。
///
/// 引擎名非法（未知）按 nginx 处理——配置生成侧另有 `from_name` 校验兜底。
pub fn preset_rules_for_engine(engine: &str, preset: RewritePreset) -> String {
    match engine.to_lowercase().as_str() {
        "apache" => preset.apache_rules().to_string(),
        "openlitespeed" | "ols" => preset.ols_rules(),
        "caddy" => preset.caddy_rules().to_string(),
        _ => preset.nginx_rules().to_string(),
    }
}

/// PHP 站点 fastcgi 配置片段（nginx 语法；其它引擎由调用方按能力适配）。
///
/// `runtime_version` 取绑定运行时的版本（如 "8.2"），生成独立 pool socket。
pub fn nginx_php_fastcgi(runtime_version: &str, site: &Website) -> String {
    let socket = php_fpm_socket(runtime_version, &site.domain);
    format!(
        r#"location ~ [^/]\.php(/|$) {{
    fastcgi_pass unix:{socket};
    fastcgi_index index.php;
    fastcgi_param SCRIPT_FILENAME $document_root$fastcgi_script_name;
    include fastcgi_params;
}}

location ~ /\.ht {{
    deny all;
}}
"#,
        socket = socket
    )
}

/// 项目站点反代片段（nginx 语法）：`proxy_pass http://127.0.0.1:<port>` + WebSocket upgrade。
pub fn nginx_project_proxy(project_port: i64) -> String {
    format!(
        r#"location / {{
    proxy_pass http://127.0.0.1:{port};
    proxy_http_version 1.1;
    proxy_set_header Upgrade $http_upgrade;
    proxy_set_header Connection "upgrade";
    proxy_set_header Host $host;
    proxy_set_header X-Real-IP $remote_addr;
    proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
    proxy_set_header X-Forwarded-Proto $scheme;
    proxy_read_timeout 300s;
}}
"#,
        port = project_port
    )
}

/// OLS 项目站点反代片段（lsphp handler 同源思路：context proxy）。
pub fn ols_project_proxy(project_port: i64) -> String {
    format!(
        r#"context / {{
    type proxy
    handler 127.0.0.1:{port}
    addDefaultCharset off
}}
"#,
        port = project_port
    )
}

/// Apache 项目站点反代片段。
pub fn apache_project_proxy(project_port: i64) -> String {
    format!(
        r#"ProxyPreserveHost On
ProxyPass / http://127.0.0.1:{port}/
ProxyPassReverse / http://127.0.0.1:{port}/
"#,
        port = project_port
    )
}

/// Caddy 项目站点反代片段。
pub fn caddy_project_proxy(project_port: i64) -> String {
    format!("reverse_proxy 127.0.0.1:{port}\n", port = project_port)
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn site() -> Website {
        Website {
            id: 1,
            name: "blog".into(),
            domain: "blog.example.com".into(),
            root_path: "/var/www/blog".into(),
            status: "active".into(),
            node_id: 1,
            engine: "nginx".into(),
            ssl_enabled: false,
            proxy_enabled: false,
            proxy_pass: None,
            site_type: "runtime_php".into(),
            runtime_id: Some(3),
            project_id: None,
            rewrite_preset: None,
            created_at: Utc::now(),
            resource_version: 0,
        }
    }

    #[test]
    fn php_socket_contains_version_and_site() {
        let sock = php_fpm_socket("8.2", "blog.example.com");
        assert_eq!(sock, "/run/php/php8.2-fpm-blog.example.com.sock");
        let cfg = nginx_php_fastcgi("8.2", &site());
        assert!(cfg.contains("fastcgi_pass unix:/run/php/php8.2-fpm-blog.example.com.sock;"));
        assert!(cfg.contains("deny all;"));
    }

    #[test]
    fn project_proxy_has_websocket_upgrade() {
        let cfg = nginx_project_proxy(3000);
        assert!(cfg.contains("proxy_pass http://127.0.0.1:3000;"));
        assert!(cfg.contains("proxy_set_header Upgrade $http_upgrade;"));
        assert!(cfg.contains("Connection \"upgrade\";"));
    }

    #[test]
    fn rewrite_presets_roundtrip_and_rules() {
        for name in [
            "wordpress",
            "discuz",
            "typecho",
            "laravel",
            "thinkphp",
            "springboot",
        ] {
            let preset = RewritePreset::from_name(name).expect("preset");
            assert_eq!(preset.as_str(), name);
            assert!(!preset.nginx_rules().is_empty());
            assert!(!preset.apache_rules().is_empty());
        }
        assert!(RewritePreset::from_name("evil").is_none());
    }

    /// 伪静态规则多引擎适配（Doc/20 §5.5 收口）：同一模板在各引擎下语法不同。
    #[test]
    fn preset_rules_adapt_across_engines() {
        let wp = RewritePreset::from_name("wordpress").unwrap();
        // nginx：location 块
        assert!(wp.nginx_rules().contains("location / {"));
        // apache：mod_rewrite 指令
        assert!(wp.apache_rules().contains("RewriteEngine On"));
        // caddy：Caddyfile 指令
        assert!(wp.caddy_rules().contains("try_files {path}"));
        // OLS：rewrite 包裹块
        let ols = wp.ols_rules();
        assert!(ols.contains("rewrite {"));
        assert!(ols.contains("END_REWRITE_RULES"));

        // 适配入口按引擎名路由（大小写不敏感；未知引擎回落 nginx 语法）
        assert!(preset_rules_for_engine("apache", wp).contains("RewriteEngine On"));
        assert!(preset_rules_for_engine("Apache", wp).contains("RewriteEngine On"));
        assert!(preset_rules_for_engine("openlitespeed", wp).contains("rewrite {"));
        assert!(preset_rules_for_engine("ols", wp).contains("rewrite {"));
        assert!(preset_rules_for_engine("caddy", wp).contains("try_files {path}"));
        assert!(preset_rules_for_engine("nginx", wp).contains("location / {"));
        assert!(preset_rules_for_engine("unknown-engine", wp).contains("location / {"));
    }

    /// SpringBoot 模板为反代场景：Apache 走 ProxyPass、OLS 走 context proxy、Caddy 走 reverse_proxy。
    #[test]
    fn springboot_preset_is_proxy_flavored() {
        let sb = RewritePreset::from_name("springboot").unwrap();
        assert!(sb
            .apache_rules()
            .contains("ProxyPass / http://127.0.0.1:8080/"));
        assert!(sb.ols_rules().contains("type proxy"));
        assert!(sb.caddy_rules().contains("reverse_proxy 127.0.0.1:8080"));
    }

    /// Caddy 路由块：未套用目录浏览、静态类模板保留 file_server、SpringBoot 互斥不叠加。
    #[test]
    fn caddy_route_block_semantics() {
        let plain = site();
        assert_eq!(caddy_route_block(&plain), "file_server browse");

        let mut wp = site();
        wp.rewrite_preset = Some("wordpress".into());
        let route = caddy_route_block(&wp);
        assert!(route.contains("try_files {path}"));
        assert!(route.ends_with("file_server"));
        assert!(!route.contains("browse"));

        let mut sb = site();
        sb.rewrite_preset = Some("springboot".into());
        assert_eq!(caddy_route_block(&sb), "reverse_proxy 127.0.0.1:8080");
    }

    /// 站点模板解析：未套用 / 空串 / 未知名 → None（静默降级，不爆配置生成）。
    #[test]
    fn site_preset_resolution_degrades_gracefully() {
        assert!(site_preset(&site()).is_none());

        let mut empty = site();
        empty.rewrite_preset = Some(String::new());
        assert!(site_preset(&empty).is_none());

        let mut dirty = site();
        dirty.rewrite_preset = Some("not-a-preset".into());
        assert!(site_preset(&dirty).is_none());
        assert!(nginx_site_rules(&dirty).is_none());
        assert!(apache_site_rules(&dirty).is_none());
        assert!(ols_site_rules(&dirty).is_none());

        let mut ok = site();
        ok.rewrite_preset = Some("Typecho".into()); // 大小写不敏感
        assert_eq!(site_preset(&ok).map(|p| p.as_str()), Some("typecho"));
    }

    /// 模板清单常量与 `from_name` 往返一致（前端下拉 / 后端校验单一事实来源）。
    #[test]
    fn preset_all_matches_from_name() {
        assert_eq!(RewritePreset::ALL.len(), 6);
        for name in RewritePreset::ALL {
            let p = RewritePreset::from_name(name).expect("ALL 中每项都必须可解析");
            assert_eq!(p.as_str(), name);
        }
    }

    #[test]
    fn site_types_whitelist() {
        assert_eq!(SITE_TYPES.len(), 4);
        for t in SITE_TYPES {
            assert!(Website::is_valid_site_type(t));
        }
    }

    #[test]
    fn proxy_targets_per_engine() {
        assert!(ols_project_proxy(8080).contains("handler 127.0.0.1:8080"));
        assert!(apache_project_proxy(8080).contains("ProxyPass / http://127.0.0.1:8080/"));
        assert!(caddy_project_proxy(8080).contains("reverse_proxy 127.0.0.1:8080"));
        assert_eq!(project_proxy_target(3000), "http://127.0.0.1:3000");
    }
}
