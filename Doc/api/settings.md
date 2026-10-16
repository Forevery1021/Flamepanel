<!-- 由 scripts/gen-api-docs.py 从 frontend/openapi.json 生成，请勿手改；修改 utoipa 注解后运行 just api-docs -->

# API · settings

> 5 个端点 · 通用约定见 [README](./README.md)

| 方法 | 路径 | 摘要 | 权限 |
|---|---|---|---|
| GET | `/api/settings` | 设置列表（分页） | `settings:read` |
| PUT | `/api/settings` | 更新设置 | `settings:update` |
| GET | `/api/settings/access` | 访问来源与访问控制策略 | `settings:read` |
| PATCH | `/api/settings/batch` | 批量更新设置（一次事务原子写入多键） | `settings:update` |
| GET | `/api/settings/{key}` | 读取单个设置项 | `settings:read` |

### GET `/api/settings`

设置列表（分页）

- 认证：Bearer JWT
- 权限：`settings:read`
- operationId：`list_settings`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `page` | query | integer(int64)? | 否 | 页码（从 1 开始） |
| `page_size` | query | integer(int64)? | 否 | 每页条数（1~200） |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 设置列表 | PaginatedResponse<SettingEntry>_SettingEntry |
| 401 | 未认证 |  |

<details><summary>响应示例</summary>

```json
{
  "data": [
    {
      "description": "string",
      "key": "string",
      "value": "string"
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
fpctl api GET '/api/settings' --json
```

### PUT `/api/settings`

更新设置

- 认证：Bearer JWT
- 权限：`settings:update`
- operationId：`update_setting`

**请求体**（`application/json`，必填）：UpdateSettingRequest

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `key` | string | 是 |  |
| `value` | string | 是 |  |

```json
{
  "key": "string",
  "value": "string"
}
```

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 更新成功 |  |
| 400 | 值不合法 / 保留键 / 保存后会锁定当前访问来源 |  |
| 401 | 未认证 |  |

```bash
fpctl api PUT '/api/settings' -d '{"key":"string","value":"string"}' --json
```

### GET `/api/settings/access`

访问来源与访问控制策略

- 认证：Bearer JWT
- 权限：`settings:read`
- operationId：`access_info`

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 当前访问来源与生效策略 | AccessInfo |
| 401 | 未认证 |  |
| 403 | 无权限 |  |

<details><summary>响应示例</summary>

```json
{
  "allowed_ips": [
    "string"
  ],
  "bind_domains": [
    "string"
  ],
  "client_ip": "string",
  "host": "string",
  "local": false,
  "login_lockout_minutes": 0,
  "login_max_failures": 0,
  "session_timeout_minutes": 0
}
```

</details>

```bash
fpctl api GET '/api/settings/access' --json
```

### PATCH `/api/settings/batch`

批量更新设置（一次事务原子写入多键）

- 认证：Bearer JWT
- 权限：`settings:update`
- operationId：`update_settings_batch`

**请求体**（`application/json`，必填）：UpdateSettingsBatchRequest

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `settings` | any[][] | 是 | 批量设置项（key -> value），在一次事务内原子写入。 |

```json
{
  "settings": [
    []
  ]
}
```

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 批量更新成功 |  |
| 400 | 请求体为空 / 值不合法（一次列出全部错误）/ 保存后会锁定当前访问来源 |  |
| 401 | 未认证 |  |

```bash
fpctl api PATCH '/api/settings/batch' -d '{"settings":[[]]}' --json
```

### GET `/api/settings/{key}`

读取单个设置项

- 认证：Bearer JWT
- 权限：`settings:read`
- operationId：`get_setting`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `key` | path | string | 是 | 设置键 |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 设置项 | SettingEntry |
| 401 | 未认证 |  |
| 404 | 设置项不存在 |  |

<details><summary>响应示例</summary>

```json
{
  "description": "string",
  "key": "string",
  "value": "string"
}
```

</details>

```bash
fpctl api GET '/api/settings/1' --json
```
