<!-- 由 scripts/gen-api-docs.py 从 frontend/openapi.json 生成，请勿手改；修改 utoipa 注解后运行 just api-docs -->

# API · databases

> 18 个端点 · 通用约定见 [README](./README.md)

| 方法 | 路径 | 摘要 | 权限 |
|---|---|---|---|
| GET | `/api/databases` | 数据库实例列表（分页） | `database:read` |
| PATCH | `/api/databases/batch-status` | Phase A2 扩展：数据库实例批量状态原子更新（批量写事务，PATCH /api/databases/batch-status）。 批量更新实例状态（单事务） | `database:update` |
| POST | `/api/databases/mongodb/install` | 经 MongoDB 官方仓库（HTTPS + 签名校验）安装 mongod 并登记实例（端口缺省 27017； 管理员密码经 mongosh stdin 传递，不进 argv） | `database:create` |
| POST | `/api/databases/mysql/install` | 经系统包管理器安装 mysql 并登记实例（端口缺省 3306；口令经 stdin / 环境变量传递，不进 argv） | `database:create` |
| POST | `/api/databases/postgres/install` | 经系统包管理器安装 postgres 并登记实例（端口缺省 5432；口令经 stdin / 环境变量传递，不进 argv） | `database:create` |
| POST | `/api/databases/redis/install` | 经系统包管理器安装 redis 并登记实例（端口缺省 6379；口令经 stdin / 环境变量传递，不进 argv） | `database:create` |
| GET | `/api/databases/{id}` | 单个数据库实例 | `database:read` |
| DELETE | `/api/databases/{id}` | 删除实例登记（不卸载软件；卸载见 uninstall） | `database:delete` |
| GET | `/api/databases/{id}/databases` | 列出用户库（系统库已排除） | `database:read` |
| POST | `/api/databases/{id}/databases` | 创建库（MySQL / MariaDB 可指定字符集，缺省 utf8mb4；PostgreSQL 忽略字符集） | `database:create` |
| DELETE | `/api/databases/{id}/databases/{db_name}` | 删除库（系统库受保护） | `database:delete` |
| POST | `/api/databases/{id}/restart` | 重启数据库服务 | `database:start` |
| POST | `/api/databases/{id}/start` | 启动数据库服务 | `database:start` |
| GET | `/api/databases/{id}/status` | 服务运行状态（`running` / `stopped`） | `database:read` |
| POST | `/api/databases/{id}/stop` | 停止数据库服务 | `database:stop` |
| POST | `/api/databases/{id}/uninstall` | 卸载数据库软件并删除实例登记 | `database:delete` |
| POST | `/api/databases/{id}/users` | 创建数据库用户（口令不进 argv） | `database:create` |
| DELETE | `/api/databases/{id}/users/{username}` | 删除数据库用户（host 固定 localhost；内置超级用户受保护） | `database:delete` |

### GET `/api/databases`

数据库实例列表（分页）

- 认证：Bearer JWT
- 权限：`database:read`
- operationId：`list`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `page` | query | integer(int64)? | 否 | 页码（从 1 开始） |
| `page_size` | query | integer(int64)? | 否 | 每页条数（1~200） |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 实例分页 | PaginatedResponse<DatabaseInstanceResponse>_DatabaseInstanceResponse |
| 401 | 未认证 |  |
| 403 | 缺少 database:read 权限 |  |

<details><summary>响应示例</summary>

```json
{
  "data": [
    {
      "config_file": "string",
      "created_at": "string",
      "data_dir": "string",
      "db_type": "string",
      "id": 0,
      "install_path": "string",
      "name": "string",
      "port": 0,
      "resource_version": 0,
      "root_user": "string",
      "status": "string",
      "updated_at": "string",
      "version": "string"
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
fpctl api GET '/api/databases' --json
```

### PATCH `/api/databases/batch-status`

Phase A2 扩展：数据库实例批量状态原子更新（批量写事务，PATCH /api/databases/batch-status）。
批量更新实例状态（单事务）

- 认证：Bearer JWT
- 权限：`database:update`
- operationId：`update_batch_status`

**请求体**（`application/json`，必填）：BatchStatusRequest

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `updates` | any[][] | 是 | 每项为 `[instance_id, 状态]`，状态限 `running` / `stopped` / `error`，单次 ≤ 200 项 |

