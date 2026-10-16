<!-- 由 scripts/gen-api-docs.py 从 frontend/openapi.json 生成，请勿手改；修改 utoipa 注解后运行 just api-docs -->

# API · nodes

> 15 个端点 · 通用约定见 [README](./README.md)

| 方法 | 路径 | 摘要 | 权限 |
|---|---|---|---|
| GET | `/api/nodes` | 节点列表（分页） | `node:read` |
| POST | `/api/nodes` | 注册节点 | `node:create` |
| POST | `/api/nodes/batch-execute` | 批量在多个节点并行执行命令 | `node:execute` |
| POST | `/api/nodes/heartbeat/{id}` | Agent 心跳上报（免 JWT，Agent token 已在中间件集中校验——S15） | 公开（无需登录） |
| POST | `/api/nodes/register` | Agent 注册（白名单免 JWT，供 Agent 启动时自动注册） 请求体：`{name, host, agent_port, auth_token}`，返回 `{"id": <node_id>}` | `node:create` |
| PUT | `/api/nodes/{id}` | 更新节点 | `node:update` |
| DELETE | `/api/nodes/{id}` | 删除节点 | `node:delete` |
| POST | `/api/nodes/{id}/action` | 调用 Agent 动作枚举（Phase A1：安全动作白名单） | `node:execute` |
| POST | `/api/nodes/{id}/execute` | 在远程节点执行命令 | `node:execute` |
| GET | `/api/nodes/{id}/files` | 列出远程节点目录 | `node:execute` |
| GET | `/api/nodes/{id}/files/download` | 下载远程节点文件 | `node:execute` |
| POST | `/api/nodes/{id}/files/upload` | 上传文件到远程节点 | `node:execute` |
| GET | `/api/nodes/{id}/metrics` | 节点最近指标快照 | `node:read` |
| GET | `/api/nodes/{id}/metrics/history` | 节点指标历史（内存窗口：Agent 10 s 一点、最多 120 点 ≈ 20 分钟；面板重启后重新积累） | `node:read` |
| GET | `/api/nodes/{id}/status` | 节点在线状态（惰性判定） | `node:read` |

### GET `/api/nodes`

节点列表（分页）

- 认证：Bearer JWT
- 权限：`node:read`
- operationId：`list_nodes`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `page` | query | integer(int64)? | 否 | 页码（从 1 开始） |
| `page_size` | query | integer(int64)? | 否 | 每页条数（1~200） |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 节点列表 | PaginatedResponse<ServerNode>_ServerNode |
| 401 | 未认证 |  |

<details><summary>响应示例</summary>

```json
{
  "data": [
    {
      "agent_port": 0,
      "auth_token": "string",
      "created_at": "2026-01-01T00:00:00Z",
      "hostname": "string",
      "id": 0,
      "ip_address": "string",
      "last_heartbeat_at": "2026-01-01T00:00:00Z",
      "metrics_json": "string",
      "name": "string",
      "status": "string"
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
fpctl api GET '/api/nodes' --json
```

### POST `/api/nodes`

注册节点

- 认证：Bearer JWT
- 权限：`node:create`
- operationId：`create_node`

**请求体**（`application/json`，必填）：CreateNodeRequest

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `agent_port` | integer(int32) | 否 | Agent HTTP 服务端口（默认 9527） |
| `auth_token` | string? | 否 |  |
| `hostname` | string | 否 |  |
| `ip_address` | string | 否 |  |
| `name` | string | 否 |  |
| `node` | ServerNode? | 否 |  |
| `status` | string | 否 |  |

```json
{
  "agent_port": 0,
  "auth_token": "string",
  "hostname": "string",
  "ip_address": "string",
  "name": "string",
  "node": {
    "agent_port": 0,
    "auth_token": "string",
    "created_at": "2026-01-01T00:00:00Z",
    "hostname": "string",
    "id": 0,
    "ip_address": "string",
    "last_heartbeat_at": "2026-01-01T00:00:00Z",
    "metrics_json": "string",
    "name": "string",
    "status": "string"
  },
  "status": "string"
}
```

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 注册成功，返回节点 ID | integer(int64) (`text/plain`) |
| 400 | 参数错误 |  |

