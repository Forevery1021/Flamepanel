<div align="center">

# 🔥 FlamePanel

> **现代化、高性能、自托管的服务器运维管理面板**

基于 **Rust + Vue 3** 构建，采用六边形（Clean）架构，参考 1Panel 能力设计，提供 Docker、Web 服务器、数据库、应用商店、防火墙等全栈运维能力。

![Rust](https://img.shields.io/badge/Rust-1.85-orange)
![Axum](https://img.shields.io/badge/Axum-0.8-red)
![Vue](https://img.shields.io/badge/Vue-3.5-brightgreen)
![TypeScript](https://img.shields.io/badge/TypeScript-6.0-blue)
![License](https://img.shields.io/badge/license-MIT-blue)
![PRs](https://img.shields.io/badge/PRs-welcome-brightgreen)

</div>

---

## ✨ 核心特性

| 模块 | 说明 |
|------|------|
| 🖥️ **系统监控** | WebSocket 实时推送 CPU / 内存 / 磁盘 / 负载 / 网络 IO，ECharts 趋势图 + 负载仪表 + 进程 TOP |
| 🐳 **Docker 管理** | 容器 / 镜像 / 网络 / 卷 / Compose 全生命周期管理（33 端点，参考 1Panel） |
| 🌐 **Web 服务器** | 原生支持 Nginx / Apache / OpenLiteSpeed / OpenResty / Caddy，自动生成配置，性能预设 + 引擎一键切换 |
| 🗄️ **数据库管理** | MySQL / MariaDB / Redis 原生安装与启停、数据库 / 用户 CRUD |
| 🧩 **应用商店** | 统一支持 1Panel / 宝塔 / Flame 三格式应用包，容器 / 原生 / WASM 三模式安装编排 |
| 📁 **文件管理** | Web 端浏览 / 编辑 / 上传 / 下载 / 重命名 / 权限控制 |
| 🛡️ **防火墙管理** | ufw / firewalld / iptables 自动检测，规则 CRUD / 应用 / 开关 |
| 💻 **Web 终端** | xterm.js + WebSocket，浏览器内直接连接服务器 Shell |
| ⚙️ **WASM 插件系统** | wasmtime 沙箱，生命周期钩子 / 指标追踪 / 热重载 / 依赖校验 |
| 🔐 **用户 & RBAC** | JWT + bcrypt + 滑动过期刷新，admin / operator / viewer 三角色，73 项权限 |
| 🛰️ **节点管理** | Agent 心跳对接（Bearer token 鉴权），在线状态惰性判定 + 实时指标快照，批量命令执行；节点注册受 Bootstrap Token 保护 |
| 🧭 **Setup 向导** | 首次部署 6 步可视化向导（管理员 / 数据库 / 端口 / 主题），自签证书自动签发，支持无人值守环境变量一键初始化 |
| 🌍 **国际化** | 简体中文 / English / 日本語 三语言，前端实时切换 |

> 另含：多页签工作区、⌘K 命令面板、主题定制系统（4 预设 + 品牌色实时调色）、统一任务中心（生命周期状态机）、审计日志、事件驱动 + Outbox + 邮件通知、Circuit Breaker / Retry 容错、统一错误体系、vue-query 数据层 + 虚拟列表性能优化。

---

## 🧱 技术栈

| 层面 | 技术 |
|------|------|
| **后端架构** | Clean Architecture + Hexagonal（domain → application → infrastructure → api） |
| **后端框架** | Rust + Axum 0.8 |
| **数据库** | SQLite（sqlx 0.9）+ InMemory 双模式 |
| **认证** | jsonwebtoken 9 + bcrypt |
| **WASM 沙箱** | wasmtime |
| **前端框架** | Vue 3.5 + TypeScript 6.0 + Vite 8 |
| **UI 组件库** | OpenVue 0.7（PrimeVue v4 社区延续版） |
| **状态 / 路由** | Pinia 3 + Vue Router 5 |
| **样式** | UnoCSS + OKLCH 设计令牌 |
| **图表** | ECharts 6（按需引入） |
| **国际化** | vue-i18n 10（zh-CN / en-US / ja-JP） |
| **终端** | xterm.js 5.5 + @xterm/addon-fit |
| **节点 Agent** | 轻量 Rust（reqwest + sysinfo） |

---

## 🚀 快速开始

```bash
# 终端 1：启动后端（端口 8080）
cd flame-kernel
cargo run

# 终端 2：启动前端（端口 5173，自动代理 /api 与 /ws）
cd frontend
npm install        # 或 pnpm install
npm run dev        # 或 pnpm run dev
```

访问 `http://localhost:5173`，**首次启动会自动进入 6 步 Setup 向导**（管理员账号 / 数据库 / 端口 / 主题 / 语言），完成后跳转登录页。老版本数据库（已有用户）启动后直接进入登录页，无需向导。

无人值守部署：配置 `OP_ADMIN_PASSWORD` 环境变量后跳过向导，启动时自动创建管理员（并强制首次登录改密）。更多配置见 [部署文档](./Doc/06-部署运维指南.md)。

生产环境推荐使用 `install.sh` 一键安装（systemd + 非 root 用户）。

> ⚠️ **Docker 部署安全提示**：镜像内为 root 用户。生产请使用 `docker compose up -d`（默认 profile **不挂 docker.sock**、最小能力 `cap_drop: ALL`）；`docker compose --profile dev up -d` 挂载 docker.sock，仅限本地开发——「root 容器 + docker.sock」组合被攻破即等价宿主机 root。

---

## 📁 项目结构

```
Flamepanel/
├── flame-kernel/        # Rust 核心后端（Axum + 六边形架构）
│   ├── src/
│   │   ├── domain/          # 实体 + Repository trait + 领域端口（零依赖）
│   │   ├── application/     # 服务层（每域一文件：user/node/website/docker/role/web_server…）
│   │   ├── infrastructure/  # 仓库实现（InMemory/SQLite/Docker/OS + app_store/firewall 适配器）
│   │   ├── api/             # HTTP 层（22 个 handler 模块 + 分域拆分 types/dto/permissions）
│   │   ├── plugin/          # WASM 沙箱 + 注册表
│   │   ├── webserver/       # 5 种 Web 引擎 + 性能预设 + 原生控制
│   │   ├── database/        # MySQL / Redis 原生管理
│   │   ├── firewall/        # 防火墙管理（ufw/firewalld/iptables）
│   │   ├── terminal/        # Web 终端（bash 子进程管道）
│   │   ├── event/           # 事件总线 + Outbox + 邮件通知
│   │   ├── resilience/      # Circuit Breaker + Retry
│   │   └── utils/           # JWT / bcrypt / AuthCache / 校验
│   └── tests/               # 集成 + 单元测试
├── frontend/            # Vue 3 + OpenVue 前端（23 个视图，3 语言 i18n）
├── agent/               # 轻量 Rust 节点 Agent
├── Doc/                 # 19 份开发 / 部署 / API 权威文档
├── install.sh           # 一键安装脚本
├── docker-compose.yml   # Docker 部署
├── Dockerfile
└── justfile             # 开发 / 构建 / 测试命令
```

---

## 🗺️ 进度与方向

> 最新进展详见 [CHANGELOG.md](./CHANGELOG.md) 与 [开发路线图](./Doc/13-开发路线图与后续规划.md)。

### ✅ 已完成（Phase 1 – 8）

| 阶段 | 内容 |
|------|------|
| **Phase 1–2** | 核心框架：Clean Architecture、错误体系、JWT + RBAC；用户 / 节点 / 网站 / Docker CRUD、实时 WS |
| **Phase 3** | 高级特性：WASM 插件沙箱、Compose、Circuit Breaker / Retry、多 Web 引擎、数据库 / 文件 / 防火墙管理、Web 终端 |
| **Phase 4** | 前端 UI 重构：i18n 三语言、暗色主题、分页、CRUD 补全、定时任务、备份系统 |
| **Phase 5** | 应用商店（三格式 / 三模式）、Web 引擎统一（性能预设 + 引擎切换） |
| **Phase 6** | 内核优化：统一错误体系、Docker 增强（参考 1Panel）、Web 服务器原生控制 |
| **Phase 7** | **P0 生产可用（v0.6.0）**：节点心跳、生产安全（强制改密 / 登录锁定 / 审计）、自动备份、发行体系、可观测性 |
| **Phase 8** | **前端全面重构（v0.7.0）**：Element Plus → OpenVue 0.7、OKLCH 设计令牌、主题定制、⌘K 命令面板 |
| **Phase 9** | **Setup 向导 + 安全加固**：首次部署 6 步向导（`/api/setup/status|initialize`）、自签证书签发、无人值守初始化；Agent 注册 Bootstrap Token 防护、心跳 Bearer 鉴权、配置解析失败拒绝启动、上传路径 `O_NOFOLLOW` 加固、JWT 轮换密钥强随机 |
| **后端重构（Stage 0–9）** | 按 [Doc/19](./Doc/19-后端架构分析与完善落地手册.md) 完成：分页下沉、鉴权短缓存 AuthCache、限流升级（去全局锁+分级限额）、任务生命周期、权限路由元数据化+默认拒绝、错误映射细分、JWT 加固、Docker 门面拆分、事件 Outbox 可重试 |

**当前基线**：181 条 HTTP 路由 + 3 条 WebSocket · 73 项 RBAC 权限 · 329 个测试全部通过

### 🧩 前端现代化（已落地）

> 依据 [Doc/17 重构与现代化落地手册](./Doc/17-重构与现代化落地手册.md) 完成，核心硬性规范见 [Doc/04 前端开发指南 §15](./Doc/04-前端开发指南.md#15-修复计划与现代化规范与-doc17-衔接)。

| 阶段 | 内容 |
|------|------|
| **F0 稳定性** | 统一请求与错误处理、keep-alive 资源销毁（useECharts/useWebSocket/usePolling）、FpStatePanel 三态 |
| **F1 数据与性能** | vue-query 统一数据层、FpTable 虚拟列表、Dashboard 图表节流 |
| **F2 设计系统** | OKLCH 令牌补全、玻璃降级、Fp* 封装层 30+ 组件、views 收敛 |
| **F3 a11y & IA** | 命令面板 a11y、角色化侧栏、v-permission 指令 76 处落地、平板响应式 |
| **F4 工程化** | OpenAPI 类型单源、/dev/ui 预览页、vitest + CI typecheck |
| **M1–M11 现代化** | 命令集中配置、vue-query 覆盖 6 视图、主题 JSON v2 导入导出、Fp 组件文档、列表筛选条、eslint 强制 Fp* 边界 |

### 🔭 进行中 / 规划方向

| 优先级 | 方向 | 说明 |
|--------|------|------|
| **P0** | 生产安全加固 | HTTPS 重定向、登录图形验证码、备份二次确认 |
| **P1** | 认证增强 | "记住我"令牌有效期选择、验证码 |
| **P1** | 前端功能联调 | 文件上传 / 下载编辑器、防火墙规则编辑表单补全 |
| **P2** | 应用商店生态 | NativeInstaller trait 抽象、Compose 生命周期钩子、包签名与可信来源、远程商店源 |
| **P2** | 多节点能力 | 远程文件管理、节点聚合指标面板 |
| **P2** | 高可用与生态 | docker-compose 健康检查 / 重启策略、WASM 插件市场、备份异地存储（SFTP / S3） |

---

## 📖 文档中心

| 主题 | 文档 |
|------|------|
| 🏗️ 架构设计 | [01-架构设计](./Doc/01-架构设计.md) |
| 📡 API 接口 | [02-API接口文档](./Doc/02-API接口文档.md) |
| 🗄️ 数据库设计 | [03-数据库设计](./Doc/03-数据库设计.md) |
| 💻 前端 / 后端开发 | [04-前端指南](./Doc/04-前端开发指南.md) · [05-后端指南](./Doc/05-后端开发指南.md) |
| 🚀 部署运维 | [06-部署运维指南](./Doc/06-部署运维指南.md) |
| 🔐 权限体系 | [07-权限体系设计](./Doc/07-权限体系设计.md) |
| 🧩 应用商店 & SDK | [08-应用商店与插件系统](./Doc/08-应用商店与插件系统.md) · [15-SDK开发指南](./Doc/15-应用商店SDK开发指南.md) · [16-1Panel兼容](./Doc/16-1Panel与原生软件兼容性开发指导.md) |
| 🛠️ 重构与现代化 | [17-重构与现代化落地手册](./Doc/17-重构与现代化落地手册.md) |
| 🔒 兼容性与安全 | [18-兼容性与安全基线](./Doc/18-兼容性与安全基线.md) |
| 📐 后端架构完善 | [19-后端架构分析与完善落地手册](./Doc/19-后端架构分析与完善落地手册.md) |
| 🗺️ 路线图 | [13-开发路线图](./Doc/13-开发路线图与后续规划.md) |
| 🧪 测试 / 排障 | [09-测试指南](./Doc/09-测试指南.md) · [10-故障排查手册](./Doc/10-故障排查手册.md) |
| 🤝 协作 / 事件 | [11-Agent节点通信协议](./Doc/11-Agent节点通信协议.md) · [12-事件与通知系统](./Doc/12-事件与通知系统.md) · [14-开发协作与发布流程](./Doc/14-开发协作与发布流程.md) |
| 📜 变更日志 | [CHANGELOG.md](./CHANGELOG.md) |
| 📚 文档导航 | [Doc/README](./Doc/README.md) |

---

## 📜 License

[MIT](./LICENSE)
