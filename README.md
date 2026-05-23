# Flamepanel

> 基于 Rust + Vue 3 的下一代服务器运维管理面板

![Rust](https://img.shields.io/badge/Rust-1.85-orange)
![Vue](https://img.shields.io/badge/Vue-3.5-brightgreen)
![TypeScript](https://img.shields.io/badge/TypeScript-6.0-blue)
![License](https://img.shields.io/badge/license-MIT-blue)

**Flamepanel** 是一个现代化、高性能、自托管的服务器运维面板，后端使用 Rust + Axum，前端使用 Vue 3 + Element Plus。

## 核心特性

- **系统监控**：实时 CPU、内存、磁盘、负载、网络接口监控
- **Docker 管理**：容器列表、启动 / 停止 / 重启、日志查看
- **网站托管**：Nginx 站点一键创建、启用 / 禁用、SSL 管理
- **WAF 防火墙**：正则规则引擎，支持 URL / Header / Body / Cookie 匹配
- **文件管理**：可视化文件浏览、编辑、上传、新建文件夹
- **Web 终端**：基于 xterm.js + WebSocket 的浏览器终端
- **仪表盘**：系统概览 + 容器 / 网站 / WAF 统计 + 操作日志
- **安全认证**：JWT + bcrypt 密码哈希，中间件保护
- **系统清理**：扫描并清理系统缓存、Docker 缓存、包管理器缓存、日志文件、构建产物

## 技术栈

| 层面 | 技术 |
|------|------|
| 后端框架 | Rust + Axum 0.8 |
| 数据库 | SQLite (sqlx 0.9, 运行时迁移) |
| 认证 | jsonwebtoken 10 + bcrypt 0.19 |
| 前端框架 | Vue 3.5 + TypeScript 6.0 |
| UI 组件 | Element Plus 2.9 |
| 终端 | xterm.js 5.5 + WebSocket |
| 构建 | Vite 8 + vue-tsc |
| 系统信息 | sysinfo 0.39 |

## 项目结构

```
Flamepanel/
├── backend/
│   ├── migrations/          # SQLite 数据库迁移
│   ├── src/
│   │   ├── api/             # HTTP handlers (auth, cleanup, dashboard, docker, file, system, waf, website)
│   │   ├── application.rs   # AppState, AuthService, DashboardService, WafService, CleanupService
│   │   ├── config.rs        # 配置加载 (figment: TOML + 环境变量)
│   │   ├── core/            # AppError, 错误处理
│   │   ├── domain.rs        # 领域实体 (User, Website, WafRule, ServerInfo, etc.)
│   │   ├── infrastructure.rs # Repository 实现 (SQLite + Docker CLI)
│   │   ├── main.rs          # 入口 + 路由注册
│   │   ├── middleware/       # JWT 认证中间件
│   │   ├── plugin/          # 插件系统框架
│   │   ├── utils.rs
│   │   └── websocket/       # Web 终端 WebSocket 处理
│   └── Cargo.toml
├── frontend/
│   └── src/
│       ├── api/             # Axios HTTP 客户端
│       ├── components/      # Sidebar 等共享组件
│       ├── layout/          # 主布局
│       ├── router/          # Vue Router 配置
│       ├── stores/          # Pinia 状态管理 (auth, dashboard, system, docker)
│       ├── types/           # TypeScript 类型定义
│       └── views/           # 页面组件 (9 个视图)
├── docker-compose.yml
├── Dockerfile
├── install.sh
└── justfile
```

## 当前开发进度

### 已完成

- **P1-P4 核心基础**：Clean Architecture 分层、错误处理、配置加载、JWT 中间件、SQLite Repository
- **P5 API 层**：
  - `auth` — 登录 / 注册 / 修改密码 / 当前用户
  - `dashboard` — 系统概览聚合（CPU / 内存 / 磁盘 / Docker / 网站 / WAF / 日志）
  - `docker` — 容器列表 / 启动 / 停止 / 重启 / 日志 / 镜像列表
  - `file` — 文件浏览 / 读取 / 写入 / 创建目录 / 删除 / 上传
  - `system` — 系统信息 / 进程列表
  - `cleanup` — 系统垃圾扫描 / 分类清理（temp / docker / package / logs / dev）
  - `waf` — WAF 规则 CRUD + 启用 / 禁用 + IP 黑白名单
  - `website` — Nginx 站点 CRUD + 启用 / 禁用 + SSL
- **P6 WebSocket**：交互式 Web 终端（bash / sh）
- **P6 插件系统**：插件清单加载、启动 / 停止管理框架
- **P7 WAF 防火墙**：正则规则引擎 + 5 条默认安全规则
- **P7 仪表盘**：Dashboard 数据聚合服务
- **前端界面**：9 个视图（登录 / 仪表盘 / Docker / 文件管理 / 网站 / WAF / 终端 / 进程管理 / 系统清理）
- **数据库表**：users, websites, operation_logs, waf_rules, waf_ip_rules

### 待开发

- 集成测试（axum-test + sqlx）
- utoipa OpenAPI 文档
- 告警通知（邮件 / Telegram / Webhook）
- 多服务器集中管理
- CI/CD（GitHub Actions）
- Docker 镜像发布

## 快速开始

### 环境要求

- Rust 1.85+
- Node.js 20+
- 可选：Docker、Nginx

### 开发模式

```bash
# 克隆项目
git clone https://github.com/Forevery1021/Flamepanel.git
cd Flamepanel

# 终端 1：启动后端（端口 8080）
cd backend
cargo run

# 终端 2：启动前端（端口 5173，自动代理到后端）
cd frontend
npm install
npm run dev
```

访问 `http://localhost:5173`，默认账号：`admin` / `admin123`。

### CLI 命令行工具

Flamepanel 内置了类似 `1pctl` 的 CLI 管理工具，无需启动服务即可进行日常运维操作：

```bash
# 查看所有命令
cargo run -- --help

# 查看版本
cargo run -- version

# 查看当前配置
cargo run -- config

# 查看用户信息
cargo run -- user-info admin

# 列出所有用户
cargo run -- user-list

# 重置用户密码（指定密码）
cargo run -- reset-password admin newpassword

# 重置用户密码（自动生成随机密码）
cargo run -- reset-password admin

# 检查服务运行状态
cargo run -- status
```

生产环境构建后，使用二进制名称直接调用：

```bash
flamepanel version
flamepanel config
flamepanel user-list
flamepanel reset-password admin
flamepanel status
```

### 使用 just 命令

```bash
just dev              # 后端热重载（需要 cargo-watch）
just build-frontend   # 构建前端
just build            # 完整构建（前端 + 后端 release）
just run              # 构建并运行
just clean            # 清理构建产物和数据库
```

### Docker 部署

```bash
# 构建镜像
docker compose build --no-cache

# 启动服务
docker compose up -d

# 查看日志
docker compose logs -f

# 停止
docker compose down
```

### Linux 生产部署

#### 一键脚本安装

```bash
chmod +x install.sh

# 交互式安装（推荐）
sudo ./install.sh

# 自定义账号、密码和端口
sudo ./install.sh -u myadmin -p 'Str0ng!P@ss' -P 9090

# 静默安装，全部自动生成
sudo ./install.sh -n
```

脚本会完成以下步骤：

1. 检测系统并安装依赖（curl、wget、openssl）
2. 创建 `/opt/flamepanel` 目录结构
3. 部署二进制到 `/usr/local/bin/flamepanel`
4. 配置 systemd 服务（开机自启）
5. 自动生成 JWT 密钥和管理员密码（如未指定）

#### 脚本参数

| 参数 | 说明 | 默认值 |
|------|------|--------|
| `-u, --username` | 管理员用户名 | `admin` |
| `-p, --password` | 管理员密码 | 交互输入 / 自动生成 |
| `-P, --port` | 面板监听端口 | `8080` |
| `-s, --secret` | JWT 签名密钥 | 随机生成 |
| `-n, --non-interactive` | 非交互模式 | 关闭 |

#### 从源码构建部署

```bash
# 1. 构建 release 二进制
just build
# 或者手动构建：
# cd frontend && npm install && npm run build
# cd backend && cargo build --release

# 2. 二进制位于 target/release/ops-panel-backend
#    前端静态文件位于 frontend/dist/

# 3. 复制到服务器后运行安装脚本
sudo ./install.sh

# 如果本地构建了二进制，脚本会自动检测并使用
```

#### systemd 服务管理

安装完成后，通过 systemd 管理服务：

```bash
systemctl status   flamepanel   # 查看运行状态
systemctl start    flamepanel   # 启动服务
systemctl stop     flamepanel   # 停止服务
systemctl restart  flamepanel   # 重启服务
systemctl enable   flamepanel   # 开机自启
systemctl disable  flamepanel   # 取消自启

journalctl -u flamepanel -f    # 实时日志
journalctl -u flamepanel -n 100 # 最近 100 条日志
```

### 配置项

通过环境变量或 `backend/config.toml` 文件配置。安装脚本会自动将以下变量写入 systemd 服务文件：

| 变量 | 默认值 | 说明 |
|------|--------|------|
| `OP_PORT` | `8080` | 后端监听端口 |
| `OP_DATABASE_URL` | `sqlite:data/ops_panel.db?mode=rwc` | 数据库路径 |
| `OP_JWT_SECRET` | 内置默认值 | JWT 签名密钥（生产环境务必修改） |
| `OP_ADMIN_USERNAME` | `admin` | 初始管理员账号 |
| `OP_ADMIN_PASSWORD` | `admin123` | 初始管理员密码 |

首次启动时，系统会自动：
1. 运行数据库迁移（创建表 + 5 条默认 WAF 安全规则）
2. 创建管理员账号（以环境变量中的用户名和密码为准）

### 配置覆盖顺序

```
命令行/环境变量 OP_*  >  config.toml  >  内置默认值
```

例如要临时修改端口：`OP_PORT=9090 ./flamepanel`

## License

MIT
