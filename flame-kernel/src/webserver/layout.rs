//! 站点配置落盘布局（Doc/20 §5.5 站点配置流水线）。
//!
//! 设计要点：
//! - **面板自有目录**：每个引擎的站点文件统一写入 `<引擎配置目录>/flamepanel/sites/`，
//!   文件名 `<domain>.<ext>`，停用即重命名为 `<file>.disabled`（引擎 glob 不再匹配）。
//!   不依赖发行版各异的 `sites-enabled` / `a2ensite` 约定，RHEL / Debian / Alpine 行为一致。
//! - **主配置钩子**：在主配置中插入一行 include/import（幂等，重复执行不产生重复行）。
//! - **OpenLiteSpeed**：OLS 不支持在 listener 外声明映射，因此由面板在 `httpd_config.conf`
//!   中维护带标记的 `virtualHost` 块与 listener `map` 行（均可被精确识别与移除）。
//!
//! 本模块只做纯函数计算（路径 / 文本变换），不触碰文件系统，便于穷举单测；
//! 落盘由 [`super::host::EngineHost`] 完成。

use super::engine::WebServerEngine;
use crate::infrastructure::os::DistroType;

/// 主配置文件中的标记（便于人工识别，也用于 OLS 块定位）。
pub const HOOK_MARKER: &str = "# flamepanel:sites (managed by Flamepanel, do not edit)";

/// 主配置接入方式。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Hook {
    /// 在首个 `http {` 行之后插入 `include <glob>;`（Nginx / OpenResty）。
    HttpBlockInclude { directive: String },
    /// 在文件末尾追加指令行（Apache `IncludeOptional` / Caddy `import`）。
    Append { directive: String },
    /// OLS：逐站点注册 virtualHost + listener map（见 [`ols_register`]）。
    OlsRegistry,
}

/// 某引擎在某发行版上的站点布局。
#[derive(Debug, Clone, PartialEq)]
pub struct SiteLayout {
    pub engine: WebServerEngine,
    /// 面板自有站点目录（OLS 为 vhosts 根目录）。
    pub sites_dir: String,
    /// 站点文件扩展名（不含点；OLS 不使用）。
    pub ext: &'static str,
    /// 主配置文件（引擎是否已安装以它是否存在为准）。
    pub main_config: String,
    pub hook: Hook,
    /// 旧版本写入过的位置模板（`{domain}` 占位），部署 / 删除时顺带清理。
    legacy: Vec<String>,
}

/// 停用后缀。
pub const DISABLED_SUFFIX: &str = ".disabled";

