<!-- 由 scripts/gen-api-docs.py 从 frontend/openapi.json 生成，请勿手改；修改 utoipa 注解后运行 just api-docs -->

# API · waf

> 9 个端点 · 通用约定见 [README](./README.md)

| 方法 | 路径 | 摘要 | 权限 |
|---|---|---|---|
| POST | `/api/waf/apply` | 下发 WAF 规则到本机 Web 引擎（Nginx / OpenResty 原生配置；Doc/32 W3） | `waf:update` |
| GET | `/api/waf/logs` | WAF 拦截日志（分页，按时间倒序） | `waf:read` |
| POST | `/api/waf/logs/clear` | 清理 WAF 拦截日志（website_id = 0 清空全部） | `waf:update` |
| GET | `/api/waf/rules` | WAF 规则列表（website_id 查询参数过滤，0 = 全部） | `waf:read` |
| POST | `/api/waf/rules` | 创建 WAF 规则 | `waf:create` |
| GET | `/api/waf/rules/{id}` | 获取 WAF 规则详情 | `waf:read` |
| PUT | `/api/waf/rules/{id}` | 更新 WAF 规则 | `waf:update` |
| DELETE | `/api/waf/rules/{id}` | 删除 WAF 规则（拦截日志保留，规则名快照仍可读） | `waf:delete` |
| GET | `/api/waf/status` | WAF 引擎诊断（启用规则数 / 是否配置 Web 引擎下发 / 命中日志回灌状态） | `waf:read` |

### POST `/api/waf/apply`

下发 WAF 规则到本机 Web 引擎（Nginx / OpenResty 原生配置；Doc/32 W3）

规则变更时会自动尽力下发；本端点用于手动重试与查看逐引擎结果（校验失败时已回滚）。
未配置下发能力返回 503。

- 认证：Bearer JWT
- 权限：`waf:update`
- operationId：`apply_rules`

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 逐引擎下发报告 | WafApplyReport |
| 401 | 未认证 |  |
| 403 | 无权限 |  |
| 503 | 未配置 Web 引擎下发 |  |

<details><summary>响应示例</summary>

```json
{
  "engines": [
    {
      "applied": false,
      "enforced": [
        0
      ],
      "engine": "string",
      "error": "string",
      "reason": "string",
      "redeployed_sites": [
        "string"
      ],
      "reloaded": false,
      "skipped": []
    }
  ],
  "ok": false
}
```

</details>

```bash
fpctl api POST '/api/waf/apply' --json
```

### GET `/api/waf/logs`

WAF 拦截日志（分页，按时间倒序）

- 认证：Bearer JWT
- 权限：`waf:read`
- operationId：`list_logs`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `pagination` | query | PaginationParams | 是 |  |
| `website_id` | query | integer(int64) | 否 | 过滤网站 ID（0 = 全部） |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 拦截日志分页 | PaginatedResponse<WafLog>_WafLog |
| 401 | 未认证 |  |

<details><summary>响应示例</summary>

```json
{
  "data": [
    {
      "action": "string",
      "client_ip": "string",
      "created_at": "2026-01-01T00:00:00Z",
      "id": 0,
      "request_summary": "string",
      "rule_id": 0,
      "rule_name": "string",
      "rule_type": "string",
      "user_agent": "string",
      "website_id": 0
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
fpctl api GET '/api/waf/logs' --json
```

### POST `/api/waf/logs/clear`

清理 WAF 拦截日志（website_id = 0 清空全部）

- 认证：Bearer JWT
- 权限：`waf:update`
- operationId：`clear_logs`

**请求体**（`application/json`，必填）：ClearLogsRequest

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `website_id` | integer(int64) | 否 | 0 或缺省 = 清空全部 |

```json
{
  "website_id": 0
}
```

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 清理条数 | integer(int64) (`text/plain`) |
| 401 | 未认证 |  |

```bash
fpctl api POST '/api/waf/logs/clear' -d '{"website_id":0}' --json
```

### GET `/api/waf/rules`

WAF 规则列表（website_id 查询参数过滤，0 = 全部）

- 认证：Bearer JWT
- 权限：`waf:read`
- operationId：`list_rules`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `website_id` | query | integer(int64) | 否 | 网站 ID 过滤（0 = 全部） |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 规则列表 | WafRule[] |
| 401 | 未认证 |  |

<details><summary>响应示例</summary>

```json
[
  {
    "action": "string",
    "config_json": "string",
    "created_at": "2026-01-01T00:00:00Z",
    "enabled": false,
    "id": 0,
    "name": "string",
    "pattern": "string",
    "rule_type": "string",
    "updated_at": "2026-01-01T00:00:00Z",
    "website_id": 0
  }
]
```

</details>

```bash
fpctl api GET '/api/waf/rules' --json
```

### POST `/api/waf/rules`

