<!-- 由 scripts/gen-api-docs.py 从 frontend/openapi.json 生成，请勿手改；修改 utoipa 注解后运行 just api-docs -->

# API · iac

> 1 个端点 · 通用约定见 [README](./README.md)

| 方法 | 路径 | 摘要 | 权限 |
|---|---|---|---|
| GET | `/api/iac/export` |  | `settings:read` |

### GET `/api/iac/export`

- 认证：Bearer JWT
- 权限：`settings:read`
- operationId：`exportIacConfiguration`

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 无凭据配置清单；无读权限的资源区块省略，不是可直接执行的部署声明 | IacManifest (`application/yaml`) |
| 401 | 未认证 |  |
| 403 | 缺少 settings:read |  |

```bash
fpctl api GET '/api/iac/export' --json
```
