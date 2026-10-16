<!-- 由 scripts/gen-api-docs.py 从 frontend/openapi.json 生成，请勿手改；修改 utoipa 注解后运行 just api-docs -->

# API · outbox

> 1 个端点 · 通用约定见 [README](./README.md)

| 方法 | 路径 | 摘要 | 权限 |
|---|---|---|---|
| GET | `/api/outbox-events` | `GET /api/outbox-events` — 分页查询事件落库（审计）。 | `outbox:read` |

### GET `/api/outbox-events`

`GET /api/outbox-events` — 分页查询事件落库（审计）。

- 认证：无
- 权限：`outbox:read`
- operationId：`list`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `page` | path | integer(int64)? | 是 | 页码（从 1 开始） |
| `page_size` | path | integer(int64)? | 是 | 每页条数（1~200） |
| `page` | query | integer(int64)? | 否 |  |
| `page_size` | query | integer(int64)? | 否 |  |
| `type` | query | string? | 否 | 按事件类型过滤（如 `type=AppInstalled` 精确匹配变体名） |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 事件落库分页列表 | PaginatedResponse<OutboxEvent>_OutboxEvent |

<details><summary>响应示例</summary>

```json
{
  "data": [
    {
      "created_at": "2026-01-01T00:00:00Z",
      "event_type": "string",
      "id": 0,
      "payload": "string",
      "published": false
    }
  ],
  "page": 0,
  "page_size": 0,
  "total": 0,
  "total_pages": 0
}
```

</details>