```json
{
  "updates": [
    []
  ]
}
```

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | `"updated"` | string (`text/plain`) |
| 400 | 为空 / 超过 200 项 / 状态不在 running、stopped、error 之内 |  |
| 401 | 未认证 |  |
| 403 | 缺少 database:update 权限 |  |

```bash
fpctl api PATCH '/api/databases/batch-status' -d '{"updates":[[]]}' --json
```

### POST `/api/databases/mongodb/install`

经 MongoDB 官方仓库（HTTPS + 签名校验）安装 mongod 并登记实例（端口缺省 27017；
管理员密码经 mongosh stdin 传递，不进 argv）

- 认证：Bearer JWT
- 权限：`database:create`
- operationId：`install_mongodb`

**请求体**（`application/json`，必填）：InstallMongodbRequest

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `name` | string | 是 |  |
| `password` | string? | 否 |  |
| `port` | integer(int32)? | 否 |  |
| `version` | string? | 否 |  |

```json
{
  "name": "string",
  "password": "string",
  "port": 0,
  "version": "string"
}
```

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 已安装并登记 | DatabaseInstanceResponse |
| 400 | 不支持的版本 / 发行版、密码过短或含控制字符、已安装 |  |
| 401 | 未认证 |  |
| 403 | 缺少 database:create 权限 |  |
| 500 | 安装失败（非 root 且无 sudo 时不可用，见 Doc/10） |  |

<details><summary>响应示例</summary>

```json
{
  "config_file": "string",
  "created_at": "string",
  "data_dir": "string",
  "db_type": "string",
  "id": 0,
  "install_path": "string",
  "name": "string",
  "port": 0,
  "resource_version": 0,
  "root_user": "string",
  "status": "string",
  "updated_at": "string",
  "version": "string"
}
```

</details>

```bash
fpctl api POST '/api/databases/mongodb/install' -d '{"name":"string","password":"string","port":0,"version":"string"}' --json
```

### POST `/api/databases/mysql/install`

经系统包管理器安装 mysql 并登记实例（端口缺省 3306；口令经 stdin / 环境变量传递，不进 argv）

- 认证：Bearer JWT
- 权限：`database:create`
- operationId：`install_mysql`

**请求体**（`application/json`，必填）：InstallMysqlRequest

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `name` | string | 是 |  |
| `port` | integer(int32)? | 否 |  |
| `root_password` | string? | 否 |  |
| `version` | string? | 否 |  |

```json
{
  "name": "string",
  "port": 0,
  "root_password": "string",
  "version": "string"
}
```

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 已安装并登记 | DatabaseInstanceResponse |
| 400 | 口令含控制字符等校验失败 |  |
| 401 | 未认证 |  |
| 403 | 缺少 database:create 权限 |  |
| 500 | 安装失败（非 root 且无 sudo 时不可用，见 Doc/10） |  |

<details><summary>响应示例</summary>

```json
{
  "config_file": "string",
  "created_at": "string",
  "data_dir": "string",
  "db_type": "string",
  "id": 0,
  "install_path": "string",
  "name": "string",
  "port": 0,
  "resource_version": 0,
  "root_user": "string",
  "status": "string",
  "updated_at": "string",
  "version": "string"
}
```

</details>

```bash
fpctl api POST '/api/databases/mysql/install' -d '{"name":"string","port":0,"root_password":"string","version":"string"}' --json
```

### POST `/api/databases/postgres/install`

经系统包管理器安装 postgres 并登记实例（端口缺省 5432；口令经 stdin / 环境变量传递，不进 argv）

- 认证：Bearer JWT
- 权限：`database:create`
- operationId：`install_postgres`

**请求体**（`application/json`，必填）：InstallPostgresRequest

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `name` | string | 是 |  |
| `password` | string? | 否 |  |
| `port` | integer(int32)? | 否 |  |
| `version` | string? | 否 |  |

```json
{
  "name": "string",
  "password": "string",
  "port": 0,
  "version": "string"
}
```

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 已安装并登记 | DatabaseInstanceResponse |
| 400 | 口令含控制字符等校验失败 |  |
| 401 | 未认证 |  |
| 403 | 缺少 database:create 权限 |  |
| 500 | 安装失败（非 root 且无 sudo 时不可用，见 Doc/10） |  |

<details><summary>响应示例</summary>

