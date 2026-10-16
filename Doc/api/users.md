<!-- 由 scripts/gen-api-docs.py 从 frontend/openapi.json 生成，请勿手改；修改 utoipa 注解后运行 just api-docs -->

# API · users

> 5 个端点 · 通用约定见 [README](./README.md)

| 方法 | 路径 | 摘要 | 权限 |
|---|---|---|---|
| GET | `/api/users` | 用户列表（分页） | `user:read` |
| POST | `/api/users` | 创建用户 | `user:create` |
| PUT | `/api/users/{id}` | 更新用户 | `user:update` |
| DELETE | `/api/users/{id}` | 删除用户 | `user:delete` |
| PUT | `/api/users/{id}/2fa/reset` | 管理员重置指定用户的两步验证（用户丢失认证器且恢复码用尽时；需 `user:update` + admin 角色） | `user:update` |

### GET `/api/users`

用户列表（分页）

- 认证：Bearer JWT
- 权限：`user:read`
- operationId：`list_users`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `page` | query | integer(int64)? | 否 | 页码（从 1 开始） |
| `page_size` | query | integer(int64)? | 否 | 每页条数（1~200） |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 用户列表 | PaginatedResponse<User>_User |
| 401 | 未认证 |  |

<details><summary>响应示例</summary>

```json
{
  "data": [
    {
      "created_at": "2026-01-01T00:00:00Z",
      "id": 0,
      "must_change_password": false,
      "password_hash": "string",
      "role": "string",
      "username": "string"
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
fpctl api GET '/api/users' --json
```

### POST `/api/users`

创建用户

- 认证：Bearer JWT
- 权限：`user:create`
- operationId：`create_user`

**请求体**（`application/json`，必填）：CreateUserRequest

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `password_hash` | string | 是 |  |
| `role` | string | 是 |  |
| `username` | string | 是 |  |

```json
{
  "password_hash": "string",
  "role": "string",
  "username": "string"
}
```

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 创建成功 | User |
| 400 | 参数错误 |  |
| 401 | 未认证 |  |

<details><summary>响应示例</summary>

```json
{
  "created_at": "2026-01-01T00:00:00Z",
  "id": 0,
  "must_change_password": false,
  "password_hash": "string",
  "role": "string",
  "username": "string"
}
```

</details>

```bash
fpctl api POST '/api/users' -d '{"password_hash":"string","role":"string","username":"string"}' --json
```

### PUT `/api/users/{id}`

更新用户

- 认证：Bearer JWT
- 权限：`user:update`
- operationId：`update_user`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 用户 ID |

**请求体**（`application/json`，必填）：UpdateUserRequest

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `password_hash` | string? | 否 |  |
| `role` | string | 是 |  |
| `username` | string | 是 |  |

```json
{
  "password_hash": "string",
  "role": "string",
  "username": "string"
}
```

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 更新成功 | User |
| 404 | 用户不存在 |  |

<details><summary>响应示例</summary>

```json
{
  "created_at": "2026-01-01T00:00:00Z",
  "id": 0,
  "must_change_password": false,
  "password_hash": "string",
  "role": "string",
  "username": "string"
}
```

</details>

```bash
fpctl api PUT '/api/users/1' -d '{"password_hash":"string","role":"string","username":"string"}' --json
```

### DELETE `/api/users/{id}`

删除用户

- 认证：Bearer JWT
- 权限：`user:delete`
- operationId：`delete_user`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 用户 ID |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 删除成功 |  |
| 404 | 用户不存在 |  |

```bash
fpctl api DELETE '/api/users/1' --json
```

### PUT `/api/users/{id}/2fa/reset`

管理员重置指定用户的两步验证（用户丢失认证器且恢复码用尽时；需 `user:update` + admin 角色）

- 认证：Bearer JWT
- 权限：`user:update`
- operationId：`reset_user_two_factor`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 用户 ID |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 已重置 |  |
| 401 | 未认证 |  |
| 403 | 缺少 user:update 权限或非 admin 角色 |  |
| 404 | 用户不存在 |  |

```bash
fpctl api PUT '/api/users/1/2fa/reset' --json
```
