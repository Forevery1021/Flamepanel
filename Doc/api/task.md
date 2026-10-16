<!-- 由 scripts/gen-api-docs.py 从 frontend/openapi.json 生成，请勿手改；修改 utoipa 注解后运行 just api-docs -->

# API · task

> 4 个端点 · 通用约定见 [README](./README.md)

| 方法 | 路径 | 摘要 | 权限 |
|---|---|---|---|
| GET | `/api/tasks` | 列出全部任务（可按状态过滤）。 | `task:read` |
| POST | `/api/tasks/prune` | 清理全部终态任务。 | `task:delete` |
| GET | `/api/tasks/{id}` | 查询单个任务。 | `task:read` |
| POST | `/api/tasks/{id}/cancel` | 取消任务（`Pending → Cancelled` / `Running → Cancelled`）。 | `task:execute` |

### GET `/api/tasks`

列出全部任务（可按状态过滤）。

- 认证：Bearer JWT
- 权限：`task:read`
- operationId：`list_tasks`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `state` | query | string | 否 | 按状态过滤：pending/running/success/failed/cancelled |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 任务列表 | TaskListResponse |
| 401 | 未认证 |  |

<details><summary>响应示例</summary>

```json
{
  "tasks": [
    {
      "created_at": "2026-01-01T00:00:00Z",
      "id": 0,
      "kind": "install",
      "message": "string",
      "name": "string",
      "progress": 0,
      "state": "pending",
      "updated_at": "2026-01-01T00:00:00Z"
    }
  ],
  "total": 0
}
```

</details>

```bash
fpctl api GET '/api/tasks' --json
```

### POST `/api/tasks/prune`

清理全部终态任务。

- 认证：Bearer JWT
- 权限：`task:delete`
- operationId：`prune_tasks`

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 清理数量 |  |

```bash
fpctl api POST '/api/tasks/prune' --json
```

### GET `/api/tasks/{id}`

查询单个任务。

- 认证：Bearer JWT
- 权限：`task:read`
- operationId：`get_task`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 任务 id |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 任务详情 | TaskRecord |
| 404 | 任务不存在 |  |

<details><summary>响应示例</summary>

```json
{
  "created_at": "2026-01-01T00:00:00Z",
  "id": 0,
  "kind": "install",
  "message": "string",
  "name": "string",
  "progress": 0,
  "state": "pending",
  "updated_at": "2026-01-01T00:00:00Z"
}
```

</details>

```bash
fpctl api GET '/api/tasks/1' --json
```

### POST `/api/tasks/{id}/cancel`

取消任务（`Pending → Cancelled` / `Running → Cancelled`）。

- 认证：Bearer JWT
- 权限：`task:execute`
- operationId：`cancel_task`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 任务 id |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 已取消 | TaskRecord |
| 404 | 任务不存在 |  |
| 409 | 任务已处于终态，无法取消 |  |

<details><summary>响应示例</summary>

```json
{
  "created_at": "2026-01-01T00:00:00Z",
  "id": 0,
  "kind": "install",
  "message": "string",
  "name": "string",
  "progress": 0,
  "state": "pending",
  "updated_at": "2026-01-01T00:00:00Z"
}
```

</details>

```bash
fpctl api POST '/api/tasks/1/cancel' --json
```
