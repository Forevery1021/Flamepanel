<!-- 由 scripts/gen-api-docs.py 从 frontend/openapi.json 生成，请勿手改；修改 utoipa 注解后运行 just api-docs -->

# API · mcp

> 15 个端点 · 通用约定见 [README](./README.md)

| 方法 | 路径 | 摘要 | 权限 |
|---|---|---|---|
| GET | `/api/mcp` | `GET /api/mcp` — 不提供 SSE 推送流。 | 登录即可 |
| POST | `/api/mcp` | `POST /api/mcp` — MCP JSON-RPC 消息（单条或批量） | 登录即可 |
| DELETE | `/api/mcp` | `DELETE /api/mcp` — 终止会话。 | 登录即可 |
| GET | `/api/mcp/approvals` | `GET /api/mcp/approvals` — 审批单分页 | `mcp:read` |
| GET | `/api/mcp/approvals/{id}` | `GET /api/mcp/approvals/{id}` — 审批单详情 | `mcp:read` |
| POST | `/api/mcp/approvals/{id}/approve` | `POST /api/mcp/approvals/{id}/approve` — 批准并执行 | `mcp:update` |
| POST | `/api/mcp/approvals/{id}/reject` | `POST /api/mcp/approvals/{id}/reject` — 拒绝 | `mcp:update` |
| GET | `/api/mcp/calls` | `GET /api/mcp/calls` — 工具调用审计 | `mcp:read` |
| GET | `/api/mcp/principals` | `GET /api/mcp/principals` — 凭证列表 | `mcp:read` |
| POST | `/api/mcp/principals` | `POST /api/mcp/principals` — 为当前用户签发凭证 | `mcp:create` |
| POST | `/api/mcp/principals/{id}/revoke` | `POST /api/mcp/principals/{id}/revoke` — 吊销凭证 | `mcp:create` |
| GET | `/api/mcp/prompts` | `GET /api/mcp/prompts` — 运维技能库 | `mcp:read` |
| GET | `/api/mcp/prompts/{name}` | `GET /api/mcp/prompts/{name}` — 渲染技能（查询参数即技能参数） | `mcp:read` |
| GET | `/api/mcp/sessions` | `GET /api/mcp/sessions` — 会话列表 | `mcp:read` |
| GET | `/api/mcp/tools` | `GET /api/mcp/tools` — 工具目录 | `mcp:read` |

### GET `/api/mcp`

`GET /api/mcp` — 不提供 SSE 推送流。

- 认证：Bearer JWT
- 权限：登录即可
- operationId：`get`

| 状态 | 说明 | 响应体 |
|---|---|---|
| 405 | 不提供服务端推送流 |  |

```bash
fpctl api GET '/api/mcp' --json
```

### POST `/api/mcp`

`POST /api/mcp` — MCP JSON-RPC 消息（单条或批量）

- 认证：Bearer JWT
- 权限：登录即可
- operationId：`post`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `Mcp-Session-Id` | header | string? | 否 | initialize 下发的会话 ID；缺省为无状态模式 |

**请求体**（`application/json`，必填）：object

```json
{}
```

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | JSON-RPC 响应（initialize 时响应头含 Mcp-Session-Id） | object |
| 202 | 仅通知 / 响应，无响应体 |  |
| 400 | JSON 解析失败或非法批量 |  |
| 401 | fpk_ 凭证缺失 / 无效 / 吊销 / 过期 |  |
| 403 | Origin 与 Host 不一致 |  |
| 404 | 会话不存在或已终止 |  |

```bash
fpctl api POST '/api/mcp' -d '{}' --json
```

### DELETE `/api/mcp`

`DELETE /api/mcp` — 终止会话。

- 认证：Bearer JWT
- 权限：登录即可
- operationId：`delete`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `Mcp-Session-Id` | header | string | 是 | 要终止的会话 ID |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 204 | 会话已终止 |  |
| 400 | 缺少 Mcp-Session-Id |  |
| 401 | fpk_ 凭证无效 |  |
| 404 | 会话不存在或已终止 |  |

```bash
fpctl api DELETE '/api/mcp' --json
```

### GET `/api/mcp/approvals`

`GET /api/mcp/approvals` — 审批单分页

