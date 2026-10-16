<!-- 由 scripts/gen-api-docs.py 从 frontend/openapi.json 生成，请勿手改；修改 utoipa 注解后运行 just api-docs -->

# API · certificates

> 10 个端点 · 通用约定见 [README](./README.md)

| 方法 | 路径 | 摘要 | 权限 |
|---|---|---|---|
| GET | `/api/certificates` | 证书列表 | `certificate:read` |
| POST | `/api/certificates` | 上传第三方证书 | `certificate:update` |
| POST | `/api/certificates/issue/acme` | ACME 签发（Let's Encrypt 等；HTTP-01 / DNS-01） | `certificate:update` |
| POST | `/api/certificates/issue/self-signed` | 自签证书（内网/无公网场景） | `certificate:update` |
| GET | `/api/certificates/{id}` | 证书详情 | `certificate:read` |
| PUT | `/api/certificates/{id}` | 更新证书（auto_renew / website_id） | `certificate:update` |
| DELETE | `/api/certificates/{id}` | 删除证书记录（不删除落盘文件——运维可手动留存） | `certificate:delete` |
| POST | `/api/certificates/{id}/bind` | 证书绑定/解绑网站（website_id = None 解绑） | `certificate:update` |
| POST | `/api/certificates/{id}/renew` | 手动触发续签（ACME 来源；自签/上传返回 400） | `certificate:update` |
| GET | `/api/certificates/{id}/renewals` | 续签历史（S-10）：签发 / 自动续签成功与失败 / 到期预警时间线，数据来自事件落库 | `certificate:read` |

### GET `/api/certificates`

证书列表

- 认证：Bearer JWT
- 权限：`certificate:read`
- operationId：`list_certificates`

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 证书列表 | CertificateListResponse |
| 401 | 未认证 |  |

<details><summary>响应示例</summary>

```json
{
  "certificates": [
    {
      "auto_renew": false,
      "cert_path": "string",
      "created_at": "2026-01-01T00:00:00Z",
      "dns_provider": "string",
      "domains": "string",
      "expires_at": "2026-01-01T00:00:00Z",
      "id": 0,
      "key_path": "string",
      "last_renewed_at": "2026-01-01T00:00:00Z",
      "source": "string",
      "updated_at": "2026-01-01T00:00:00Z",
      "website_id": 0
    }
  ],
  "total": 0
}
```

</details>

```bash
fpctl api GET '/api/certificates' --json
```

### POST `/api/certificates`

上传第三方证书

- 认证：Bearer JWT
- 权限：`certificate:update`
- operationId：`upload_certificate`

**请求体**（`application/json`，必填）：UploadCertificateRequest

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `cert_pem` | string | 是 | 证书 PEM（全链，含中间证书） |
| `domains` | string[] | 是 |  |
| `key_pem` | string | 是 | 私钥 PEM |
| `website_id` | integer(int64)? | 否 |  |

```json
{
  "cert_pem": "string",
  "domains": [
    "string"
  ],
  "key_pem": "string",
  "website_id": 0
}
```

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 上传成功 | Certificate |
| 400 | PEM 非法 / 域名列表非法 |  |

<details><summary>响应示例</summary>

```json
{
  "auto_renew": false,
  "cert_path": "string",
  "created_at": "2026-01-01T00:00:00Z",
  "dns_provider": "string",
  "domains": "string",
  "expires_at": "2026-01-01T00:00:00Z",
  "id": 0,
  "key_path": "string",
  "last_renewed_at": "2026-01-01T00:00:00Z",
  "source": "string",
  "updated_at": "2026-01-01T00:00:00Z",
  "website_id": 0
}
```

</details>

```bash
fpctl api POST '/api/certificates' -d '{"cert_pem":"string","domains":["string"],"key_pem":"string","website_id":0}' --json
```

### POST `/api/certificates/issue/acme`

ACME 签发（Let's Encrypt 等；HTTP-01 / DNS-01）

- 认证：Bearer JWT
- 权限：`certificate:update`
- operationId：`issue_acme`

