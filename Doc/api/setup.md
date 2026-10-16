<!-- 由 scripts/gen-api-docs.py 从 frontend/openapi.json 生成，请勿手改；修改 utoipa 注解后运行 just api-docs -->

# API · setup

> 2 个端点 · 通用约定见 [README](./README.md)

| 方法 | 路径 | 摘要 | 权限 |
|---|---|---|---|
| POST | `/api/setup/initialize` | `POST /api/setup/initialize` | 公开（无需登录） |
| GET | `/api/setup/status` | `GET /api/setup/status` | `database:read` |

### POST `/api/setup/initialize`

`POST /api/setup/initialize`

- 认证：无
- 权限：公开（无需登录）
- operationId：`initialize`

**请求体**（`application/json`，必填）：SetupRequest

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `admin` | SetupAdminInput? | 否 |  |
| `database` | SetupDatabaseInput? | 否 |  |
| `language` | string? | 否 |  |
| `step` | string | 是 | `database` \| `admin` |
| `theme` | string? | 否 |  |

```json
{
  "admin": {
    "password": "string",
    "username": "string"
  },
  "database": {
    "db_type": "string",
    "host": "string",
    "mysql_root_password": "string",
    "name": "string",
    "password": "string",
    "port": 0,
    "user": "string"
  },
  "language": "string",
  "step": "string",
  "theme": "string"
}
```

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 步骤完成；`admin` 步骤返回登录令牌 | InitializeResponse |
| 400 | 参数错误 |  |
| 409 | 面板已初始化 |  |

<details><summary>响应示例</summary>

```json
{
  "message": "string",
  "refresh_token": "string",
  "role": "string",
  "status": "string",
  "token": "string",
  "username": "string"
}
```

</details>

### GET `/api/setup/status`

`GET /api/setup/status`

- 认证：无
- 权限：`database:read`
- operationId：`status`

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 初始化状态与环境探测 | SetupStatusResponse |

<details><summary>响应示例</summary>

```json
{
  "docker": false,
  "language": "string",
  "nginx": false,
  "status": "string",
  "theme": "string"
}
```

</details>
