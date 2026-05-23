# Flamepanel 开发路线图 & 功能模块规划

**项目版本**：v0.1.x（早期预览阶段）  
**更新日期**：2026-05-23（最新：2.4 应用商店完成）  
**作者**：Flamepanel 团队（欢迎社区贡献）

Flamepanel 是一款基于 **Rust + Axum + Vue3** 开发的下一代高性能服务器运维面板，目标是成为资源占用最低、安全性最高、架构最现代的开源 O&M 工具。

**核心差异化**：  
- 极致性能与低资源占用（Rust 原生优势）  
- Clean Architecture + 强类型安全  
- 轻量级设计，适合低配服务器、边缘计算与生产环境

---

## 1. 总体开发策略

### 开发原则
- **Rust 优先**：充分利用 Tokio 异步、强类型、内存安全优势。
- **渐进式对标**：先实现核心功能，再补生态，最后差异化（AI + 极致轻量）。
- **性能至上**：所有模块必须考虑缓存、异步、非阻塞、资源限流。
- **可扩展性**：插件系统（WASM）、多后端支持（SQLite → PostgreSQL）、多 Web Server（Nginx + Caddy）。

### 版本规划
- **v0.2.x**（基础稳定）：核心监控 + 基础运维可用  
- **v0.5.x**（功能追平）：对标 1Panel 主流功能  
- **v1.0.x**（生产级）：App Store + 多节点 + AI  
- **v2.0.x**（领先）：高可用、企业特性、Rust 生态插件

---

## 2. 详细功能模块开发路线

### Phase 1: 基础框架完善（1-2 个月）

#### 1.1 仪表盘 (Dashboard)
- **子功能**：
  - 实时系统概览（CPU、内存、磁盘、Swap、网络 I/O、负载、运行时间）
  - 趋势图表（ECharts + WebSocket 实时推送）
  - 快捷操作（重启服务、快速备份、系统清理）
  - 系统健康评分 + 告警提示
  - 资源自监控（面板自身 CPU/内存占用）
- **Rust 技术点**：
  - `sysinfo` + `tokio::interval` + `moka` 缓存（采样间隔 10-30s 可配置）
  - Prometheus metrics exporter
- **优先级**：高  
- **里程碑**：v0.2.0

#### 1.2 主机监控与系统管理
- 详细硬件信息、进程列表、GPU 监控（nvml-wrapper）
- 系统设置：主机名、时区、Swap、DNS、NTP、内核参数
- 安全设置：防火墙（ufw/firewalld）、SSH 密钥管理
- **优化**：使用 `dashmap` 缓存进程信息

#### 1.3 文件管理
- 文件/文件夹浏览、上传、下载、编辑、删除、复制、移动
- 权限修改、压缩/解压（zip/tar）、文本编辑器
- 大文件分块上传 + 预览（图片、视频、PDF）
- 搜索、批量操作、回收站
- **Rust 优化**：`tokio::fs` + chunked 处理、`image` crate 预览、`mimalloc`

#### 1.4 Web 终端 ✅（已完成）
- ✅ xterm.js + WebSocket 终端
- ⬜ 多会话管理、会话录像、命令历史
- ⬜ 权限控制 + 操作审计

#### 1.5 操作日志与审计 ✅（基础完成）
- ✅ 操作记录（登录、文件、容器、设置等）
- ✅ 分页查看日志列表
- ⬜ 导出（CSV/JSON）、搜索过滤、保留策略

#### 1.6 面板设置 ✅（2026-05-23 完成）
- ✅ 用户密码修改（接入 /api/auth/change-password）
- ✅ 主题切换（亮/暗），CSS 变量驱动，Element Plus 深度适配
- ✅ 设置持久化（SQLite settings 表 + REST API）
- ✅ 面板信息展示（版本号、技术栈、许可证）
- ⬜ 语言切换（i18n，当前仅简体中文）
- ⬜ 菜单排序、版本升级检测

**Phase 1 完成标准**：面板可稳定用于日常服务器基础运维。

**Phase 1 进度**：1.1 ✅ | 1.2 ✅（基础） | 1.3 ✅ | 1.4 ✅（基础） | 1.5 ✅（基础） | 1.6 ✅（核心完成）
**Phase 2 进度**：2.1 ✅（基础） | 2.2 ✅ | 2.3 ✅（基础） | 2.4 ✅ | 2.5 ✅ | 2.6 ✅（基础）

---

### Phase 2: 核心功能追平（3-6 个月）

