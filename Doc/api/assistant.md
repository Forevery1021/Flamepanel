<!-- 由 scripts/gen-api-docs.py 从 frontend/openapi.json 生成，请勿手改；修改 utoipa 注解后运行 just api-docs -->

# API · assistant

> 4 个端点 · 通用约定见 [README](./README.md)

| 方法 | 路径 | 摘要 | 权限 |
|---|---|---|---|
| POST | `/api/assistant/chat` | `POST /api/assistant/chat` | `mcp:create` |
| GET | `/api/assistant/config` | `GET /api/assistant/config` | `mcp:read` |
| PUT | `/api/assistant/config` | `PUT /api/assistant/config` | `settings:update` |
| POST | `/api/assistant/test` | `POST /api/assistant/test` | `settings:update` |

### POST `/api/assistant/chat`

`POST /api/assistant/chat`

- 认证：Bearer JWT
- 权限：`mcp:create`
- operationId：`chat`

**请求体**（`application/json`，必填）：ChatRequest

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `conversation_id` | string? | 否 | 会话 ID（首轮留空，由服务端生成；后续轮次回传以便审计归组） |
| `messages` | ChatTurn[] | 是 | 对话历史（≤ 40 条，末条为用户消息） |

```json
{
  "conversation_id": "string",
  "messages": [
    {
      "content": "string",
      "role": "string"
    }
  ]
}
```

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 助手回复与工具轨迹 | ChatResponse |
| 400 | 消息不合法（角色、长度、条数） |  |
| 401 | 未认证 |  |
| 403 | 缺少 mcp:create 权限 |  |
| 429 | 上一轮对话尚未结束 |  |
| 503 | 助手未启用 / 模型服务不可用 |  |

<details><summary>响应示例</summary>

```json
{
  "approval_ids": [
    0
  ],
  "conversation_id": "string",
  "reply": "string",
  "step_limit_reached": false,
  "steps": [
    {
      "approval_id": 0,
      "arguments": "string",
      "ok": false,
      "output": "string",
      "tool": "string"
    }
  ]
}
```

</details>

```bash
fpctl api POST '/api/assistant/chat' -d '{"conversation_id":"string","messages":[{"content":"string","role":"string"}]}' --json
```

### GET `/api/assistant/config`

`GET /api/assistant/config`

- 认证：Bearer JWT
- 权限：`mcp:read`
- operationId：`get_config`

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 助手配置（不含密钥） | AssistantConfigView |
| 401 | 未认证 |  |
| 403 | 缺少 mcp:read 权限 |  |

<details><summary>响应示例</summary>

```json
{
  "base_url": "string",
  "effective_base_url": "string",
  "enabled": false,
  "has_api_key": false,
  "max_steps": 0,
  "model": "string",
  "provider": "openai",
  "temperature": 0.0,
  "timeout_secs": 0
}
```

</details>

```bash
fpctl api GET '/api/assistant/config' --json
```

### PUT `/api/assistant/config`

`PUT /api/assistant/config`

- 认证：Bearer JWT
- 权限：`settings:update`
- operationId：`update_config`

**请求体**（`application/json`，必填）：AssistantConfigPatch

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `api_key` | string? | 否 | 缺省 = 保留原密钥；空字符串 = 清除；其他 = 替换 |
| `base_url` | string? | 否 |  |
| `enabled` | boolean? | 否 |  |
| `max_steps` | integer(int32)? | 否 |  |
| `model` | string? | 否 |  |
| `provider` | LlmProviderKind? | 否 |  |
| `temperature` | number(float)? | 否 |  |
| `timeout_secs` | integer(int64)? | 否 |  |

```json
{
  "api_key": "string",
  "base_url": "string",
  "enabled": false,
  "max_steps": 0,
  "model": "string",
  "provider": "openai",
  "temperature": 0.0,
  "timeout_secs": 0
}
```

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 更新后的配置 | AssistantConfigView |
| 400 | 字段不合法（如启用但未填模型、base_url 内嵌凭据） |  |
| 401 | 未认证 |  |
| 403 | 缺少 settings:update 权限 |  |
| 503 | 主密钥不可用，拒绝明文保存 API Key |  |

<details><summary>响应示例</summary>

```json
{
  "base_url": "string",
  "effective_base_url": "string",
  "enabled": false,
  "has_api_key": false,
  "max_steps": 0,
  "model": "string",
  "provider": "openai",
  "temperature": 0.0,
  "timeout_secs": 0
}
```

</details>

```bash
fpctl api PUT '/api/assistant/config' -d '{"api_key":"string","base_url":"string","enabled":false,"max_steps":0,"model":"string","provider":"openai","temperature":0.0,"timeout_secs":0}' --json
```

### POST `/api/assistant/test`

`POST /api/assistant/test`

- 认证：Bearer JWT
- 权限：`settings:update`
- operationId：`test_connection`

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 连通性测试结果 | TestResult |
| 401 | 未认证 |  |
| 403 | 缺少 settings:update 权限 |  |
| 503 | 无法连接模型服务 / 鉴权失败 |  |

<details><summary>响应示例</summary>

```json
{
  "model_found": false,
  "models": [
    "string"
  ],
  "ok": false
}
```

</details>

```bash
fpctl api POST '/api/assistant/test' --json
```
