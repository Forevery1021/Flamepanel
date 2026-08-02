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
- **Web 服务器引擎** — 原生支持 Nginx/Apache/OpenLiteSpeed/OpenResty/Caddy，自动生成配置，进程管理（19 端点）；性能预设（low/medium/high/ultra 资源感知推荐）+ 一键引擎切换
- **数据库管理** — MySQL/MariaDB/Redis 原生安装（apt/yum/apk），数据库/用户 CRUD，服务启停（15 端点）
- **应用商店** — 统一支持 1Panel / 宝塔 / Flame 内置三格式应用包，容器 / 原生 / WASM 三模式安装编排（compose 模板变量映射 + 安全扫描 + 失败回滚），升级 / 卸载 / 日志全生命周期（11 端点）
- **文件管理** — Web 端浏览/编辑/上传/下载/重命名/权限（10 端点）
- **防火墙管理** — ufw/firewalld/iptables 自动检测，规则 CRUD/应用/开关（11 端点）
- **Web 终端** — xterm.js + WebSocket，浏览器内直接连接服务器 Shell
- **WASM 插件系统** — wasmtime 沙箱，生命周期钩子/指标追踪/热重载/依赖校验；内置 WASM 工具（插件表持久化 + 启动恢复）
- **面板配置** — Key-Value 配置，主题/多语言/端口/日志/2FA/JWT 密钥轮换
- **用户 & RBAC** — JWT + bcrypt，admin/operator/viewer 三角色，60+ 路由权限映射；认证+RBAC 合并中间件（一次查库）
- **统一错误体系** — 全部错误（含中间件/404/JSON 解析失败）返回 `{code, error, message}` JSON，8 个稳定错误码，前端按码国际化提示；内部错误完整日志链
- **国际化** — 简体中文 / English / 日本語 三语言支持，前端实时切换
- **暗色主题** — 跟随系统 / 手动切换，Element Plus 暗色变量全覆盖
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
| 国际化 | vue-i18n 10（zh-CN / en-US / ja-JP） |
| 终端 | xterm.js 5.5 + @xterm/addon-fit |

## 项目结构

```
Flamepanel/
├── flame-kernel/          # Rust 核心后端
│   ├── src/
│   │   ├── domain/        # 实体 (User/Node/Website/Plugin/AppMetadata/DatabaseInstance/…)
│   │   ├── application/   # 服务层 (UserService/DockerService/AppStoreService/…)
│   │   ├── infrastructure/# 仓库实现 (InMemory + SQLite + Bollard + OS 抽象)
│   │   │   └── app_store/ # 应用商店适配器 (Flame/1Panel/宝塔) + 变量映射 + 安全扫描
│   │   ├── api/           # HTTP 层 (17 个 handler 模块各带 routes(), 113 路由, 统一错误/中间件)
│   │   ├── plugin/        # WASM 沙箱 + 注册表
│   │   ├── webserver/     # 5 引擎配置生成 + 性能预设 + 进程管理
│   │   ├── database/      # MySQL/Redis 原生管理
│   │   ├── firewall/      # 防火墙管理器 (ufw/firewalld/iptables)
│   │   ├── terminal/      # Web 终端 (bash 子进程管道)
│   │   ├── event/         # 事件总线
│   │   ├── utils/         # JWT/bcrypt/验证
│   │   └── resilience/    # Circuit Breaker + Retry
│   └── tests/             # 84 集成测试 + 55 单元测试
├── frontend/              # Vue 3 前端 (17 个视图, 3 语言 i18n)
├── agent/                 # 轻量 Rust Agent
├── docker-compose.yml
├── install.sh
└── uninstall.sh
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

脚本自动完成：安装依赖 → 创建 `/opt/flamepanel` → 部署后端二进制与前端静态资源 → 配置 nginx 反向代理（80 端口）→ 注册 systemd 服务 → 启动。

> 脚本优先使用本地构建产物（`target/release/flame-kernel`、`frontend/dist`），否则从 GitHub Releases 下载；密钥写入 `/opt/flamepanel/flamepanel.env`（600 权限），不出现在 systemd unit 中。
>
> 访问面板：`http://<服务器IP>/`（经 nginx），后端 API 位于 `http://<IP>:<端口>/api`。

### 方式二：手动 systemd 部署