impl SiteLayout {
    pub fn for_engine(engine: WebServerEngine, distro: &DistroType) -> Self {
        match engine {
            WebServerEngine::Nginx => {
                let sites = "/etc/nginx/flamepanel/sites".to_string();
                Self {
                    engine,
                    hook: Hook::HttpBlockInclude {
                        directive: format!("include {sites}/*.conf;"),
                    },
                    sites_dir: sites,
                    ext: "conf",
                    main_config: "/etc/nginx/nginx.conf".into(),
                    legacy: vec![
                        "/etc/nginx/sites-available/{domain}".into(),
                        "/etc/nginx/sites-enabled/{domain}".into(),
                    ],
                }
            }
            WebServerEngine::OpenResty => {
                let base = "/usr/local/openresty/nginx/conf";
                let sites = format!("{base}/flamepanel/sites");
                Self {
                    engine,
                    hook: Hook::HttpBlockInclude {
                        directive: format!("include {sites}/*.conf;"),
                    },
                    sites_dir: sites,
                    ext: "conf",
                    main_config: format!("{base}/nginx.conf"),
                    legacy: vec![
                        format!("{base}/sites-available/{{domain}}"),
                        format!("{base}/sites-enabled/{{domain}}"),
                    ],
                }
            }
            WebServerEngine::Apache => {
                let (base, main, legacy): (&str, String, Vec<String>) = match distro {
                    DistroType::Debian | DistroType::Ubuntu => (
                        "/etc/apache2",
                        "/etc/apache2/apache2.conf".into(),
                        vec![
                            "/etc/apache2/sites-available/{domain}".into(),
                            "/etc/apache2/sites-enabled/{domain}".into(),
                        ],
                    ),
                    DistroType::Alpine => {
                        ("/etc/apache2", "/etc/apache2/httpd.conf".into(), vec![])
                    }
                    _ => (
                        "/etc/httpd",
                        "/etc/httpd/conf/httpd.conf".into(),
                        vec!["/etc/httpd/conf.d/{domain}".into()],
                    ),
                };
                let sites = format!("{base}/flamepanel/sites");
                Self {
                    engine,
                    hook: Hook::Append {
                        directive: format!("IncludeOptional {sites}/*.conf"),
                    },
                    sites_dir: sites,
                    ext: "conf",
                    main_config: main,
                    legacy,
                }
            }
            WebServerEngine::Caddy => {
                let sites = "/etc/caddy/flamepanel/sites".to_string();
                Self {
                    engine,
                    hook: Hook::Append {
                        directive: format!("import {sites}/*.caddy"),
                    },
                    sites_dir: sites,
                    ext: "caddy",
                    main_config: "/etc/caddy/Caddyfile".into(),
                    legacy: vec!["/etc/caddy/{domain}".into()],
                }
            }
            WebServerEngine::OpenLiteSpeed => Self {
                engine,
                hook: Hook::OlsRegistry,
                sites_dir: "/usr/local/lsws/conf/vhosts".into(),
                ext: "conf",
                main_config: "/usr/local/lsws/conf/httpd_config.conf".into(),
                // 旧实现把 virtualHost 块写成 vhosts/<domain> 文件（与新目录同名，须先清理）
                legacy: vec!["/usr/local/lsws/conf/vhosts/{domain}".into()],
            },
        }
    }

    /// 启用态站点文件路径。
    pub fn site_path(&self, domain: &str) -> String {
        match self.engine {
            WebServerEngine::OpenLiteSpeed => format!("{}/{domain}/vhconf.conf", self.sites_dir),
            _ => format!("{}/{domain}.{}", self.sites_dir, self.ext),
        }
    }

    /// 面板自有 WAF 目录 `<引擎配置根>/flamepanel/waf`（与站点目录同级；OLS 站点目录是
    /// vhosts 根，WAF 目录固定在 `conf/flamepanel/waf`）。
    pub fn waf_dir(&self) -> String {
        match self.engine {
            WebServerEngine::OpenLiteSpeed => "/usr/local/lsws/conf/flamepanel/waf".into(),
            _ => match self.sites_dir.strip_suffix("/sites") {
                Some(base) => format!("{base}/waf"),
                None => format!("{}/waf", self.sites_dir),
            },
        }
    }

    /// 停用态站点文件路径。
    pub fn disabled_path(&self, domain: &str) -> String {
        format!("{}{DISABLED_SUFFIX}", self.site_path(domain))
    }

    /// 旧版本位置（需清理）。
    pub fn legacy_paths(&self, domain: &str) -> Vec<String> {
        self.legacy
            .iter()
            .map(|t| t.replace("{domain}", domain))
            .collect()
    }
}

/// 钩子插入失败原因。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HookError {
    /// 主配置中找不到可插入的 `http {` 行。
    NoHttpBlock,
}

impl std::fmt::Display for HookError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoHttpBlock => write!(
                f,
                "no standalone `http {{` line found in main config; add the flamepanel include manually"
            ),
        }
    }
}

/// 去掉行尾 `#` 注释与首尾空白，用于指令比对。
fn directive_part(line: &str) -> &str {
    line.split('#').next().unwrap_or("").trim()
}

fn has_directive(main: &str, directive: &str) -> bool {
    let want = directive.trim();
    main.lines().any(|l| directive_part(l) == want)
}