#### 2.1 网站管理 (Websites)
- 支持 Nginx + Caddy
- 网站创建（静态、反代、PHP/Node/Python/Go 等运行时）
- 一键 SSL（Let's Encrypt + 手动上传）
- 配置：域名绑定、HTTPS 强制、重定向、伪静态、防盗链、密码访问
- 网站备份/恢复/克隆、日志查看
- **技术**：配置文件模板 + 热重载、WAF 集成

#### 2.2 数据库管理 ✅（2026-05-23 完成）
- ✅ 支持 5 种数据库：MySQL、MariaDB、PostgreSQL、Redis、MongoDB
- ✅ Docker 一键部署（自动拉取镜像、创建容器、配置环境变量和持久化卷）
- ✅ 启动/停止/删除（容器 + 数据目录清理）
- ✅ 自动备份（docker exec + mysqldump/pg_dumpall/redis SAVE/mongodump → docker cp）
- ✅ 备份记录管理 + 连接串展示
- ⬜ 数据库/表管理、用户权限管理、导入导出

#### 2.3 容器管理 (Docker)
- 容器、镜像、网络、Volume 完整管理
- Docker Compose 项目支持（解析 + 管理）
- 资源限制、日志流式查看、端口映射、环境变量
- **迁移计划**：全面切换到 `bollard` Rust SDK

#### 2.4 应用商店 (App Store) —— 核心生态模块 ✅（2026-05-23 完成）
- ✅ Manifest 标准（JSON 格式，含 compose 模板 + 端口/图标/分类）
- ✅ 内置 8 款应用（WordPress、Portainer、Gitea、phpMyAdmin、Nginx、Node.js、Redis、Uptime Kuma）
- ✅ Docker Compose 一键部署（模板变量渲染：端口/名称/数据目录）
- ✅ 分类浏览 + 搜索过滤 + 安装/启动/停止/重启/卸载
- ✅ 应用日志查看（docker compose logs --tail=100）
- ⬜ 版本升级、社区贡献流程、更多应用

#### 2.5 计划任务 (Cron) ✅（2026-05-23 完成）
- ✅ Shell 命令执行（sh -c / cmd /C）
- ✅ URL HTTP 请求（GET，记录状态码和响应体）
- ✅ Cron 表达式解析（5 字段：min hour dom mon dow，支持 */N / 范围 / 列表）
- ✅ 后台调度器（tokio::interval 每 30s 检查，自动更新 next_run）
- ✅ 执行日志（成功/失败状态 + 输出内容 + 时间线）
- ✅ 手动立即执行 + 启用/禁用开关
- ⬜ 容器内执行、Webhook/邮件通知、依赖任务、并发控制
- 依赖任务、并发控制

#### 2.6 安全工具箱
- 增强 WAF 规则引擎
- 系统安全扫描、漏洞检测
- 防火墙可视化管理
- 快照、系统清理（垃圾文件、缓存）

**Phase 2 完成标准**：v0.5.0 功能覆盖 1Panel 主流使用场景。

---

### Phase 3: 高级特性与差异化领先（6-12 个月）

#### 3.1 多节点 / 集群管理
- 轻量 Rust Agent（二进制 < 10MB）
- 统一仪表盘、多服务器批量操作
- 文件跨节点传输、资源聚合监控

#### 3.2 AI 集成（核心差异化）
- Ollama 本地 LLM 集成
- AI 助手：日志智能分析、故障诊断、命令生成、配置优化建议
- OpenClaw-like Agent 支持
- GPU 监控与管理
- MCP/Skills 框架（Rust 实现）

#### 3.3 备份与高可用
- 全量/增量备份（本地 + S3 + 阿里云等）
- 一键恢复、备份策略
- 支持 PostgreSQL + Redis 作为后端存储
- 集群高可用方案

#### 3.4 插件与扩展系统
- WASM 插件支持
- 自定义仪表盘组件
- 第三方集成（Prometheus、Grafana、GitOps）

#### 3.5 企业级特性（可选 Pro）
- 审计合规（SOC2 等）
- 精细化权限控制（RBAC）
- 多租户支持
- 高级 WAF + 防篡改

---

## 3. 非功能性开发路线

- **性能与资源优化**（持续）：
  - mimalloc/jemallocator
  - 全面异步 + 背压控制
  - 指标监控与自动调优建议

- **安全性**：
  - 最小权限原则、JWT 强化、速率限制
  - 安全扫描集成、依赖漏洞自动检测

- **部署与 CI/CD**：
  - 官方多架构 Docker 镜像
  - justfile 构建脚本
  - GitHub Actions 全自动化测试与发布

- **文档与社区**：
  - 完整中文/英文文档站
  - API 文档（Swagger）
  - 贡献指南、代码规范（clippy + rustfmt）

