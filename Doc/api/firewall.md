<!-- 由 scripts/gen-api-docs.py 从 frontend/openapi.json 生成，请勿手改；修改 utoipa 注解后运行 just api-docs -->

# API · firewall

> 11 个端点 · 通用约定见 [README](./README.md)

| 方法 | 路径 | 摘要 | 权限 |
|---|---|---|---|
| POST | `/api/firewall/apply` | 把全部已启用规则下发到系统防火墙（ufw / firewalld / iptables；面板非 root 时经特权助手） | `firewall:apply` |
| POST | `/api/firewall/disable` | 停用系统防火墙 | `firewall:enable` |
| POST | `/api/firewall/enable` | 启用系统防火墙 | `firewall:enable` |
| POST | `/api/firewall/reorder` | 按给定顺序重排规则优先级 | `firewall:update` |
| GET | `/api/firewall/rules` | 防火墙规则列表（分页，按优先级） | `firewall:read` |
| POST | `/api/firewall/rules` | 新建防火墙规则（仅落库，需 apply 才生效） | `firewall:create` |
| GET | `/api/firewall/rules/{id}` | 单条防火墙规则 | `firewall:read` |
| PUT | `/api/firewall/rules/{id}` | 更新防火墙规则（字段缺省即保持不变） | `firewall:update` |
| DELETE | `/api/firewall/rules/{id}` | 删除防火墙规则 | `firewall:delete` |
| POST | `/api/firewall/rules/{id}/toggle` | 启用 / 停用单条规则 | `firewall:enable` |
| GET | `/api/firewall/status` | 防火墙后端状态（检测到的后端、是否启用等键值） | `database:read` |

### POST `/api/firewall/apply`

把全部已启用规则下发到系统防火墙（ufw / firewalld / iptables；面板非 root 时经特权助手）

- 认证：Bearer JWT
- 权限：`firewall:apply`
- operationId：`apply_all`

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | `"applied"` | string (`text/plain`) |
| 401 | 未认证 |  |
| 403 | 缺少 firewall:apply 权限 |  |
| 500 | 防火墙后端执行失败 |  |

```bash
fpctl api POST '/api/firewall/apply' --json
```

### POST `/api/firewall/disable`

停用系统防火墙

- 认证：Bearer JWT
- 权限：`firewall:enable`
- operationId：`disable`

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | `"disabled"` | string (`text/plain`) |
| 401 | 未认证 |  |
| 403 | 缺少 firewall:enable 权限 |  |

```bash
fpctl api POST '/api/firewall/disable' --json
```

### POST `/api/firewall/enable`

启用系统防火墙

- 认证：Bearer JWT
- 权限：`firewall:enable`
- operationId：`enable`

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | `"enabled"` | string (`text/plain`) |
| 401 | 未认证 |  |
| 403 | 缺少 firewall:enable 权限 |  |

```bash
fpctl api POST '/api/firewall/enable' --json
```

### POST `/api/firewall/reorder`

按给定顺序重排规则优先级

- 认证：Bearer JWT
- 权限：`firewall:update`
- operationId：`reorder`

**请求体**（`application/json`，必填）：ReorderRequest

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `ids` | integer(int64)[] | 是 | 规则 ID，按顺序赋优先级 10、20、30…（单事务）；未列出的规则保持原优先级，不存在的 ID 忽略 |

```json
{
  "ids": [
    0
  ]
}
```

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | `"reordered"` | string (`text/plain`) |
| 401 | 未认证 |  |
| 403 | 缺少 firewall:update 权限 |  |

```bash
fpctl api POST '/api/firewall/reorder' -d '{"ids":[0]}' --json
```

### GET `/api/firewall/rules`

防火墙规则列表（分页，按优先级）

- 认证：Bearer JWT
- 权限：`firewall:read`
- operationId：`list`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `page` | query | integer(int64)? | 否 | 页码（从 1 开始） |
| `page_size` | query | integer(int64)? | 否 | 每页条数（1~200） |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 规则分页 | PaginatedResponse<FirewallRuleResponse>_FirewallRuleResponse |
| 401 | 未认证 |  |
| 403 | 缺少 firewall:read 权限 |  |

<details><summary>响应示例</summary>

```json
{
  "data": [
    {
      "action": "string",
      "created_at": "string",
      "description": "string",
      "destination": "string",
      "direction": "string",
      "enabled": false,
      "id": 0,
      "name": "string",
      "port": "string",
      "priority": 0,
      "protocol": "string",
      "source": "string",
      "updated_at": "string"
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
fpctl api GET '/api/firewall/rules' --json
```

### POST `/api/firewall/rules`

新建防火墙规则（仅落库，需 apply 才生效）

- 认证：Bearer JWT
- 权限：`firewall:create`
- operationId：`create`

**请求体**（`application/json`，必填）：CreateFirewallRuleRequest

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `action` | string? | 否 |  |
| `description` | string? | 否 | 缺省 `tcp`；`tcp` / `udp` / `both` / `icmp` |
| `destination` | string? | 否 |  |
| `direction` | string? | 否 |  |
| `enabled` | boolean? | 否 |  |
| `name` | string | 是 |  |
| `port` | string? | 否 |  |
| `priority` | integer(int32)? | 否 |  |
| `protocol` | string? | 否 |  |
| `source` | string? | 否 |  |

