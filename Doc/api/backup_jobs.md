<!-- 由 scripts/gen-api-docs.py 从 frontend/openapi.json 生成，请勿手改；修改 utoipa 注解后运行 just api-docs -->

# API · backup_jobs

> 7 个端点 · 通用约定见 [README](./README.md)

| 方法 | 路径 | 摘要 | 权限 |
|---|---|---|---|
| GET | `/api/backup-jobs` | 备份任务列表 | `backup:read` |
| POST | `/api/backup-jobs` | 创建备份任务 | `backup:create` |
| POST | `/api/backup-jobs/restore-cloud` | 从云端账号直接拉取并恢复（按 key 前缀分流：面板库 .db / 应用数据 app/<key>/*.tar.gz） | `backup:create` |
| PUT | `/api/backup-jobs/{id}` | 更新备份任务 | `backup:update` |
| DELETE | `/api/backup-jobs/{id}` | 删除备份任务 | `backup:delete` |
| PUT | `/api/backup-jobs/{id}/enabled` | 启停备份任务 | `backup:update` |
| POST | `/api/backup-jobs/{id}/run` | 立即执行一次 | `backup:update` |

### GET `/api/backup-jobs`

备份任务列表

- 认证：Bearer JWT
- 权限：`backup:read`
- operationId：`list_jobs`

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 任务列表 | JobDto[] |
| 401 | 未认证 |  |

<details><summary>响应示例</summary>

```json
[
  {
    "created_at": "2026-01-01T00:00:00Z",
    "enabled": false,
    "id": 0,
    "last_message": "string",
    "last_remote_key": "string",
    "last_run_at": "2026-01-01T00:00:00Z",
    "last_status": "string",
    "name": "string",
    "next_run_at": "2026-01-01T00:00:00Z",
    "retention": 0,
    "schedule": "string",
    "scope": "string",
    "scope_key": "string",
    "target_account_id": 0,
    "updated_at": "2026-01-01T00:00:00Z"
  }
]
```

</details>

```bash
fpctl api GET '/api/backup-jobs' --json
```

### POST `/api/backup-jobs`

创建备份任务

- 认证：Bearer JWT
- 权限：`backup:create`
- operationId：`create_job`

**请求体**（`application/json`，必填）：CreateJobRequest

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `enabled` | boolean | 否 |  |
| `name` | string | 是 |  |
| `retention` | integer(int64)? | 否 | 备份保留份数（0 = 用默认 7） |
| `schedule` | string | 是 | 标准 5 字段 cron 表达式 |
| `scope` | string | 是 | 备份范围：panel（面板库）\| app（应用目录） |
| `scope_key` | string? | 否 | 范围内具体对象：panel 省略；app 为应用 key |
| `target_account_id` | integer(int64) | 是 | 目标备份账号 ID |

```json
{
  "enabled": false,
  "name": "string",
  "retention": 0,
  "schedule": "string",
  "scope": "string",
  "scope_key": "string",
  "target_account_id": 0
}
```

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 创建成功 | JobDto |
| 400 | 参数错误（cron 非法 / 账号不存在） |  |

<details><summary>响应示例</summary>

```json
{
  "created_at": "2026-01-01T00:00:00Z",
  "enabled": false,
  "id": 0,
  "last_message": "string",
  "last_remote_key": "string",
  "last_run_at": "2026-01-01T00:00:00Z",
  "last_status": "string",
  "name": "string",
  "next_run_at": "2026-01-01T00:00:00Z",
  "retention": 0,
  "schedule": "string",
  "scope": "string",
  "scope_key": "string",
  "target_account_id": 0,
  "updated_at": "2026-01-01T00:00:00Z"
}
```

</details>

```bash
fpctl api POST '/api/backup-jobs' -d '{"enabled":false,"name":"string","retention":0,"schedule":"string","scope":"string","scope_key":"string","target_account_id":0}' --json
```

### POST `/api/backup-jobs/restore-cloud`

从云端账号直接拉取并恢复（按 key 前缀分流：面板库 .db / 应用数据 app/<key>/*.tar.gz）

- 认证：Bearer JWT
- 权限：`backup:create`
- operationId：`restore_from_cloud`

**请求体**（`application/json`，必填）：RestoreFromCloudRequest

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `account_id` | integer(int64) | 是 | 目标备份账号 ID |
| `remote_key` | string | 是 | 远端对象 key（如 panel/flamepanel-20260907.db） |

```json
{
  "account_id": 0,
  "remote_key": "string"
}
```

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 恢复完成，返回范围/对象名与生效提示 | RestoreFromCloudResponse |
| 400 | 参数错误（key 非法 / 包格式不支持 / 包内容不安全） |  |
| 404 | 账号、远端对象或目标应用不存在 |  |

<details><summary>响应示例</summary>

```json
{
  "filename": "string",
  "message": "string",
  "scope": "string"
}
```

</details>

```bash
fpctl api POST '/api/backup-jobs/restore-cloud' -d '{"account_id":0,"remote_key":"string"}' --json
```

### PUT `/api/backup-jobs/{id}`

更新备份任务

- 认证：Bearer JWT
- 权限：`backup:update`
- operationId：`update_job`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 任务 ID |

**请求体**（`application/json`，必填）：UpdateJobRequest

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `enabled` | boolean? | 否 |  |
| `name` | string? | 否 |  |
| `retention` | integer(int64)? | 否 |  |
| `schedule` | string? | 否 |  |
| `scope` | string? | 否 |  |
| `scope_key` | string? | 否 |  |
| `target_account_id` | integer(int64)? | 否 |  |

```json
{
  "enabled": false,
  "name": "string",
  "retention": 0,
  "schedule": "string",
  "scope": "string",
  "scope_key": "string",
  "target_account_id": 0
}
```

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 更新成功 | JobDto |
| 404 | 任务不存在 |  |

<details><summary>响应示例</summary>

```json
{
  "created_at": "2026-01-01T00:00:00Z",
  "enabled": false,
  "id": 0,
  "last_message": "string",
  "last_remote_key": "string",
  "last_run_at": "2026-01-01T00:00:00Z",
  "last_status": "string",
  "name": "string",
  "next_run_at": "2026-01-01T00:00:00Z",
  "retention": 0,
  "schedule": "string",
  "scope": "string",
  "scope_key": "string",
  "target_account_id": 0,
  "updated_at": "2026-01-01T00:00:00Z"
}
```

</details>

```bash
fpctl api PUT '/api/backup-jobs/1' -d '{"enabled":false,"name":"string","retention":0,"schedule":"string","scope":"string","scope_key":"string","target_account_id":0}' --json
```

### DELETE `/api/backup-jobs/{id}`

删除备份任务

- 认证：Bearer JWT
- 权限：`backup:delete`
- operationId：`delete_job`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 任务 ID |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 删除成功 |  |
| 404 | 任务不存在 |  |

```bash
fpctl api DELETE '/api/backup-jobs/1' --json
```

### PUT `/api/backup-jobs/{id}/enabled`

启停备份任务

- 认证：Bearer JWT
- 权限：`backup:update`
- operationId：`toggle_job`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 任务 ID |

**请求体**（`application/json`，必填）：ToggleJobRequest

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
| 200 | 已更新 | JobDto |
| 404 | 任务不存在 |  |

<details><summary>响应示例</summary>

```json
{
  "created_at": "2026-01-01T00:00:00Z",
  "enabled": false,
  "id": 0,
  "last_message": "string",
  "last_remote_key": "string",
  "last_run_at": "2026-01-01T00:00:00Z",
  "last_status": "string",
  "name": "string",
  "next_run_at": "2026-01-01T00:00:00Z",
  "retention": 0,
  "schedule": "string",
  "scope": "string",
  "scope_key": "string",
  "target_account_id": 0,
  "updated_at": "2026-01-01T00:00:00Z"
}
```

</details>

```bash
fpctl api PUT '/api/backup-jobs/1/enabled' -d '{"enabled":false}' --json
```

### POST `/api/backup-jobs/{id}/run`

立即执行一次

- 认证：Bearer JWT
- 权限：`backup:update`
- operationId：`run_job`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 任务 ID |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 执行完成，返回任务（含执行结果） | JobDto |
| 404 | 任务不存在 |  |

<details><summary>响应示例</summary>

```json
{
  "created_at": "2026-01-01T00:00:00Z",
  "enabled": false,
  "id": 0,
  "last_message": "string",
  "last_remote_key": "string",
  "last_run_at": "2026-01-01T00:00:00Z",
  "last_status": "string",
  "name": "string",
  "next_run_at": "2026-01-01T00:00:00Z",
  "retention": 0,
  "schedule": "string",
  "scope": "string",
  "scope_key": "string",
  "target_account_id": 0,
  "updated_at": "2026-01-01T00:00:00Z"
}
```

</details>

```bash
fpctl api POST '/api/backup-jobs/1/run' --json
```