---
# Flamepanel Core 开发指南

**文档版本**：v0.1  
**更新日期**：2026-05-23  
**对标项目**：1Panel Core（v2）  
**适用对象**：Flamepanel 后端开发者

---

## 1. 文档说明

本文参照 **1Panel Core** 架构，为 Flamepanel（基于 Rust + Axum）设计了 Core 层的完整开发指南。目标是构建一个**高性能、类型安全、清晰可维护、可扩展**的核心引擎。

---

## 2. 1Panel Core 架构简析（参考）

1Panel Core 采用典型的 Go Web 项目结构，主要特点包括：

- 按业务领域模块化组织（website、container、database、host 等）
- 全局统一管理（global、constant、init）
- 清晰的分层结构（路由、中间件、业务逻辑、工具）
- 集中的初始化流程（数据库、配置、定时任务等）

---

## 3. Flamepanel Core 推荐架构（Rust 版）

Flamepanel Core 采用 **Clean Architecture** 原则，结合 Rust 的模块系统和 Cargo 特性，充分发挥 Rust 的**并发性能、内存安全和类型安全**优势。

### 3.1 推荐目录结构

```bash
flamepanel/
├── Cargo.toml                     # Workspace 主文件
├── justfile                       # 构建任务脚本（推荐使用）
├── core/                          # ← Core 主模块（建议独立 crate）
│   ├── Cargo.toml
│   └── src/
│       ├── main.rs                # 二进制入口（生产启动）
│       ├── lib.rs                 # 库入口
│       │
│       ├── application/           # 应用服务层（Use Cases）
│       ├── domain/                # 领域模型层
│       ├── infrastructure/        # 基础设施实现层
│       │
│       ├── api/                   # 接口层
│       │   ├── handlers/          # 请求处理器
│       │   ├── routes.rs          # Axum 路由定义
│       │   └── middleware/        # 中间件
│       │
│       ├── config/                # 配置管理
│       ├── constant/              # 常量定义
│       ├── error/                 # 错误处理
│       ├── global/                # 全局单例
│       ├── init/                  # 初始化逻辑
│       ├── service/               # 业务服务层（按模块拆分）
│       ├── repository/            # Repository Trait
│       ├── monitor/               # 系统监控
│       ├── event/                 # 事件总线
│       ├── utils/                 # 通用工具函数
│       └── migrations/            # sqlx 迁移文件
│
├── agent/                         # 多节点轻量 Agent（后续开发）
├── frontend/                      # Vue3 前端
└── docs/                          # 文档目录
4. Core 各模块详细开发建议
4.1 Config & Global

配置管理：使用 figment 支持 TOML + ENV + CLI
全局单例：std::sync::OnceLock 或 tokio::sync::OnceCell
核心全局对象：
Config
SqlitePool / PgPool
bollard::Docker
JWT Secret
Logger（tracing）


4.2 Init 初始化模块
启动时必须完成：

数据库连接与自动迁移
默认管理员账号初始化
必要目录创建
Docker 连接检查
WAF 规则加载
监控任务启动

推荐使用异步并行初始化。
4.3 Error Handling

使用 thiserror + anyhow 定义统一 AppError
支持错误码与国际化
实现 IntoResponse trait 转换为 Axum 响应
4.5 Application / Service 层
按业务模块拆分：

service::host
service::website
service::container
service::appstore
service::backup
service::ai

严格遵循调用顺序：Handler → Application Service → Domain → Repository
4.6 Domain & Repository

Domain：纯业务实体与值对象
Repository：定义 Trait，由 Infrastructure 实现

4.7 Infrastructure

infrastructure::database
infrastructure::docker（强烈推荐使用 bollard SDK）
infrastructure::filesystem
infrastructure::webserver（Nginx / Caddy 配置模板管理）

4.8 Monitor & Event

系统监控独立模块（sysinfo + tokio::interval）
事件总线：tokio::sync::broadcast 用于内部解耦


5. 开发最佳实践

性能优化
全局使用 mimalloc 或 jemallocator
阻塞操作使用 tokio::task::spawn_blocking
引入 moka 缓存
暴露 Prometheus metrics

代码质量
强制执行 cargo clippy --fix 和 rustfmt
核心模块单元测试覆盖率 ≥ 80%
使用 utoipa 自动生成 OpenAPI 文档

安全性
最小权限运行
定期 cargo audit
所有外部输入严格验证

构建部署
多阶段 Dockerfile（极小体积）
支持 musl 静态链接
justfile 统一命令（just dev、just build、just release）