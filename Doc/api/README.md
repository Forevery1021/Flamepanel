<!-- 由 scripts/gen-api-docs.py 从 frontend/openapi.json 生成，请勿手改；修改 utoipa 注解后运行 just api-docs -->

# FlamePanel REST API 参考

> 版本 1.0.0 · 328 个已文档化端点 · 覆盖率 100.0%（[未文档化清单](./coverage.md)）

## 通用约定

- **基础地址**：`http://<面板>/api/...`；机器可读文档 `GET /api/openapi.json`（无需登录），面板内浏览：侧边栏「API 文档」（`/api-docs`）。
- **认证**：`POST /api/auth/login` 取 `access_token`，之后请求头 `Authorization: Bearer <token>`；部署主机上可直接用 `fpctl api <METHOD> <PATH>`（现签 60 s 管理员令牌，免登录，走完整鉴权与审计）。
- **权限**：每个端点标注 `resource:action`（与 `flame-kernel/src/api/permissions.rs` 同源，角色授权见 [Doc/07](../07-权限体系设计.md)）；「登录即可」表示只校验身份。
- **错误**：非 2xx 响应体为 `{"code": "…", "error": "…", "message": "…"}`，响应头 `x-request-id` 可在「系统日志 → 内核日志」或 `fpctl logs --grep <id>` 中检索。常见状态：400 参数错误 · 401 未登录 / 令牌失效 · 403 无权限或需先修改初始密码 · 404 不存在 · 409 冲突 · 422 校验失败 · 429 频率限制。
- **时间**：RFC 3339（UTC）。**分页**：`page` / `page_size`（各端点参数表为准）。

## 按模块

| 模块 | 端点数 | 文档 |
|---|---|---|
| app_store | 26 | [app_store.md](./app_store.md) |
| assistant | 4 | [assistant.md](./assistant.md) |
| auth | 12 | [auth.md](./auth.md) |
| backup_accounts | 8 | [backup_accounts.md](./backup_accounts.md) |
| backup_jobs | 7 | [backup_jobs.md](./backup_jobs.md) |
| backups | 5 | [backups.md](./backups.md) |
| certificates | 10 | [certificates.md](./certificates.md) |
| databases | 18 | [databases.md](./databases.md) |
| dns_providers | 5 | [dns_providers.md](./dns_providers.md) |
| docker | 35 | [docker.md](./docker.md) |
| files | 18 | [files.md](./files.md) |
| firewall | 11 | [firewall.md](./firewall.md) |
| health | 2 | [health.md](./health.md) |
| iac | 1 | [iac.md](./iac.md) |
| inspection | 2 | [inspection.md](./inspection.md) |
| kernel-logs | 6 | [kernel-logs.md](./kernel-logs.md) |
| logs | 2 | [logs.md](./logs.md) |
| mcp | 15 | [mcp.md](./mcp.md) |
| memos | 4 | [memos.md](./memos.md) |
| metrics | 3 | [metrics.md](./metrics.md) |
| nodes | 15 | [nodes.md](./nodes.md) |
| operation-logs | 3 | [operation-logs.md](./operation-logs.md) |
| outbox | 1 | [outbox.md](./outbox.md) |
| overview | 1 | [overview.md](./overview.md) |
| plugins | 16 | [plugins.md](./plugins.md) |
| projects | 21 | [projects.md](./projects.md) |
| runtimes | 8 | [runtimes.md](./runtimes.md) |
| scheduled_tasks | 7 | [scheduled_tasks.md](./scheduled_tasks.md) |
| settings | 5 | [settings.md](./settings.md) |
| setup | 2 | [setup.md](./setup.md) |
| task | 4 | [task.md](./task.md) |
| toolbox | 3 | [toolbox.md](./toolbox.md) |
| users | 5 | [users.md](./users.md) |
| waf | 9 | [waf.md](./waf.md) |
| web-servers | 23 | [web-servers.md](./web-servers.md) |
| websites | 11 | [websites.md](./websites.md) |

## 维护

```bash
just openapi     # 重新导出 frontend/openapi.json（后端注解变更后）
just api-docs    # 重新生成 Doc/api/（本目录）
```

CI（CNB installer-regression / GitHub ci-cd）执行 `python3 scripts/gen-api-docs.py --check`，生成物与快照不一致即失败。
