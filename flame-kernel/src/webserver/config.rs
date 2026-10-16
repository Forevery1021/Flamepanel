//! 站点配置生成（各引擎分支）。
//!
//! **伪静态注入**（Doc/20 §5.5 收口）：站点套用的模板名（`Website::rewrite_preset`）
//! 在此解析为对应引擎语法的规则块并替换/注入默认路由——「套用即生效」，
//! 且套用结果随任何一次配置重生成（证书绑定、引擎切换、手工改配置）持续保留。

use super::engine::WebServerEngine;
use super::site_binding;
use crate::domain::entity::Website;

pub trait WebServerConfigGenerator: Send + Sync {
    fn engine(&self) -> WebServerEngine;
    fn generate_global_config(&self, port: u16, worker_processes: u32) -> String;
    /// `ssl_cert` / `ssl_key` 是引擎可访问的文件路径，不是 PEM 正文。
    fn generate_site_config(
        &self,
        site: &Website,
        ssl_cert: Option<&str>,
        ssl_key: Option<&str>,
    ) -> String;
    fn generate_reverse_proxy_config(&self, domain: &str, proxy_pass: &str, port: u16) -> String;
}

/// 逐行缩进（缩进块拼接用；行尾 `\n` 保留，空行不补缩进避免尾随空格）。
fn indent_block(text: &str, spaces: usize) -> String {
    let pad = " ".repeat(spaces);
    let mut out = String::new();
    for line in text.lines() {
        if line.trim().is_empty() {
            out.push('\n');
        } else {
            out.push_str(&pad);
            out.push_str(line);
            out.push('\n');
        }
    }
    out
}

/// nginx 站点默认路由（未套用伪静态模板时）。
const NGINX_DEFAULT_LOCATION: &str = "location / {\n    try_files $uri $uri/ =404;\n}";

/// nginx 站点路由块：已套用伪静态模板 → 模板规则；否则默认 `try_files =404`。
fn nginx_location_block(site: &Website) -> String {
    match site_binding::nginx_site_rules(site) {
        Some(rules) => indent_block(rules, 4),
        None => indent_block(NGINX_DEFAULT_LOCATION, 4),
    }
}

/// nginx 站点 `server {}` 块（HTTP 段）：80 监听 + 伪静态规则。
fn nginx_site_block(site: &Website, port: u16) -> String {
    format!(
        "server {{\n    listen {port};\n    server_name {domain};\n    root {root};\n    index index.html index.htm index.php;\n    {waf}\n\n{location}}}\n",
        port = port,
        domain = site.domain,
        root = site.root_path,
        waf = waf_include(&WebServerEngine::Nginx),
        location = nginx_location_block(site)
    )
}

/// WAF server 级 include（Doc/32 W3：规则由 `waf_nginx` 编译落盘；glob 缺失时 nginx -t 不报错）。
fn waf_include(engine: &WebServerEngine) -> String {
    format!(
        "# flamepanel:waf\n    {}",
        super::waf_nginx::server_include_directive(engine)
    )
}

/// Apache 站点 WAF include（Debian / RHEL 两个位置都写；`IncludeOptional` + glob 缺失不报错）。
fn apache_waf_include() -> String {
    let mut out = String::from("# flamepanel:waf");
    for d in super::waf_engines::apache_vhost_includes() {
        out.push_str("\n    ");
        out.push_str(&d);
    }
    out
}

/// Apache 站点 VirtualHost 级规则（仅反代类模板：`ProxyPass` 不允许出现在 `<Directory>` 内）。
fn apache_rules_block(site: &Website) -> String {
    match site_binding::site_preset(site) {
        Some(p) if p.is_proxy() => indent_block(p.apache_rules(), 4),
        _ => String::new(),
    }
}

