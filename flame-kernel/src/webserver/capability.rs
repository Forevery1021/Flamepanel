//! Web 引擎配套能力（安装时"各取所需"，Doc/13 §5.4 阶段 3）。
//!
//! 五类引擎的运行时依赖/配套软件差异很大（PHP 集成方式、Lua、缓存等）。
//! 本模块为每种引擎声明**能力清单**（`WebServerCapability`），并提供：
//!
//! - [`capabilities_for`]：查询某引擎支持的配套能力（安装向导展示勾选项）
//! - [`companion_packages`]：按引擎 + 能力 + 发行版给出**推荐的系统包列表**
//! - [`post_install_hints`]：安装/启用配套后的引擎侧配置提示（PHP-FPM upstream、Lua 模块等）
//!
//! 设计原则：
//! - **纯元数据 + 无副作用**：只做声明与推荐，不直接执行命令；
//!   实际安装经统一 `PackageManager` 端口执行（application 层组装）。
//! - **发行版感知**：与 `WebServerEngine::*_for(distro)` 一致的发行版路由。
//! - **向后兼容**：不改变现有安装流程；新增的"配套能力"是可选的增量路径。

use crate::infrastructure::os::DistroType;
use crate::webserver::WebServerEngine;
use serde::{Deserialize, Serialize};

/// Web 引擎可选的配套能力（安装时"各取所需"）。
///
/// 每种能力对应一组**推荐系统包**与**引擎配置提示**；用户可按需勾选，
/// 面板批量安装并给出后续配置指引。
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WebServerCapability {
    /// PHP-FPM 运行时（PHP 动态语言支持）
    Php,
    /// Lua 模块（仅 OpenResty）
    Lua,
    /// 缓存（Redis 内存缓存）
    Cache,
    /// 反向代理扩展模块（Nginx 常用编译扩展：upstream_hash 等）
    ExtraModules,
    /// Let's Encrypt / 自动 HTTPS 配套（Caddy 内置，无需额外安装）
    AutoHttps,
}

impl WebServerCapability {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Php => "php",
            Self::Lua => "lua",
            Self::Cache => "cache",
            Self::ExtraModules => "extra_modules",
            Self::AutoHttps => "auto_https",
        }
    }

    /// 能力的中文描述（前端向导展示）。
    pub fn label_zh(&self) -> &'static str {
        match self {
            Self::Php => "PHP 运行时（PHP-FPM）",
            Self::Lua => "Lua 模块（OpenResty 动态脚本）",
            Self::Cache => "Redis 缓存",
            Self::ExtraModules => "常用扩展模块",
            Self::AutoHttps => "自动 HTTPS（Let's Encrypt）",
        }
    }

    /// 能力的英文描述。
    pub fn label_en(&self) -> &'static str {
        match self {
            Self::Php => "PHP runtime (PHP-FPM)",
            Self::Lua => "Lua module (OpenResty scripting)",
            Self::Cache => "Redis cache",
            Self::ExtraModules => "Extra modules",
            Self::AutoHttps => "Auto HTTPS (Let's Encrypt)",
        }
    }

    /// 该能力是否需要引擎已安装才生效（依赖引擎本体）。
    pub fn requires_engine(&self) -> bool {
        // AutoHttps 在 Caddy 场景随引擎一起安装，不额外要求已安装
        matches!(
            self,
            Self::Php | Self::Lua | Self::Cache | Self::ExtraModules
        )
    }
}

/// 引擎支持的能力清单（按引擎差异化）。
pub fn capabilities_for(engine: &WebServerEngine) -> Vec<WebServerCapability> {
    match engine {
        WebServerEngine::Nginx => vec![
            WebServerCapability::Php,
            WebServerCapability::Cache,
            WebServerCapability::ExtraModules,
            WebServerCapability::AutoHttps,
        ],
        WebServerEngine::Apache => vec![
            WebServerCapability::Php,
            WebServerCapability::Cache,
            WebServerCapability::AutoHttps,
        ],
        WebServerEngine::OpenLiteSpeed => vec![
            WebServerCapability::Php, // lsphp* 由官方仓库提供
            WebServerCapability::Cache,
        ],
        WebServerEngine::OpenResty => vec![
            WebServerCapability::Php,
            WebServerCapability::Lua,
            WebServerCapability::Cache,
        ],
        WebServerEngine::Caddy => vec![
            WebServerCapability::Php,
            WebServerCapability::Cache,
            WebServerCapability::AutoHttps,
        ],
    }
}

