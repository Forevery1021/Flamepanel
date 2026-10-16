<!-- 由 scripts/gen-api-docs.py 从 frontend/openapi.json 生成，请勿手改；修改 utoipa 注解后运行 just api-docs -->

# API · dns_providers

> 5 个端点 · 通用约定见 [README](./README.md)

| 方法 | 路径 | 摘要 | 权限 |
|---|---|---|---|
| GET | `/api/dns-providers` | DNS 厂商账号列表 | `dns_provider:read` |
| POST | `/api/dns-providers` | 创建 DNS 厂商账号 | `dns_provider:create` |
| PUT | `/api/dns-providers/{id}` | 更新 DNS 厂商账号 | `dns_provider:update` |
| DELETE | `/api/dns-providers/{id}` | 删除 DNS 厂商账号 | `dns_provider:delete` |
| POST | `/api/dns-providers/{id}/test` | 连通性测试（Stage 4：凭据结构校验级探测；Stage 5 升级为真实厂商 API 调用） | `dns_provider:update` |

### GET `/api/dns-providers`

DNS 厂商账号列表

- 认证：Bearer JWT
- 权限：`dns_provider:read`
- operationId：`list_accounts`

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 账号列表 | DnsProviderAccountDto[] |
| 401 | 未认证 |  |

<details><summary>响应示例</summary>

```json
[
  {
    "created_at": "2026-01-01T00:00:00Z",
    "enabled": false,
    "id": 0,
    "last_test_at": "2026-01-01T00:00:00Z",
    "last_test_status": "string",
    "name": "string",
    "provider_type": "string",
    "test_message": "string",
    "updated_at": "2026-01-01T00:00:00Z"
  }
]
```

</details>

```bash
fpctl api GET '/api/dns-providers' --json
```

### POST `/api/dns-providers`

创建 DNS 厂商账号

- 认证：Bearer JWT
- 权限：`dns_provider:create`
- operationId：`create_account`

**请求体**（`application/json`，必填）：CreateDnsProviderRequest

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `credentials_json` | string? | 否 | 厂商凭据（JSON 字符串，落库前统一加密） |
| `enabled` | boolean? | 否 |  |
| `name` | string | 是 |  |
| `provider_type` | string | 是 | 厂商标识：cloudflare \| aliyun \| tencent \| huawei |

```json
{
  "credentials_json": "string",
  "enabled": false,
  "name": "string",
  "provider_type": "string"
}
```

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 创建成功 | DnsProviderAccountDto |
| 400 | 参数错误 |  |

<details><summary>响应示例</summary>

```json
{
  "created_at": "2026-01-01T00:00:00Z",
  "enabled": false,
  "id": 0,
  "last_test_at": "2026-01-01T00:00:00Z",
  "last_test_status": "string",
  "name": "string",
  "provider_type": "string",
  "test_message": "string",
  "updated_at": "2026-01-01T00:00:00Z"
}
```

</details>

```bash
fpctl api POST '/api/dns-providers' -d '{"credentials_json":"string","enabled":false,"name":"string","provider_type":"string"}' --json
```

### PUT `/api/dns-providers/{id}`

更新 DNS 厂商账号

- 认证：Bearer JWT
- 权限：`dns_provider:update`
- operationId：`update_account`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 账号 ID |

**请求体**（`application/json`，必填）：UpdateDnsProviderRequest

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `credentials_json` | string? | 否 |  |
| `enabled` | boolean? | 否 |  |
| `name` | string? | 否 |  |
| `provider_type` | string? | 否 |  |

```json
{
  "credentials_json": "string",
  "enabled": false,
  "name": "string",
  "provider_type": "string"
}
```

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 更新成功 | DnsProviderAccountDto |
| 404 | 账号不存在 |  |

<details><summary>响应示例</summary>

```json
{
  "created_at": "2026-01-01T00:00:00Z",
  "enabled": false,
  "id": 0,
  "last_test_at": "2026-01-01T00:00:00Z",
  "last_test_status": "string",
  "name": "string",
  "provider_type": "string",
  "test_message": "string",
  "updated_at": "2026-01-01T00:00:00Z"
}
```

</details>

```bash
fpctl api PUT '/api/dns-providers/1' -d '{"credentials_json":"string","enabled":false,"name":"string","provider_type":"string"}' --json
```

### DELETE `/api/dns-providers/{id}`

删除 DNS 厂商账号

- 认证：Bearer JWT
- 权限：`dns_provider:delete`
- operationId：`delete_account`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 账号 ID |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 删除成功 |  |
| 404 | 账号不存在 |  |

```bash
fpctl api DELETE '/api/dns-providers/1' --json
```

### POST `/api/dns-providers/{id}/test`

连通性测试（Stage 4：凭据结构校验级探测；Stage 5 升级为真实厂商 API 调用）

- 认证：Bearer JWT
- 权限：`dns_provider:update`
- operationId：`test_account`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 账号 ID |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 测试完成，返回账号（含 last_test_status） | DnsProviderAccountDto |
| 404 | 账号不存在 |  |

<details><summary>响应示例</summary>

```json
{
  "created_at": "2026-01-01T00:00:00Z",
  "enabled": false,
  "id": 0,
  "last_test_at": "2026-01-01T00:00:00Z",
  "last_test_status": "string",
  "name": "string",
  "provider_type": "string",
  "test_message": "string",
  "updated_at": "2026-01-01T00:00:00Z"
}
```

</details>

```bash
fpctl api POST '/api/dns-providers/1/test' --json
```