```json
{
  "action": "string",
  "description": "string",
  "destination": "string",
  "direction": "string",
  "enabled": false,
  "name": "string",
  "port": "string",
  "priority": 0,
  "protocol": "string",
  "source": "string"
}
```

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 已创建 | FirewallRuleResponse |
| 400 | 端口 / 地址 / 协议 / 方向校验失败 |  |
| 401 | 未认证 |  |
| 403 | 缺少 firewall:create 权限 |  |

<details><summary>响应示例</summary>

```json
{
  "action": "string",
  "created_at": "string",
  "description": "string",
  "destination": "string",
  "direction": "string",
  "enabled": false,
  "id": 0,
  "name": "string",
  "port": "string",
  "priority": 0,
  "protocol": "string",
  "source": "string",
  "updated_at": "string"
}
```

</details>

```bash
fpctl api POST '/api/firewall/rules' -d '{"action":"string","description":"string","destination":"string","direction":"string","enabled":false,"name":"string","port":"string","priority":0,"protocol":"string","source":"string"}' --json
```

### GET `/api/firewall/rules/{id}`

单条防火墙规则

- 认证：Bearer JWT
- 权限：`firewall:read`
- operationId：`get`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 规则 ID |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 规则 | FirewallRuleResponse |
| 401 | 未认证 |  |
| 403 | 缺少 firewall:read 权限 |  |
| 404 | 规则不存在 |  |

<details><summary>响应示例</summary>

```json
{
  "action": "string",
  "created_at": "string",
  "description": "string",
  "destination": "string",
  "direction": "string",
  "enabled": false,
  "id": 0,
  "name": "string",
  "port": "string",
  "priority": 0,
  "protocol": "string",
  "source": "string",
  "updated_at": "string"
}
```

</details>

```bash
fpctl api GET '/api/firewall/rules/1' --json
```

### PUT `/api/firewall/rules/{id}`

更新防火墙规则（字段缺省即保持不变）

- 认证：Bearer JWT
- 权限：`firewall:update`
- operationId：`update`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 规则 ID |

**请求体**（`application/json`，必填）：UpdateFirewallRuleRequest

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `action` | string? | 否 |  |
| `description` | string? | 否 |  |
| `destination` | string? | 否 |  |
| `direction` | string? | 否 |  |
| `enabled` | boolean? | 否 |  |
| `name` | string? | 否 |  |
| `port` | string? | 否 |  |
| `priority` | integer(int32)? | 否 |  |
| `protocol` | string? | 否 |  |
| `source` | string? | 否 |  |

```json
{
  "action": "string",
  "description": "string",
  "destination": "string",
  "direction": "string",
  "enabled": false,
  "name": "string",
  "port": "string",
  "priority": 0,
  "protocol": "string",
  "source": "string"
}
```

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 已更新 | FirewallRuleResponse |
| 400 | 端口 / 地址 / 协议 / 方向校验失败 |  |
| 401 | 未认证 |  |
| 403 | 缺少 firewall:update 权限 |  |
| 404 | 规则不存在 |  |

<details><summary>响应示例</summary>

```json
{
  "action": "string",
  "created_at": "string",
  "description": "string",
  "destination": "string",
  "direction": "string",
  "enabled": false,
  "id": 0,
  "name": "string",
  "port": "string",
  "priority": 0,
  "protocol": "string",
  "source": "string",
  "updated_at": "string"
}
```

</details>

```bash
fpctl api PUT '/api/firewall/rules/1' -d '{"action":"string","description":"string","destination":"string","direction":"string","enabled":false,"name":"string","port":"string","priority":0,"protocol":"string","source":"string"}' --json
```

### DELETE `/api/firewall/rules/{id}`

删除防火墙规则

- 认证：Bearer JWT
- 权限：`firewall:delete`
- operationId：`delete`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 规则 ID |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | `"deleted"` | string (`text/plain`) |
| 401 | 未认证 |  |
| 403 | 缺少 firewall:delete 权限 |  |
| 404 | 规则不存在 |  |

```bash
fpctl api DELETE '/api/firewall/rules/1' --json
```

### POST `/api/firewall/rules/{id}/toggle`

启用 / 停用单条规则

- 认证：Bearer JWT
- 权限：`firewall:enable`
- operationId：`toggle`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 规则 ID |

**请求体**（`application/json`，必填）：ToggleRequest

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
| 200 | 切换后的规则 | FirewallRuleResponse |
| 401 | 未认证 |  |
| 403 | 缺少 firewall:enable 权限 |  |
| 404 | 规则不存在 |  |

<details><summary>响应示例</summary>

```json
{
  "action": "string",
  "created_at": "string",
  "description": "string",
  "destination": "string",
  "direction": "string",
  "enabled": false,
  "id": 0,
  "name": "string",
  "port": "string",
  "priority": 0,
  "protocol": "string",
  "source": "string",
  "updated_at": "string"
}
```

</details>

```bash
fpctl api POST '/api/firewall/rules/1/toggle' -d '{"enabled":false}' --json
```

### GET `/api/firewall/status`

防火墙后端状态（检测到的后端、是否启用等键值）

- 认证：Bearer JWT
- 权限：`database:read`
- operationId：`get_status`

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 状态键值 | map<string, string> |
| 401 | 未认证 |  |
| 403 | 缺少 firewall:read 权限 |  |

```bash
fpctl api GET '/api/firewall/status' --json
```