/// 引擎是否支持该能力（安装向导勾选校验用）。
pub fn supports_capability(engine: &WebServerEngine, cap: &WebServerCapability) -> bool {
    capabilities_for(engine).contains(cap)
}

/// 按 引擎 + 能力 + 发行版 返回推荐的系统包列表。
///
/// 与 `WebServerEngine::package_name_for` 一致，对发行版做差异化路由：
/// - Debian/Ubuntu：`php-fpm` / `lua-nginx-module` / `redis-server` / `libnginx-mod-http-*`
/// - RHEL/CentOS/Fedora：`php-fpm` / `lua-resty-*` / `redis` / `nginx-mod-http-*`
/// - Alpine：`php-fpm` / `lua-resty-*` / `redis` / `nginx-mod-*`
///
/// OpenLiteSpeed 的 PHP 走官方仓库 `lsphp*`（与发行版无关，包名固定）。
pub fn companion_packages(
    engine: &WebServerEngine,
    cap: &WebServerCapability,
    distro: &DistroType,
) -> Vec<String> {
    // OpenLiteSpeed：PHP 由 LiteSpeed 官方仓库提供 lsphp（lsphp81/lsphp82 等）
    if *engine == WebServerEngine::OpenLiteSpeed && *cap == WebServerCapability::Php {
        return vec!["lsphp82".to_string()];
    }
    match cap {
        WebServerCapability::Php => match distro {
            DistroType::Ubuntu | DistroType::Debian => vec![
                "php-fpm".to_string(),
                "php-cli".to_string(),
                "php-mysql".to_string(),
            ],
            DistroType::CentOS | DistroType::RHEL | DistroType::Fedora => {
                vec![
                    "php-fpm".to_string(),
                    "php-cli".to_string(),
                    "php-mysqlnd".to_string(),
                ]
            }
            DistroType::Alpine => vec!["php-fpm".to_string(), "php-cli".to_string()],
            DistroType::Unknown(_) => vec!["php-fpm".to_string(), "php-cli".to_string()],
        },
        WebServerCapability::Lua => match distro {
            DistroType::Ubuntu | DistroType::Debian => {
                vec!["libnginx-mod-http-lua".to_string()]
            }
            _ => vec!["lua-nginx-module".to_string()],
        },
        // Doc/33 D-3：与 RedisManager 共用发行版布局（此前 SUSE/Arch 误用 Debian 包名）
        WebServerCapability::Cache => {
            vec![crate::infrastructure::os::redis_layout(distro)
                .package
                .to_string()]
        }
        WebServerCapability::ExtraModules => match engine {
            WebServerEngine::Nginx => match distro {
                DistroType::Ubuntu | DistroType::Debian => {
                    vec![
                        "libnginx-mod-http-upstream-fair".to_string(),
                        "libnginx-mod-http-headers-more-filter".to_string(),
                    ]
                }
                _ => vec![
                    "nginx-mod-http-upstream-fair".to_string(),
                    "nginx-mod-http-headers-more-filter".to_string(),
                ],
            },
            WebServerEngine::Apache => match distro {
                DistroType::Ubuntu | DistroType::Debian => {
                    vec!["libapache2-mod-php".to_string()]
                }
                _ => vec!["mod_ssl".to_string()],
            },
            _ => vec![],
        },
        WebServerCapability::AutoHttps => {
            // Caddy 内置自动 HTTPS；Nginx/Apache 推荐 certbot 获取 Let's Encrypt 证书
            if *engine == WebServerEngine::Caddy {
                vec![]
            } else {
                match distro {
                    DistroType::Ubuntu | DistroType::Debian => vec!["certbot".to_string()],
                    _ => vec!["certbot".to_string()],
                }
            }
        }
    }
}

