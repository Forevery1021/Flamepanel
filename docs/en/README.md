# FlamePanel documentation (English)

[English project overview](../../README.md) · [简体中文文档](../../Doc/README.md)

## Translation status

The default project README and agent agreement are English, with Chinese copies retained. The 27 numbered guides below are **condensed English translations**, not complete line-by-line translations. They preserve the main operating/development boundaries but omit detailed tables, examples, and historical evidence. The complete Chinese originals remain authoritative for omitted detail.

**Still pending:** unabridged guide translation; generated API descriptions/reference; CHANGELOG; `frontend/design.md`; auxiliary `docs/*.md`; skills and IDE/image guidance. Do not count these as fully translated.

| Guide | English | Complete Chinese | Status |
| --- | --- | --- | --- |
| 01 | [Architecture and code navigation](./01-architecture.md) | [01-架构设计.md](../../Doc/01-架构设计.md) | Condensed |
| 02 | [REST and WebSocket API](./02-api.md) | [02-API接口文档.md](../../Doc/02-API接口文档.md) | Condensed |
| 03 | [Database design and migrations](./03-database.md) | [03-数据库设计.md](../../Doc/03-数据库设计.md) | Condensed |
| 04 | [Frontend development](./04-frontend.md) | [04-前端开发指南.md](../../Doc/04-前端开发指南.md) | Condensed |
| 05 | [Backend development](./05-backend.md) | [05-后端开发指南.md](../../Doc/05-后端开发指南.md) | Condensed |
| 06 | [Testing and quality gates](./06-testing.md) | [06-测试指南.md](../../Doc/06-测试指南.md) | Condensed |
| 07 | [JWT authentication and RBAC](./07-permissions.md) | [07-权限体系设计.md](../../Doc/07-权限体系设计.md) | Condensed |
| 10 | [Deployment and operations](./10-deployment.md) | [10-部署运维指南.md](../../Doc/10-部署运维指南.md) | Condensed |
| 12 | [Security baseline and compatibility](./12-security-compatibility.md) | [12-安全基线与兼容矩阵.md](../../Doc/12-安全基线与兼容矩阵.md) | Condensed |
| 13 | [Troubleshooting](./13-troubleshooting.md) | [13-故障排查手册.md](../../Doc/13-故障排查手册.md) | Condensed |
| 20 | [App store and WASM plugins](./20-app-store.md) | [20-应用商店与插件系统.md](../../Doc/20-应用商店与插件系统.md) | Condensed |
| 21 | [App package and plugin SDK](./21-app-sdk.md) | [21-应用商店SDK开发指南.md](../../Doc/21-应用商店SDK开发指南.md) | Condensed |
| 22 | [1Panel, aaPanel, and native compatibility](./22-app-compatibility.md) | [22-1Panel与原生软件兼容性.md](../../Doc/22-1Panel与原生软件兼容性.md) | Condensed |
| 23 | [Agent communication and Edge](./23-agent-protocol.md) | [23-Agent节点通信协议.md](../../Doc/23-Agent节点通信协议.md) | Condensed |
| 24 | [Events, notifications, and persistent tasks](./24-events.md) | [24-事件与通知系统.md](../../Doc/24-事件与通知系统.md) | Condensed |
| 30 | [Maintenance roadmap](./30-roadmap.md) | [30-路线图与里程碑.md](../../Doc/30-路线图与里程碑.md) | Condensed |
| 31 | [Development workflow](./31-development-workflow.md) | [31-开发流程.md](../../Doc/31-开发流程.md) | Condensed |
| 32 | [Historical v1.x execution plan](./32-execution-history.md) | [32-v1.x执行计划.md](../../Doc/32-v1.x执行计划.md) | Condensed |
| 33 | [Code audit and optimization history](./33-code-audit.md) | [33-代码审计与优化报告.md](../../Doc/33-代码审计与优化报告.md) | Condensed |
| 34 | [Built-in MCP server](./34-mcp.md) | [34-MCP-Server接入指南.md](../../Doc/34-MCP-Server接入指南.md) | Condensed |
| 35 | [Engineering cleanup evidence](./35-engineering-cleanup.md) | [35-工程清理清单.md](../../Doc/35-工程清理清单.md) | Condensed |
| 36 | [Product proposals and scope boundaries](./36-product-proposals.md) | [36-后续产品路线规划.md](../../Doc/36-后续产品路线规划.md) | Condensed |
| 37 | [Isolated SQLite recovery drill](./37-sqlite-recovery-drill.md) | [37-SQLite隔离恢复演练.md](../../Doc/37-SQLite隔离恢复演练.md) | Condensed |
| 38 | [Dedicated recovery probe and HTTP acceptance](./38-recovery-probe.md) | [38-恢复探针启动隔离.md](../../Doc/38-恢复探针启动隔离.md) | Condensed |
| 39 | [Recovery assets and external key inventory](./39-recovery-assets.md) | [39-恢复资产与外部密钥清单.md](../../Doc/39-恢复资产与外部密钥清单.md) | Condensed |
| 40 | [Explicitly authorized encryption-key matching](./40-recovery-keycheck.md) | [40-恢复主密钥匹配验证.md](../../Doc/40-恢复主密钥匹配验证.md) | Condensed |
| 41 | [Recovery evidence preflight and next acceptance stages](./41-recovery-preflight.md) | [41-恢复证据预检与后续验收路线.md](../../Doc/41-恢复证据预检与后续验收路线.md) | Condensed |

## GitHub migration

[Migration, binary assets, and publication boundaries](./github-migration.md).

## Sources of truth

- Toolchains: `rust-toolchain.toml`, `.node-version`, `frontend/package.json`.
- Dependencies: `Cargo.lock`, `frontend/package-lock.json`.
- API: handler annotations/registration → `frontend/openapi.json` → generated reference/types.
- Permissions: `flame-kernel/src/api/permissions.rs`; migrations: SQLite implementation.
- Gates: `scripts/ci-local.sh` and the actual coverage/bundle configuration.
- Releases: actual tags/releases and CHANGELOG, not roadmap headings.

Before running a sensitive command, read its full reference and current help. English documentation does not authorize production secret reads, kernel startup against restored data, merges, tags, or publication.