创建 WAF 规则

- 认证：Bearer JWT
- 权限：`waf:create`
- operationId：`create_rule`

**请求体**（`application/json`，必填）：CreateWafRuleRequest

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `action` | string | 否 | 命中动作：log（默认，仅告警）\| block |
| `config_json` | string | 否 | 类型相关参数 JSON（cc：`{"threshold":120,"window_secs":60}`） |
| `enabled` | boolean | 否 |  |
| `name` | string | 是 |  |
| `pattern` | string | 否 |  |
| `rule_type` | string | 是 | 规则类型：cc \| sql_injection \| xss \| ip_blacklist \| ip_whitelist \| ua_blacklist \| ua_whitelist \| path_block |
| `website_id` | integer(int64) | 否 | 作用网站 ID（0 = 全局规则） |

```json
{
  "action": "string",
  "config_json": "string",
  "enabled": false,
  "name": "string",
  "pattern": "string",
  "rule_type": "string",
  "website_id": 0
}
```

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 创建成功 | WafRule |
| 400 | 参数错误 |  |

<details><summary>响应示例</summary>

```json
{
  "action": "string",
  "config_json": "string",
  "created_at": "2026-01-01T00:00:00Z",
  "enabled": false,
  "id": 0,
  "name": "string",
  "pattern": "string",
  "rule_type": "string",
  "updated_at": "2026-01-01T00:00:00Z",
  "website_id": 0
}
```

</details>

```bash
fpctl api POST '/api/waf/rules' -d '{"action":"string","config_json":"string","enabled":false,"name":"string","pattern":"string","rule_type":"string","website_id":0}' --json
```

### GET `/api/waf/rules/{id}`

获取 WAF 规则详情

- 认证：Bearer JWT
- 权限：`waf:read`
- operationId：`get_rule`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 规则 ID |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 规则详情 | WafRule |
| 404 | 规则不存在 |  |

<details><summary>响应示例</summary>

```json
{
  "action": "string",
  "config_json": "string",
  "created_at": "2026-01-01T00:00:00Z",
  "enabled": false,
  "id": 0,
  "name": "string",
  "pattern": "string",
  "rule_type": "string",
  "updated_at": "2026-01-01T00:00:00Z",
  "website_id": 0
}
```

</details>

```bash
fpctl api GET '/api/waf/rules/1' --json
```

### PUT `/api/waf/rules/{id}`

更新 WAF 规则

- 认证：Bearer JWT
- 权限：`waf:update`
- operationId：`update_rule`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 规则 ID |

**请求体**（`application/json`，必填）：UpdateWafRuleRequest

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `action` | string? | 否 |  |
| `config_json` | string? | 否 |  |
| `enabled` | boolean? | 否 |  |
| `name` | string? | 否 |  |
| `pattern` | string? | 否 |  |
| `rule_type` | string? | 否 |  |
| `website_id` | integer(int64)? | 否 |  |

```json
{
  "action": "string",
  "config_json": "string",
  "enabled": false,
  "name": "string",
  "pattern": "string",
  "rule_type": "string",
  "website_id": 0
}
```

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 更新成功 | WafRule |
| 404 | 规则不存在 |  |

<details><summary>响应示例</summary>

```json
{
  "action": "string",
  "config_json": "string",
  "created_at": "2026-01-01T00:00:00Z",
  "enabled": false,
  "id": 0,
  "name": "string",
  "pattern": "string",
  "rule_type": "string",
  "updated_at": "2026-01-01T00:00:00Z",
  "website_id": 0
}
```

</details>

```bash
fpctl api PUT '/api/waf/rules/1' -d '{"action":"string","config_json":"string","enabled":false,"name":"string","pattern":"string","rule_type":"string","website_id":0}' --json
```

### DELETE `/api/waf/rules/{id}`

删除 WAF 规则（拦截日志保留，规则名快照仍可读）

- 认证：Bearer JWT
- 权限：`waf:delete`
- operationId：`delete_rule`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 规则 ID |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 删除成功 |  |
| 404 | 规则不存在 |  |

```bash
fpctl api DELETE '/api/waf/rules/1' --json
```

### GET `/api/waf/status`

WAF 引擎诊断（启用规则数 / 是否配置 Web 引擎下发 / 命中日志回灌状态）

- 认证：Bearer JWT
- 权限：`waf:read`
- operationId：`status`

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 引擎状态 | WafEngineStatus |
| 401 | 未认证 |  |

<details><summary>响应示例</summary>

```json
{
  "enforcement": false,
  "ingest": [
    {
      "engine": "string",
      "imported": 0,
      "last_error": "string",
      "last_run_at": "2026-01-01T00:00:00Z",
      "offset": 0
    }
  ],
  "loaded_rules": 0
}
```

</details>

```bash
fpctl api GET '/api/waf/status' --json
```
