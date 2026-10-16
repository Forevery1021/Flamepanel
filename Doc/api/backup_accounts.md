<!-- 由 scripts/gen-api-docs.py 从 frontend/openapi.json 生成，请勿手改；修改 utoipa 注解后运行 just api-docs -->

# API · backup_accounts

> 8 个端点 · 通用约定见 [README](./README.md)

| 方法 | 路径 | 摘要 | 权限 |
|---|---|---|---|
| GET | `/api/backup-accounts` | 备份账号列表 | `backup:read` |
| POST | `/api/backup-accounts` | 创建备份账号 | `backup:create` |
| PUT | `/api/backup-accounts/{id}` | 更新备份账号 | `backup:update` |
| DELETE | `/api/backup-accounts/{id}` | 删除备份账号 | `backup:delete` |
| GET | `/api/backup-accounts/{id}/objects` | 列出云端账号的备份对象 | `backup:read` |
| DELETE | `/api/backup-accounts/{id}/objects` | 删除云端账号的备份对象（key 不存在时幂等成功） | `backup:delete` |
| POST | `/api/backup-accounts/{id}/objects/upload` | 上传本地备份文件到云端账号 | `backup:create` |
| POST | `/api/backup-accounts/{id}/test` | 连通性测试（Stage 2：按账号类型构造真实后端探测，结果落库） | `backup:update` |

### GET `/api/backup-accounts`

备份账号列表

- 认证：Bearer JWT
- 权限：`backup:read`
- operationId：`list_accounts`

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 账号列表 | AccountDto[] |
| 401 | 未认证 |  |

<details><summary>响应示例</summary>

```json
[
  {
    "account_type": "string",
    "created_at": "2026-01-01T00:00:00Z",
    "enabled": false,
    "id": 0,
    "last_test_at": "2026-01-01T00:00:00Z",
    "last_test_status": "string",
    "name": "string",
    "test_message": "string",
    "updated_at": "2026-01-01T00:00:00Z"
  }
]
```

</details>

```bash
fpctl api GET '/api/backup-accounts' --json
```

### POST `/api/backup-accounts`

创建备份账号

- 认证：Bearer JWT
- 权限：`backup:create`
- operationId：`create_account`

**请求体**（`application/json`，必填）：CreateAccountRequest

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `account_type` | string | 是 | 账号类型：s3 \| sftp \| local |
| `config_json` | string? | 否 | 后端连接配置（JSON 字符串，敏感字段按各后端约定脱敏存储） |
| `enabled` | boolean? | 否 |  |
| `name` | string | 是 |  |

```json
{
  "account_type": "string",
  "config_json": "string",
  "enabled": false,
  "name": "string"
}
```

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 创建成功 | AccountDto |
| 400 | 参数错误 |  |

<details><summary>响应示例</summary>

```json
{
  "account_type": "string",
  "created_at": "2026-01-01T00:00:00Z",
  "enabled": false,
  "id": 0,
  "last_test_at": "2026-01-01T00:00:00Z",
  "last_test_status": "string",
  "name": "string",
  "test_message": "string",
  "updated_at": "2026-01-01T00:00:00Z"
}
```

</details>

```bash
fpctl api POST '/api/backup-accounts' -d '{"account_type":"string","config_json":"string","enabled":false,"name":"string"}' --json
```

### PUT `/api/backup-accounts/{id}`

更新备份账号

- 认证：Bearer JWT
- 权限：`backup:update`
- operationId：`update_account`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 账号 ID |

**请求体**（`application/json`，必填）：UpdateAccountRequest

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `account_type` | string? | 否 |  |
| `config_json` | string? | 否 |  |
| `enabled` | boolean? | 否 |  |
| `name` | string? | 否 |  |

```json
{
  "account_type": "string",
  "config_json": "string",
  "enabled": false,
  "name": "string"
}
```

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 更新成功 | AccountDto |
| 404 | 账号不存在 |  |

<details><summary>响应示例</summary>

```json
{
  "account_type": "string",
  "created_at": "2026-01-01T00:00:00Z",
  "enabled": false,
  "id": 0,
  "last_test_at": "2026-01-01T00:00:00Z",
  "last_test_status": "string",
  "name": "string",
  "test_message": "string",
  "updated_at": "2026-01-01T00:00:00Z"
}
```

</details>

```bash
fpctl api PUT '/api/backup-accounts/1' -d '{"account_type":"string","config_json":"string","enabled":false,"name":"string"}' --json
```

### DELETE `/api/backup-accounts/{id}`

删除备份账号

- 认证：Bearer JWT
- 权限：`backup:delete`
- operationId：`delete_account`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 账号 ID |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 删除成功 |  |
| 404 | 账号不存在 |  |

```bash
fpctl api DELETE '/api/backup-accounts/1' --json
```

### GET `/api/backup-accounts/{id}/objects`

列出云端账号的备份对象

- 认证：Bearer JWT
- 权限：`backup:read`
- operationId：`list_remote_objects`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 账号 ID |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 远端对象列表 | RemoteObjectDto[] |
| 404 | 账号不存在 |  |

<details><summary>响应示例</summary>

```json
[
  {
    "key": "string",
    "size": 0
  }
]
```

</details>

```bash
fpctl api GET '/api/backup-accounts/1/objects' --json
```

### DELETE `/api/backup-accounts/{id}/objects`

删除云端账号的备份对象（key 不存在时幂等成功）

- 认证：Bearer JWT
- 权限：`backup:delete`
- operationId：`delete_remote_object`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 账号 ID |
| `key` | query | string | 是 | 远端对象 key |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 删除成功 |  |
| 404 | 账号不存在 |  |

```bash
fpctl api DELETE '/api/backup-accounts/1/objects' --json
```

### POST `/api/backup-accounts/{id}/objects/upload`

上传本地备份文件到云端账号

- 认证：Bearer JWT
- 权限：`backup:create`
- operationId：`upload_remote_object`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 账号 ID |

**请求体**（`application/json`，必填）：UploadObjectRequest

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `local_filename` | string | 是 | 本地备份文件名（位于备份目录内，如 `flamepanel-20260907.db`） |
| `remote_key` | string? | 否 | 远端对象 key（缺省用文件名） |

```json
{
  "local_filename": "string",
  "remote_key": "string"
}
```

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 上传成功，返回远端对象 | RemoteObjectDto |
| 400 | 参数错误（文件名非法 / 账号停用） |  |
| 404 | 账号或本地文件不存在 |  |

<details><summary>响应示例</summary>

```json
{
  "key": "string",
  "size": 0
}
```

</details>

```bash
fpctl api POST '/api/backup-accounts/1/objects/upload' -d '{"local_filename":"string","remote_key":"string"}' --json
```

### POST `/api/backup-accounts/{id}/test`

连通性测试（Stage 2：按账号类型构造真实后端探测，结果落库）

- 认证：Bearer JWT
- 权限：`backup:update`
- operationId：`test_account`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 账号 ID |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 测试完成，返回账号（含 last_test_status） | AccountDto |
| 404 | 账号不存在 |  |

<details><summary>响应示例</summary>

```json
{
  "account_type": "string",
  "created_at": "2026-01-01T00:00:00Z",
  "enabled": false,
  "id": 0,
  "last_test_at": "2026-01-01T00:00:00Z",
  "last_test_status": "string",
  "name": "string",
  "test_message": "string",
  "updated_at": "2026-01-01T00:00:00Z"
}
```

</details>

```bash
fpctl api POST '/api/backup-accounts/1/test' --json
```