- 认证：Bearer JWT
- 权限：`mcp:read`
- operationId：`list_approvals`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `status` | query | string? | 否 | 按状态过滤：pending / executing / succeeded / failed / rejected / expired |
| `principal_id` | query | integer(int64)? | 否 |  |
| `page` | query | integer(int64)? | 否 |  |
| `page_size` | query | integer(int64)? | 否 |  |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 审批单分页（最新在前） | PaginatedResponse<ToolApproval>_ToolApproval |
| 400 | status 不合法 |  |
| 401 | 未认证 |  |
| 403 | 缺少 mcp:read 权限 |  |

<details><summary>响应示例</summary>

```json
{
  "data": [
    {
      "arguments": "string",
      "decided_at": "2026-01-01T00:00:00Z",
      "decided_by": 0,
      "expires_at": "2026-01-01T00:00:00Z",
      "finished_at": "2026-01-01T00:00:00Z",
      "id": 0,
      "preview": "string",
      "principal_id": 0,
      "reason": "string",
      "requested_at": "2026-01-01T00:00:00Z",
      "result": "string",
      "risk": "read",
      "session_id": "string",
      "status": "pending",
      "tool": "string"
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
fpctl api GET '/api/mcp/approvals' --json
```

### GET `/api/mcp/approvals/{id}`

`GET /api/mcp/approvals/{id}` — 审批单详情

- 认证：Bearer JWT
- 权限：`mcp:read`
- operationId：`get_approval`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 审批单 ID |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 审批单 | ToolApproval |
| 401 | 未认证 |  |
| 403 | 缺少 mcp:read 权限 |  |
| 404 | 不存在 |  |

<details><summary>响应示例</summary>

```json
{
  "arguments": "string",
  "decided_at": "2026-01-01T00:00:00Z",
  "decided_by": 0,
  "expires_at": "2026-01-01T00:00:00Z",
  "finished_at": "2026-01-01T00:00:00Z",
  "id": 0,
  "preview": "string",
  "principal_id": 0,
  "reason": "string",
  "requested_at": "2026-01-01T00:00:00Z",
  "result": "string",
  "risk": "read",
  "session_id": "string",
  "status": "pending",
  "tool": "string"
}
```

</details>

```bash
fpctl api GET '/api/mcp/approvals/1' --json
```

### POST `/api/mcp/approvals/{id}/approve`

`POST /api/mcp/approvals/{id}/approve` — 批准并执行

- 认证：Bearer JWT
- 权限：`mcp:update`
- operationId：`approve`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 审批单 ID |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 已执行；status 为 succeeded 或 failed，result 为输出或错误 | ToolApproval |
| 401 | 未认证 |  |
| 403 | 缺少 mcp:update，或缺少工具本身权限 / 高危工具的 mcp:delete |  |
| 404 | 不存在 |  |
| 409 | 非待审批状态、已过期、凭证失效或签发者权限已变更 |  |

<details><summary>响应示例</summary>

```json
{
  "arguments": "string",
  "decided_at": "2026-01-01T00:00:00Z",
  "decided_by": 0,
  "expires_at": "2026-01-01T00:00:00Z",
  "finished_at": "2026-01-01T00:00:00Z",
  "id": 0,
  "preview": "string",
  "principal_id": 0,
  "reason": "string",
  "requested_at": "2026-01-01T00:00:00Z",
  "result": "string",
  "risk": "read",
  "session_id": "string",
  "status": "pending",
  "tool": "string"
}
```

</details>

```bash
fpctl api POST '/api/mcp/approvals/1/approve' --json
```

### POST `/api/mcp/approvals/{id}/reject`

`POST /api/mcp/approvals/{id}/reject` — 拒绝

- 认证：Bearer JWT
- 权限：`mcp:update`
- operationId：`reject`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 审批单 ID |

**请求体**（`application/json`，必填）：RejectRequest

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `reason` | string? | 否 | 拒绝理由（≤ 500 字节，回传给 AI） |

```json
{
  "reason": "string"
}
```

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 已拒绝 | ToolApproval |
| 401 | 未认证 |  |
| 403 | 缺少 mcp:update 权限 |  |
| 404 | 不存在 |  |
| 409 | 非待审批状态 |  |

<details><summary>响应示例</summary>

