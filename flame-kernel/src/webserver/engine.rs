use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum WebServerEngine {
    Nginx,
    Apache,
    OpenLiteSpeed,
    OpenResty,
    Caddy,
}

impl WebServerEngine {
    pub fn from_name(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "nginx" => Some(Self::Nginx),
            "apache" => Some(Self::Apache),
            "openlitespeed" | "ols" => Some(Self::OpenLiteSpeed),
            "openresty" => Some(Self::OpenResty),
            "caddy" => Some(Self::Caddy),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Nginx => "nginx",
            Self::Apache => "apache",
            Self::OpenLiteSpeed => "openlitespeed",
            Self::OpenResty => "openresty",
            Self::Caddy => "caddy",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            Self::Nginx => "Nginx - 高性能 HTTP 和反向代理服务器，市场占有率最高",
            Self::Apache => "Apache HTTP Server - 最成熟的 Web 服务器，模块丰富",
            Self::OpenLiteSpeed => "OpenLiteSpeed - 轻量级高性能 Web 服务器，兼容 Apache Rewrite",
            Self::OpenResty => "OpenResty - 基于 Nginx + LuaJIT 的动态 Web 平台",
            Self::Caddy => "Caddy - 自动 HTTPS 的现代化 Web 服务器",
        }
    }

    pub fn binary_name(&self) -> &'static str {
        match self {
            Self::Nginx => "nginx",
            Self::Apache => "httpd",
            Self::OpenLiteSpeed => "lshttpd",
            Self::OpenResty => "openresty",
            Self::Caddy => "caddy",
        }
    }

    /// 原生安装时的系统包名（apt/yum/apk）
    pub fn package_name(&self) -> &'static str {
        match self {
            Self::Nginx => "nginx",
            Self::Apache => "httpd",
            Self::OpenLiteSpeed => "openlitespeed",
            Self::OpenResty => "openresty",
            Self::Caddy => "caddy",
        }
    }

    /// 原生安装后的 systemd 服务名
    pub fn service_name(&self) -> &'static str {
        match self {
            Self::Nginx => "nginx",
            Self::Apache => "httpd",
            Self::OpenLiteSpeed => "lsws",
            Self::OpenResty => "openresty",
            Self::Caddy => "caddy",
        }
    }

    pub fn default_config_path(&self) -> &'static str {
        match self {
            Self::Nginx => "/etc/nginx/nginx.conf",
            Self::Apache => "/etc/httpd/conf/httpd.conf",
            Self::OpenLiteSpeed => "/usr/local/lsws/conf/httpd_config.conf",
            Self::OpenResty => "/usr/local/openresty/nginx/conf/nginx.conf",
            Self::Caddy => "/etc/caddy/Caddyfile",
        }
    }

    pub fn sites_available_dir(&self) -> &'static str {
        match self {
            Self::Nginx => "/etc/nginx/sites-available",
            Self::Apache => "/etc/httpd/conf.d",
            Self::OpenLiteSpeed => "/usr/local/lsws/conf/vhosts",
            Self::OpenResty => "/usr/local/openresty/nginx/conf/sites-available",
            Self::Caddy => "/etc/caddy",
        }
    }

    pub fn sites_enabled_dir(&self) -> &'static str {
        match self {
            Self::Nginx => "/etc/nginx/sites-enabled",
            Self::Apache => "/etc/httpd/conf.d",
            Self::OpenLiteSpeed => "/usr/local/lsws/conf/vhosts",
            Self::OpenResty => "/usr/local/openresty/nginx/conf/sites-enabled",
            Self::Caddy => "/etc/caddy",
        }
    }

    pub fn default_port(&self) -> u16 {
        match self {
            Self::Nginx => 80,
            Self::Apache => 80,
            Self::OpenLiteSpeed => 8088,
            Self::OpenResty => 80,
            Self::Caddy => 80,
        }
    }

    pub fn default_ssl_port(&self) -> u16 {
        match self {
            Self::Nginx => 443,
            Self::Apache => 443,
            Self::OpenLiteSpeed => 443,
            Self::OpenResty => 443,
            Self::Caddy => 443,
        }
    }

    pub fn supports_ssl(&self) -> bool {
        true
    }

    pub fn supports_rewrite(&self) -> bool {
        true
    }

    pub fn supports_reverse_proxy(&self) -> bool {
        true
    }

    pub fn supports_load_balancing(&self) -> bool {
        !matches!(self, Self::Caddy)
    }

    /// 真实二进制探测路径（原生安装后）。
    ///
    /// 多数引擎二进制在 PATH 中可被 `which` 找到；OpenLiteSpeed 安装到
    /// `/usr/local/lsws/bin/lshttpd`，PATH 探测不到，需要候选路径回退。
    /// Apache 在 Debian 系为 `/usr/sbin/apache2`，RHEL 系为 `/usr/sbin/httpd`。
    pub fn binary_candidates(&self) -> &'static [&'static str] {
        match self {
            Self::Nginx => &["/usr/sbin/nginx", "/usr/bin/nginx"],
            Self::Apache => &["/usr/sbin/apache2", "/usr/sbin/httpd", "/usr/bin/httpd"],
            Self::OpenLiteSpeed => &[
                "/usr/local/lsws/bin/lshttpd",
                "/usr/local/openlitespeed/bin/lshttpd",
            ],
            Self::OpenResty => &["/usr/local/openresty/nginx/sbin/nginx"],
            Self::Caddy => &["/usr/bin/caddy"],
        }
    }

    /// 实例可登记的二进制（`web_server_instances.binary_path`）。
    ///
    /// `start` 会以特权执行器运行 `<binary> -c <config_path>`，而二者都来自用户可写的实例表——
    /// 不设限时 `binary_path=/usr/bin/python3` 即可把 `config_path` 当作代码执行。只放行：
    /// 裸程序名（PATH 解析）、[`Self::binary_candidates`]、或系统 bin 目录下与引擎同名的文件。
    pub fn binary_allowed(&self, path: &str) -> bool {
        const SYSTEM_BIN_DIRS: &[&str] = &[
            "/usr/sbin",
            "/usr/bin",
            "/usr/local/sbin",
            "/usr/local/bin",
            "/sbin",
            "/bin",
        ];
        let names: &[&str] = match self {
            Self::Nginx => &["nginx"],
            Self::Apache => &["apache2", "httpd"],
            Self::OpenLiteSpeed => &["lshttpd"],
            Self::OpenResty => &["openresty", "nginx"],
            Self::Caddy => &["caddy"],
        };
        if names.contains(&path) || self.binary_candidates().contains(&path) {
            return true;
        }
        match path.rsplit_once('/') {
            Some((dir, name)) => SYSTEM_BIN_DIRS.contains(&dir) && names.contains(&name),
            None => false,
        }
    }

    /// 版本探测命令（程序 + 参数）。
    ///
    /// OpenLiteSpeed 用 `-v` 时输出到 stderr 且 `lshttpd -v` 即可；统一在此声明，
    /// 避免调用方按引擎硬编码。
    pub fn version_command(&self) -> (&'static str, &'static [&'static str]) {
        match self {
            Self::Caddy => ("caddy", &["version"]),
            Self::OpenLiteSpeed => ("/usr/local/lsws/bin/lshttpd", &["-v"]),
            _ => (self.binary_name(), &["-v"]),
        }
    }

    /// OpenLiteSpeed 服务控制脚本（lswsctrl）：start/stop/restart/reload/status/configtest 统一入口。
    ///
    /// 返回 `None` 表示引擎不使用 lswsctrl（走 systemd / 直接命令）。
    pub fn control_script(&self) -> Option<&'static str> {
        match self {
            Self::OpenLiteSpeed => Some("/usr/local/lsws/bin/lswsctrl"),
            _ => None,
        }
    }

    /// Apache 探测用二进制名（Debian 系 `apache2`，RHEL 系 `httpd`）。
    ///
    /// `which` 探测按发行版取名；其余引擎直接复用 [`Self::binary_name`]。
    pub fn binary_name_for(&self, distro: &crate::infrastructure::os::DistroType) -> &'static str {
        match self {
            Self::Apache => match distro {
                crate::infrastructure::os::DistroType::Debian
                | crate::infrastructure::os::DistroType::Ubuntu => "apache2",
                _ => "httpd",
            },
            other => other.binary_name(),
        }
    }

    /// 进程名（pgrep/killall 使用）。
    ///
    /// 多数引擎进程名与二进制名一致；个别引擎的 CLI 包装名与真实服务进程名不同
    /// （如 OpenResty 的服务进程实际是 `nginx`），需在此声明差异。
    pub fn process_name(&self) -> &'static str {
        match self {
            Self::OpenResty => "nginx",
            other => other.binary_name(),
        }
    }

    /// 实例启停默认二进制路径。
    ///
    /// 实例未登记二进制路径时的兜底值；OpenLiteSpeed/OpenResty 官方包二进制
    /// 不在 PATH，统一落到绝对路径。
    pub fn default_binary_path(&self) -> &'static str {
        match self {
            Self::Nginx => "nginx",
            Self::Apache => "httpd",
            Self::OpenLiteSpeed => "/usr/local/lsws/bin/lshttpd",
            Self::OpenResty => "/usr/local/openresty/nginx/sbin/nginx",
            Self::Caddy => "caddy",
        }
    }

    // ── Apache 发行版感知（Debian 系 apache2 / RHEL 系 httpd）────────────────
    //
    // Apache 是唯一一个包名/服务名/配置布局在主流发行版间完全分叉的引擎：
    //   Debian/Ubuntu: 包 `apache2`、服务 `apache2`、二进制 `/usr/sbin/apache2`、
    //                  配置 `/etc/apache2/`、控制工具 `apache2ctl`
    //   RHEL/CentOS:   包 `httpd`、服务 `httpd`、二进制 `/usr/sbin/httpd`、
    //                  配置 `/etc/httpd/`、控制工具 `httpd`
    // 以下方法按发行版返回差异化元数据；无法识别时回退 RHEL 风格（`httpd`），
    // 与历史行为一致（旧版本对 Apache 硬编码 httpd）。

    /// Apache 原生包名（Debian 系 `apache2`，RHEL 系 `httpd`）。
    /// 仅 Apache 依赖发行版；其余引擎直接复用 [`Self::package_name`]。
    pub fn package_name_for(&self, distro: &crate::infrastructure::os::DistroType) -> &'static str {
        match self {
            Self::Apache => match distro {
                crate::infrastructure::os::DistroType::Debian
                | crate::infrastructure::os::DistroType::Ubuntu => "apache2",
                _ => "httpd",
            },
            other => other.package_name(),
        }
    }

    /// Apache systemd 服务名（Debian 系 `apache2`，RHEL 系 `httpd`）。
    pub fn service_name_for(&self, distro: &crate::infrastructure::os::DistroType) -> &'static str {
        match self {
            Self::Apache => match distro {
                crate::infrastructure::os::DistroType::Debian
                | crate::infrastructure::os::DistroType::Ubuntu => "apache2",
                _ => "httpd",
            },
            other => other.service_name(),
        }
    }

    /// Apache 配置主文件路径（Debian 系 `/etc/apache2/apache2.conf`，
    /// RHEL 系 `/etc/httpd/conf/httpd.conf`）。
    pub fn default_config_path_for(
        &self,
        distro: &crate::infrastructure::os::DistroType,
    ) -> &'static str {
        match self {
            Self::Apache => match distro {
                crate::infrastructure::os::DistroType::Debian
                | crate::infrastructure::os::DistroType::Ubuntu => "/etc/apache2/apache2.conf",
                _ => "/etc/httpd/conf/httpd.conf",
            },
            other => other.default_config_path(),
        }
    }

    /// Apache 站点配置目录（Debian 系 `/etc/apache2/sites-available` + symlink 机制，
    /// RHEL 系 `/etc/httpd/conf.d`）。
    pub fn sites_available_dir_for(
        &self,
        distro: &crate::infrastructure::os::DistroType,
    ) -> &'static str {
        match self {
            Self::Apache => match distro {
                crate::infrastructure::os::DistroType::Debian
                | crate::infrastructure::os::DistroType::Ubuntu => "/etc/apache2/sites-available",
                _ => "/etc/httpd/conf.d",
            },
            other => other.sites_available_dir(),
        }
    }

    /// Apache 站点启用目录（Debian 系 `/etc/apache2/sites-enabled`，
    /// RHEL 系 `/etc/httpd/conf.d`）。
    pub fn sites_enabled_dir_for(
        &self,
        distro: &crate::infrastructure::os::DistroType,
    ) -> &'static str {
        match self {
            Self::Apache => match distro {
                crate::infrastructure::os::DistroType::Debian
                | crate::infrastructure::os::DistroType::Ubuntu => "/etc/apache2/sites-enabled",
                _ => "/etc/httpd/conf.d",
            },
            other => other.sites_enabled_dir(),
        }
    }

    /// Apache 配置测试命令（Debian 系 `apache2ctl configtest`，RHEL 系 `httpd -t`）。
    pub fn config_test_command_for(
        &self,
        distro: &crate::infrastructure::os::DistroType,
    ) -> &'static str {
        match self {
            Self::Apache => match distro {
                crate::infrastructure::os::DistroType::Debian
                | crate::infrastructure::os::DistroType::Ubuntu => "apache2ctl configtest",
                _ => "httpd -t",
            },
            other => other.config_test_command(),
        }
    }

    /// Apache reload 命令（Debian 系 `apache2ctl graceful`，RHEL 系 `httpd -k graceful`）。
    pub fn reload_command_for(
        &self,
        distro: &crate::infrastructure::os::DistroType,
    ) -> &'static str {
        match self {
            Self::Apache => match distro {
                crate::infrastructure::os::DistroType::Debian
                | crate::infrastructure::os::DistroType::Ubuntu => "apache2ctl graceful",
                _ => "httpd -k graceful",
            },
            other => other.reload_command(),
        }
    }

    /// Apache 版本探测命令（Debian 系 `apache2 -v`，RHEL 系 `httpd -v`）。
    pub fn version_command_for(
        &self,
        distro: &crate::infrastructure::os::DistroType,
    ) -> (&'static str, &'static [&'static str]) {
        match self {
            Self::Apache => match distro {
                crate::infrastructure::os::DistroType::Debian
                | crate::infrastructure::os::DistroType::Ubuntu => ("apache2", &["-v"]),
                _ => ("httpd", &["-v"]),
            },
            other => other.version_command(),
        }
    }

    /// Apache 实例启停默认二进制（Debian 系 `/usr/sbin/apache2`，RHEL 系 `httpd`）。
    pub fn default_binary_path_for(
        &self,
        distro: &crate::infrastructure::os::DistroType,
    ) -> &'static str {
        match self {
            Self::Apache => match distro {
                crate::infrastructure::os::DistroType::Debian
                | crate::infrastructure::os::DistroType::Ubuntu => "/usr/sbin/apache2",
                _ => "httpd",
            },
            other => other.default_binary_path(),
        }
    }

    /// Apache 进程名（pgrep/killall 使用）：Debian 系主进程为 `apache2`，
    /// RHEL 系为 `httpd`。
    pub fn process_name_for(&self, distro: &crate::infrastructure::os::DistroType) -> &'static str {
        match self {
            Self::Apache => match distro {
                crate::infrastructure::os::DistroType::Debian
                | crate::infrastructure::os::DistroType::Ubuntu => "apache2",
                _ => "httpd",
            },
            other => other.process_name(),
        }
    }

    /// Apache 是否为 Debian 系布局（`apache2ctl` / `a2enmod` / sites-enabled 可用）。
    pub fn apache_debian_layout(&self, distro: &crate::infrastructure::os::DistroType) -> bool {
        matches!(
            (self, distro),
            (
                Self::Apache,
                crate::infrastructure::os::DistroType::Debian
                    | crate::infrastructure::os::DistroType::Ubuntu
            )
        )
    }

    pub fn config_test_command(&self) -> &'static str {
        match self {
            Self::Nginx => "nginx -t",
            Self::Apache => "httpd -t",
            Self::OpenLiteSpeed => "/usr/local/lsws/bin/lswsctrl configtest",
            Self::OpenResty => "openresty -t",
            // Caddy 校验必须显式带配置路径：`caddy validate --config /etc/caddy/Caddyfile`，
            // 否则默认从当前目录找 Caddyfile 会失败。
            Self::Caddy => "caddy validate --config /etc/caddy/Caddyfile",
        }
    }

    pub fn reload_command(&self) -> &'static str {
        match self {
            Self::Nginx => "nginx -s reload",
            Self::Apache => "httpd -k graceful",
            Self::OpenLiteSpeed => "/usr/local/lsws/bin/lswsctrl reload",
            Self::OpenResty => "openresty -s reload",
            // Caddy reload 走热加载（API），不带 --config 参数时 Caddy 2.8+ 会因
            // 缺少 --pid 或 --config 直接报错；这里显式指定配置路径。
            Self::Caddy => "caddy reload --config /etc/caddy/Caddyfile",
        }
    }

    /// 该引擎是否使用官方仓库安装（非发行版默认仓库）。
    ///
    /// OpenLiteSpeed / OpenResty / Caddy 不在多数发行版默认仓库（或版本陈旧），
    /// 安装需先写入官方 apt/yum 源；nginx/apache 保持系统包管理器安装。
    pub fn uses_official_repo(&self) -> bool {
        matches!(self, Self::OpenLiteSpeed | Self::OpenResty | Self::Caddy)
    }

    /// Caddy 是否支持 `caddy fmt`（Caddyfile 格式化辅助）。
    ///
    /// Caddy 用 `caddy fmt --overwrite /etc/caddy/Caddyfile` 统一格式；
    /// 其余引擎无等价官方工具，返回 false。
    pub fn supports_config_format(&self) -> bool {
        matches!(self, Self::Caddy)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum WebServerStatus {
    Running,
    Stopped,
    Error(String),
    Reloading,
}

impl WebServerStatus {
    pub fn as_str(&self) -> &str {
        match self {
            Self::Running => "running",
            Self::Stopped => "stopped",
            Self::Error(_) => "error",
            Self::Reloading => "reloading",
        }
    }
}