```json
{
  "config_file": "string",
  "created_at": "string",
  "data_dir": "string",
  "db_type": "string",
  "id": 0,
  "install_path": "string",
  "name": "string",
  "port": 0,
  "resource_version": 0,
  "root_user": "string",
  "status": "string",
  "updated_at": "string",
  "version": "string"
}
```

</details>

```bash
fpctl api POST '/api/databases/postgres/install' -d '{"name":"string","password":"string","port":0,"version":"string"}' --json
```

### POST `/api/databases/redis/install`

经系统包管理器安装 redis 并登记实例（端口缺省 6379；口令经 stdin / 环境变量传递，不进 argv）

- 认证：Bearer JWT
- 权限：`database:create`
- operationId：`install_redis`

**请求体**（`application/json`，必填）：InstallRedisRequest

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `name` | string | 是 |  |
| `password` | string? | 否 |  |
| `port` | integer(int32)? | 否 |  |
| `version` | string? | 否 |  |

```json
{
  "name": "string",
  "password": "string",
  "port": 0,
  "version": "string"
}
```

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 已安装并登记 | DatabaseInstanceResponse |
| 400 | 口令含控制字符等校验失败 |  |
| 401 | 未认证 |  |
| 403 | 缺少 database:create 权限 |  |
| 500 | 安装失败（非 root 且无 sudo 时不可用，见 Doc/10） |  |

<details><summary>响应示例</summary>

```json
{
  "config_file": "string",
  "created_at": "string",
  "data_dir": "string",
  "db_type": "string",
  "id": 0,
  "install_path": "string",
  "name": "string",
  "port": 0,
  "resource_version": 0,
  "root_user": "string",
  "status": "string",
  "updated_at": "string",
  "version": "string"
}
```

</details>

```bash
fpctl api POST '/api/databases/redis/install' -d '{"name":"string","password":"string","port":0,"version":"string"}' --json
```

### GET `/api/databases/{id}`

单个数据库实例

- 认证：Bearer JWT
- 权限：`database:read`
- operationId：`get`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 实例 ID |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 实例 | DatabaseInstanceResponse |
| 401 | 未认证 |  |
| 403 | 缺少 database:read 权限 |  |
| 404 | 实例不存在 |  |

<details><summary>响应示例</summary>

```json
{
  "config_file": "string",
  "created_at": "string",
  "data_dir": "string",
  "db_type": "string",
  "id": 0,
  "install_path": "string",
  "name": "string",
  "port": 0,
  "resource_version": 0,
  "root_user": "string",
  "status": "string",
  "updated_at": "string",
  "version": "string"
}
```

</details>

```bash
fpctl api GET '/api/databases/1' --json
```

### DELETE `/api/databases/{id}`

删除实例登记（不卸载软件；卸载见 uninstall）

- 认证：Bearer JWT
- 权限：`database:delete`
- operationId：`delete`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 实例 ID |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | `"deleted"` | string (`text/plain`) |
| 401 | 未认证 |  |
| 403 | 缺少 database:delete 权限 |  |
| 404 | 实例不存在 |  |

```bash
fpctl api DELETE '/api/databases/1' --json
```

### GET `/api/databases/{id}/databases`

列出用户库（系统库已排除）

- 认证：Bearer JWT
- 权限：`database:read`
- operationId：`list_databases`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 实例 ID |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 库名列表 | string[] |
| 400 | Redis 不支持 |  |
| 401 | 未认证 |  |
| 403 | 缺少 database:read 权限 |  |
| 404 | 实例不存在 |  |

<details><summary>响应示例</summary>

```json
[
  "string"
]
```

</details>

```bash
fpctl api GET '/api/databases/1/databases' --json
```

### POST `/api/databases/{id}/databases`

创建库（MySQL / MariaDB 可指定字符集，缺省 utf8mb4；PostgreSQL 忽略字符集）

- 认证：Bearer JWT
- 权限：`database:create`
- operationId：`create_database`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 实例 ID |

**请求体**（`application/json`，必填）：CreateDatabaseRequest

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `charset` | string? | 否 |  |
| `name` | string | 是 |  |

```json
{
  "charset": "string",
  "name": "string"
}
```

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | `"created"` | string (`text/plain`) |
| 400 | 库名需为 1–64 位字母数字 / `_` / `$`；字符集不在白名单；Redis 不支持 |  |
| 401 | 未认证 |  |
| 403 | 缺少 database:create 权限 |  |
| 404 | 实例不存在 |  |

