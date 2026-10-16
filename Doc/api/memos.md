<!-- 由 scripts/gen-api-docs.py 从 frontend/openapi.json 生成，请勿手改；修改 utoipa 注解后运行 just api-docs -->

# API · memos

> 4 个端点 · 通用约定见 [README](./README.md)

| 方法 | 路径 | 摘要 | 权限 |
|---|---|---|---|
| GET | `/api/memos` | 备忘录列表（仪表盘便签与待办） | `memo:read` |
| POST | `/api/memos` | 新建备忘 / 待办 | `memo:create` |
| PUT | `/api/memos/{id}` | 更新备忘内容或完成状态 | `memo:update` |
| DELETE | `/api/memos/{id}` | 删除备忘 | `memo:delete` |

### GET `/api/memos`

备忘录列表（仪表盘便签与待办）

- 认证：Bearer JWT
- 权限：`memo:read`
- operationId：`list`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `kind` | query | string? | 否 | `memo` \| `todo` |
| `done` | query | boolean? | 否 |  |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 备忘 / 待办列表 | Memo[] |
| 401 | 未认证 |  |

<details><summary>响应示例</summary>

```json
[
  {
    "content": "string",
    "created_at": "2026-01-01T00:00:00Z",
    "done": false,
    "id": 0,
    "kind": "string",
    "updated_at": "2026-01-01T00:00:00Z"
  }
]
```

</details>

```bash
fpctl api GET '/api/memos' --json
```

### POST `/api/memos`

新建备忘 / 待办

- 认证：Bearer JWT
- 权限：`memo:create`
- operationId：`create`

**请求体**（`application/json`，必填）：CreateMemoRequest

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `content` | string | 是 |  |
| `kind` | string | 否 |  |

```json
{
  "content": "string",
  "kind": "string"
}
```

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 新建的条目 | Memo |
| 400 | 内容为空 |  |
| 401 | 未认证 |  |

<details><summary>响应示例</summary>

```json
{
  "content": "string",
  "created_at": "2026-01-01T00:00:00Z",
  "done": false,
  "id": 0,
  "kind": "string",
  "updated_at": "2026-01-01T00:00:00Z"
}
```

</details>

```bash
fpctl api POST '/api/memos' -d '{"content":"string","kind":"string"}' --json
```

### PUT `/api/memos/{id}`

更新备忘内容或完成状态

- 认证：Bearer JWT
- 权限：`memo:update`
- operationId：`update`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | ID |

**请求体**（`application/json`，必填）：UpdateMemoRequest

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `content` | string? | 否 |  |
| `done` | boolean? | 否 |  |

```json
{
  "content": "string",
  "done": false
}
```

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 更新后的条目 | Memo |
| 401 | 未认证 |  |
| 404 | 条目不存在 |  |

<details><summary>响应示例</summary>

```json
{
  "content": "string",
  "created_at": "2026-01-01T00:00:00Z",
  "done": false,
  "id": 0,
  "kind": "string",
  "updated_at": "2026-01-01T00:00:00Z"
}
```

</details>

```bash
fpctl api PUT '/api/memos/1' -d '{"content":"string","done":false}' --json
```

### DELETE `/api/memos/{id}`

删除备忘

- 认证：Bearer JWT
- 权限：`memo:delete`
- operationId：`delete`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | ID |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 删除成功 | string (`text/plain`) |
| 401 | 未认证 |  |
| 404 | 条目不存在 |  |

```bash
fpctl api DELETE '/api/memos/1' --json
```