```json
{
  "arguments": "string",
  "decided_at": "2026-01-01T00:00:00Z",
  "decided_by": 0,
  "expires_at": "2026-01-01T00:00:00Z",
  "finished_at": "2026-01-01T00:00:00Z",
  "id": 0,
  "preview": "string",
  "principal_id": 0,
  "reason": "string",
  "requested_at": "2026-01-01T00:00:00Z",
  "result": "string",
  "risk": "read",
  "session_id": "string",
  "status": "pending",
  "tool": "string"
}
```

</details>

```bash
fpctl api POST '/api/mcp/approvals/1/reject' -d '{"reason":"string"}' --json
```

### GET `/api/mcp/calls`

`GET /api/mcp/calls` — 工具调用审计

- 认证：Bearer JWT
- 权限：`mcp:read`
- operationId：`list_calls`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `principal_id` | query | integer(int64)? | 否 |  |
| `session_id` | query | string? | 否 |  |
| `tool` | query | string? | 否 |  |
| `page` | query | integer(int64)? | 否 |  |
| `page_size` | query | integer(int64)? | 否 |  |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 调用记录分页（最新在前） | PaginatedResponse<ToolCallRecord>_ToolCallRecord |
| 401 | 未认证 |  |
| 403 | 缺少 mcp:read 权限 |  |

<details><summary>响应示例</summary>

```json
{
  "data": [
    {
      "arguments": "string",
      "created_at": "2026-01-01T00:00:00Z",
      "duration_ms": 0,
      "error": "string",
      "id": 0,
      "ok": false,
      "principal_id": 0,
      "session_id": "string",
      "tool": "string"
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
fpctl api GET '/api/mcp/calls' --json
```

### GET `/api/mcp/principals`

`GET /api/mcp/principals` — 凭证列表

- 认证：Bearer JWT
- 权限：`mcp:read`
- operationId：`list_principals`

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 凭证列表（不含令牌） | PrincipalView[] |
| 401 | 未认证 |  |
| 403 | 缺少 mcp:read 权限 |  |

<details><summary>响应示例</summary>

```json
[
  {
    "allow_write": false,
    "created_at": "2026-01-01T00:00:00Z",
    "expires_at": "2026-01-01T00:00:00Z",
    "id": 0,
    "last_used_at": "2026-01-01T00:00:00Z",
    "name": "string",
    "revoked_at": "2026-01-01T00:00:00Z",
    "token_prefix": "string",
    "user_id": 0,
    "active": false,
    "owner": "string"
  }
]
```

</details>

```bash
fpctl api GET '/api/mcp/principals' --json
```

### POST `/api/mcp/principals`

`POST /api/mcp/principals` — 为当前用户签发凭证

- 认证：Bearer JWT
- 权限：`mcp:create`
- operationId：`create_principal`

**请求体**（`application/json`，必填）：CreatePrincipalRequest

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `allow_write` | boolean | 否 | 允许发起写操作（M-AI-2）；写操作仍须面板人工批准。缺省 false（只读） |
| `expires_in_days` | integer(int64)? | 否 | 有效天数（1–3650）；缺省 / null 表示永不过期 |
| `name` | string | 是 | 凭证名称（1–64 字符），如 `claude-desktop@laptop` |

```json
{
  "allow_write": false,
  "expires_in_days": 0,
  "name": "string"
}
```

| 状态 | 说明 | 响应体 |
|---|---|---|
| 201 | 已签发；token 仅此一次返回 | CreatePrincipalResponse |
| 400 | 名称或有效期不合法 |  |
| 401 | 未认证 |  |
| 403 | 缺少 mcp:create 权限 |  |

<details><summary>响应示例</summary>

```json
{
  "endpoint": "string",
  "principal": {
    "allow_write": false,
    "created_at": "2026-01-01T00:00:00Z",
    "expires_at": "2026-01-01T00:00:00Z",
    "id": 0,
    "last_used_at": "2026-01-01T00:00:00Z",
    "name": "string",
    "revoked_at": "2026-01-01T00:00:00Z",
    "token_prefix": "string",
    "user_id": 0,
    "active": false,
    "owner": "string"
  },
  "token": "string"
}
```

</details>

