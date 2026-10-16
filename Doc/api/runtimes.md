<!-- 由 scripts/gen-api-docs.py 从 frontend/openapi.json 生成，请勿手改；修改 utoipa 注解后运行 just api-docs -->

# API · runtimes

> 8 个端点 · 通用约定见 [README](./README.md)

| 方法 | 路径 | 摘要 | 权限 |
|---|---|---|---|
| GET | `/api/runtimes` | 运行时列表 | `runtime:read` |
| GET | `/api/runtimes/available` | 可安装版本清单（声明版本 × 系统探测） | `runtime:read` |
| POST | `/api/runtimes/install` | 安装运行时 | `database:create` |
| GET | `/api/runtimes/kind/{kind}` | 按语言过滤运行时 | `runtime:read` |
| GET | `/api/runtimes/{id}` | 运行时详情（GET /api/runtimes/{id}） | `runtime:read` |
| DELETE | `/api/runtimes/{id}` | 卸载运行时 | `runtime:delete` |
| POST | `/api/runtimes/{id}/default` | 设为同语言默认版本 | `runtime:update` |
| GET | `/api/runtimes/{id}/sites` | 按运行时列出绑定站点（M-B 主线二 Stage A-6 · Doc/20 §5.5） | `website:read` |

### GET `/api/runtimes`

运行时列表

- 认证：Bearer JWT
- 权限：`runtime:read`
- operationId：`list_runtimes`

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 运行时列表 | RuntimeDto[] |

<details><summary>响应示例</summary>

```json
[
  {
    "created_at": "2026-01-01T00:00:00Z",
    "id": 0,
    "install_path": "string",
    "is_default": false,
    "kind": "string",
    "metadata_json": "string",
    "status": "string",
    "updated_at": "2026-01-01T00:00:00Z",
    "version": "string"
  }
]
```

</details>

```bash
fpctl api GET '/api/runtimes' --json
```

### GET `/api/runtimes/available`

可安装版本清单（声明版本 × 系统探测）

- 认证：Bearer JWT
- 权限：`runtime:read`
- operationId：`available_runtimes`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `kind` | query | string | 是 | 语言种类：php \| java \| node \| python \| go |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 版本清单 | RuntimeVersionInfo[] |
| 400 | 不支持的语言 |  |

<details><summary>响应示例</summary>

```json
[
  {
    "installed": false,
    "is_default": false,
    "runtime_id": 0,
    "version": "string"
  }
]
```

</details>

```bash
fpctl api GET '/api/runtimes/available' --json
```

### POST `/api/runtimes/install`

安装运行时

- 认证：Bearer JWT
- 权限：`database:create`
- operationId：`install_runtime`

**请求体**（`application/json`，必填）：InstallRuntimeRequest

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `kind` | string | 是 | php \| java \| node \| python \| go |
| `pm2` | boolean? | 否 | 仅 Node：安装后同时执行 `npm install -g pm2`（其余 kind 传 true 返回 400） |
| `version` | string | 是 | 版本号（如 8.2 / 21 / 20）。Node 可写主版本（`22`）或次版本（`22.11`）， 安装时按镜像索引解析到最新补丁版（解析结果见 `metadata_json.version`） |

```json
{
  "kind": "string",
  "pm2": false,
  "version": "string"
}
```

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 安装成功（或已登记并安装中） | RuntimeDto |
| 400 | 参数校验失败 |  |
| 409 | 已安装 |  |

<details><summary>响应示例</summary>

```json
{
  "created_at": "2026-01-01T00:00:00Z",
  "id": 0,
  "install_path": "string",
  "is_default": false,
  "kind": "string",
  "metadata_json": "string",
  "status": "string",
  "updated_at": "2026-01-01T00:00:00Z",
  "version": "string"
}
```

</details>

```bash
fpctl api POST '/api/runtimes/install' -d '{"kind":"string","pm2":false,"version":"string"}' --json
```

### GET `/api/runtimes/kind/{kind}`

按语言过滤运行时

- 认证：Bearer JWT
- 权限：`runtime:read`
- operationId：`list_runtimes_by_kind`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `kind` | path | string | 是 | 语言种类 |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 该语言运行时列表 | RuntimeDto[] |

<details><summary>响应示例</summary>

```json
[
  {
    "created_at": "2026-01-01T00:00:00Z",
    "id": 0,
    "install_path": "string",
    "is_default": false,
    "kind": "string",
    "metadata_json": "string",
    "status": "string",
    "updated_at": "2026-01-01T00:00:00Z",
    "version": "string"
  }
]
```

</details>

```bash
fpctl api GET '/api/runtimes/kind/1' --json
```

### GET `/api/runtimes/{id}`

运行时详情（GET /api/runtimes/{id}）

- 认证：Bearer JWT
- 权限：`runtime:read`
- operationId：`get_runtime`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 运行时 ID |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 运行时详情 | RuntimeDto |
| 404 | 运行时不存在 |  |

<details><summary>响应示例</summary>

```json
{
  "created_at": "2026-01-01T00:00:00Z",
  "id": 0,
  "install_path": "string",
  "is_default": false,
  "kind": "string",
  "metadata_json": "string",
  "status": "string",
  "updated_at": "2026-01-01T00:00:00Z",
  "version": "string"
}
```

</details>

```bash
fpctl api GET '/api/runtimes/1' --json
```

### DELETE `/api/runtimes/{id}`

卸载运行时

- 认证：Bearer JWT
- 权限：`runtime:delete`
- operationId：`uninstall_runtime`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 运行时 ID |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 卸载成功 |  |

```bash
fpctl api DELETE '/api/runtimes/1' --json
```

### POST `/api/runtimes/{id}/default`

设为同语言默认版本

- 认证：Bearer JWT
- 权限：`runtime:update`
- operationId：`set_default_runtime`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 运行时 ID |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 设置成功 | RuntimeDto |
| 400 | 未安装 |  |

<details><summary>响应示例</summary>

```json
{
  "created_at": "2026-01-01T00:00:00Z",
  "id": 0,
  "install_path": "string",
  "is_default": false,
  "kind": "string",
  "metadata_json": "string",
  "status": "string",
  "updated_at": "2026-01-01T00:00:00Z",
  "version": "string"
}
```

</details>

```bash
fpctl api POST '/api/runtimes/1/default' --json
```

### GET `/api/runtimes/{id}/sites`

按运行时列出绑定站点（M-B 主线二 Stage A-6 · Doc/20 §5.5）

- 认证：Bearer JWT
- 权限：`website:read`
- operationId：`list_runtime_sites`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 运行时 ID |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 绑定该运行时的站点列表 | WebsiteDto[] |
| 404 | 运行时不存在 |  |

<details><summary>响应示例</summary>

```json
[
  {
    "created_at": "string",
    "domain": "string",
    "engine": "string",
    "id": 0,
    "name": "string",
    "node_id": 0,
    "project_id": 0,
    "proxy_enabled": false,
    "proxy_pass": "string",
    "root_path": "string",
    "runtime_id": 0,
    "site_type": "string",
    "ssl_enabled": false,
    "status": "string"
  }
]
```

</details>

```bash
fpctl api GET '/api/runtimes/1/sites' --json
```
