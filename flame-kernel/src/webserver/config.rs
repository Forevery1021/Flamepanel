use crate::domain::entity::Website;
use super::engine::WebServerEngine;

pub trait WebServerConfigGenerator: Send + Sync {
    fn engine(&self) -> WebServerEngine;
    fn generate_global_config(&self, port: u16, worker_processes: u32) -> String;
    fn generate_site_config(&self, site: &Website, ssl_cert: Option<&str>, ssl_key: Option<&str>) -> String;
    fn generate_reverse_proxy_config(&self, domain: &str, proxy_pass: &str, port: u16) -> String;
}

pub struct NginxConfig;
pub struct ApacheConfig;
pub struct OpenLiteSpeedConfig;
pub struct OpenRestyConfig;
pub struct CaddyConfig;

impl WebServerConfigGenerator for NginxConfig {
    fn engine(&self) -> WebServerEngine { WebServerEngine::Nginx }

    fn generate_global_config(&self, port: u16, worker_processes: u32) -> String {
        format!(r#"user www-data;
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

    server {{
        listen {port};
        server_name localhost;
        root /var/www/html;
    }}

    include /etc/nginx/conf.d/*.conf;
    include /etc/nginx/sites-enabled/*;
}}
"#, wp = worker_processes, port = port)
    }

    fn generate_site_config(&self, site: &Website, ssl_cert: Option<&str>, ssl_key: Option<&str>) -> String {
        let mut config = String::new();
        if let Some(cert) = ssl_cert {
            if let Some(key) = ssl_key {
                config.push_str(&format!(r#"server {{
    listen {sp} ssl http2;
    server_name {domain};
    root {root};
    index index.html index.htm index.php;

    ssl_certificate {cert};
    ssl_certificate_key {key};
    ssl_protocols TLSv1.2 TLSv1.3;
    ssl_ciphers HIGH:!aNULL:!MD5;

    location / {{
        try_files $uri $uri/ =404;
    }}
}}
"#, sp = 443, domain = site.domain, root = site.root_path, cert = cert, key = key));
            }
        }
        config.push_str(&format!(r#"server {{
    listen {port};
    server_name {domain};
    root {root};
    index index.html index.htm index.php;

    location / {{
        try_files $uri $uri/ =404;
    }}
}}
"#, port = 80, domain = site.domain, root = site.root_path));
        config
    }

    fn generate_reverse_proxy_config(&self, domain: &str, proxy_pass: &str, port: u16) -> String {
        format!(r#"server {{
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
"#, port = port, domain = domain, proxy = proxy_pass)
    }
}

impl WebServerConfigGenerator for ApacheConfig {
    fn engine(&self) -> WebServerEngine { WebServerEngine::Apache }

    fn generate_global_config(&self, port: u16, worker_processes: u32) -> String {
        format!(r#"ServerRoot "/etc/httpd"
Listen {port}
User apache
Group apache
ServerAdmin admin@localhost
ServerTokens Prod
Include conf.modules.d/*.conf
Include conf.d/*.conf

<IfModule mpm_prefork_module>
    StartServers {wp}
    MinSpareServers 5
    MaxSpareServers 20
    MaxRequestWorkers 256
    MaxConnectionsPerChild 10000
</IfModule>
"#, port = port, wp = worker_processes)
    }

    fn generate_site_config(&self, site: &Website, ssl_cert: Option<&str>, ssl_key: Option<&str>) -> String {
        let mut config = String::new();
        if let Some(cert) = ssl_cert {
            if let Some(key) = ssl_key {
                config.push_str(&format!(r#"<VirtualHost *:{sp}>
    ServerName {domain}
    DocumentRoot {root}
    SSLEngine on
    SSLCertificateFile {cert}
    SSLCertificateKeyFile {key}
    SSLProtocol all -SSLv3 -TLSv1 -TLSv1.1
    SSLCipherSuite HIGH:!aNULL:!MD5

    <Directory {root}>
        Options Indexes FollowSymLinks
        AllowOverride All
        Require all granted
    </Directory>
</VirtualHost>
"#, sp = 443, domain = site.domain, root = site.root_path, cert = cert, key = key));
            }
        }
        config.push_str(&format!(r#"<VirtualHost *:{port}>
    ServerName {domain}
    DocumentRoot {root}

    <Directory {root}>
        Options Indexes FollowSymLinks
        AllowOverride All
        Require all granted
    </Directory>
</VirtualHost>
"#, port = 80, domain = site.domain, root = site.root_path));
        config
    }

    fn generate_reverse_proxy_config(&self, domain: &str, proxy_pass: &str, port: u16) -> String {
        format!(r#"<VirtualHost *:{port}>
    ServerName {domain}

    ProxyPreserveHost On
    ProxyPass / {proxy}/
    ProxyPassReverse / {proxy}/
</VirtualHost>
"#, port = port, domain = domain, proxy = proxy_pass)
    }
}

impl WebServerConfigGenerator for OpenLiteSpeedConfig {
    fn engine(&self) -> WebServerEngine { WebServerEngine::OpenLiteSpeed }

    fn generate_global_config(&self, port: u16, _worker_processes: u32) -> String {
        format!(r#"listener "HTTP" {{
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
"#, port = port)
    }

    fn generate_site_config(&self, site: &Website, _ssl_cert: Option<&str>, _ssl_key: Option<&str>) -> String {
        format!(r#"virtualHost {domain} {{
    vhRoot {root}
    configFile $SERVER_ROOT/conf/vhosts/{domain}/vhconf.conf
    allowSymbolLink 1
    enableScript 1
    restrained 0
    maxConns 100
    pcookieExpire 0
    adminEmails admin@localhost
}}
"#, domain = site.domain, root = site.root_path)
    }

    fn generate_reverse_proxy_config(&self, domain: &str, proxy_pass: &str, _port: u16) -> String {
        format!(r#"virtualHost {domain} {{
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
"#, domain = domain, proxy = proxy_pass)
    }
}

impl WebServerConfigGenerator for OpenRestyConfig {
    fn engine(&self) -> WebServerEngine { WebServerEngine::OpenResty }

    fn generate_global_config(&self, port: u16, worker_processes: u32) -> String {
        format!(r#"user www-data;
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
}}
"#, wp = worker_processes, port = port)
    }

    fn generate_site_config(&self, site: &Website, ssl_cert: Option<&str>, ssl_key: Option<&str>) -> String {
        let mut config = String::new();
        if let Some(cert) = ssl_cert {
            if let Some(key) = ssl_key {
                config.push_str(&format!(r#"server {{
    listen 443 ssl http2;
    server_name {domain};
    root {root};

    ssl_certificate {cert};
    ssl_certificate_key {key};
    ssl_protocols TLSv1.2 TLSv1.3;
    ssl_ciphers HIGH:!aNULL:!MD5;

    location / {{
        default_type text/html;
        content_by_lua_block {{
            ngx.say("Hello from OpenResty - {domain}")
        }}
    }}
}}
"#, domain = site.domain, root = site.root_path, cert = cert, key = key));
            }
        }
        config.push_str(&format!(r#"server {{
    listen 80;
    server_name {domain};
    root {root};

    location / {{
        try_files $uri $uri/ /index.html;
    }}
}}
"#, domain = site.domain, root = site.root_path));
        config
    }

    fn generate_reverse_proxy_config(&self, domain: &str, proxy_pass: &str, port: u16) -> String {
        format!(r#"server {{
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
"#, port = port, domain = domain, proxy = proxy_pass)
    }
}

impl WebServerConfigGenerator for CaddyConfig {
    fn engine(&self) -> WebServerEngine { WebServerEngine::Caddy }

    fn generate_global_config(&self, _port: u16, _worker_processes: u32) -> String {
        r#"{
    admin off
    ocsp_stapling on
}

*.localhost, localhost {
    respond "Caddy is running"
}
"#.to_string()
    }

    fn generate_site_config(&self, site: &Website, _ssl_cert: Option<&str>, _ssl_key: Option<&str>) -> String {
        format!(r#"{domain} {{
    root * {root}
    encode gzip
    file_server browse

    header {{
        -Server
        X-Content-Type-Options "nosniff"
        X-Frame-Options "DENY"
    }}
}}

http://{domain} {{
    redir https://{{host}}{{uri}} 301
}}
"#, domain = site.domain, root = site.root_path)
    }

    fn generate_reverse_proxy_config(&self, domain: &str, proxy_pass: &str, _port: u16) -> String {
        format!(r#"{domain} {{
    reverse_proxy {proxy}
}}
"#, domain = domain, proxy = proxy_pass)
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