/// 把钩子应用到主配置文本。已存在返回 `Ok(None)`（幂等），否则返回新文本。
///
/// `OlsRegistry` 恒返回 `Ok(None)`（OLS 走逐站点注册）。
pub fn apply_hook(main: &str, hook: &Hook) -> Result<Option<String>, HookError> {
    match hook {
        Hook::OlsRegistry => Ok(None),
        Hook::Append { directive } => {
            if has_directive(main, directive) {
                return Ok(None);
            }
            let mut out = main.to_string();
            if !out.is_empty() && !out.ends_with('\n') {
                out.push('\n');
            }
            // Apache 不允许行尾注释：标记独占一行
            out.push_str(&format!("\n{HOOK_MARKER}\n{directive}\n"));
            Ok(Some(out))
        }
        Hook::HttpBlockInclude { directive } => {
            if has_directive(main, directive) {
                return Ok(None);
            }
            let mut out = String::with_capacity(main.len() + directive.len() + 80);
            let mut inserted = false;
            for line in main.split_inclusive('\n') {
                out.push_str(line);
                if !inserted && is_http_open_line(line) {
                    if !line.ends_with('\n') {
                        out.push('\n');
                    }
                    out.push_str(&format!("    {HOOK_MARKER}\n    {directive}\n"));
                    inserted = true;
                }
            }
            if inserted {
                Ok(Some(out))
            } else {
                Err(HookError::NoHttpBlock)
            }
        }
    }
}

/// 独立的 `http {` 行（允许行尾注释 / 空白差异；`http{` 亦可）。
fn is_http_open_line(line: &str) -> bool {
    let d = directive_part(line);
    let Some(rest) = d.strip_prefix("http") else {
        return false;
    };
    rest.trim() == "{"
}

// ─────────────────────────── OpenLiteSpeed 注册 ───────────────────────────

fn ols_begin(domain: &str) -> String {
    format!("# >>> flamepanel:vhost {domain}")
}
fn ols_end(domain: &str) -> String {
    format!("# <<< flamepanel:vhost {domain}")
}
fn ols_map_marker(domain: &str) -> String {
    format!("# flamepanel:map {domain}")
}

/// 面板管理的 OLS virtualHost 声明块（带起止标记）。
pub fn ols_vhost_block(domain: &str, root: &str) -> String {
    format!(
        "{begin}\nvirtualHost {domain} {{\n  vhRoot                  {root}\n  configFile              $SERVER_ROOT/conf/vhosts/{domain}/vhconf.conf\n  allowSymbolLink         1\n  enableScript            1\n  restrained              1\n}}\n{end}\n",
        begin = ols_begin(domain),
        end = ols_end(domain),
    )
}

/// OLS 注册失败原因。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OlsError {
    /// 已存在同名但非面板管理的 virtualHost（例如在 WebAdmin 手工创建）。
    UnmanagedVhostExists,
    /// 没有任何非 SSL listener 可挂载 map。
    NoListener,
}

impl std::fmt::Display for OlsError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::UnmanagedVhostExists => write!(
                f,
                "a virtualHost with the same name exists and is not managed by Flamepanel; remove it in OLS WebAdmin first"
            ),
            Self::NoListener => write!(
                f,
                "no non-SSL listener found in httpd_config.conf; create one in OLS WebAdmin first"
            ),
        }
    }
}

/// 非托管的同名 virtualHost 声明（关键字大小写不敏感）。
fn has_unmanaged_vhost(main: &str, domain: &str) -> bool {
    let begin = ols_begin(domain);
    let end = ols_end(domain);
    let mut in_managed = false;
    for line in main.lines() {
        let t = line.trim();
        if t == begin {
            in_managed = true;
            continue;
        }
        if t == end {
            in_managed = false;
            continue;
        }
        if in_managed {
            continue;
        }
        let mut it = directive_part(line).split_whitespace();
        if let (Some(kw), Some(name)) = (it.next(), it.next()) {
            if kw.eq_ignore_ascii_case("virtualhost") && name == domain {
                return true;
            }
        }
    }
    false
}