/// Apache `<Directory>` 块：伪静态模板（htaccess 语法）注入目录上下文。
///
/// Doc/33 C-1：旧实现把 `RewriteBase /` 与 `%{REQUEST_FILENAME} !-f` 写在 VirtualHost
/// 级——`RewriteBase` 在该上下文非法（configtest 失败），`REQUEST_FILENAME` 也尚未映射
/// 到文件系统路径（`-f/-d` 恒为假），且 `^topic-…` 这类无前导 `/` 的模式永不匹配。
/// 模板本身是 htaccess 语义，放进 `<Directory {root}>` 后三者均按预期工作。
fn apache_directory_block(site: &Website) -> String {
    let dir_rules = match site_binding::site_preset(site) {
        Some(p) if !p.is_proxy() => format!("{}\n", indent_block(p.apache_rules(), 8)),
        _ => String::new(),
    };
    format!(
        "<Directory {root}>\n        Options Indexes FollowSymLinks\n        AllowOverride All\n        Require all granted\n{dir_rules}    </Directory>",
        root = site.root_path,
        dir_rules = dir_rules,
    )
}

/// Apache 站点 `VirtualHost` 块（HTTP 段）：80 监听 + 伪静态规则注入。
fn apache_vhost_block(site: &Website, port: u16) -> String {
    let rules = apache_rules_block(site);
    let rules_section = if rules.is_empty() {
        String::new()
    } else {
        format!("\n{}\n", rules)
    };
    format!(
        "<VirtualHost *:{port}>\n    ServerName {domain}\n    DocumentRoot {root}\n    {waf}\n\n    {directory}\n{rules}</VirtualHost>\n",
        port = port,
        domain = site.domain,
        root = site.root_path,
        directory = apache_directory_block(site),
        rules = rules_section,
        waf = apache_waf_include()
    )
}

/// OpenLiteSpeed 站点规则块（已套用伪静态模板时注入 virtualHost 主体）。
fn ols_rules_block(site: &Website) -> String {
    match site_binding::ols_site_rules(site) {
        Some(rules) => format!("\n{}", indent_block(&rules, 4)),
        None => String::new(),
    }
}

/// OpenResty 站点路由块：已套用伪静态模板 → 模板规则；
/// 否则沿用既有 SPA 语义（`try_files … /index.html`）。
fn openresty_location_block(site: &Website) -> String {
    const SPA: &str = "location / {\n    try_files $uri $uri/ /index.html;\n}";
    match site_binding::nginx_site_rules(site) {
        Some(rules) => indent_block(rules, 4),
        None => indent_block(SPA, 4),
    }
}

pub struct NginxConfig;
pub struct ApacheConfig;
pub struct OpenLiteSpeedConfig;
pub struct OpenRestyConfig;
pub struct CaddyConfig;

impl WebServerConfigGenerator for NginxConfig {
    fn engine(&self) -> WebServerEngine {
        WebServerEngine::Nginx
    }