```bash
fpctl api POST '/api/mcp/principals' -d '{"allow_write":false,"expires_in_days":0,"name":"string"}' --json
```

### POST `/api/mcp/principals/{id}/revoke`

`POST /api/mcp/principals/{id}/revoke` — 吊销凭证

- 认证：Bearer JWT
- 权限：`mcp:create`
- operationId：`revoke_principal`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 凭证 ID |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 已吊销（幂等） | PrincipalView |
| 401 | 未认证 |  |
| 403 | 缺少 mcp:create 权限，或吊销他人凭证缺少 mcp:delete |  |
| 404 | 凭证不存在 |  |

<details><summary>响应示例</summary>

```json
{
  "allow_write": false,
  "created_at": "2026-01-01T00:00:00Z",
  "expires_at": "2026-01-01T00:00:00Z",
  "id": 0,
  "last_used_at": "2026-01-01T00:00:00Z",
  "name": "string",
  "revoked_at": "2026-01-01T00:00:00Z",
  "token_prefix": "string",
  "user_id": 0,
  "active": false,
  "owner": "string"
}
```

</details>

```bash
fpctl api POST '/api/mcp/principals/1/revoke' --json
```

### GET `/api/mcp/prompts`

`GET /api/mcp/prompts` — 运维技能库

- 认证：Bearer JWT
- 权限：`mcp:read`
- operationId：`list_prompts`

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 技能列表（与 MCP prompts/list 一致） | McpPromptInfo[] |
| 401 | 未认证 |  |
| 403 | 缺少 mcp:read 权限 |  |

<details><summary>响应示例</summary>

```json
[
  {
    "arguments": [
      {
        "description": "string",
        "name": "string",
        "required": false
      }
    ],
    "description": "string",
    "name": "string",
    "title": "string"
  }
]
```

</details>

```bash
fpctl api GET '/api/mcp/prompts' --json
```

### GET `/api/mcp/prompts/{name}`

`GET /api/mcp/prompts/{name}` — 渲染技能（查询参数即技能参数）

- 认证：Bearer JWT
- 权限：`mcp:read`
- operationId：`render_prompt`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `name` | path | string | 是 | 技能名称 |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 渲染后的提示词 | McpPromptRendered |
| 400 | 缺少必填参数 / 未声明的参数 |  |
| 401 | 未认证 |  |
| 403 | 缺少 mcp:read 权限 |  |
| 404 | 技能不存在 |  |

<details><summary>响应示例</summary>

```json
{
  "name": "string",
  "text": "string",
  "title": "string"
}
```

</details>

```bash
fpctl api GET '/api/mcp/prompts/1' --json
```

### GET `/api/mcp/sessions`

`GET /api/mcp/sessions` — 会话列表

- 认证：Bearer JWT
- 权限：`mcp:read`
- operationId：`list_sessions`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `principal_id` | query | integer(int64)? | 否 | 按凭证过滤 |
| `limit` | query | integer(int64)? | 否 | 返回条数（1–500，缺省 100） |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 会话列表（最近活跃在前） | AgentSession[] |
| 401 | 未认证 |  |
| 403 | 缺少 mcp:read 权限 |  |

<details><summary>响应示例</summary>

```json
[
  {
    "client_name": "string",
    "client_version": "string",
    "closed_at": "2026-01-01T00:00:00Z",
    "created_at": "2026-01-01T00:00:00Z",
    "id": "string",
    "last_seen_at": "2026-01-01T00:00:00Z",
    "principal_id": 0,
    "protocol_version": "string"
  }
]
```

</details>

```bash
fpctl api GET '/api/mcp/sessions' --json
```

### GET `/api/mcp/tools`

`GET /api/mcp/tools` — 工具目录

- 认证：Bearer JWT
- 权限：`mcp:read`
- operationId：`list_tools`

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 工具目录 | ToolInfo[] |
| 401 | 未认证 |  |
| 403 | 缺少 mcp:read 权限 |  |

<details><summary>响应示例</summary>

```json
[
  {
    "allowed": false,
    "description": "string",
    "input_schema": {},
    "name": "string",
    "permission": "string",
    "risk": "string",
    "title": "string"
  }
]
```

</details>

```bash
fpctl api GET '/api/mcp/tools' --json
```
