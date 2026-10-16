<!-- 由 scripts/gen-api-docs.py 从 frontend/openapi.json 生成，请勿手改；修改 utoipa 注解后运行 just api-docs -->

# API · plugins

> 16 个端点 · 通用约定见 [README](./README.md)

| 方法 | 路径 | 摘要 | 权限 |
|---|---|---|---|
| GET | `/api/plugins` | 已加载插件列表 | `plugin:read` |
| POST | `/api/plugins` | 加载 WASM 插件并持久化（同 ID 覆盖） | `plugin:create` |
| GET | `/api/plugins/market` | 插件市场目录 | `plugin:read` |
| POST | `/api/plugins/market/{id}/install` | 从市场安装插件 | `plugin:create` |
| GET | `/api/plugins/{id}` | 单个插件 | `plugin:read` |
| POST | `/api/plugins/{id}` | 卸载插件（返回 id / status=unloaded / exec_count） | `plugin:delete` |
| PUT | `/api/plugins/{id}/capabilities` | 变更插件宿主能力授予（整体替换，立即生效并持久化） | `plugin:create` |
| POST | `/api/plugins/{id}/disable` | 禁用插件 | `plugin:create` |
| POST | `/api/plugins/{id}/enable` | 启用插件 | `plugin:create` |
| POST | `/api/plugins/{id}/execute/{function}` | 执行插件导出函数（fuel 限额 + 内存上限） | `plugin:execute` |
| GET | `/api/plugins/{id}/metrics` | 插件执行指标 | `plugin:read` |
| DELETE | `/api/plugins/{id}/metrics` | 重置插件执行指标 | `plugin:config` |
| POST | `/api/plugins/{id}/reload` | 热重载插件字节码 | `plugin:create` |
| GET | `/api/plugins/{id}/settings` | 插件设置（键值） | `plugin:config` |
| POST | `/api/plugins/{id}/settings` | 写入一项插件设置 | `plugin:config` |
| GET | `/api/plugins/{id}/settings/{key}` | 读取一项插件设置（id / key / value） | `plugin:config` |

### GET `/api/plugins`

已加载插件列表

- 认证：Bearer JWT
- 权限：`plugin:read`
- operationId：`list_plugins`

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 插件列表 | PluginResponse[] |
| 401 | 未认证 |  |
| 403 | 缺少 plugin:read 权限 |  |

<details><summary>响应示例</summary>

```json
[
  {
    "author": "string",
    "capabilities": [
      "string"
    ],
    "dependencies": [
      "string"
    ],
    "description": "string",
    "enabled": false,
    "exec_count": 0,
    "homepage": "string",
    "id": "string",
    "last_executed_at": "string",
    "license": "string",
    "loaded_at": "string",
    "name": "string",
    "status": "string",
    "tags": [
      "string"
    ],
    "version": "string"
  }
]
```

</details>

```bash
fpctl api GET '/api/plugins' --json
```

### POST `/api/plugins`

加载 WASM 插件并持久化（同 ID 覆盖）

- 认证：Bearer JWT
- 权限：`plugin:create`
- operationId：`load_plugin`

**请求体**（`application/json`，必填）：LoadPluginRequest

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `author` | string? | 否 |  |
| `capabilities` | string[]? | 否 | 授予的宿主能力（Doc/21 §5.7）：`fs:read:<dir>` / `fs:write:<dir>` / `metrics:read` / `settings:write`；缺省 = 无能力 |
| `description` | string? | 否 |  |
| `homepage` | string? | 否 |  |
| `id` | string | 是 | 1–64 位 `[A-Za-z0-9._-]`，不以 `.` 开头 |
| `license` | string? | 否 |  |
| `memory_limit_bytes` | integer? | 否 | 1 MiB – 256 MiB，缺省 64 MiB |
| `name` | string? | 否 |  |
| `tags` | string[]? | 否 |  |
| `timeout_ms` | integer(int64)? | 否 | 1 – 300000 ms，缺省 30000 |
| `version` | string? | 否 |  |
| `wasm_base64` | string | 是 |  |

```json
{
  "author": "string",
  "capabilities": [
    "string"
  ],
  "description": "string",
  "homepage": "string",
  "id": "string",
  "license": "string",
  "memory_limit_bytes": 0,
  "name": "string",
  "tags": [
    "string"
  ],
  "timeout_ms": 0,
  "version": "string",
  "wasm_base64": "string"
}
```

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 插件 | PluginResponse |
| 400 | base64 / WASM 非法、为空；ID、内存或超时越界 |  |
| 401 | 未认证 |  |
| 403 | 缺少 plugin:create 权限 |  |