/// 该行是否为指向 `domain` 虚拟主机的 map 行。
fn is_map_for(line: &str, domain: &str) -> bool {
    let mut it = directive_part(line).split_whitespace();
    matches!((it.next(), it.next()), (Some(kw), Some(vh)) if kw.eq_ignore_ascii_case("map") && vh == domain)
}

/// 移除面板管理的 vhost 块与 map 行（未注册时原样返回）。
pub fn ols_unregister(main: &str, domain: &str) -> String {
    let begin = ols_begin(domain);
    let end = ols_end(domain);
    let marker = ols_map_marker(domain);
    let mut out = String::with_capacity(main.len());
    let mut skipping = false;
    let mut skip_next_map = false;
    for line in main.split_inclusive('\n') {
        let t = line.trim();
        if skipping {
            if t == end {
                skipping = false;
            }
            continue;
        }
        if t == begin {
            skipping = true;
            continue;
        }
        if t == marker {
            skip_next_map = true;
            continue;
        }
        if skip_next_map && is_map_for(line, domain) {
            skip_next_map = false;
            continue;
        }
        skip_next_map = false;
        out.push_str(line);
    }
    // 注册时在块前补过空行：移除后把尾部多余空行收敛为一个换行，保证注册 / 注销可逆
    if out.len() != main.len() {
        let body = out.trim_end_matches('\n').len();
        out.truncate(body);
        if body > 0 {
            out.push('\n');
        }
    }
    out
}