**请求体**（`application/json`，必填）：IssueAcmeRequest

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `directory_url` | string? | 否 | ACME 目录 URL（空 = Let's Encrypt 生产；staging 联调见常量） |
| `dns01` | boolean | 否 | 是否 DNS-01（泛域名必须） |
| `dns_provider_account_id` | integer(int64)? | 否 | DNS-01 厂商账号 ID（Stage 5：按账号解析真实厂商凭据；泛域名必填） |
| `domains` | string[] | 是 | SAN 域名列表（主域名 + 备用/泛域名） |
| `webroot` | string? | 否 | HTTP-01 webroot（网站 root_path；DNS-01 时忽略） |
| `website_id` | integer(int64)? | 否 | 签发后绑定网站 ID（可选） |

```json
{
  "directory_url": "string",
  "dns01": false,
  "dns_provider_account_id": 0,
  "domains": [
    "string"
  ],
  "webroot": "string",
  "website_id": 0
}
```

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 签发成功，返回证书记录 | Certificate |
| 400 | 参数非法（泛域名未用 DNS-01 / 未配 DNS 厂商） |  |
| 502 | ACME 服务器/验证失败 |  |

<details><summary>响应示例</summary>

```json
{
  "auto_renew": false,
  "cert_path": "string",
  "created_at": "2026-01-01T00:00:00Z",
  "dns_provider": "string",
  "domains": "string",
  "expires_at": "2026-01-01T00:00:00Z",
  "id": 0,
  "key_path": "string",
  "last_renewed_at": "2026-01-01T00:00:00Z",
  "source": "string",
  "updated_at": "2026-01-01T00:00:00Z",
  "website_id": 0
}
```

</details>

```bash
fpctl api POST '/api/certificates/issue/acme' -d '{"directory_url":"string","dns01":false,"dns_provider_account_id":0,"domains":["string"],"webroot":"string","website_id":0}' --json
```

### POST `/api/certificates/issue/self-signed`

自签证书（内网/无公网场景）

- 认证：Bearer JWT
- 权限：`certificate:update`
- operationId：`issue_self_signed`

**请求体**（`application/json`，必填）：IssueSelfSignedRequest

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `domains` | string[] | 是 |  |
| `website_id` | integer(int64)? | 否 |  |

```json
{
  "domains": [
    "string"
  ],
  "website_id": 0
}
```

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 签发成功 | Certificate |
| 400 | 域名列表非法 |  |

<details><summary>响应示例</summary>

```json
{
  "auto_renew": false,
  "cert_path": "string",
  "created_at": "2026-01-01T00:00:00Z",
  "dns_provider": "string",
  "domains": "string",
  "expires_at": "2026-01-01T00:00:00Z",
  "id": 0,
  "key_path": "string",
  "last_renewed_at": "2026-01-01T00:00:00Z",
  "source": "string",
  "updated_at": "2026-01-01T00:00:00Z",
  "website_id": 0
}
```

</details>

```bash
fpctl api POST '/api/certificates/issue/self-signed' -d '{"domains":["string"],"website_id":0}' --json
```

### GET `/api/certificates/{id}`

证书详情

- 认证：Bearer JWT
- 权限：`certificate:read`
- operationId：`get_certificate`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 证书 ID |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 证书详情 | Certificate |
| 404 | 证书记录不存在 |  |

<details><summary>响应示例</summary>

```json
{
  "auto_renew": false,
  "cert_path": "string",
  "created_at": "2026-01-01T00:00:00Z",
  "dns_provider": "string",
  "domains": "string",
  "expires_at": "2026-01-01T00:00:00Z",
  "id": 0,
  "key_path": "string",
  "last_renewed_at": "2026-01-01T00:00:00Z",
  "source": "string",
  "updated_at": "2026-01-01T00:00:00Z",
  "website_id": 0
}
```

</details>

```bash
fpctl api GET '/api/certificates/1' --json
```

### PUT `/api/certificates/{id}`

更新证书（auto_renew / website_id）

- 认证：Bearer JWT
- 权限：`certificate:update`
- operationId：`update_certificate`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 证书 ID |

**请求体**（`application/json`，必填）：UpdateCertificateRequest

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `auto_renew` | boolean? | 否 |  |
| `website_id` | integer(int64)? | 否 |  |

```json
{
  "auto_renew": false,
  "website_id": 0
}
```

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 更新后的证书 | Certificate |
| 400 | 自签 / 上传证书不支持开启自动续签 |  |
| 404 | 证书记录不存在 |  |

<details><summary>响应示例</summary>

