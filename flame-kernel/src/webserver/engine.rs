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
    pub fn from_str(s: &str) -> Option<Self> {
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
        match self {
            Self::Caddy => false,
            _ => true,
        }
    }

    pub fn config_test_command(&self) -> &'static str {
        match self {
            Self::Nginx => "nginx -t",
            Self::Apache => "httpd -t",
            Self::OpenLiteSpeed => "/usr/local/lsws/bin/lswsctrl configtest",
            Self::OpenResty => "openresty -t",
            Self::Caddy => "caddy validate",
        }
    }

    pub fn reload_command(&self) -> &'static str {
        match self {
            Self::Nginx => "nginx -s reload",
            Self::Apache => "httpd -k graceful",
            Self::OpenLiteSpeed => "/usr/local/lsws/bin/lswsctrl reload",
            Self::OpenResty => "openresty -s reload",
            Self::Caddy => "caddy reload",
        }
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
