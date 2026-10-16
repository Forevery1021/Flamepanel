<!-- 由 scripts/gen-api-docs.py 从 frontend/openapi.json 生成，请勿手改；修改 utoipa 注解后运行 just api-docs -->

# API · kernel-logs

> 6 个端点 · 通用约定见 [README](./README.md)

| 方法 | 路径 | 摘要 | 权限 |
|---|---|---|---|
| GET | `/api/kernel-logs` | `GET /api/kernel-logs` — 概览 | `log:read` |
| GET | `/api/kernel-logs/entries` | `GET /api/kernel-logs/entries` — 倒序分页读取 | `log:read` |
| DELETE | `/api/kernel-logs/files/{name}` | `DELETE /api/kernel-logs/files/{name}` — 删除历史日志文件 | `log:delete` |
| GET | `/api/kernel-logs/files/{name}/download` | `GET /api/kernel-logs/files/{name}/download` — 下载原始日志文件 | `log:read` |
| PUT | `/api/kernel-logs/level` | `PUT /api/kernel-logs/level` — 运行时调整内核日志级别 | `settings:update` |
| GET | `/api/kernel-logs/stats` | `GET /api/kernel-logs/stats` — 五级计数 | `log:read` |

### GET `/api/kernel-logs`

`GET /api/kernel-logs` — 概览

- 认证：Bearer JWT
- 权限：`log:read`
- operationId：`overview`

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 内核日志概览 | KernelLogOverview |
| 401 | 未认证 |  |
| 403 | 缺少 log:read 权限 |  |

<details><summary>响应示例</summary>

```json
{
  "dir": "string",
  "dropped_lines": 0,
  "enabled": false,
  "files": [
    {
      "current": false,
      "modified": "string",
      "name": "string",
      "size": 0
    }
  ],
  "level": "trace",
  "retention_days": 0
}
```

</details>

```bash
fpctl api GET '/api/kernel-logs' --json
```

### GET `/api/kernel-logs/entries`

`GET /api/kernel-logs/entries` — 倒序分页读取

- 认证：Bearer JWT
- 权限：`log:read`
- operationId：`entries`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `file` | query | string? | 否 | 文件名；缺省为当前正在写入的文件 |
| `levels` | query | string? | 否 | 级别（逗号分隔，如 `warn,error`）；缺省不过滤 |
| `q` | query | string? | 否 | 关键字（不区分大小写，匹配整行，含请求编号 / 用户名等 span 字段） |
| `target` | query | string? | 否 | target 前缀，如 `flame_kernel::api` |
| `limit` | query | integer? | 否 | 返回条数：缺省 200，范围 1–1000 |
| `before` | query | integer(int64)? | 否 | 续读游标（上一页的 `next_before`） |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 日志分页（最新在前） | LogPage |
| 400 | 文件名或级别非法 |  |
| 401 | 未认证 |  |
| 403 | 缺少 log:read 权限 |  |
| 404 | 文件不存在或文件日志未启用 |  |

<details><summary>响应示例</summary>

```json
{
  "entries": [
    {
      "fields": null,
      "level": "trace",
      "message": "string",
      "request_id": "string",
      "target": "string",
      "timestamp": "string"
    }
  ],
  "file": "string",
  "file_size": 0,
  "next_before": 0,
  "scanned_bytes": 0
}
```

</details>

```bash
fpctl api GET '/api/kernel-logs/entries' --json
```

### DELETE `/api/kernel-logs/files/{name}`

`DELETE /api/kernel-logs/files/{name}` — 删除历史日志文件

- 认证：Bearer JWT
- 权限：`log:delete`
- operationId：`delete_file`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `name` | path | string | 是 | 日志文件名 |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 已删除 |  |
| 400 | 文件名非法或为当前正在写入的文件 |  |
| 401 | 未认证 |  |
| 403 | 缺少 log:delete 权限 |  |
| 404 | 文件不存在 |  |

```bash
fpctl api DELETE '/api/kernel-logs/files/1' --json
```

### GET `/api/kernel-logs/files/{name}/download`

`GET /api/kernel-logs/files/{name}/download` — 下载原始日志文件

- 认证：Bearer JWT
- 权限：`log:read`
- operationId：`download`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `name` | path | string | 是 | 日志文件名 |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 原始 JSON Lines 文本 | (`text/plain`) |
| 400 | 文件名非法或文件过大 |  |
| 401 | 未认证 |  |
| 403 | 缺少 log:read 权限 |  |
| 404 | 文件不存在 |  |

```bash
fpctl api GET '/api/kernel-logs/files/1/download' --json
```

### PUT `/api/kernel-logs/level`

`PUT /api/kernel-logs/level` — 运行时调整内核日志级别

- 认证：Bearer JWT
- 权限：`settings:update`
- operationId：`set_level`

**请求体**（`application/json`，必填）：SetKernelLogLevelRequest

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `level` | string | 是 | `trace` / `debug` / `info` / `warn` / `error` |

```json
{
  "level": "string"
}
```

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 新级别已生效 | KernelLogLevelResponse |
| 400 | 级别非法 |  |
| 401 | 未认证 |  |
| 403 | 缺少 settings:update 权限 |  |
| 409 | 日志系统未初始化（嵌入式 / 测试进程） |  |

<details><summary>响应示例</summary>

```json
{
  "level": "trace"
}
```

</details>

```bash
fpctl api PUT '/api/kernel-logs/level' -d '{"level":"string"}' --json
```

### GET `/api/kernel-logs/stats`

`GET /api/kernel-logs/stats` — 五级计数

- 认证：Bearer JWT
- 权限：`log:read`
- operationId：`stats`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `file` | query | string | 是 | 文件名 |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 级别计数 | KernelLogStats |
| 400 | 文件名非法 |  |
| 401 | 未认证 |  |
| 403 | 缺少 log:read 权限 |  |
| 404 | 文件不存在 |  |

<details><summary>响应示例</summary>

```json
{
  "counts": {
    "debug": 0,
    "error": 0,
    "info": 0,
    "total": 0,
    "trace": 0,
    "warn": 0
  },
  "file": "string",
  "truncated": false
}
```

</details>

```bash
fpctl api GET '/api/kernel-logs/stats' --json
```