<details><summary>响应示例</summary>

```json
{
  "author": "string",
  "capabilities": [
    "string"
  ],
  "dependencies": [
    "string"
  ],
  "description": "string",
  "enabled": false,
  "exec_count": 0,
  "homepage": "string",
  "id": "string",
  "last_executed_at": "string",
  "license": "string",
  "loaded_at": "string",
  "name": "string",
  "status": "string",
  "tags": [
    "string"
  ],
  "version": "string"
}
```

</details>

```bash
fpctl api POST '/api/plugins' -d '{"author":"string","capabilities":["string"],"description":"string","homepage":"string","id":"string","license":"string","memory_limit_bytes":0,"name":"string","tags":["string"],"timeout_ms":0,"version":"string","wasm_base64":"string"}' --json
```

### GET `/api/plugins/market`

插件市场目录

- 认证：Bearer JWT
- 权限：`plugin:read`
- operationId：`list_market`

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 市场目录（内置 + 远程） | MarketListResponse |
| 401 | 未认证 |  |
| 403 | 缺少 plugin:read 权限 |  |

<details><summary>响应示例</summary>

```json
{
  "plugins": [
    {
      "author": null,
      "capabilities": null,
      "description": null,
      "entry": null,
      "homepage": null,
      "id": null,
      "license": null,
      "name": null,
      "source": null,
      "tags": null,
      "version": null,
      "wasm_sha256": null,
      "wasm_url": null,
      "installed": false,
      "installed_version": "string"
    }
  ],
  "remote_error": "string",
  "remote_url": "string"
}
```

</details>

```bash
fpctl api GET '/api/plugins/market' --json
```

### POST `/api/plugins/market/{id}/install`

从市场安装插件

- 认证：Bearer JWT
- 权限：`plugin:create`
- operationId：`install_market_plugin`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | string | 是 | 市场插件 ID |

**请求体**（`application/json`，必填）：MarketInstallRequest

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `accept_capabilities` | boolean | 否 | 插件申请了宿主能力时必须为 true（管理员已查看并同意能力清单） |

```json
{
  "accept_capabilities": false
}
```

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 已安装 | PluginResponse |
| 400 | 未同意能力申请 / 包体哈希不符 / WASM 非法 |  |
| 401 | 未认证 |  |
| 403 | 缺少 plugin:create 权限 |  |
| 404 | 市场中无此插件 |  |
| 409 | 同 ID 插件已安装 |  |
| 503 | 远程市场不可达或 WASM 支持未编译 |  |

<details><summary>响应示例</summary>

```json
{
  "author": "string",
  "capabilities": [
    "string"
  ],
  "dependencies": [
    "string"
  ],
  "description": "string",
  "enabled": false,
  "exec_count": 0,
  "homepage": "string",
  "id": "string",
  "last_executed_at": "string",
  "license": "string",
  "loaded_at": "string",
  "name": "string",
  "status": "string",
  "tags": [
    "string"
  ],
  "version": "string"
}
```

</details>

```bash
fpctl api POST '/api/plugins/market/1/install' -d '{"accept_capabilities":false}' --json
```

### GET `/api/plugins/{id}`

单个插件

- 认证：Bearer JWT
- 权限：`plugin:read`
- operationId：`get_plugin`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | string | 是 | 插件 ID |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 插件 | PluginResponse |
| 401 | 未认证 |  |
| 403 | 缺少 plugin:read 权限 |  |
| 404 | 插件不存在 |  |

<details><summary>响应示例</summary>

```json
{
  "author": "string",
  "capabilities": [
    "string"
  ],
  "dependencies": [
    "string"
  ],
  "description": "string",
  "enabled": false,
  "exec_count": 0,
  "homepage": "string",
  "id": "string",
  "last_executed_at": "string",
  "license": "string",
  "loaded_at": "string",
  "name": "string",
  "status": "string",
  "tags": [
    "string"
  ],
  "version": "string"
}
```

</details>

```bash
fpctl api GET '/api/plugins/1' --json
```

### POST `/api/plugins/{id}`

卸载插件（返回 id / status=unloaded / exec_count）

- 认证：Bearer JWT
- 权限：`plugin:delete`
- operationId：`unload_plugin`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | string | 是 | 插件 ID |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 已卸载 |  |
| 401 | 未认证 |  |
| 403 | 缺少 plugin:delete 权限 |  |
| 404 | 插件不存在 |  |

```bash
fpctl api POST '/api/plugins/1' --json
```

