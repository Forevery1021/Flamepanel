# FlamePanel

> 基于 Rust + Vue 3 的服务器运维管理面板

![Rust](https://img.shields.io/badge/Rust-1.85-orange)
![Vue](https://img.shields.io/badge/Vue-3.5-brightgreen)
![TypeScript](https://img.shields.io/badge/TypeScript-6.0-blue)
![License](https://img.shields.io/badge/license-MIT-blue)

**FlamePanel** 是一个现代化、高性能、自托管的服务器运维面板。后端采用 Rust + Axum 六边形架构，前端使用 Vue 3 + Element Plus，支持 JWT + RBAC 权限体系。

## 核心特性

- **系统监控** — WebSocket 实时推送 CPU/内存/磁盘/负载，ECharts 趋势图表
- **Docker 管理** — 容器/镜像/Compose 全生命周期管理（13 端点）
- **Web 服务器引擎** — 原生支持 Nginx/Apache/OpenLiteSpeed/OpenResty/Caddy，自动生成配置，进程管理（16 端点）
- **数据库管理** — MySQL/MariaDB/Redis 原生安装（apt/yum/apk），数据库/用户 CRUD，服务启停（15 端点）
- **文件管理** — Web 端浏览/编辑/上传/下载/重命名/权限（10 端点）
- **防火墙管理** — ufw/firewalld/iptables 自动检测，规则 CRUD/应用/开关（11 端点）
- **Web 终端** — xterm.js + WebSocket，浏览器内直接连接服务器 Shell
- **WASM 插件系统** — wasmtime 沙箱，生命周期钩子/指标追踪/热重载/依赖校验
- **面板配置** — Key-Value 配置，主题/多语言/端口/日志/2FA/JWT 密钥轮换
- **用户 & RBAC** — JWT + bcrypt，admin/operator/viewer 三角色，60+ 路由权限映射
- **审计日志 & 系统日志** — REST + WebSocket 双通道
- **弹性与容错** — Circuit Breaker + Retry，Docker 不可用时 InMemory 自动降级

## 技术栈

| 层面 | 技术 |
|------|------|
| 后端架构 | Clean Architecture + Hexagonal (domain → application → infrastructure → api) |
| 后端框架 | Rust + Axum 0.6 |
| 数据库 | SQLite (sqlx 0.9) + InMemory 双模式 |
| 认证 | jsonwebtoken 9 + bcrypt |
| WASM | wasmtime 29 |
| 前端 | Vue 3.5 + TypeScript 6.0 + Element Plus + Vite 8 |
| 状态/路由 | Pinia 3 + Vue Router 5 |
| 终端 | xterm.js 5.5 + @xterm/addon-fit |

## 项目结构

```
Flamepanel/
├── flame-kernel/          # Rust 核心后端
│   ├── src/
│   │   ├── domain/        # 实体 (User/Node/Website/Plugin/DatabaseInstance/FirewallRule/…)
│   │   ├── application/   # 服务层 (UserService/DockerService/FileService/FirewallService/…)
│   │   ├── infrastructure/# 仓库实现 (InMemory + SQLite + Bollard + OS 抽象)
│   │   ├── api/           # HTTP 层 (15 个 handler 模块, 86+ 路由, JWT+RBAC 中间件)
│   │   ├── plugin/        # WASM 沙箱 + 注册表
│   │   ├── webserver/     # 5 引擎配置生成 + 进程管理
│   │   ├── database/      # MySQL/Redis 原生管理
│   │   ├── firewall/      # 防火墙管理器 (ufw/firewalld/iptables)
│   │   ├── terminal/      # Web 终端 (bash 子进程管道)
│   │   ├── event/         # 事件总线
│   │   ├── utils/         # JWT/bcrypt/验证
│   │   └── resilience/    # Circuit Breaker + Retry
│   └── tests/             # 74 集成测试 + 11 单元测试
├── frontend/              # Vue 3 前端 (15 个视图)
├── agent/                 # 轻量 Rust Agent
├── docker-compose.yml
└── install.sh
```

## 环境要求

| 组件 | 版本 | 用途 |
|------|------|------|
| Rust | 1.85+ | 编译后端 |
| Node.js | 20+ | 构建前端 |
| Docker (可选) | 任意 | 容器化部署 |

## 快速开发

```bash
# 终端 1：启动后端（端口 8080）
cd flame-kernel
cargo run

# 终端 2：启动前端（端口 5173，自动代理 /api/* -> :8080）
cd frontend
npm install
npm run dev
```

访问 `http://localhost:5173`，默认账号 `admin` / `admin123`。

## 生产部署

### 方式一：一键安装脚本（推荐 Linux）

```bash
# 交互式安装
sudo ./install.sh

# 静默安装
sudo ./install.sh -n

# 自定义安装
sudo ./install.sh -u admin -p 'YourP@ss123' -P 9090 -s 'your-jwt-secret'
```

脚本自动完成：安装依赖 → 创建 `/opt/flamepanel` → 部署二进制 → 注册 systemd 服务 → 启动。

### 方式二：手动 systemd 部署

```bash
# 1. 编译后端
cd flame-kernel && cargo build --release

# 2. 准备目录
sudo mkdir -p /opt/flamepanel/{data,logs}

# 3. 部署二进制
sudo cp target/release/flame-kernel /usr/local/bin/flamepanel

# 4. 创建 systemd 服务
sudo tee /etc/systemd/system/flamepanel.service > /dev/null << 'EOF'
[Unit]
Description=FlamePanel
After=network.target

[Service]
Type=simple
User=root
ExecStart=/usr/local/bin/flamepanel
WorkingDirectory=/opt/flamepanel
Restart=always
RestartSec=5
LimitNOFILE=65535
Environment=OP_PORT=8080
Environment=OP_DATABASE_URL=sqlite:/opt/flamepanel/data/flamepanel.db?mode=rwc
Environment=OP_JWT_SECRET=your-secret-key

[Install]
WantedBy=multi-user.target
EOF

# 5. 启动
sudo systemctl daemon-reload
sudo systemctl enable --now flamepanel
```

### 方式三：Docker Compose 部署

```bash
# 构建并启动
docker compose up -d

# 查看日志
docker compose logs -f

# 停止
docker compose down
```

默认 Docker 配置（`docker-compose.yml`）：
- 端口 `8080:8080`
- 挂载 `./data` 持久化数据库
- 挂载 `/var/run/docker.sock` 实现 Docker 管理
- 挂载 `/etc/nginx` 实现 Web 服务器管理

### 方式四：Nginx 反向代理 + 后端

适用于生产环境，Nginx 负责 TLS 证书和静态资源，后端处理 API 请求。

```nginx
server {
    listen 443 ssl;
    server_name panel.example.com;

    ssl_certificate     /etc/ssl/certs/panel.crt;
    ssl_certificate_key /etc/ssl/private/panel.key;

    root /opt/flamepanel/frontend/dist;
    index index.html;

    location / {
        try_files $uri $uri/ /index.html;
    }

    location /api/ {
        proxy_pass http://127.0.0.1:8080;
        proxy_set_header Host $host;
        proxy_set_header X-Real-IP $remote_addr;
        proxy_set_header X-Forwarded-For $proxy_add_x_forwarded_for;
        proxy_set_header X-Forwarded-Proto $scheme;
    }

    location /ws/ {
        proxy_pass http://127.0.0.1:8080;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "Upgrade";
    }
}
```

## 配置

### 环境变量

| 变量 | 默认值 | 说明 |
|------|--------|------|
| `OP_PORT` | `8080` | 监听端口 |
| `OP_HOST` | `0.0.0.0` | 监听地址 |
| `OP_DATABASE_URL` | `sqlite:data/app.db?mode=rwc` | 数据库连接 |
| `OP_JWT_SECRET` | 自动生成 | JWT 签名密钥 |
| `RUST_LOG` | `info` | 日志级别 |

### 面板运行时设置

首次登录后通过面板设置页面修改（存储在 `panel_settings` 表）：

| 设置项 | 默认值 | 说明 |
|--------|--------|------|
| `panel_name` | `FlamePanel` | 面板名称 |
| `theme` | `light` | 主题 (`light` / `dark`) |
| `language` | `zh-CN` | 语言 (`zh-CN` / `en-US` / `ja-JP`) |
| `session_timeout_minutes` | `1440` | 会话超时 (分钟) |
| `log_level` | `info` | 日志级别 |
| `log_retention_days` | `30` | 日志保留天数 |
| `two_factor_enabled` | `false` | 2FA 开关 |

## 服务管理

```bash
# Systemd
sudo systemctl start|stop|restart|status flamepanel
sudo journalctl -u flamepanel -f

# Docker
docker compose up -d|down|restart
docker compose logs -f

# 数据库备份
cp /opt/flamepanel/data/flamepanel.db /backup/flamepanel-$(date +%Y%m%d).db
```

> 默认凭据：`admin` / `admin123`（首次启动自动创建，建议立即修改）

## API 端点概览

| 模块 | 端点数 | 路径前缀 |
|------|--------|----------|
| 健康检查 | 1 | `GET /health` |
| 认证 | 2 | `/api/auth/*` |
| 用户 | 2 | `/api/users` |
| 节点 | 2 | `/api/nodes` |
| 网站 | 2 | `/api/websites` |
| Docker | 13 | `/api/docker/*` |
| 插件 | 13 | `/api/plugins/*` |
| Web 服务器 | 16 | `/api/web-servers/*` |
| 数据库 | 15 | `/api/databases/*` |
| 文件 | 10 | `/api/files/*` |
| 防火墙 | 11 | `/api/firewall/*` |
| 配置 | 3 | `/api/settings` |
| 操作日志 | 1 | `/api/operation-logs` |
| 系统日志 | 1 | `/api/logs` |
| WebSocket | 3 | `/ws/metrics`, `/ws/logs`, `/ws/terminal` |

## 开发进度

- **Phase 1** ✅ 核心框架：Clean Architecture、错误处理、JWT + RBAC
- **Phase 2** ✅ 业务模块：用户/节点/网站/Docker CRUD、日志、实时 WS
- **Phase 3** ✅ 高级特性：WASM 插件沙箱、Docker Compose、Circuit Breaker/Retry、多 Web 引擎、面板配置、数据库管理、文件管理、防火墙管理、Web 终端
- **Phase 4** 🔄 进行中：前端 UI 重构（顶部导航栏 + 二级侧边栏 + 暗色主题 + 中文化）、SSL 证书、定时任务、备份系统、告警通知
- **测试** ✅ 74 集成测试 + 11 单元测试，全部通过

## License

MIT
