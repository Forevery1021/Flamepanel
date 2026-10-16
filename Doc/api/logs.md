<!-- 由 scripts/gen-api-docs.py 从 frontend/openapi.json 生成，请勿手改；修改 utoipa 注解后运行 just api-docs -->

# API · logs

> 2 个端点 · 通用约定见 [README](./README.md)

| 方法 | 路径 | 摘要 | 权限 |
|---|---|---|---|
| GET | `/api/logs` | 系统事件日志列表 | `log:read` |
| DELETE | `/api/logs/{id}` | 删除一条系统事件日志 | `log:delete` |

### GET `/api/logs`

系统事件日志列表

- 认证：Bearer JWT
- 权限：`log:read`
- operationId：`list`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `page` | query | integer(int64)? | 否 | 页码（从 1 开始） |
| `page_size` | query | integer(int64)? | 否 | 每页条数（1~200） |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 系统事件日志（分页，新在前） | PaginatedResponse<LogEntry>_LogEntry |
| 401 | 未认证 |  |
| 403 | 无权限 |  |

<details><summary>响应示例</summary>

```json
{
  "data": [
    {
      "created_at": "2026-01-01T00:00:00Z",
      "id": 0,
      "level": "string",
      "message": "string",
      "metadata": "string",
      "source": "string"
    }
  ],
  "page": 0,
  "page_size": 0,
  "total": 0,
  "total_pages": 0
}
```

</details>

```bash
fpctl api GET '/api/logs' --json
```

### DELETE `/api/logs/{id}`

删除一条系统事件日志

- 认证：Bearer JWT
- 权限：`log:delete`
- operationId：`delete`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | ID |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 删除成功 | string (`text/plain`) |
| 401 | 未认证 |  |
| 403 | 无权限 |  |
| 404 | 日志不存在 |  |

```bash
fpctl api DELETE '/api/logs/1' --json
```