### PUT `/api/plugins/{id}/capabilities`

变更插件宿主能力授予（整体替换，立即生效并持久化）

- 认证：Bearer JWT
- 权限：`plugin:create`
- operationId：`update_plugin_capabilities`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | string | 是 | 插件 ID |

**请求体**（`application/json`，必填）：UpdateCapabilitiesRequest

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `capabilities` | string[] | 是 |  |

```json
{
  "capabilities": [
    "string"
  ]
}
```

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 插件 | PluginResponse |
| 400 | 能力非法（未知能力 / 敏感目录 / 超过 16 项） |  |
| 401 | 未认证 |  |
| 403 | 缺少 plugin:create 权限 |  |
| 404 | 插件不存在 |  |

<details><summary>响应示例</summary>

```json
{
  "author": "string",
  "capabilities": [
    "string"
  ],
  "dependencies": [
    "string"
  ],
  "description": "string",
  "enabled": false,
  "exec_count": 0,
  "homepage": "string",
  "id": "string",
  "last_executed_at": "string",
  "license": "string",
  "loaded_at": "string",
  "name": "string",
  "status": "string",
  "tags": [
    "string"
  ],
  "version": "string"
}
```

</details>

```bash
fpctl api PUT '/api/plugins/1/capabilities' -d '{"capabilities":["string"]}' --json
```

### POST `/api/plugins/{id}/disable`

禁用插件

- 认证：Bearer JWT
- 权限：`plugin:create`
- operationId：`disable_plugin`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | string | 是 | 插件 ID |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 插件 | PluginResponse |
| 401 | 未认证 |  |
| 403 | 缺少 plugin:create 权限 |  |
| 404 | 插件不存在 |  |

<details><summary>响应示例</summary>

```json
{
  "author": "string",
  "capabilities": [
    "string"
  ],
  "dependencies": [
    "string"
  ],
  "description": "string",
  "enabled": false,
  "exec_count": 0,
  "homepage": "string",
  "id": "string",
  "last_executed_at": "string",
  "license": "string",
  "loaded_at": "string",
  "name": "string",
  "status": "string",
  "tags": [
    "string"
  ],
  "version": "string"
}
```

</details>

```bash
fpctl api POST '/api/plugins/1/disable' --json
```

### POST `/api/plugins/{id}/enable`

启用插件

- 认证：Bearer JWT
- 权限：`plugin:create`
- operationId：`enable_plugin`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | string | 是 | 插件 ID |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 插件 | PluginResponse |
| 401 | 未认证 |  |
| 403 | 缺少 plugin:create 权限 |  |
| 404 | 插件不存在 |  |

<details><summary>响应示例</summary>

```json
{
  "author": "string",
  "capabilities": [
    "string"
  ],
  "dependencies": [
    "string"
  ],
  "description": "string",
  "enabled": false,
  "exec_count": 0,
  "homepage": "string",
  "id": "string",
  "last_executed_at": "string",
  "license": "string",
  "loaded_at": "string",
  "name": "string",
  "status": "string",
  "tags": [
    "string"
  ],
  "version": "string"
}
```

</details>

```bash
fpctl api POST '/api/plugins/1/enable' --json
```

### POST `/api/plugins/{id}/execute/{function}`

执行插件导出函数（fuel 限额 + 内存上限）

- 认证：Bearer JWT
- 权限：`plugin:execute`
- operationId：`execute_plugin`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | string | 是 | 插件 ID |
| `function` | path | string | 是 | 导出函数名 |

**请求体**（`application/json`，必填）：ExecutePluginRequest

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `args` | integer(int32)[]? | 否 | 兼容旧客户端，已不使用 |
| `input` | string? | 否 | 调用输入（插件经 `flame.input_read` 读取），≤ 1 MiB |

```json
{
  "args": [
    0
  ],
  "input": "string"
}
```

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 执行结果 | ExecutionResponse |
| 400 | 插件已禁用 |  |
| 401 | 未认证 |  |
| 403 | 缺少 plugin:execute 权限 |  |
| 404 | 插件不存在 |  |

<details><summary>响应示例</summary>

```json
{
  "execution_ms": 0,
  "logs": [
    "string"
  ],
  "output": [
    0
  ],
  "output_hex": "string",
  "output_string": "string"
}
```

</details>

```bash
fpctl api POST '/api/plugins/1/execute/1' -d '{"args":[0],"input":"string"}' --json
```

### GET `/api/plugins/{id}/metrics`

插件执行指标