```bash
# 1. 编译后端
cd flame-kernel && cargo build --release

# 2. 构建前端
cd frontend && npm ci && npm run build

# 3. 准备目录
sudo mkdir -p /opt/flamepanel/{data,logs,frontend}
sudo cp ../target/release/flame-kernel /usr/local/bin/flamepanel
sudo cp -r ../frontend/dist /opt/flamepanel/frontend/

# 4. 环境变量文件（600 权限）
sudo tee /opt/flamepanel/flamepanel.env > /dev/null << 'EOF'
OP_PORT=8080
OP_HOST=0.0.0.0
OP_DATABASE_URL=sqlite:/opt/flamepanel/data/flamepanel.db?mode=rwc
OP_JWT_SECRET=your-secret-key
EOF
sudo chmod 600 /opt/flamepanel/flamepanel.env

# 5. 创建 systemd 服务
sudo tee /etc/systemd/system/flamepanel.service > /dev/null << 'EOF'
[Unit]
Description=FlamePanel
After=network.target

[Service]
Type=simple
User=root
ExecStart=/usr/local/bin/flamepanel
WorkingDirectory=/opt/flamepanel
EnvironmentFile=/opt/flamepanel/flamepanel.env
Restart=always
RestartSec=5
LimitNOFILE=65535

[Install]
WantedBy=multi-user.target
EOF

# 6. Nginx 反向代理（静态资源 + API + WebSocket）
#    参考仓库 nginx.conf 或运行 install.sh 自动生成
sudo tee /etc/nginx/conf.d/flamepanel.conf > /dev/null << 'EOF'
server {
    listen 80;
    server_name _;
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
    }
    location /ws/ {
        proxy_pass http://127.0.0.1:8080;
        proxy_http_version 1.1;
        proxy_set_header Upgrade $http_upgrade;
        proxy_set_header Connection "Upgrade";
    }
    location /assets/ {
        expires 30d;
        add_header Cache-Control "public";
    }
}
EOF

# 7. 启动
sudo systemctl daemon-reload
sudo systemctl enable --now flamepanel nginx
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
- 端口 `8080:80`（容器内 nginx 提供前端静态资源 + API/WS 反向代理）
- 挂载 `./data` 持久化数据库
- 挂载 `/var/run/docker.sock` 实现 Docker 管理

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
| `OP_JWT_SECRET` | `flamepanel-secret` | JWT 签名密钥（生产环境务必修改） |
| `OP_ADMIN_USERNAME` | `admin` | 初始管理员用户名 |
| `OP_ADMIN_PASSWORD` | `admin123` | 初始管理员密码 |
| `OP_SMTP_HOST` | `localhost` | SMTP 服务器地址 |
| `OP_SMTP_PORT` | `25` | SMTP 端口 |
| `OP_SMTP_USERNAME` | 空 | SMTP 用户名 |
| `OP_SMTP_PASSWORD` | 空 | SMTP 密码 |
| `OP_SMTP_FROM` | `noreply@flamepanel.local` | 发件人地址 |
| `OP_SMTP_TLS` | `false` | 启用 TLS |
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

## 卸载

### Systemd 部署

```bash
# 卸载（保留数据）
sudo ./uninstall.sh

# 完全卸载（删除所有数据）
sudo ./uninstall.sh -p

# 或手动卸载
sudo systemctl stop flamepanel
sudo systemctl disable flamepanel
sudo rm -f /usr/local/bin/flamepanel
sudo rm -f /etc/systemd/system/flamepanel.service
sudo rm -f /etc/nginx/conf.d/flamepanel.conf && sudo systemctl restart nginx
sudo systemctl daemon-reload
sudo rm -rf /opt/flamepanel   # 删除数据
```

### Docker 部署

```bash
docker compose down
rm -rf ./data   # 删除数据
```

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
| 用户 | 4 | `/api/users` |
| 节点 | 4 | `/api/nodes` |
| 网站 | 6 | `/api/websites` |
| Docker | 13 | `/api/docker/*` |
| 插件 | 13 | `/api/plugins/*` |
| 应用商店 | 11 | `/api/app-store/*` |
| Web 服务器 | 19 | `/api/web-servers/*` |
| 数据库 | 15 | `/api/databases/*` |
| 文件 | 10 | `/api/files/*` |
| 防火墙 | 11 | `/api/firewall/*` |
| 配置 | 3 | `/api/settings` |
| 操作日志 | 2 | `/api/operation-logs` |
| 系统日志 | 2 | `/api/logs` |
| WebSocket | 3 | `/ws/metrics`, `/ws/logs`, `/ws/terminal` |

## 开发进度

- **Phase 1** ✅ 核心框架：Clean Architecture、错误处理、JWT + RBAC
- **Phase 2** ✅ 业务模块：用户/节点/网站/Docker CRUD、日志、实时 WS
- **Phase 3** ✅ 高级特性：WASM 插件沙箱、Docker Compose、Circuit Breaker/Retry、多 Web 引擎、面板配置、数据库管理、文件管理、防火墙管理、Web 终端
- **Phase 4** ✅ 前端 UI 重构：i18n 国际化（zh-CN/en-US/ja-JP）、暗色主题、语言切换器、TopBar 重组、全部 15 个视图 i18n 化
- **Phase 4** ✅ 分页支持：后端所有列表端点支持 `?page=&page_size=` 分页查询，前端统一分页控件
- **Phase 4** ✅ CRUD 补全：Website 完整 CRUD、User/Node 更新、OperationLog/Log 删除端点
- **Phase 4** ✅ 前端编辑对话框：用户/节点/网站视图编辑功能
- **Phase 4** ✅ 优雅关闭：SIGTERM/Ctrl+C 信号处理
- **Phase 4** 🔄 进行中：SSL 证书、定时任务、备份系统、告警通知、Web 服务器 / 数据库管理增强
- **Phase 5** ✅ 应用商店：1Panel/宝塔/Flame 三格式适配器 + 容器/原生/WASM 三模式安装编排（变量映射、安全扫描、失败回滚）、WASM 内置工具持久化、完整 API + 前端商店视图（动态表单安装向导）
- **Phase 5** ✅ Web 引擎统一：性能预设（资源感知推荐）+ 引擎一键切换（Web 服务器 & 网站）+ 预设应用，前端预设/切换 UI
- **Phase 6** ✅ 内核优化：统一错误体系（8 稳定错误码 + JSON 化中间件/404/ApiJson）、认证+RBAC 合并中间件、Services 聚合 + 路由分模块、release profile 优化（18MB stripped）、前端错误码 i18n
- **测试** ✅ 141 测试全部通过（86 集成 + 55 单元）

## License

MIT