```bash
fpctl api POST '/api/nodes' -d '{"agent_port":0,"auth_token":"string","hostname":"string","ip_address":"string","name":"string","node":{"agent_port":0,"auth_token":"string","created_at":"2026-01-01T00:00:00Z","hostname":"string","id":0,"ip_address":"string","last_heartbeat_at":"2026-01-01T00:00:00Z","metrics_json":"string","name":"string","status":"string"},"status":"string"}' --json
```

### POST `/api/nodes/batch-execute`

批量在多个节点并行执行命令

S4：同样走 Agent `/action`（`whitelisted_command`），agent 模式下仅执行白名单命令。

- 认证：Bearer JWT
- 权限：`node:execute`
- operationId：`batch_execute`

**请求体**（`application/json`，必填）：RemoteBatchExecRequest

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `command` | string | 是 |  |
| `node_ids` | integer(int64)[] | 是 |  |
| `timeout_secs` | integer(int64)? | 否 |  |

```json
{
  "command": "string",
  "node_ids": [
    0
  ],
  "timeout_secs": 0
}
```

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 各节点执行结果聚合（agent 模式下受命令白名单约束） |  |
| 401 | 未认证 |  |

```bash
fpctl api POST '/api/nodes/batch-execute' -d '{"command":"string","node_ids":[0],"timeout_secs":0}' --json
```

### POST `/api/nodes/heartbeat/{id}`

Agent 心跳上报（免 JWT，Agent token 已在中间件集中校验——S15）

- 认证：无
- 权限：公开（无需登录）
- operationId：`heartbeat`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 节点 ID |

**请求体**（`application/json`，必填）：HeartbeatRequest

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `cpu_cores` | integer(int64) | 否 |  |
| `cpu_usage` | number(float) | 是 |  |
| `disk_total_gb` | number(double) | 否 |  |
| `disk_usage_percent` | number(float) | 是 |  |
| `disk_used_gb` | number(double) | 否 |  |
| `load_fifteen` | number(float) | 否 |  |
| `load_five` | number(float) | 否 |  |
| `load_one` | number(float) | 是 |  |
| `memory_total_mb` | integer(int64) | 否 |  |
| `memory_usage_percent` | number(float) | 是 |  |
| `memory_used_mb` | integer(int64) | 否 |  |
| `network_rx_mbps` | number(double) | 否 | 网络接收速率（MB/s，Agent 按两次心跳间的字节差 / 实际间隔计算） |
| `network_tx_mbps` | number(double) | 否 | 网络发送速率（MB/s） |

```json
{
  "cpu_cores": 0,
  "cpu_usage": 0.0,
  "disk_total_gb": 0.0,
  "disk_usage_percent": 0.0,
  "disk_used_gb": 0.0,
  "load_fifteen": 0.0,
  "load_five": 0.0,
  "load_one": 0.0,
  "memory_total_mb": 0,
  "memory_usage_percent": 0.0,
  "memory_used_mb": 0,
  "network_rx_mbps": 0.0,
  "network_tx_mbps": 0.0
}
```

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 心跳记录成功 |  |
| 401 | Agent token 无效 |  |

### POST `/api/nodes/register`

Agent 注册（白名单免 JWT，供 Agent 启动时自动注册）
请求体：`{name, host, agent_port, auth_token}`，返回 `{"id": <node_id>}`

- 认证：无
- 权限：`node:create`
- operationId：`register_agent`

**请求体**（`application/json`，必填）：CreateNodeRequest

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `agent_port` | integer(int32) | 否 | Agent HTTP 服务端口（默认 9527） |
| `auth_token` | string? | 否 |  |
| `hostname` | string | 否 |  |
| `ip_address` | string | 否 |  |
| `name` | string | 否 |  |
| `node` | ServerNode? | 否 |  |
| `status` | string | 否 |  |