- 认证：Bearer JWT
- 权限：`plugin:read`
- operationId：`get_plugin_metrics`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | string | 是 | 插件 ID |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 指标 | PluginMetricsResponse |
| 401 | 未认证 |  |
| 403 | 缺少 plugin:read 权限 |  |
| 404 | 插件不存在 |  |

<details><summary>响应示例</summary>

```json
{
  "avg_execution_ms": 0.0,
  "failed_executions": 0,
  "last_execution_ms": 0,
  "max_execution_ms": 0,
  "min_execution_ms": 0,
  "peak_memory_bytes": 0,
  "successful_executions": 0,
  "total_executions": 0
}
```

</details>

```bash
fpctl api GET '/api/plugins/1/metrics' --json
```

### DELETE `/api/plugins/{id}/metrics`

重置插件执行指标

- 认证：Bearer JWT
- 权限：`plugin:config`
- operationId：`reset_plugin_metrics`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | string | 是 | 插件 ID |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 已重置 |  |
| 401 | 未认证 |  |
| 403 | 缺少 plugin:config 权限 |  |
| 404 | 插件不存在 |  |

```bash
fpctl api DELETE '/api/plugins/1/metrics' --json
```

### POST `/api/plugins/{id}/reload`

热重载插件字节码

- 认证：Bearer JWT
- 权限：`plugin:create`
- operationId：`reload_plugin`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | string | 是 | 插件 ID |

**请求体**（`application/json`，必填）：PluginReloadRequest

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `memory_limit_bytes` | integer? | 否 | 1 MiB – 256 MiB，缺省 64 MiB |
| `timeout_ms` | integer(int64)? | 否 | 1 – 300000 ms，缺省 30000 |
| `wasm_base64` | string | 是 |  |

```json
{
  "memory_limit_bytes": 0,
  "timeout_ms": 0,
  "wasm_base64": "string"
}
```

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 插件 | PluginResponse |
| 400 | base64 / WASM 非法；内存或超时越界 |  |
| 401 | 未认证 |  |
| 403 | 缺少 plugin:create 权限 |  |
| 404 | 插件不存在 |  |

<details><summary>响应示例</summary>

```json
{
  "author": "string",
  "capabilities": [
    "string"
  ],
  "dependencies": [
    "string"
  ],
  "description": "string",
  "enabled": false,
  "exec_count": 0,
  "homepage": "string",
  "id": "string",
  "last_executed_at": "string",
  "license": "string",
  "loaded_at": "string",
  "name": "string",
  "status": "string",
  "tags": [
    "string"
  ],
  "version": "string"
}
```

</details>

```bash
fpctl api POST '/api/plugins/1/reload' -d '{"memory_limit_bytes":0,"timeout_ms":0,"wasm_base64":"string"}' --json
```

### GET `/api/plugins/{id}/settings`

插件设置（键值）

- 认证：Bearer JWT
- 权限：`plugin:config`
- operationId：`list_plugin_settings`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | string | 是 | 插件 ID |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 设置 | map<string, string> |
| 401 | 未认证 |  |
| 403 | 缺少 plugin:config 权限 |  |
| 404 | 插件不存在 |  |

```bash
fpctl api GET '/api/plugins/1/settings' --json
```

### POST `/api/plugins/{id}/settings`

写入一项插件设置

- 认证：Bearer JWT
- 权限：`plugin:config`
- operationId：`set_plugin_setting`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | string | 是 | 插件 ID |

**请求体**（`application/json`，必填）：PluginSettingRequest

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `key` | string | 是 | 1–128 位 `[A-Za-z0-9._-]` |
| `value` | string | 是 | ≤ 4096 字节 |

```json
{
  "key": "string",
  "value": "string"
}
```

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 已保存（message / id / key） |  |
| 400 | key 须 1–128 位 `[A-Za-z0-9._-]`；value ≤ 4096 字节 |  |
| 401 | 未认证 |  |
| 403 | 缺少 plugin:config 权限 |  |
| 404 | 插件不存在 |  |

```bash
fpctl api POST '/api/plugins/1/settings' -d '{"key":"string","value":"string"}' --json
```

### GET `/api/plugins/{id}/settings/{key}`

读取一项插件设置（id / key / value）

- 认证：Bearer JWT
- 权限：`plugin:config`
- operationId：`get_plugin_setting`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | string | 是 | 插件 ID |
| `key` | path | string | 是 | 设置键 |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 设置值 |  |
| 401 | 未认证 |  |
| 403 | 缺少 plugin:config 权限 |  |
| 404 | 插件不存在 |  |

```bash
fpctl api GET '/api/plugins/1/settings/1' --json
```
