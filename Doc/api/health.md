<!-- 由 scripts/gen-api-docs.py 从 frontend/openapi.json 生成，请勿手改；修改 utoipa 注解后运行 just api-docs -->

# API · health

> 2 个端点 · 通用约定见 [README](./README.md)

| 方法 | 路径 | 摘要 | 权限 |
|---|---|---|---|
| GET | `/api/health` | `GET /api/health` — 详细健康检查（免认证） | 公开（无需登录） |
| GET | `/health` | 存活探针：进程能响应即返回纯文本 `OK`（负载均衡 / 容器健康检查用；详细检查见 `/api/health`） | 公开（无需登录） |

### GET `/api/health`

`GET /api/health` — 详细健康检查（免认证）

- 认证：无
- 权限：公开（无需登录）
- operationId：`detail`

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 健康状态 | HealthDetail |

<details><summary>响应示例</summary>

```json
{
  "checks": {
    "database": {
      "detail": "string",
      "status": "string"
    },
    "disk": {
      "detail": "string",
      "status": "string"
    },
    "docker": {
      "detail": "string",
      "status": "string"
    }
  },
  "status": "string",
  "uptime_secs": 0,
  "version": "string"
}
```

</details>

### GET `/health`

存活探针：进程能响应即返回纯文本 `OK`（负载均衡 / 容器健康检查用；详细检查见 `/api/health`）

- 认证：无
- 权限：公开（无需登录）
- operationId：`liveness`

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | OK | string (`text/plain`) |
