use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AppManifest {
    pub key: String,
    pub name: String,
    pub category: String,
    pub description: String,
    pub version: String,
    pub default_port: i32,
    pub icon: String,
    /// Docker Compose YAML content
    pub compose: String,
}

/// Built-in app catalog — 8 popular applications
pub fn builtin_apps() -> Vec<AppManifest> {
    vec![
        // ─── CMS ──────────────────────────────────────────────────────────────
        AppManifest {
            key: "wordpress".into(),
            name: "WordPress".into(),
            category: "CMS / 博客".into(),
            description: "全球最流行的内容管理系统，适合搭建博客和企业网站".into(),
            version: "6.7".into(),
            default_port: 8081,
            icon: "📝".into(),
            compose: r#"version: '3'
services:
  wordpress:
    image: wordpress:6.7
    container_name: fp-wordpress-{name}
    ports:
      - "{port}:80"
    environment:
      WORDPRESS_DB_HOST: db
      WORDPRESS_DB_USER: wordpress
      WORDPRESS_DB_PASSWORD: wp_{name}_pass
      WORDPRESS_DB_NAME: wordpress
    volumes:
      - {data_dir}/wp-content:/var/www/html/wp-content
    restart: unless-stopped
    depends_on:
      - db
  db:
    image: mysql:8
    container_name: fp-wordpress-db-{name}
    environment:
      MYSQL_DATABASE: wordpress
      MYSQL_USER: wordpress
      MYSQL_PASSWORD: wp_{name}_pass
      MYSQL_ROOT_PASSWORD: root_{name}_pass
    volumes:
      - {data_dir}/mysql:/var/lib/mysql
    restart: unless-stopped
"#
            .into(),
        },

        // ─── DevOps ───────────────────────────────────────────────────────────
        AppManifest {
            key: "portainer".into(),
            name: "Portainer".into(),
            category: "DevOps / 工具".into(),
            description: "Docker 可视化容器管理界面，管理容器、镜像、网络和卷".into(),
            version: "2.21".into(),
            default_port: 9443,
            icon: "🐳".into(),
            compose: r#"version: '3'
services:
  portainer:
    image: portainer/portainer-ce:2.21.5
    container_name: fp-portainer-{name}
    ports:
      - "{port}:9000"
    volumes:
      - /var/run/docker.sock:/var/run/docker.sock
      - {data_dir}:/data
    restart: unless-stopped
"#
            .into(),
        },

        AppManifest {
            key: "gitea".into(),
            name: "Gitea".into(),
            category: "DevOps / 工具".into(),
            description: "轻量级自托管 Git 服务，类似 GitHub / GitLab".into(),
            version: "1.22".into(),
            default_port: 3000,
            icon: "🦊".into(),
            compose: r#"version: '3'
services:
  gitea:
    image: gitea/gitea:1.22
    container_name: fp-gitea-{name}
    ports:
      - "{port}:3000"
      - "{ssh_port}:22"
    environment:
      - USER_UID=1000
      - USER_GID=1000
    volumes:
      - {data_dir}:/data
      - /etc/timezone:/etc/timezone:ro
      - /etc/localtime:/etc/localtime:ro
    restart: unless-stopped
"#
            .into(),
        },

        // ─── 数据库管理 ──────────────────────────────────────────────────────
        AppManifest {
            key: "phpmyadmin".into(),
            name: "phpMyAdmin".into(),
            category: "数据库管理".into(),
            description: "MySQL/MariaDB 数据库 Web 管理工具".into(),
            version: "5.2".into(),
            default_port: 8082,
            icon: "🗄️".into(),
            compose: r#"version: '3'
services:
  phpmyadmin:
    image: phpmyadmin:5.2
    container_name: fp-pma-{name}
    ports:
      - "{port}:80"
    environment:
      PMA_ARBITRARY: "1"
      PMA_HOST: "{db_host}"
      PMA_PORT: "{db_port}"
    restart: unless-stopped
"#
            .into(),
        },

        // ─── 反向代理 / Web ──────────────────────────────────────────────────
        AppManifest {
            key: "nginx".into(),
            name: "Nginx".into(),
            category: "Web / 反向代理".into(),
            description: "高性能 HTTP 服务器和反向代理，带静态文件服务".into(),
            version: "1.27".into(),
            default_port: 8083,
            icon: "🌐".into(),
            compose: r#"version: '3'
services:
  nginx:
    image: nginx:1.27-alpine
    container_name: fp-nginx-{name}
    ports:
      - "{port}:80"
    volumes:
      - {data_dir}/html:/usr/share/nginx/html:ro
      - {data_dir}/nginx.conf:/etc/nginx/nginx.conf:ro
    restart: unless-stopped
"#
            .into(),
        },

        // ─── 开发环境 ────────────────────────────────────────────────────────
        AppManifest {
            key: "nodejs".into(),
            name: "Node.js 应用".into(),
            category: "开发环境".into(),
            description: "运行 Node.js 项目的容器化环境，支持 npm / yarn / pnpm".into(),
            version: "22".into(),
            default_port: 3000,
            icon: "💚".into(),
            compose: r#"version: '3'
services:
  node:
    image: node:22-alpine
    container_name: fp-node-{name}
    ports:
      - "{port}:{port}"
    working_dir: /app
    volumes:
      - {data_dir}/app:/app
    command: sh -c "npm install && node index.js"
    restart: unless-stopped
"#
            .into(),
        },

        // ─── 缓存 / 消息 ────────────────────────────────────────────────────
        AppManifest {
            key: "redis".into(),
            name: "Redis 缓存".into(),
            category: "缓存 / 消息队列".into(),
            description: "高性能内存键值存储，适用于缓存、会话存储和消息队列".into(),
            version: "7.4".into(),
            default_port: 6379,
            icon: "🔴".into(),
            compose: r#"version: '3'
services:
  redis:
    image: redis:7.4-alpine
    container_name: fp-redis-{name}
    ports:
      - "{port}:6379"
    volumes:
      - {data_dir}:/data
    command: redis-server --appendonly yes --requirepass app_{name}_pass
    restart: unless-stopped
"#
            .into(),
        },

        // ─── 监控 ────────────────────────────────────────────────────────────
        AppManifest {
            key: "uptime-kuma".into(),
            name: "Uptime Kuma".into(),
            category: "监控 / 告警".into(),
            description: "自托管网站和服务状态监控面板，支持多种通知渠道".into(),
            version: "1.23".into(),
            default_port: 8084,
            icon: "📡".into(),
            compose: r#"version: '3'
services:
  uptime-kuma:
    image: louislam/uptime-kuma:1
    container_name: fp-uptime-{name}
    ports:
      - "{port}:3001"
    volumes:
      - {data_dir}:/app/data
    restart: unless-stopped
"#
            .into(),
        },
    ]
}
