<!-- 由 scripts/gen-api-docs.py 从 frontend/openapi.json 生成，请勿手改；修改 utoipa 注解后运行 just api-docs -->

# API · overview

> 1 个端点 · 通用约定见 [README](./README.md)

| 方法 | 路径 | 摘要 | 权限 |
|---|---|---|---|
| GET | `/api/overview/badges` | `GET /api/overview/badges` — 顶栏徽标聚合（节点 / 容器 / 任务 / 告警一次返回） | 登录即可 |

### GET `/api/overview/badges`

`GET /api/overview/badges` — 顶栏徽标聚合（节点 / 容器 / 任务 / 告警一次返回）

服务端 10 s TTL 缓存 + 单飞：多页签并发轮询只触发一次真实聚合；`cache_age_secs > 0` 表示命中缓存。

- 认证：Bearer JWT
- 权限：登录即可
- operationId：`badges`

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 顶栏徽标聚合快照 | OverviewBadges |
| 401 | 未认证 |  |

<details><summary>响应示例</summary>

```json
{
  "alerts": {
    "certificates_expiring": 0,
    "nodes_offline": 0,
    "tasks_failed": 0,
    "total": 0
  },
  "cache_age_secs": 0,
  "containers": {
    "running": 0,
    "total": 0
  },
  "generated_at": "2026-01-01T00:00:00Z",
  "nodes": {
    "online": 0,
    "total": 0
  },
  "tasks": {
    "failed": 0,
    "pending": 0,
    "running": 0
  }
}
```

</details>

```bash
fpctl api GET '/api/overview/badges' --json
```