/// 安装/启用配套后，引擎侧的配置提示（展示给用户，指引后续手动/面板配置）。
pub fn post_install_hints(engine: &WebServerEngine, cap: &WebServerCapability) -> &'static str {
    match (engine, cap) {
        (WebServerEngine::Nginx, WebServerCapability::Php) => {
            "Nginx + PHP-FPM：站点 server 块需添加 fastcgi_pass unix:/run/php/php-fpm.sock; 或 127.0.0.1:9000;"
        }
        (WebServerEngine::Nginx, WebServerCapability::Cache) => {
            "Nginx 缓存：可启用 proxy_cache_path 或 fastcgi_cache_path 加速动态站点。"
        }
        (WebServerEngine::Nginx, WebServerCapability::ExtraModules) => {
            "Nginx 扩展模块：安装后需确认 /etc/nginx/modules-enabled 内符号链接已生成。"
        }
        (WebServerEngine::Nginx, WebServerCapability::AutoHttps) => {
            "certbot --nginx -d your.domain 即可自动签发并配置 HTTPS。"
        }
        (WebServerEngine::Apache, WebServerCapability::Php) => {
            "Apache + PHP：Debian 系已自动 a2enmod proxy_fcgi/setenvif，将 PHP 配置为 PHP-FPM 模式（mod_proxy_fcgi）。"
        }
        (WebServerEngine::Apache, WebServerCapability::Cache) => {
            "Apache 缓存：可启用 mod_cache + mod_cache_disk 缓存静态资源。"
        }
        (WebServerEngine::Apache, WebServerCapability::AutoHttps) => {
            "certbot --apache -d your.domain 即可自动签发并配置 HTTPS。"
        }
        (WebServerEngine::OpenLiteSpeed, WebServerCapability::Php) => {
            "OpenLiteSpeed：lsphp82 已安装，可在 LSWS 管理台为虚拟主机绑定 PHP 版本。"
        }
        (WebServerEngine::OpenLiteSpeed, WebServerCapability::Cache) => {
            "OpenLiteSpeed：内置 Cache 模块（LSCache），可在 vhost 开启。"
        }
        (WebServerEngine::OpenResty, WebServerCapability::Php) => {
            "OpenResty + PHP-FPM：通过 content_by_lua 或 proxy_pass 反向代理到 PHP-FPM。"
        }
        (WebServerEngine::OpenResty, WebServerCapability::Lua) => {
            "OpenResty Lua：在 nginx.conf 使用 content_by_lua_block / access_by_lua_block 编写动态逻辑。"
        }
        (WebServerEngine::OpenResty, WebServerCapability::Cache) => {
            "OpenResty：结合 lua-resty-redis + ngx.shared.DICT 实现高性能缓存。"
        }
        (WebServerEngine::Caddy, WebServerCapability::Php) => {
            "Caddy + PHP：使用 php_fastcgi 指令（frankenphp / php-fpm upstream）自动处理 PHP。"
        }
        (WebServerEngine::Caddy, WebServerCapability::Cache) => {
            "Caddy 缓存：可启用 @cache 路由 + reverse_proxy 缓存响应头。"
        }
        (WebServerEngine::Caddy, WebServerCapability::AutoHttps) => {
            "Caddy 内置 Let's Encrypt 自动 HTTPS，无需额外安装。"
        }
        _ => "",
    }
}

/// 引擎配套能力的聚合描述（API 响应：engine → capabilities → packages/hints）。
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EngineCapabilityInfo {
    pub engine: String,
    pub capabilities: Vec<CapabilityInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize, utoipa::ToSchema)]
pub struct CapabilityInfo {
    pub key: String,
    pub label_zh: String,
    pub label_en: String,
    /// 推荐系统包（按当前发行版计算；空表示引擎内置/无需安装）
    pub packages: Vec<String>,
    pub hint: String,
}