```bash
fpctl api POST '/api/databases/1/databases' -d '{"charset":"string","name":"string"}' --json
```

### DELETE `/api/databases/{id}/databases/{db_name}`

删除库（系统库受保护）

- 认证：Bearer JWT
- 权限：`database:delete`
- operationId：`drop_database`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 实例 ID |
| `db_name` | path | string | 是 | 库名 |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | `"dropped"` | string (`text/plain`) |
| 400 | 库名非法 / 系统库（mysql、information_schema、performance_schema、sys、postgres、template0/1）不可删除 |  |
| 401 | 未认证 |  |
| 403 | 缺少 database:delete 权限 |  |
| 404 | 实例不存在 |  |

```bash
fpctl api DELETE '/api/databases/1/databases/1' --json
```

### POST `/api/databases/{id}/restart`

重启数据库服务

- 认证：Bearer JWT
- 权限：`database:start`
- operationId：`restart`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 实例 ID |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | `"restarted"` | string (`text/plain`) |
| 401 | 未认证 |  |
| 403 | 缺少 database:start 权限 |  |
| 404 | 实例不存在 |  |

```bash
fpctl api POST '/api/databases/1/restart' --json
```

### POST `/api/databases/{id}/start`

启动数据库服务

- 认证：Bearer JWT
- 权限：`database:start`
- operationId：`start`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 实例 ID |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | `"started"` | string (`text/plain`) |
| 401 | 未认证 |  |
| 403 | 缺少 database:start 权限 |  |
| 404 | 实例不存在 |  |

```bash
fpctl api POST '/api/databases/1/start' --json
```

### GET `/api/databases/{id}/status`

服务运行状态（`running` / `stopped`）

- 认证：Bearer JWT
- 权限：`database:read`
- operationId：`check_status`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 实例 ID |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 状态 | string (`text/plain`) |
| 401 | 未认证 |  |
| 403 | 缺少 database:read 权限 |  |
| 404 | 实例不存在 |  |

```bash
fpctl api GET '/api/databases/1/status' --json
```

### POST `/api/databases/{id}/stop`

停止数据库服务

- 认证：Bearer JWT
- 权限：`database:stop`
- operationId：`stop`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 实例 ID |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | `"stopped"` | string (`text/plain`) |
| 401 | 未认证 |  |
| 403 | 缺少 database:stop 权限 |  |
| 404 | 实例不存在 |  |

```bash
fpctl api POST '/api/databases/1/stop' --json
```

### POST `/api/databases/{id}/uninstall`

卸载数据库软件并删除实例登记

- 认证：Bearer JWT
- 权限：`database:delete`
- operationId：`uninstall`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 实例 ID |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | `"uninstalled"` | string (`text/plain`) |
| 401 | 未认证 |  |
| 403 | 缺少 database:delete 权限 |  |
| 404 | 实例不存在 |  |

```bash
fpctl api POST '/api/databases/1/uninstall' --json
```

### POST `/api/databases/{id}/users`

创建数据库用户（口令不进 argv）

- 认证：Bearer JWT
- 权限：`database:create`
- operationId：`create_user`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 实例 ID |

**请求体**（`application/json`，必填）：DatabaseCreateUserRequest

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `host` | string? | 否 |  |
| `password` | string | 是 |  |
| `username` | string | 是 |  |

```json
{
  "host": "string",
  "password": "string",
  "username": "string"
}
```

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | `"created"` | string (`text/plain`) |
| 400 | 用户名 / host / 口令校验失败 |  |
| 401 | 未认证 |  |
| 403 | 缺少 database:update 权限 |  |
| 404 | 实例不存在 |  |

```bash
fpctl api POST '/api/databases/1/users' -d '{"host":"string","password":"string","username":"string"}' --json
```

### DELETE `/api/databases/{id}/users/{username}`

删除数据库用户（host 固定 localhost；内置超级用户受保护）

- 认证：Bearer JWT
- 权限：`database:delete`
- operationId：`drop_user`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 实例 ID |
| `username` | path | string | 是 | 用户名 |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | `"dropped"` | string (`text/plain`) |
| 400 | 用户名非法 / root、postgres 等内置超级用户不可删除 |  |
| 401 | 未认证 |  |
| 403 | 缺少 database:update 权限 |  |
| 404 | 实例不存在 |  |

```bash
fpctl api DELETE '/api/databases/1/users/1' --json
```