```json
{
  "agent_port": 0,
  "auth_token": "string",
  "hostname": "string",
  "ip_address": "string",
  "name": "string",
  "node": {
    "agent_port": 0,
    "auth_token": "string",
    "created_at": "2026-01-01T00:00:00Z",
    "hostname": "string",
    "id": 0,
    "ip_address": "string",
    "last_heartbeat_at": "2026-01-01T00:00:00Z",
    "metrics_json": "string",
    "name": "string",
    "status": "string"
  },
  "status": "string"
}
```

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 注册成功，返回节点 ID |  |
| 400 | 参数错误 |  |

### PUT `/api/nodes/{id}`

更新节点

- 认证：Bearer JWT
- 权限：`node:update`
- operationId：`update`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | ID |

**请求体**（`application/json`，必填）：CreateNodeRequest

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `agent_port` | integer(int32) | 否 | Agent HTTP 服务端口（默认 9527） |
| `auth_token` | string? | 否 |  |
| `hostname` | string | 否 |  |
| `ip_address` | string | 否 |  |
| `name` | string | 否 |  |
| `node` | ServerNode? | 否 |  |
| `status` | string | 否 |  |

```json
{
  "agent_port": 0,
  "auth_token": "string",
  "hostname": "string",
  "ip_address": "string",
  "name": "string",
  "node": {
    "agent_port": 0,
    "auth_token": "string",
    "created_at": "2026-01-01T00:00:00Z",
    "hostname": "string",
    "id": 0,
    "ip_address": "string",
    "last_heartbeat_at": "2026-01-01T00:00:00Z",
    "metrics_json": "string",
    "name": "string",
    "status": "string"
  },
  "status": "string"
}
```

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 更新后的节点 | ServerNode |
| 401 | 未认证 |  |
| 403 | 无权限 |  |
| 404 | 节点不存在 |  |

<details><summary>响应示例</summary>

```json
{
  "agent_port": 0,
  "auth_token": "string",
  "created_at": "2026-01-01T00:00:00Z",
  "hostname": "string",
  "id": 0,
  "ip_address": "string",
  "last_heartbeat_at": "2026-01-01T00:00:00Z",
  "metrics_json": "string",
  "name": "string",
  "status": "string"
}
```

</details>

```bash
fpctl api PUT '/api/nodes/1' -d '{"agent_port":0,"auth_token":"string","hostname":"string","ip_address":"string","name":"string","node":{"agent_port":0,"auth_token":"string","created_at":"2026-01-01T00:00:00Z","hostname":"string","id":0,"ip_address":"string","last_heartbeat_at":"2026-01-01T00:00:00Z","metrics_json":"string","name":"string","status":"string"},"status":"string"}' --json
```

### DELETE `/api/nodes/{id}`

删除节点

- 认证：Bearer JWT
- 权限：`node:delete`
- operationId：`delete`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | ID |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 删除成功 | string (`text/plain`) |
| 401 | 未认证 |  |
| 403 | 无权限 |  |
| 404 | 节点不存在 |  |

```bash
fpctl api DELETE '/api/nodes/1' --json
```

### POST `/api/nodes/{id}/action`

调用 Agent 动作枚举（Phase A1：安全动作白名单）

- 认证：Bearer JWT
- 权限：`node:execute`
- operationId：`remote_action`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 节点 ID |

**请求体**（`application/json`，必填）：RemoteActionRequest

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `action` | string | 是 |  |
| `params` | any | 否 |  |

```json
{
  "action": "string",
  "params": null
}
```

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 动作结果 |  |
| 400 | 非法动作或参数 |  |
| 401 | 未认证 |  |

```bash
fpctl api POST '/api/nodes/1/action' -d '{"action":"string","params":null}' --json
```

### POST `/api/nodes/{id}/execute`

在远程节点执行命令

S4：统一走 Agent `/action`（`whitelisted_command`）。在 `execution_mode=agent` 下，
仅执行白名单命令，非白名单命令将返回 `ACTION_NOT_ALLOWED`（功能降级）。

