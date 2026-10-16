<!-- 由 scripts/gen-api-docs.py 从 frontend/openapi.json 生成，请勿手改；修改 utoipa 注解后运行 just api-docs -->

# API · scheduled_tasks

> 7 个端点 · 通用约定见 [README](./README.md)

| 方法 | 路径 | 摘要 | 权限 |
|---|---|---|---|
| GET | `/api/scheduled-tasks` | 定时任务列表（分页） | `scheduled_task:read` |
| POST | `/api/scheduled-tasks` | 创建定时任务 | `scheduled_task:create` |
| GET | `/api/scheduled-tasks/{id}` | 定时任务详情 | `scheduled_task:read` |
| PUT | `/api/scheduled-tasks/{id}` | 更新定时任务 | `scheduled_task:update` |
| DELETE | `/api/scheduled-tasks/{id}` | 删除定时任务 | `scheduled_task:delete` |
| POST | `/api/scheduled-tasks/{id}/run` | 立即执行定时任务 | `scheduled_task:execute` |
| POST | `/api/scheduled-tasks/{id}/toggle` | 启用 / 停用定时任务 | `scheduled_task:update` |

### GET `/api/scheduled-tasks`

定时任务列表（分页）

- 认证：Bearer JWT
- 权限：`scheduled_task:read`
- operationId：`list_tasks`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `page` | query | integer(int64)? | 否 | 页码（从 1 开始） |
| `page_size` | query | integer(int64)? | 否 | 每页条数（1~200） |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 任务列表 | PaginatedResponse<ScheduledTask>_ScheduledTask |
| 401 | 未认证 |  |

<details><summary>响应示例</summary>

```json
{
  "data": [
    {
      "command": "string",
      "created_at": "2026-01-01T00:00:00Z",
      "enabled": false,
      "id": 0,
      "last_output": "string",
      "last_run_at": "2026-01-01T00:00:00Z",
      "last_status": "string",
      "name": "string",
      "next_run_at": "2026-01-01T00:00:00Z",
      "schedule": "string",
      "updated_at": "2026-01-01T00:00:00Z"
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
fpctl api GET '/api/scheduled-tasks' --json
```

### POST `/api/scheduled-tasks`

创建定时任务

- 认证：Bearer JWT
- 权限：`scheduled_task:create`
- operationId：`create_task`

**请求体**（`application/json`，必填）：CreateTaskRequest

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `command` | string | 是 |  |
| `enabled` | boolean? | 否 |  |
| `name` | string | 是 |  |
| `schedule` | string? | 否 |  |

```json
{
  "command": "string",
  "enabled": false,
  "name": "string",
  "schedule": "string"
}
```

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 创建成功 | ScheduledTask |
| 400 | 参数错误 |  |

<details><summary>响应示例</summary>

```json
{
  "command": "string",
  "created_at": "2026-01-01T00:00:00Z",
  "enabled": false,
  "id": 0,
  "last_output": "string",
  "last_run_at": "2026-01-01T00:00:00Z",
  "last_status": "string",
  "name": "string",
  "next_run_at": "2026-01-01T00:00:00Z",
  "schedule": "string",
  "updated_at": "2026-01-01T00:00:00Z"
}
```

</details>

```bash
fpctl api POST '/api/scheduled-tasks' -d '{"command":"string","enabled":false,"name":"string","schedule":"string"}' --json
```

### GET `/api/scheduled-tasks/{id}`

定时任务详情

- 认证：Bearer JWT
- 权限：`scheduled_task:read`
- operationId：`get_task`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 任务 ID |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 任务详情 | ScheduledTask |
| 401 | 未认证 |  |
| 404 | 任务不存在 |  |

<details><summary>响应示例</summary>