    fn generate_global_config(&self, port: u16, worker_processes: u32) -> String {
        format!(
            r#"user www-data;
worker_processes {wp};
pid /run/nginx.pid;

events {{
    worker_connections 1024;
    use epoll;
    multi_accept on;
}}

http {{
    include /etc/nginx/mime.types;
    default_type application/octet-stream;
    sendfile on;
    tcp_nopush on;
    tcp_nodelay on;
    keepalive_timeout 65;
    types_hash_max_size 2048;
    server_tokens off;
    # 显式声明：WAF 命中日志也是 http 级 access_log（二者并存继承；缺省隐式日志会被替换）
    access_log /var/log/nginx/access.log;

    server {{
        listen {port};
        server_name localhost;
        root /var/www/html;
    }}

    include /etc/nginx/conf.d/*.conf;
    include /etc/nginx/sites-enabled/*;
    # flamepanel:sites（面板部署的站点，见 webserver::layout）
    include /etc/nginx/flamepanel/sites/*.conf;
}}
"#,
            wp = worker_processes,
            port = port
        )
    }

    fn generate_site_config(
        &self,
        site: &Website,
        ssl_cert: Option<&str>,
        ssl_key: Option<&str>,
    ) -> String {
        let mut config = String::new();
        if let Some(cert) = ssl_cert {
            if let Some(key) = ssl_key {
                config.push_str(&format!(
                    r#"server {{
    listen {sp} ssl http2;
    server_name {domain};
    root {root};
    index index.html index.htm index.php;

    ssl_certificate {cert};
    ssl_certificate_key {key};
    ssl_protocols TLSv1.2 TLSv1.3;
    ssl_ciphers HIGH:!aNULL:!MD5;
    {waf}

{location}}}
"#,
                    sp = 443,
                    domain = site.domain,
                    root = site.root_path,
                    cert = cert,
                    key = key,
                    waf = waf_include(&WebServerEngine::Nginx),
                    location = nginx_location_block(site)
                ));
            }
        }
        config.push_str(&nginx_site_block(site, 80));
        config
    }

    fn generate_reverse_proxy_config(&self, domain: &str, proxy_pass: &str, port: u16) -> String {
        format!(
            r#"server {{
    listen {port};
    server_name {domain};

    location / {{
        proxy_pass {proxy};
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
        proxy_buffering off;
    }}
}}
"#,
            port = port,
            domain = domain,
            proxy = proxy_pass
        )
    }
}

impl WebServerConfigGenerator for ApacheConfig {
    fn engine(&self) -> WebServerEngine {
        WebServerEngine::Apache
    }

    fn generate_global_config(&self, port: u16, worker_processes: u32) -> String {
        format!(
            r#"ServerRoot "/etc/httpd"
Listen {port}
User apache
Group apache
ServerAdmin admin@localhost
ServerTokens Prod
Include conf.modules.d/*.conf
Include conf.d/*.conf
# flamepanel:sites（面板部署的站点，见 webserver::layout）
IncludeOptional /etc/httpd/flamepanel/sites/*.conf

<IfModule mpm_prefork_module>
    StartServers {wp}
    MinSpareServers 5
    MaxSpareServers 20
    MaxRequestWorkers 256
    MaxConnectionsPerChild 10000
</IfModule>
"#,
            port = port,
            wp = worker_processes
        )
    }

    fn generate_site_config(
        &self,
        site: &Website,
        ssl_cert: Option<&str>,
        ssl_key: Option<&str>,
    ) -> String {
        let mut config = String::new();
        if let Some(cert) = ssl_cert {
            if let Some(key) = ssl_key {
                config.push_str(&format!(
                    r#"<VirtualHost *:{sp}>
    ServerName {domain}
    DocumentRoot {root}
    SSLEngine on
    SSLCertificateFile {cert}
    SSLCertificateKeyFile {key}
    SSLProtocol all -SSLv3 -TLSv1 -TLSv1.1
    SSLCipherSuite HIGH:!aNULL:!MD5
    {waf}

    {directory}

    {rules}
</VirtualHost>
"#,
                    sp = 443,
                    domain = site.domain,
                    root = site.root_path,
                    cert = cert,
                    key = key,
                    rules = apache_rules_block(site),
                    directory = apache_directory_block(site),
                    waf = apache_waf_include()
                ));
            }
        }
        config.push_str(&apache_vhost_block(site, 80));
        config
    }

    fn generate_reverse_proxy_config(&self, domain: &str, proxy_pass: &str, port: u16) -> String {
        format!(
            r#"<VirtualHost *:{port}>
    ServerName {domain}

    ProxyPreserveHost On
    ProxyPass / {proxy}/
    ProxyPassReverse / {proxy}/
</VirtualHost>
"#,
            port = port,
            domain = domain,
            proxy = proxy_pass
        )
    }
}

impl WebServerConfigGenerator for OpenLiteSpeedConfig {
    fn engine(&self) -> WebServerEngine {
        WebServerEngine::OpenLiteSpeed
    }

    fn generate_global_config(&self, port: u16, _worker_processes: u32) -> String {
        format!(
            r#"listener "HTTP" {{
    address *:{port}
    secure 0
}}

listener "HTTPS" {{
    address *:443
    secure 1
}}

vhosts {{
    auto_restrict 0
}}
"#,
            port = port
        )
    }

    fn generate_site_config(
        &self,
        site: &Website,
        _ssl_cert: Option<&str>,
        _ssl_key: Option<&str>,
    ) -> String {
        // 输出 vhosts/<domain>/vhconf.conf 内容；virtualHost 声明与 listener map
        // 由部署流水线注册到 httpd_config.conf（webserver::layout::ols_register）。
        format!(
            r#"# {domain} (managed by Flamepanel)
docRoot                   $VH_ROOT/
enableGzip                1

index  {{
  useServer               0
  indexFiles              index.html, index.php
}}
{rules}"#,
            domain = site.domain,
            rules = ols_rules_block(site)
        )
    }

    fn generate_reverse_proxy_config(&self, domain: &str, proxy_pass: &str, _port: u16) -> String {
        format!(
            r#"virtualHost {domain} {{
    vhRoot /usr/local/lsws/{domain}
    enableScript 1
    restrained 0

    rewrite {{
        enable 1
        autoLoadHtaccess 1
    }}

    context / {{
        type proxy
        handler {proxy}
        addDefaultCharset off
        extraHeaders <<<END_extraHeaders
X-Forwarded-For $proxy_add_x_forwarded_for
X-Real-IP $remote_addr
END_extraHeaders
    }}
}}
"#,
            domain = domain,
            proxy = proxy_pass
        )
    }
}

impl WebServerConfigGenerator for OpenRestyConfig {
    fn engine(&self) -> WebServerEngine {
        WebServerEngine::OpenResty
    }

    fn generate_global_config(&self, port: u16, worker_processes: u32) -> String {
        format!(
            r#"user www-data;
worker_processes {wp};
pid /run/openresty.pid;

events {{
    worker_connections 1024;
}}

http {{
    include mime.types;
    default_type application/octet-stream;
    sendfile on;
    keepalive_timeout 65;
    access_log logs/access.log;
    lua_package_path "/usr/local/openresty/lualib/?.lua;;";

    init_by_lua_block {{
        -- global init
    }}

    server {{
        listen {port};
        server_name localhost;
        location / {{
            root html;
        }}
    }}

    include sites-enabled/*;
    # flamepanel:sites（面板部署的站点，见 webserver::layout）
    include /usr/local/openresty/nginx/conf/flamepanel/sites/*.conf;
}}
"#,
            wp = worker_processes,
            port = port
        )
    }

    fn generate_site_config(
        &self,
        site: &Website,
        ssl_cert: Option<&str>,
        ssl_key: Option<&str>,
    ) -> String {
        let mut config = String::new();
        if let Some(cert) = ssl_cert {
            if let Some(key) = ssl_key {
                config.push_str(&format!(
                    r#"server {{
    listen 443 ssl http2;
    server_name {domain};
    root {root};

    ssl_certificate {cert};
    ssl_certificate_key {key};
    ssl_protocols TLSv1.2 TLSv1.3;
    ssl_ciphers HIGH:!aNULL:!MD5;
    {waf}

{location}}}
"#,
                    domain = site.domain,
                    root = site.root_path,
                    cert = cert,
                    key = key,
                    waf = waf_include(&WebServerEngine::OpenResty),
                    location = openresty_location_block(site)
                ));
            }
        }
        config.push_str(&format!(
            r#"server {{
    listen 80;
    server_name {domain};
    root {root};
    {waf}

{location}}}
"#,
            domain = site.domain,
            root = site.root_path,
            waf = waf_include(&WebServerEngine::OpenResty),
            location = openresty_location_block(site)
        ));
        config
    }

    fn generate_reverse_proxy_config(&self, domain: &str, proxy_pass: &str, port: u16) -> String {
        format!(
            r#"server {{
    listen {port};
    server_name {domain};

    location / {{
        proxy_pass {proxy};
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
    }}

    location /__lua {{
        content_by_lua_block {{
            ngx.say("OpenResty reverse proxy active")
        }}
    }}
}}
"#,
            port = port,
            domain = domain,
            proxy = proxy_pass
        )
    }
}

impl WebServerConfigGenerator for CaddyConfig {
    fn engine(&self) -> WebServerEngine {
        WebServerEngine::Caddy
    }

    fn generate_global_config(&self, _port: u16, _worker_processes: u32) -> String {
        // 不能 `admin off`：`caddy reload` 依赖本机 admin API（localhost:2019），
        // 关闭后所有站点变更的 reload 都会失败并回滚。
        r#"{
    ocsp_stapling on
}

*.localhost, localhost {
    respond "Caddy is running"
}

# flamepanel:sites（面板部署的站点，见 webserver::layout）
import /etc/caddy/flamepanel/sites/*.caddy
"#
        .to_string()
    }

    fn generate_site_config(
        &self,
        site: &Website,
        ssl_cert: Option<&str>,
        ssl_key: Option<&str>,
    ) -> String {
        // 显式证书：`tls <cert> <key>`；未提供时交给 Caddy 自动 HTTPS
        let tls = match (ssl_cert, ssl_key) {
            (Some(c), Some(k)) => format!("    tls {c} {k}\n"),
            _ => String::new(),
        };
        format!(
            r#"{domain} {{
{tls}    root * {root}
    encode gzip
    # flamepanel:waf
    {waf}

{route}
    header {{
        -Server
        X-Content-Type-Options "nosniff"
        X-Frame-Options "DENY"
    }}
}}

http://{domain} {{
    redir https://{{host}}{{uri}} 301
}}
"#,
            domain = site.domain,
            root = site.root_path,
            tls = tls,
            route = indent_block(&site_binding::caddy_route_block(site), 4),
            waf = super::waf_engines::caddy_import_directive()
        )
    }

    fn generate_reverse_proxy_config(&self, domain: &str, proxy_pass: &str, _port: u16) -> String {
        format!(
            r#"{domain} {{
    reverse_proxy {proxy}
}}
"#,
            domain = domain,
            proxy = proxy_pass
        )
    }
}

pub fn get_config_generator(engine: &WebServerEngine) -> Box<dyn WebServerConfigGenerator> {
    match engine {
        WebServerEngine::Nginx => Box::new(NginxConfig),
        WebServerEngine::Apache => Box::new(ApacheConfig),
        WebServerEngine::OpenLiteSpeed => Box::new(OpenLiteSpeedConfig),
        WebServerEngine::OpenResty => Box::new(OpenRestyConfig),
        WebServerEngine::Caddy => Box::new(CaddyConfig),
    }
}

// ── 伪静态注入：五引擎生成器单测（Doc/20 §5.5 收口）──────────────────
#[cfg(test)]
mod tests {
    use super::*;
    use chrono::Utc;

    fn site(engine: &str, preset: Option<&str>) -> Website {
        Website {
            id: 1,
            name: "s".into(),
            domain: "cfg.test.com".into(),
            root_path: "/var/www/cfg".into(),
            status: "active".into(),
            node_id: 1,
            engine: engine.into(),
            ssl_enabled: false,
            proxy_enabled: false,
            proxy_pass: None,
            site_type: "runtime_php".into(),
            runtime_id: None,
            project_id: None,
            rewrite_preset: preset.map(|s| s.to_string()),
            created_at: Utc::now(),
            resource_version: 0,
        }
    }

    fn config(engine: &WebServerEngine, site: &Website) -> String {
        get_config_generator(engine).generate_site_config(site, None, None)
    }

    #[test]
    fn nginx_rule_injected_and_default_preserved() {
        let plain = config(&WebServerEngine::Nginx, &site("nginx", None));
        assert!(plain.contains("try_files $uri $uri/ =404;"));

        let wp = config(&WebServerEngine::Nginx, &site("nginx", Some("wordpress")));
        assert!(wp.contains("try_files $uri $uri/ /index.php?$args;"));
        assert!(!wp.contains("=404"));
        // 缩进完整（规则块落在 server 块内）
        assert!(wp.contains("\n    location / {"));
    }

    #[test]
    fn apache_rule_injected_additively() {
        let plain = config(&WebServerEngine::Apache, &site("apache", None));
        assert!(!plain.contains("RewriteEngine"));

        let wp = config(&WebServerEngine::Apache, &site("apache", Some("wordpress")));
        // Apache 规则是「追加」语义：静态服务与 Directory 块保持
        assert!(wp.contains("RewriteEngine On"));
        assert!(wp.contains("DocumentRoot /var/www/cfg"));
        assert!(wp.contains("Require all granted"));
        // SpringBoot 模板走 ProxyPass
        let sb = config(
            &WebServerEngine::Apache,
            &site("apache", Some("springboot")),
        );
        assert!(sb.contains("ProxyPassReverse / http://127.0.0.1:8080/"));
    }

    /// Doc/33 C-1：htaccess 语义规则必须落在 `<Directory>` 内；反代模板留在 VirtualHost 级。
    #[test]
    fn apache_rewrite_rules_live_in_directory_context() {
        for preset in ["wordpress", "discuz", "typecho", "laravel", "thinkphp"] {
            for cfg in [
                config(&WebServerEngine::Apache, &site("apache", Some(preset))),
                get_config_generator(&WebServerEngine::Apache).generate_site_config(
                    &site("apache", Some(preset)),
                    Some("/c.pem"),
                    Some("/k.pem"),
                ),
            ] {
                let mut rest = cfg.as_str();
                let mut seen = 0;
                while let Some(start) = rest.find("<Directory ") {
                    let end = rest[start..].find("</Directory>").unwrap() + start;
                    let block = &rest[start..end];
                    assert!(block.contains("RewriteEngine On"), "{preset}: {block}");
                    seen += 1;
                    let outside = &rest[..start];
                    assert!(
                        !outside.contains("Rewrite"),
                        "{preset} 规则泄漏到 VirtualHost 级"
                    );
                    rest = &rest[end..];
                }
                assert!(seen >= 1);
                assert!(
                    !rest.contains("Rewrite"),
                    "{preset} 规则泄漏到 VirtualHost 级"
                );
            }
        }
        let sb = config(
            &WebServerEngine::Apache,
            &site("apache", Some("springboot")),
        );
        let dir_start = sb.find("<Directory ").unwrap();
        let dir_end = sb.find("</Directory>").unwrap();
        assert!(!sb[dir_start..dir_end].contains("ProxyPass"));
        assert!(sb[dir_end..].contains("ProxyPass / http://127.0.0.1:8080/"));
    }

    /// 真实引擎回归：每个模板生成的 VirtualHost 都须通过 `apache2 -t`（未安装则跳过）。
    #[test]
    fn real_apache_accepts_every_preset_vhost() {
        let bin = ["/usr/sbin/apache2", "/usr/sbin/httpd", "apache2", "httpd"]
            .into_iter()
            .find(|b| std::process::Command::new(b).arg("-v").output().is_ok());
        let modules = [
            "/usr/lib/apache2/modules",
            "/usr/lib64/httpd/modules",
            "/usr/lib/httpd/modules",
        ]
        .into_iter()
        .find(|d| std::path::Path::new(d).join("mod_rewrite.so").exists());
        let (Some(bin), Some(m)) = (bin, modules) else {
            eprintln!("apache not installed; skipping");
            return;
        };
        let dir = std::env::temp_dir().join(format!("fp-preset-apache-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let d = dir.display().to_string();
        for preset in [
            None,
            Some("wordpress"),
            Some("discuz"),
            Some("typecho"),
            Some("laravel"),
            Some("thinkphp"),
            Some("springboot"),
        ] {
            let mut ws = site("apache", preset);
            ws.root_path = d.clone();
            ws.domain = "preset.test".into();
            let vhost: String = get_config_generator(&WebServerEngine::Apache)
                .generate_site_config(&ws, None, None)
                .lines()
                // WAF include 指向宿主真实目录，与本用例无关
                .filter(|l| !l.contains("flamepanel/waf"))
                .map(|l| format!("{}\n", l.replace("*:80", "127.0.0.1:18091")))
                .collect();
            let mut main = format!(
                "ServerRoot {d}\nPidFile {d}/httpd.pid\nErrorLog {d}/error.log\nListen 127.0.0.1:18091\n"
            );
            for (name, file) in [
                ("mpm_event_module", "mod_mpm_event.so"),
                ("authz_core_module", "mod_authz_core.so"),
                ("rewrite_module", "mod_rewrite.so"),
                ("proxy_module", "mod_proxy.so"),
                ("proxy_http_module", "mod_proxy_http.so"),
            ] {
                main.push_str(&format!(
                    "<IfModule !{name}>\nLoadModule {name} {m}/{file}\n</IfModule>\n"
                ));
            }
            main.push_str(&vhost);
            std::fs::write(dir.join("httpd.conf"), &main).unwrap();
            let out = std::process::Command::new(bin)
                .arg("-t")
                .arg("-f")
                .arg(dir.join("httpd.conf"))
                .output()
                .unwrap();
            let stderr = String::from_utf8_lossy(&out.stderr).to_string();
            assert!(
                out.status.success(),
                "{preset:?}: {bin} -t failed:\n{stderr}\n{main}"
            );
        }
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn ols_rule_injected_in_vhost() {
        let plain = config(
            &WebServerEngine::OpenLiteSpeed,
            &site("openlitespeed", None),
        );
        assert!(!plain.contains("rewrite {"));

        let wp = config(
            &WebServerEngine::OpenLiteSpeed,
            &site("openlitespeed", Some("wordpress")),
        );
        assert!(wp.contains("rewrite {"));
        assert!(wp.contains("enable 1"));
        assert!(wp.contains("RewriteEngine On"));
        // SpringBoot 走 context proxy
        let sb = config(
            &WebServerEngine::OpenLiteSpeed,
            &site("openlitespeed", Some("springboot")),
        );
        assert!(sb.contains("handler 127.0.0.1:8080"));
    }

    #[test]
    fn openresty_spa_default_replaced_by_rules() {
        let plain = config(&WebServerEngine::OpenResty, &site("openresty", None));
        assert!(plain.contains("try_files $uri $uri/ /index.html;"));

        let tl = config(
            &WebServerEngine::OpenResty,
            &site("openresty", Some("thinkphp")),
        );
        assert!(tl.contains("index.php?s=$1"));
        assert!(!tl.contains("/index.html;"));
    }

    #[test]
    fn caddy_route_per_preset() {
        let plain = config(&WebServerEngine::Caddy, &site("caddy", None));
        assert!(plain.contains("file_server browse"));

        let wp = config(&WebServerEngine::Caddy, &site("caddy", Some("wordpress")));
        assert!(wp.contains("try_files {path} {path}/ /index.php?{query}"));
        // 静态类模板保留静态服务
        assert!(wp.contains("file_server"));
        assert!(!wp.contains("file_server browse"));

        let sb = config(&WebServerEngine::Caddy, &site("caddy", Some("springboot")));
        assert!(sb.contains("reverse_proxy 127.0.0.1:8080"));
        // 代理与静态服务互斥：不再叠加 file_server
        assert!(!sb.contains("file_server"));
    }

    /// WAF 站点级接入（Doc/30 v1.1）：Apache 两个发行版位置、Caddy import，均先于路由规则。
    #[test]
    fn apache_and_caddy_sites_include_waf() {
        let ap = config(&WebServerEngine::Apache, &site("apache", Some("wordpress")));
        for inc in super::super::waf_engines::apache_vhost_includes() {
            assert!(ap.contains(&format!("    {inc}\n")), "{inc}");
        }
        assert!(ap.find("flamepanel/waf").unwrap() < ap.find("<Directory").unwrap());
        let ap_ssl = get_config_generator(&WebServerEngine::Apache).generate_site_config(
            &site("apache", None),
            Some("/c.pem"),
            Some("/k.pem"),
        );
        // HTTPS 与 HTTP 两个 VirtualHost 都接入
        assert_eq!(
            ap_ssl
                .matches("/etc/apache2/flamepanel/waf/vhost*.conf")
                .count(),
            2
        );

        let cd = config(&WebServerEngine::Caddy, &site("caddy", Some("wordpress")));
        let imp = super::super::waf_engines::caddy_import_directive();
        assert!(cd.contains(&format!("    {imp}\n")));
        assert!(cd.find(&imp).unwrap() < cd.find("try_files").unwrap());
        // 仅主站点块接入；http:// 跳转块不需要
        assert_eq!(cd.matches(&imp).count(), 1);
    }

    #[test]
    fn invalid_preset_name_degrades_to_default() {
        // 脏数据（未知模板名）不炸配置生成，静默回落默认路由
        let cfg = config(
            &WebServerEngine::Nginx,
            &site("nginx", Some("evil-template")),
        );
        assert!(cfg.contains("try_files $uri $uri/ =404;"));
        let empty = config(&WebServerEngine::Nginx, &site("nginx", Some("")));
        assert!(empty.contains("try_files $uri $uri/ =404;"));
    }

    #[test]
    fn https_block_also_carries_rules() {
        let s = site("nginx", Some("laravel"));
        let cfg = get_config_generator(&WebServerEngine::Nginx).generate_site_config(
            &s,
            Some("-----BEGIN CERTIFICATE-----"),
            Some("-----BEGIN PRIVATE KEY-----"),
        );
        // 443 与 80 两个 server 块都带规则
        assert_eq!(
            cfg.matches("try_files $uri $uri/ /index.php?$query_string;")
                .count(),
            2
        );
        assert!(cfg.contains("deny all;")); // Laravel 规则自带 storage 保护
    }

    // ── 站点流水线对齐（webserver::layout）：生成的全局配置已包含面板站点钩子 ──
    #[test]
    fn generated_globals_already_contain_layout_hook() {
        use crate::infrastructure::os::DistroType;
        use crate::webserver::layout::{apply_hook, SiteLayout};
        for (engine, distro) in [
            (WebServerEngine::Nginx, DistroType::Ubuntu),
            (WebServerEngine::OpenResty, DistroType::Ubuntu),
            (WebServerEngine::Apache, DistroType::RHEL),
            (WebServerEngine::Caddy, DistroType::Ubuntu),
        ] {
            let global = get_config_generator(&engine).generate_global_config(80, 2);
            let layout = SiteLayout::for_engine(engine.clone(), &distro);
            assert_eq!(
                apply_hook(&global, &layout.hook).unwrap(),
                None,
                "{engine:?} global config must already include the flamepanel sites dir"
            );
        }
    }

    #[test]
    fn caddy_global_keeps_admin_api_for_reload() {
        let g = get_config_generator(&WebServerEngine::Caddy).generate_global_config(80, 1);
        assert!(!g.contains("admin off"));
    }

    #[test]
    fn caddy_explicit_cert_emits_tls_directive() {
        let s = site("caddy", None);
        let gen = get_config_generator(&WebServerEngine::Caddy);
        let with = gen.generate_site_config(&s, Some("/c/cert.pem"), Some("/c/key.pem"));
        assert!(with.contains("    tls /c/cert.pem /c/key.pem\n"));
        let without = gen.generate_site_config(&s, None, None);
        assert!(!without.contains("tls "));
    }

    #[test]
    fn ols_site_config_is_vhconf_not_vhost_declaration() {
        let cfg = config(
            &WebServerEngine::OpenLiteSpeed,
            &site("openlitespeed", None),
        );
        assert!(cfg.contains("docRoot                   $VH_ROOT/"));
        assert!(!cfg.contains("virtualHost"));
        assert!(!cfg.contains("vhRoot"));
    }
}