/// 组装某引擎的完整能力信息（含推荐包与配置提示）。
pub fn engine_capability_info(
    engine: &WebServerEngine,
    distro: &DistroType,
) -> EngineCapabilityInfo {
    let capabilities = capabilities_for(engine)
        .into_iter()
        .map(|cap| CapabilityInfo {
            key: cap.as_str().into(),
            label_zh: cap.label_zh().into(),
            label_en: cap.label_en().into(),
            packages: companion_packages(engine, &cap, distro),
            hint: post_install_hints(engine, &cap).into(),
        })
        .collect();
    EngineCapabilityInfo {
        engine: engine.as_str().into(),
        capabilities,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn capabilities_per_engine_are_differentiated() {
        // 每个引擎支持的能力集不同，体现"各取所需"
        assert!(supports_capability(
            &WebServerEngine::Nginx,
            &WebServerCapability::Php
        ));
        assert!(supports_capability(
            &WebServerEngine::OpenResty,
            &WebServerCapability::Lua
        ));
        assert!(!supports_capability(
            &WebServerEngine::Nginx,
            &WebServerCapability::Lua
        ));
        assert!(supports_capability(
            &WebServerEngine::Caddy,
            &WebServerCapability::AutoHttps
        ));
        // Apache 不支持 Lua
        assert!(!supports_capability(
            &WebServerEngine::Apache,
            &WebServerCapability::Lua
        ));
        // OLS 支持 PHP 但不支持 Lua/AutoHttps
        assert!(supports_capability(
            &WebServerEngine::OpenLiteSpeed,
            &WebServerCapability::Php
        ));
        assert!(!supports_capability(
            &WebServerEngine::OpenLiteSpeed,
            &WebServerCapability::Lua
        ));
    }

    #[test]
    fn companion_packages_route_by_distro() {
        let debian = DistroType::Debian;
        let rhel = DistroType::RHEL;
        let alpine = DistroType::Alpine;

        // PHP：Debian 系 php-fpm + php-mysql；RHEL 系 php-mysqlnd；Alpine php-fpm
        let deb_php =
            companion_packages(&WebServerEngine::Nginx, &WebServerCapability::Php, &debian);
        assert!(deb_php.contains(&"php-fpm".to_string()));
        assert!(deb_php.contains(&"php-mysql".to_string()));
        let rhel_php =
            companion_packages(&WebServerEngine::Nginx, &WebServerCapability::Php, &rhel);
        assert!(rhel_php.contains(&"php-mysqlnd".to_string()));
        let alpine_php =
            companion_packages(&WebServerEngine::Nginx, &WebServerCapability::Php, &alpine);
        assert!(alpine_php.contains(&"php-fpm".to_string()));
        assert!(!alpine_php.contains(&"php-mysql".to_string()));

        // 缓存：Debian redis-server；RHEL/Alpine redis
        assert!(companion_packages(
            &WebServerEngine::Nginx,
            &WebServerCapability::Cache,
            &debian
        )
        .contains(&"redis-server".to_string()));
        assert!(
            companion_packages(&WebServerEngine::Nginx, &WebServerCapability::Cache, &rhel)
                .contains(&"redis".to_string())
        );
        assert!(companion_packages(
            &WebServerEngine::Nginx,
            &WebServerCapability::Cache,
            &alpine
        )
        .contains(&"redis".to_string()));

        // Lua：Debian libnginx-mod-http-lua；其他 lua-nginx-module
        assert!(companion_packages(
            &WebServerEngine::OpenResty,
            &WebServerCapability::Lua,
            &debian
        )
        .contains(&"libnginx-mod-http-lua".to_string()));
        assert!(companion_packages(
            &WebServerEngine::OpenResty,
            &WebServerCapability::Lua,
            &rhel
        )
        .contains(&"lua-nginx-module".to_string()));
    }

    #[test]
    fn ols_php_uses_lsphp_from_official_repo() {
        let pkgs = companion_packages(
            &WebServerEngine::OpenLiteSpeed,
            &WebServerCapability::Php,
            &DistroType::Debian,
        );
        assert!(pkgs.contains(&"lsphp82".to_string()));
    }

    #[test]
    fn caddy_auto_https_needs_no_package() {
        let pkgs = companion_packages(
            &WebServerEngine::Caddy,
            &WebServerCapability::AutoHttps,
            &DistroType::Debian,
        );
        assert!(pkgs.is_empty());
    }

    #[test]
    fn engine_capability_info_aggregates() {
        let info = engine_capability_info(&WebServerEngine::Nginx, &DistroType::Debian);
        assert_eq!(info.engine, "nginx");
        let keys: Vec<&str> = info.capabilities.iter().map(|c| c.key.as_str()).collect();
        assert!(keys.contains(&"php"));
        assert!(keys.contains(&"cache"));
        assert!(keys.contains(&"auto_https"));
        assert!(!keys.contains(&"lua"));
        // 每个能力都有中文标签与提示
        for cap in &info.capabilities {
            assert!(!cap.label_zh.is_empty());
            assert!(!cap.hint.is_empty() || cap.key == "auto_https");
        }
    }
}