```json
{
  "command": "string",
  "created_at": "2026-01-01T00:00:00Z",
  "enabled": false,
  "id": 0,
  "last_output": "string",
  "last_run_at": "2026-01-01T00:00:00Z",
  "last_status": "string",
  "name": "string",
  "next_run_at": "2026-01-01T00:00:00Z",
  "schedule": "string",
  "updated_at": "2026-01-01T00:00:00Z"
}
```

</details>

```bash
fpctl api GET '/api/scheduled-tasks/1' --json
```

### PUT `/api/scheduled-tasks/{id}`

更新定时任务

- 认证：Bearer JWT
- 权限：`scheduled_task:update`
- operationId：`update_task`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 任务 ID |

**请求体**（`application/json`，必填）：UpdateTaskRequest

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `command` | string? | 否 |  |
| `enabled` | boolean? | 否 |  |
| `name` | string? | 否 |  |
| `schedule` | string? | 否 |  |

```json
{
  "command": "string",
  "enabled": false,
  "name": "string",
  "schedule": "string"
}
```

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 更新成功 | ScheduledTask |
| 404 | 任务不存在 |  |

<details><summary>响应示例</summary>

```json
{
  "command": "string",
  "created_at": "2026-01-01T00:00:00Z",
  "enabled": false,
  "id": 0,
  "last_output": "string",
  "last_run_at": "2026-01-01T00:00:00Z",
  "last_status": "string",
  "name": "string",
  "next_run_at": "2026-01-01T00:00:00Z",
  "schedule": "string",
  "updated_at": "2026-01-01T00:00:00Z"
}
```

</details>

```bash
fpctl api PUT '/api/scheduled-tasks/1' -d '{"command":"string","enabled":false,"name":"string","schedule":"string"}' --json
```

### DELETE `/api/scheduled-tasks/{id}`

删除定时任务

- 认证：Bearer JWT
- 权限：`scheduled_task:delete`
- operationId：`delete_task`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 任务 ID |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 删除成功 |  |
| 404 | 任务不存在 |  |

```bash
fpctl api DELETE '/api/scheduled-tasks/1' --json
```

### POST `/api/scheduled-tasks/{id}/run`

立即执行定时任务

- 认证：Bearer JWT
- 权限：`scheduled_task:execute`
- operationId：`run_task`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 任务 ID |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 执行结果 | ScheduledTask |
| 404 | 任务不存在 |  |

<details><summary>响应示例</summary>

```json
{
  "command": "string",
  "created_at": "2026-01-01T00:00:00Z",
  "enabled": false,
  "id": 0,
  "last_output": "string",
  "last_run_at": "2026-01-01T00:00:00Z",
  "last_status": "string",
  "name": "string",
  "next_run_at": "2026-01-01T00:00:00Z",
  "schedule": "string",
  "updated_at": "2026-01-01T00:00:00Z"
}
```

</details>

```bash
fpctl api POST '/api/scheduled-tasks/1/run' --json
```

### POST `/api/scheduled-tasks/{id}/toggle`

启用 / 停用定时任务

- 认证：Bearer JWT
- 权限：`scheduled_task:update`
- operationId：`toggle_task`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 任务 ID |

**请求体**（`application/json`，必填）：ToggleTaskRequest

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `enabled` | boolean | 是 |  |

```json
{
  "enabled": false
}
```

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 更新后的任务 | ScheduledTask |
| 401 | 未认证 |  |
| 404 | 任务不存在 |  |

<details><summary>响应示例</summary>

```json
{
  "command": "string",
  "created_at": "2026-01-01T00:00:00Z",
  "enabled": false,
  "id": 0,
  "last_output": "string",
  "last_run_at": "2026-01-01T00:00:00Z",
  "last_status": "string",
  "name": "string",
  "next_run_at": "2026-01-01T00:00:00Z",
  "schedule": "string",
  "updated_at": "2026-01-01T00:00:00Z"
}
```

</details>

```bash
fpctl api POST '/api/scheduled-tasks/1/toggle' -d '{"enabled":false}' --json
```