```json
{
  "auto_renew": false,
  "cert_path": "string",
  "created_at": "2026-01-01T00:00:00Z",
  "dns_provider": "string",
  "domains": "string",
  "expires_at": "2026-01-01T00:00:00Z",
  "id": 0,
  "key_path": "string",
  "last_renewed_at": "2026-01-01T00:00:00Z",
  "source": "string",
  "updated_at": "2026-01-01T00:00:00Z",
  "website_id": 0
}
```

</details>

```bash
fpctl api PUT '/api/certificates/1' -d '{"auto_renew":false,"website_id":0}' --json
```

### DELETE `/api/certificates/{id}`

删除证书记录（不删除落盘文件——运维可手动留存）

- 认证：Bearer JWT
- 权限：`certificate:delete`
- operationId：`delete_certificate`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 证书 ID |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 删除成功 |  |
| 404 | 证书记录不存在 |  |

```bash
fpctl api DELETE '/api/certificates/1' --json
```

### POST `/api/certificates/{id}/bind`

证书绑定/解绑网站（website_id = None 解绑）

- 认证：Bearer JWT
- 权限：`certificate:update`
- operationId：`bind_website`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 证书 ID |

**请求体**（`application/json`，必填）：BindCertificateRequest

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `website_id` | integer(int64)? | 否 |  |

```json
{
  "website_id": 0
}
```

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 绑定后的证书 | Certificate |
| 400 | 证书 SAN 不覆盖站点域名 |  |
| 404 | 证书/网站不存在 |  |

<details><summary>响应示例</summary>

```json
{
  "auto_renew": false,
  "cert_path": "string",
  "created_at": "2026-01-01T00:00:00Z",
  "dns_provider": "string",
  "domains": "string",
  "expires_at": "2026-01-01T00:00:00Z",
  "id": 0,
  "key_path": "string",
  "last_renewed_at": "2026-01-01T00:00:00Z",
  "source": "string",
  "updated_at": "2026-01-01T00:00:00Z",
  "website_id": 0
}
```

</details>

```bash
fpctl api POST '/api/certificates/1/bind' -d '{"website_id":0}' --json
```

### POST `/api/certificates/{id}/renew`

手动触发续签（ACME 来源；自签/上传返回 400）

- 认证：Bearer JWT
- 权限：`certificate:update`
- operationId：`renew_certificate`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 证书 ID |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 续签成功，返回刷新后的证书记录 | Certificate |
| 400 | 来源不支持自动续签（自签/上传）或验证配置缺失 |  |
| 404 | 证书记录不存在 |  |
| 502 | ACME 签发失败 |  |

<details><summary>响应示例</summary>

```json
{
  "auto_renew": false,
  "cert_path": "string",
  "created_at": "2026-01-01T00:00:00Z",
  "dns_provider": "string",
  "domains": "string",
  "expires_at": "2026-01-01T00:00:00Z",
  "id": 0,
  "key_path": "string",
  "last_renewed_at": "2026-01-01T00:00:00Z",
  "source": "string",
  "updated_at": "2026-01-01T00:00:00Z",
  "website_id": 0
}
```

</details>

```bash
fpctl api POST '/api/certificates/1/renew' --json
```

### GET `/api/certificates/{id}/renewals`

续签历史（S-10）：签发 / 自动续签成功与失败 / 到期预警时间线，数据来自事件落库

- 认证：Bearer JWT
- 权限：`certificate:read`
- operationId：`renewal_history`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 证书 ID |
| `limit` | query | integer? | 否 | 返回条数：缺省 50，范围 1–200 |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 续签概况与时间线（最新在前） | CertificateRenewalHistory |
| 401 | 未认证 |  |
| 403 | 无 certificate:read 权限 |  |
| 404 | 证书记录不存在 |  |

<details><summary>响应示例</summary>

```json
{
  "auto_renew": false,
  "certificate_id": 0,
  "entries": [
    {
      "at": "2026-01-01T00:00:00Z",
      "domain": "string",
      "error": "string",
      "event_id": 0,
      "expires_at": "string",
      "kind": "issued",
      "source": "string"
    }
  ],
  "expires_at": "2026-01-01T00:00:00Z",
  "last_renewed_at": "2026-01-01T00:00:00Z",
  "renewal_window_starts_at": "2026-01-01T00:00:00Z",
  "scan_interval_hours": 0,
  "supports_auto_renew": false,
  "truncated": false
}
```

</details>

```bash
fpctl api GET '/api/certificates/1/renewals' --json
```