/// 注册（或刷新）面板管理的 vhost：替换 / 追加 virtualHost 块，并向每个非 SSL listener
/// 追加 `map <domain> <domain>`（已存在则跳过）。结果幂等。
pub fn ols_register(main: &str, domain: &str, root: &str) -> Result<String, OlsError> {
    if has_unmanaged_vhost(main, domain) {
        return Err(OlsError::UnmanagedVhostExists);
    }
    let cleaned = ols_unregister(main, domain);

    // 逐行扫描 listener 块，在其闭合 `}` 前插入 map
    let lines: Vec<&str> = cleaned.split_inclusive('\n').collect();
    let mut out = String::with_capacity(cleaned.len() + 512);
    let mut mapped = 0usize;
    let mut i = 0;
    while i < lines.len() {
        let line = lines[i];
        let mut it = directive_part(line).split_whitespace();
        let is_listener_open = matches!(it.next(), Some(kw) if kw.eq_ignore_ascii_case("listener"))
            && directive_part(line).ends_with('{');
        if !is_listener_open {
            out.push_str(line);
            i += 1;
            continue;
        }
        // 收集整个 listener 块（按花括号深度）
        let start = i;
        let mut depth = 0i32;
        let mut end = i;
        for (j, l) in lines.iter().enumerate().skip(i) {
            let d = directive_part(l);
            depth += d.matches('{').count() as i32;
            depth -= d.matches('}').count() as i32;
            if depth <= 0 {
                end = j;
                break;
            }
            end = j;
        }
        let block = &lines[start..=end];
        let secure = block.iter().any(|l| {
            let mut it = directive_part(l).split_whitespace();
            matches!((it.next(), it.next()), (Some("secure"), Some("1")))
        });
        for (k, l) in block.iter().enumerate() {
            if k == block.len() - 1 && !secure && depth <= 0 {
                out.push_str(&format!(
                    "  {}\n  map                     {domain} {domain}\n",
                    ols_map_marker(domain)
                ));
                mapped += 1;
            }
            out.push_str(l);
        }
        i = end + 1;
    }
    if mapped == 0 {
        return Err(OlsError::NoListener);
    }
    let body = out.trim_end_matches('\n').len();
    out.truncate(body);
    out.push_str("\n\n");
    out.push_str(&ols_vhost_block(domain, root));
    Ok(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    const NGINX_MAIN: &str = "user www-data;\nevents {\n    worker_connections 1024;\n}\n\nhttp {\n    include /etc/nginx/conf.d/*.conf;\n}\n";

    #[test]
    fn nginx_hook_inserted_after_http_line_and_idempotent() {
        let l = SiteLayout::for_engine(WebServerEngine::Nginx, &DistroType::Ubuntu);
        let out = apply_hook(NGINX_MAIN, &l.hook).unwrap().unwrap();
        let http_idx = out.find("http {").unwrap();
        let inc_idx = out
            .find("include /etc/nginx/flamepanel/sites/*.conf;")
            .unwrap();
        assert!(inc_idx > http_idx);
        assert!(inc_idx < out.find("conf.d").unwrap());
        // 幂等
        assert_eq!(apply_hook(&out, &l.hook).unwrap(), None);
    }

    #[test]
    fn nginx_hook_accepts_http_brace_variants() {
        let l = SiteLayout::for_engine(WebServerEngine::Nginx, &DistroType::Ubuntu);
        for main in ["http{\n}\n", "  http   {   # main\n}\n", "http {"] {
            let out = apply_hook(main, &l.hook).unwrap().unwrap();
            assert!(
                out.contains("include /etc/nginx/flamepanel/sites/*.conf;"),
                "{main:?}"
            );
        }
    }

    #[test]
    fn nginx_hook_errors_without_http_block() {
        let l = SiteLayout::for_engine(WebServerEngine::Nginx, &DistroType::Ubuntu);
        assert_eq!(
            apply_hook("events {}\nhttpd {\n}\n", &l.hook),
            Err(HookError::NoHttpBlock)
        );
        // 注释中的 http { 不算
        assert_eq!(
            apply_hook("# http {\n", &l.hook),
            Err(HookError::NoHttpBlock)
        );
    }

    #[test]
    fn existing_directive_detected_even_with_trailing_comment() {
        let l = SiteLayout::for_engine(WebServerEngine::Nginx, &DistroType::Ubuntu);
        let main = "http {\n  include /etc/nginx/flamepanel/sites/*.conf; # mine\n}\n";
        assert_eq!(apply_hook(main, &l.hook).unwrap(), None);
    }

    #[test]
    fn apache_append_marker_on_own_line() {
        let l = SiteLayout::for_engine(WebServerEngine::Apache, &DistroType::Debian);
        assert_eq!(l.main_config, "/etc/apache2/apache2.conf");
        let out = apply_hook("ServerRoot /etc/apache2", &l.hook)
            .unwrap()
            .unwrap();
        assert!(out.ends_with(&format!(
            "{HOOK_MARKER}\nIncludeOptional /etc/apache2/flamepanel/sites/*.conf\n"
        )));
        assert!(out.starts_with("ServerRoot /etc/apache2\n"));
        assert_eq!(apply_hook(&out, &l.hook).unwrap(), None);
    }

    #[test]
    fn apache_layout_per_distro() {
        let rhel = SiteLayout::for_engine(WebServerEngine::Apache, &DistroType::RHEL);
        assert_eq!(rhel.main_config, "/etc/httpd/conf/httpd.conf");
        assert_eq!(
            rhel.site_path("a.test"),
            "/etc/httpd/flamepanel/sites/a.test.conf"
        );
        assert_eq!(
            rhel.legacy_paths("a.test"),
            vec!["/etc/httpd/conf.d/a.test"]
        );
        let alp = SiteLayout::for_engine(WebServerEngine::Apache, &DistroType::Alpine);
        assert_eq!(alp.main_config, "/etc/apache2/httpd.conf");
    }

    #[test]
    fn caddy_layout_and_import() {
        let l = SiteLayout::for_engine(WebServerEngine::Caddy, &DistroType::Ubuntu);
        assert_eq!(
            l.site_path("a.test"),
            "/etc/caddy/flamepanel/sites/a.test.caddy"
        );
        assert_eq!(
            l.disabled_path("a.test"),
            "/etc/caddy/flamepanel/sites/a.test.caddy.disabled"
        );
        let out = apply_hook("{\n}\n", &l.hook).unwrap().unwrap();
        assert!(out.contains("\nimport /etc/caddy/flamepanel/sites/*.caddy\n"));
    }

    #[test]
    fn openresty_paths() {
        let l = SiteLayout::for_engine(WebServerEngine::OpenResty, &DistroType::Ubuntu);
        assert_eq!(l.main_config, "/usr/local/openresty/nginx/conf/nginx.conf");
        assert_eq!(
            l.site_path("a.test"),
            "/usr/local/openresty/nginx/conf/flamepanel/sites/a.test.conf"
        );
    }

    const OLS_MAIN: &str = "serverName x\n\nlistener Default {\n  address                 *:8088\n  secure                  0\n  map                     Example *\n}\n\nlistener SSL {\n  address                 *:443\n  secure                  1\n  map                     Example *\n}\n";

    #[test]
    fn ols_register_adds_block_and_map_to_non_ssl_listeners_only() {
        let out = ols_register(OLS_MAIN, "a.test", "/www/a").unwrap();
        assert!(out.contains("virtualHost a.test {"));
        assert!(out.contains("vhRoot                  /www/a"));
        assert!(out.contains("$SERVER_ROOT/conf/vhosts/a.test/vhconf.conf"));
        assert_eq!(
            out.matches("map                     a.test a.test").count(),
            1
        );
        // map 位于 Default listener 内（SSL 前）
        let map_idx = out.find("map                     a.test").unwrap();
        assert!(map_idx < out.find("listener SSL").unwrap());
        assert!(map_idx > out.find("listener Default").unwrap());
        // 幂等：再次注册不重复
        let again = ols_register(&out, "a.test", "/www/a").unwrap();
        assert_eq!(again, out);
    }

    #[test]
    fn ols_register_refreshes_root() {
        let out = ols_register(OLS_MAIN, "a.test", "/www/a").unwrap();
        let out2 = ols_register(&out, "a.test", "/www/b").unwrap();
        assert!(out2.contains("/www/b"));
        assert!(!out2.contains("/www/a"));
        assert_eq!(out2.matches("virtualHost a.test").count(), 1);
    }

    #[test]
    fn ols_unregister_restores_original() {
        let out = ols_register(OLS_MAIN, "a.test", "/www/a").unwrap();
        let back = ols_unregister(&out, "a.test");
        assert!(!back.contains("a.test"));
        assert!(back.contains("map                     Example *"));
        // 另一站点不受影响
        let two = ols_register(&out, "b.test", "/www/b").unwrap();
        let only_b = ols_unregister(&two, "a.test");
        assert!(only_b.contains("virtualHost b.test"));
        assert!(!only_b.contains("virtualHost a.test"));
        assert!(!only_b.contains("map                     a.test"));
    }

    #[test]
    fn ols_rejects_unmanaged_vhost_and_missing_listener() {
        let main = format!("{OLS_MAIN}\nvirtualhost a.test {{\n  vhRoot /x\n}}\n");
        assert_eq!(
            ols_register(&main, "a.test", "/www/a"),
            Err(OlsError::UnmanagedVhostExists)
        );
        assert_eq!(
            ols_register("serverName x\n", "a.test", "/www/a"),
            Err(OlsError::NoListener)
        );
        let only_ssl = "listener SSL {\n  secure 1\n}\n";
        assert_eq!(
            ols_register(only_ssl, "a.test", "/r"),
            Err(OlsError::NoListener)
        );
    }

    #[test]
    fn ols_paths() {
        let l = SiteLayout::for_engine(WebServerEngine::OpenLiteSpeed, &DistroType::Ubuntu);
        assert_eq!(
            l.site_path("a.test"),
            "/usr/local/lsws/conf/vhosts/a.test/vhconf.conf"
        );
        assert_eq!(
            l.legacy_paths("a.test"),
            vec!["/usr/local/lsws/conf/vhosts/a.test"]
        );
        assert_eq!(apply_hook("anything", &l.hook).unwrap(), None);
    }
}
