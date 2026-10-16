<div align="center">

# 🔥 FlamePanel

[English](./README.md) | **简体中文**

**开源、自托管的 Linux 服务器运维面板**

Rust 内核 · Vue 3 前端 · 静态前端 + SQLite · MIT

[![Rust](https://img.shields.io/badge/Rust-1.98.1-orange)](https://www.rust-lang.org/)
[![Axum](https://img.shields.io/badge/Axum-0.8-red)](https://docs.rs/axum)
[![Vue](https://img.shields.io/badge/Vue-3.5-brightgreen)](https://vuejs.org/)
[![License](https://img.shields.io/badge/license-MIT-blue)](./LICENSE)

[快速开始](#-快速开始) · [功能](#-功能) · [系统支持](#-系统支持) · [文档](#-文档) · [参与开发](#-参与开发)

</div>

---

## 这是什么

FlamePanel 对标 1Panel / 宝塔：在浏览器里管理网站、容器、数据库、证书、备份和防火墙。

- **轻**：一个 Rust 内核 + 一份静态前端，SQLite 无需外部数据库；资源目标与实际验收分开记录。
- **不绑架**：面板只是管理面，**停掉或卸载面板，你的网站和容器照常运行**（`uninstall.sh --keep-apps`）。
- **原生部署边界**：面板以非 root 用户运行，systemd 沙箱加固；Web 引擎配置由独立的本机特权助手按白名单代办；RBAC、审计日志、登录锁定开箱即用。

## 🚀 快速开始

**一键安装**（Debian 12+/Ubuntu 22.04+ 推荐，RHEL 9 系可用，详见[系统支持](#-系统支持)）：

```bash
git clone https://cnb.cool/yin0410/Flamepanel.git && cd Flamepanel
sudo bash install.sh          # 交互式；-n 全默认静默；-t 启用 HTTPS（自签，80 自动跳转 443）
```

装完浏览器打开 `http://<服务器 IP>`，按 6 步向导设置管理员即可使用。云服务器记得在**安全组**放行 80 / 443。

<details>
<summary><b>安装脚本会做什么？</b></summary>

- 按发行版安装 `curl openssl nginx`，创建 `flamepanel` 系统用户，部署到 `/opt/flamepanel`
- 生成 nginx 站点（静态资源 + `/api` `/ws` 反代、安全响应头与 CSP、2 GB 上传上限），后端只监听 `127.0.0.1`
- 自动处理 Debian 默认站点、Alpine/Arch 的 nginx 目录差异、RHEL 的 SELinux 与 firewalld
- 所有系统改动记录在 `/opt/flamepanel/.install-state`，`uninstall.sh` 据此精确回滚

完整说明见 [Doc/10 部署运维指南](./Doc/10-部署运维指南.md)。
</details>

**Docker**（任意发行版，含 Alpine）：

```bash
umask 077
echo "OP_JWT_SECRET=$(openssl rand -hex 32)" > .env
docker compose up -d          # http://<ip>:8080（默认不挂载 docker.sock）
```

**卸载**：`sudo bash uninstall.sh`（保留数据）· `--keep-apps`（应用继续运行）· `-p`（彻底删除）。

## ✨ 功能

|                   | 能力                                                                                                             |
| ----------------- | ---------------------------------------------------------------------------------------------------------------- |
| 🖥️ **监控**       | 实时 CPU / 内存 / 磁盘 / 网络 / 进程，历史曲线；`/metrics` 直供 Prometheus                                       |
| 🐳 **Docker**     | 容器、镜像、网络、卷、Compose 全生命周期；`docker run` 命令一键转 Compose，存量容器导出收编                      |
| 🌐 **网站**       | Nginx / Apache / OpenLiteSpeed / OpenResty / Caddy 原生安装与一键切换；WordPress / Laravel / SPA 等伪静态预设    |
| 🛡️ **WAF**        | 五引擎统一规则：IP / UA 黑白名单、路径封禁、SQL 注入 / XSS 特征（CC 限频限 Nginx / OpenResty），命中日志回灌面板 |
| 🔐 **证书**       | 内置 ACME（HTTP-01 / DNS-01：Cloudflare、阿里云、腾讯云、华为云）、自签、上传，到期自动续签                      |
| 🗄️ **数据库**     | MySQL / MariaDB / PostgreSQL / Redis 安装与库、用户、配置管理                                                    |
| 🧩 **应用商店**   | 兼容 1Panel / 宝塔 / x-flame 三种包格式；远程源 + Ed25519 签名；安装前结构化安全扫描                             |
| 🚀 **项目部署**   | PHP / Node / Python / Go / Java 运行时；**git push 即部署**、Dockerfile 自动构建、健康检查零停机                 |
| 💾 **备份**       | 面板快照 + 应用定时备份，S3 / SFTP / 本地，凭据 AES-256-GCM 加密                                                 |
| 📁 **文件与终端** | 沙箱文件管理（上传下载、在线编辑、压缩解压、回收站）；xterm.js 终端（会话审计）                                  |
| 🔥 **防火墙**     | ufw / firewalld / iptables 统一管理                                                                              |
| 🛰️ **多节点**     | 轻量 Rust Agent，Bootstrap Token 注册、心跳、白名单命令                                                          |
| 🤖 **自动化**     | `fpctl` JSON CLI、MCP / 人工审批、可选面板助手（默认关闭）                                                       |
| 📋 **配置导出**   | 只读 IaC YAML 清单，按资源权限与字段白名单导出；不支持导入 / 部署                                                |
| 🌍 **体验**       | ⌘K 命令面板、新手引导、OKLCH 主题与三档材质、中 / 英 / 日                                                        |

## 🐧 系统支持

| 级别             | 发行版                                                                                                | 说明                                                   |
| ---------------- | ----------------------------------------------------------------------------------------------------- | ------------------------------------------------------ |
| ✅ **Tier 1**    | Debian 12 / 13 · Ubuntu 22.04 / 24.04                                                                 | 主支持与回归目标，实际验收见兼容矩阵                   |
| 🟡 **Tier 2**    | Rocky / Alma / RHEL / Oracle 9 · CentOS Stream 9 · Fedora · openEuler / Anolis / OpenCloudOS / UOS 等 | 面板与核心功能可用；原生 MySQL / PostgreSQL 有已知限制 |
| 🟠 **Tier 3**    | openSUSE · Arch / Manjaro                                                                             | 面板可用；部分原生软件需手工安装                       |
| 🐳 **仅 Docker** | Alpine（无 systemd）                                                                                  | 使用 Docker Compose 部署                               |

架构：amd64 / arm64 预编译；需要 systemd。完整矩阵见 [Doc/12](./Doc/12-安全基线与兼容矩阵.md)，各发行版差异与已知问题见 [Doc/33 §4](./Doc/33-代码审计与优化报告.md#4-发行版适配与支持程度)。

## 🏗️ 核心架构

```text
浏览器：Vue + Pinia + VueQuery → /api /ws → Axum handler → Service → 领域端口
                                               │                      ↑
                                      JWT / RBAC / 审计         SQLite / Docker / I/O 适配器
                                               ├─ 本机特权 helper（白名单）
                                               └─ 轻量 Agent（HTTP / 可选 Edge 出站通道）
```

组合根：`flame-kernel/src/lib.rs::build_services`；后端选择：`main.rs`（默认 SQLite）；权限：`api/permissions.rs`；实时采集：`MetricsHub`；前端装配：`frontend/src/main.ts`。

| 目录            | 核心职责                                                |
| --------------- | ------------------------------------------------------- |
| `flame-kernel/` | 领域规则 / 端口、Service、API、具体适配器、任务生命周期 |
| `frontend/`     | 页面组装、业务 composables、Fp 组件、生成 API 类型      |
| `agent/`        | 模块化轻量节点进程，命令 / 文件 / token 为独立安全边界  |
| `scripts/`      | 统一门禁、工具链检查、离线安装回归、云原生开发辅助      |

原生安装的非 root 边界不能直接套到 Docker。完整启动链、当前边界债务与特性裁剪见 [Doc/01](./Doc/01-架构设计.md)，待评审的死代码 / 重复逻辑见 [Doc/35](./Doc/35-工程清理清单.md)。

## 📚 文档

| 我想…                  | 读这个                                                                                                                                                   |
| ---------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------- |
| 部署、升级、备份、卸载 | [10 部署运维](./Doc/10-部署运维指南.md) · [13 故障排查](./Doc/13-故障排查手册.md)                                                                        |
| 了解安全基线与系统兼容 | [12 安全基线与兼容矩阵](./Doc/12-安全基线与兼容矩阵.md) · [33 代码审计与优化报告](./Doc/33-代码审计与优化报告.md)                                        |
| 理解代码结构           | [01 架构](./Doc/01-架构设计.md) · [02 API](./Doc/02-API接口文档.md) · [03 数据库](./Doc/03-数据库设计.md) · [07 权限](./Doc/07-权限体系设计.md)          |
| 写后端 / 前端 / 测试   | [05 后端](./Doc/05-后端开发指南.md) · [04 前端](./Doc/04-前端开发指南.md) · [06 测试](./Doc/06-测试指南.md) · [frontend/design.md](./frontend/design.md) |
| 做应用包 / 插件        | [20 应用商店](./Doc/20-应用商店与插件系统.md) · [21 SDK](./Doc/21-应用商店SDK开发指南.md) · [22 1Panel 兼容](./Doc/22-1Panel与原生软件兼容性.md)         |
| 多节点与事件           | [23 Agent 协议](./Doc/23-Agent节点通信协议.md) · [24 事件与通知](./Doc/24-事件与通知系统.md)                                                             |
| 看维护方向、评审清理   | [30 维护路线](./Doc/30-路线图与里程碑.md) · [35 清理清单](./Doc/35-工程清理清单.md) · [32 执行历史](./Doc/32-v1.x执行计划.md)                            |

完整索引：[Doc/README.md](./Doc/README.md) · 版本变更：[CHANGELOG.md](./CHANGELOG.md)

## 🤝 参与开发

工具链：**Rust 1.98.1**（`rust-toolchain.toml`），Node（`.node-version`），npm（`frontend/package.json`）；不使用 pnpm，不随清理升级依赖锁文件。

```bash
# 终端 1：仓库根，默认 SQLite
cargo run --locked -p flame-kernel --bin flame-kernel
# 终端 2：仓库根
cd frontend && npm ci && npm run dev
```

完整验证在 **CNB 云原生开发环境**执行：

```bash
bash scripts/ci-local.sh --keep-going all
```

门禁命令、阈值与分组以实际脚本 / 配置为准；`all` 不代替性能、Rust 安全审计、跨架构发布或真机验收。用户要求零云构建时，不通过 push、PR、tag 或普通 build-start 运行测试。

流程与红线：[Doc/31](./Doc/31-开发流程.md) · [AGENTS.md](./AGENTS.md)。当前只做测试装配、工具链、文档和先列清单的清理；后续功能需单独确认，见 [Doc/30](./Doc/30-路线图与里程碑.md)。

## 📄 License

[MIT](./LICENSE)