- 认证：Bearer JWT
- 权限：`node:execute`
- operationId：`remote_execute`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 节点 ID |

**请求体**（`application/json`，必填）：RemoteExecRequest

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `command` | string | 是 |  |
| `timeout_secs` | integer(int64)? | 否 |  |

```json
{
  "command": "string",
  "timeout_secs": 0
}
```

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 执行结果（agent 模式下受命令白名单约束） |  |
| 401 | 未认证 |  |

```bash
fpctl api POST '/api/nodes/1/execute' -d '{"command":"string","timeout_secs":0}' --json
```

### GET `/api/nodes/{id}/files`

列出远程节点目录

- 认证：Bearer JWT
- 权限：`node:execute`
- operationId：`remote_list_files`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 节点 ID |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 远程目录列表 |  |
| 401 | 未认证 |  |

```bash
fpctl api GET '/api/nodes/1/files' --json
```

### GET `/api/nodes/{id}/files/download`

下载远程节点文件

- 认证：Bearer JWT
- 权限：`node:execute`
- operationId：`remote_download_file`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 节点 ID |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 文件内容（base64） |  |
| 401 | 未认证 |  |

```bash
fpctl api GET '/api/nodes/1/files/download' --json
```

### POST `/api/nodes/{id}/files/upload`

上传文件到远程节点

- 认证：Bearer JWT
- 权限：`node:execute`
- operationId：`remote_upload_file`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 节点 ID |

**请求体**（`application/json`，必填）：RemoteUploadRequest

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `content_base64` | string | 是 | base64 编码的文件内容 |
| `path` | string | 是 |  |

```json
{
  "content_base64": "string",
  "path": "string"
}
```

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 上传结果 |  |
| 401 | 未认证 |  |

```bash
fpctl api POST '/api/nodes/1/files/upload' -d '{"content_base64":"string","path":"string"}' --json
```

### GET `/api/nodes/{id}/metrics`

节点最近指标快照

- 认证：Bearer JWT
- 权限：`node:read`
- operationId：`metrics`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | ID |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 最近一次心跳上报的指标（结构随 Agent 版本演进） |  |
| 401 | 未认证 |  |
| 403 | 无权限 |  |
| 404 | 节点不存在 |  |

```bash
fpctl api GET '/api/nodes/1/metrics' --json
```

### GET `/api/nodes/{id}/metrics/history`

节点指标历史（内存窗口：Agent 10 s 一点、最多 120 点 ≈ 20 分钟；面板重启后重新积累）

- 认证：Bearer JWT
- 权限：`node:read`
- operationId：`metrics_history`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 节点 ID |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 按时间升序的快照（未收到过心跳为空数组） | MetricsSnapshot[] |
| 401 | 未认证 |  |
| 403 | 缺少 node:read 权限 |  |
| 404 | 节点不存在 |  |

<details><summary>响应示例</summary>

```json
[
  {
    "cpu_cores": 0,
    "cpu_usage": 0.0,
    "disk_total_gb": 0.0,
    "disk_usage_percent": 0.0,
    "disk_used_gb": 0.0,
    "load_fifteen": 0.0,
    "load_five": 0.0,
    "load_one": 0.0,
    "memory_total_mb": 0,
    "memory_usage_percent": 0.0,
    "memory_used_mb": 0,
    "network_rx_mbps": 0.0,
    "network_tx_mbps": 0.0,
    "timestamp": 0
  }
]
```

</details>

```bash
fpctl api GET '/api/nodes/1/metrics/history' --json
```

### GET `/api/nodes/{id}/status`

节点在线状态（惰性判定）

- 认证：Bearer JWT
- 权限：`node:read`
- operationId：`status`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 节点 ID |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 在线状态 `{id, status, edge: EdgeInfo}` |  |
| 404 | 节点不存在 |  |

```bash
fpctl api GET '/api/nodes/1/status' --json
```
