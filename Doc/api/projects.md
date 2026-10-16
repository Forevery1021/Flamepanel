<!-- 由 scripts/gen-api-docs.py 从 frontend/openapi.json 生成，请勿手改；修改 utoipa 注解后运行 just api-docs -->

# API · projects

> 21 个端点 · 通用约定见 [README](./README.md)

| 方法 | 路径 | 摘要 | 权限 |
|---|---|---|---|
| GET | `/api/projects` | 项目列表 | `project:read` |
| POST | `/api/projects` | 创建项目 | `project:create` |
| GET | `/api/projects/{id}` | 项目详情 | `project:read` |
| PUT | `/api/projects/{id}` | 更新项目 | `project:update` |
| DELETE | `/api/projects/{id}` | 删除项目（运行中自动停止；部署目录保留——零锁定纪律） | `project:delete` |
| GET | `/api/projects/{id}/builder` | 查看项目构建器决策（部署时自动协商结果，Doc/20 §5.8 ②） | `project:read` |
| POST | `/api/projects/{id}/deploy` | 部署项目（git clone / local_path 校验；部署 ≠ 启动） | `project:create` |
| POST | `/api/projects/{id}/health-check` | 手动健康探测（Stage A-9 · Doc/20 §5.8 ④） | `project:update` |
| GET | `/api/projects/{id}/links` | 项目的联链列表 | `project:read` |
| POST | `/api/projects/{id}/links` | 关联数据库（联链成功即在建库侧幂等创建业务库；连接串在启动时注入）。 | `project:update` |
| GET | `/api/projects/{id}/links/available` | 可关联的数据库实例清单（联链弹窗数据源；仅返回 running 实例）。 | `project:read` |
| DELETE | `/api/projects/{id}/links/{link_id}` | 解除联链 | `project:delete` |
| GET | `/api/projects/{id}/logs` | 最近日志（按执行面分流：容器模式 docker logs / 原生模式守护器日志） | `project:read` |
| GET | `/api/projects/{id}/procfile` | 查看项目 Procfile 进程声明 | `project:read` |
| POST | `/api/projects/{id}/restart` | 重启项目 | `database:start` |
| POST | `/api/projects/{id}/start` | 启动项目 | `database:start` |
| GET | `/api/projects/{id}/status` | 实时进程状态（按执行面分流：容器模式 docker inspect / 原生模式守护器探测） | `database:read` |
| POST | `/api/projects/{id}/stop` | 停止项目 | `database:stop` |
| POST | `/api/projects/{id}/webhook` | Webhook push 触发部署（免 JWT，令牌经中间件校验； 兼容 GitHub / GitLab / Gitee / Gitea / 通用 JSON——服务端不解析平台语义， 任何 POST 命中即视为一次 push 部署请求） | 公开（无需登录） |
| POST | `/api/projects/{id}/webhook/disable` | 关闭项目 webhook 部署 | `project:update` |
| POST | `/api/projects/{id}/webhook/enable` | 开启/重置项目 webhook 部署（返回一次性部署令牌） | `project:update` |

### GET `/api/projects`

项目列表

- 认证：Bearer JWT
- 权限：`project:read`
- operationId：`list_projects`

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 项目列表 | ProjectDto[] |
| 401 | 未认证 |  |

<details><summary>响应示例</summary>

```json
[
  {
    "auto_deploy": false,
    "build_type": "string",
    "created_at": "2026-01-01T00:00:00Z",
    "deploy_path": "string",
    "deploy_strategy": "string",
    "deploy_type": "string",
    "domain": "string",
    "has_procfile": false,
    "has_webhook_token": false,
    "health_check_path": "string",
    "id": 0,
    "kind": "string",
    "name": "string",
    "port": 0,
    "process_unit": "string",
    "run_command": "string",
    "runtime_id": 0,
    "source": "string",
    "source_url": "string",
    "status": "string",
    "updated_at": "2026-01-01T00:00:00Z"
  }
]
```

</details>

```bash
fpctl api GET '/api/projects' --json
```

### POST `/api/projects`

创建项目

- 认证：Bearer JWT
- 权限：`project:create`
- operationId：`create_project`

**请求体**（`application/json`，必填）：CreateProjectRequest

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `deploy_path` | string? | 否 | 留空自动取 /opt/flamepanel/projects/<name> |
| `deploy_strategy` | string? | 否 | 切换策略：restart \| rolling（默认 restart） |
| `domain` | string? | 否 |  |
| `health_check_path` | string? | 否 | 健康检查路径（如 /health；空 = 未启用） |
| `kind` | string | 是 | php \| jar \| war \| node \| python \| go \| static |
| `name` | string | 是 |  |
| `port` | integer(int64)? | 否 |  |
| `run_command` | string? | 否 | 启动命令（模板变量 $RUNTIME / $DEPLOY 预留，当前原样使用） |
| `runtime_id` | integer(int64)? | 否 |  |
| `source` | string | 是 | local_path \| git |
| `source_url` | string? | 否 |  |

```json
{
  "deploy_path": "string",
  "deploy_strategy": "string",
  "domain": "string",
  "health_check_path": "string",
  "kind": "string",
  "name": "string",
  "port": 0,
  "run_command": "string",
  "runtime_id": 0,
  "source": "string",
  "source_url": "string"
}
```

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 创建成功 | ProjectDto |
| 400 | 参数校验失败 |  |
| 409 | 项目重名 |  |

<details><summary>响应示例</summary>

```json
{
  "auto_deploy": false,
  "build_type": "string",
  "created_at": "2026-01-01T00:00:00Z",
  "deploy_path": "string",
  "deploy_strategy": "string",
  "deploy_type": "string",
  "domain": "string",
  "has_procfile": false,
  "has_webhook_token": false,
  "health_check_path": "string",
  "id": 0,
  "kind": "string",
  "name": "string",
  "port": 0,
  "process_unit": "string",
  "run_command": "string",
  "runtime_id": 0,
  "source": "string",
  "source_url": "string",
  "status": "string",
  "updated_at": "2026-01-01T00:00:00Z"
}
```

</details>

```bash
fpctl api POST '/api/projects' -d '{"deploy_path":"string","deploy_strategy":"string","domain":"string","health_check_path":"string","kind":"string","name":"string","port":0,"run_command":"string","runtime_id":0,"source":"string","source_url":"string"}' --json
```

### GET `/api/projects/{id}`

项目详情

- 认证：Bearer JWT
- 权限：`project:read`
- operationId：`get_project`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 项目 ID |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 项目详情 | ProjectDto |
| 404 | 项目不存在 |  |

<details><summary>响应示例</summary>

```json
{
  "auto_deploy": false,
  "build_type": "string",
  "created_at": "2026-01-01T00:00:00Z",
  "deploy_path": "string",
  "deploy_strategy": "string",
  "deploy_type": "string",
  "domain": "string",
  "has_procfile": false,
  "has_webhook_token": false,
  "health_check_path": "string",
  "id": 0,
  "kind": "string",
  "name": "string",
  "port": 0,
  "process_unit": "string",
  "run_command": "string",
  "runtime_id": 0,
  "source": "string",
  "source_url": "string",
  "status": "string",
  "updated_at": "2026-01-01T00:00:00Z"
}
```

</details>

```bash
fpctl api GET '/api/projects/1' --json
```

### PUT `/api/projects/{id}`

更新项目

- 认证：Bearer JWT
- 权限：`project:update`
- operationId：`update_project`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 项目 ID |

**请求体**（`application/json`，必填）：UpdateProjectRequest

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `deploy_path` | string? | 否 |  |
| `deploy_strategy` | string? | 否 |  |
| `domain` | string? | 否 |  |
| `health_check_path` | string? | 否 |  |
| `kind` | string | 是 |  |
| `port` | integer(int64)? | 否 |  |
| `run_command` | string? | 否 |  |
| `runtime_id` | integer(int64)? | 否 |  |
| `source` | string | 是 |  |
| `source_url` | string? | 否 |  |

```json
{
  "deploy_path": "string",
  "deploy_strategy": "string",
  "domain": "string",
  "health_check_path": "string",
  "kind": "string",
  "port": 0,
  "run_command": "string",
  "runtime_id": 0,
  "source": "string",
  "source_url": "string"
}
```

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 更新成功 | ProjectDto |
| 404 | 项目不存在 |  |
| 409 | 运行中禁止修改启动命令 |  |

<details><summary>响应示例</summary>

```json
{
  "auto_deploy": false,
  "build_type": "string",
  "created_at": "2026-01-01T00:00:00Z",
  "deploy_path": "string",
  "deploy_strategy": "string",
  "deploy_type": "string",
  "domain": "string",
  "has_procfile": false,
  "has_webhook_token": false,
  "health_check_path": "string",
  "id": 0,
  "kind": "string",
  "name": "string",
  "port": 0,
  "process_unit": "string",
  "run_command": "string",
  "runtime_id": 0,
  "source": "string",
  "source_url": "string",
  "status": "string",
  "updated_at": "2026-01-01T00:00:00Z"
}
```

</details>

```bash
fpctl api PUT '/api/projects/1' -d '{"deploy_path":"string","deploy_strategy":"string","domain":"string","health_check_path":"string","kind":"string","port":0,"run_command":"string","runtime_id":0,"source":"string","source_url":"string"}' --json
```

### DELETE `/api/projects/{id}`

删除项目（运行中自动停止；部署目录保留——零锁定纪律）

- 认证：Bearer JWT
- 权限：`project:delete`
- operationId：`delete_project`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 项目 ID |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 删除成功 |  |
| 404 | 项目不存在 |  |

```bash
fpctl api DELETE '/api/projects/1' --json
```

### GET `/api/projects/{id}/builder`

查看项目构建器决策（部署时自动协商结果，Doc/20 §5.8 ②）

- 认证：Bearer JWT
- 权限：`project:read`
- operationId：`get_project_builder`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 项目 ID |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 构建器决策 | BuilderDecisionDto |
| 404 | 项目不存在 |  |

<details><summary>响应示例</summary>

```json
{
  "build_type": "string",
  "image": "string",
  "project_id": 0
}
```

</details>

```bash
fpctl api GET '/api/projects/1/builder' --json
```

### POST `/api/projects/{id}/deploy`

部署项目（git clone / local_path 校验；部署 ≠ 启动）

- 认证：Bearer JWT
- 权限：`project:create`
- operationId：`deploy_project`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 项目 ID |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 部署成功 | ProjectDto |
| 400 | 来源校验失败 |  |
| 409 | 项目操作互斥 |  |

<details><summary>响应示例</summary>

```json
{
  "auto_deploy": false,
  "build_type": "string",
  "created_at": "2026-01-01T00:00:00Z",
  "deploy_path": "string",
  "deploy_strategy": "string",
  "deploy_type": "string",
  "domain": "string",
  "has_procfile": false,
  "has_webhook_token": false,
  "health_check_path": "string",
  "id": 0,
  "kind": "string",
  "name": "string",
  "port": 0,
  "process_unit": "string",
  "run_command": "string",
  "runtime_id": 0,
  "source": "string",
  "source_url": "string",
  "status": "string",
  "updated_at": "2026-01-01T00:00:00Z"
}
```

</details>

```bash
fpctl api POST '/api/projects/1/deploy' --json
```

### POST `/api/projects/{id}/health-check`

手动健康探测（Stage A-9 · Doc/20 §5.8 ④）

- 认证：Bearer JWT
- 权限：`project:update`
- operationId：`project_health_check`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 项目 ID |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 探测结果 | HealthCheckDto |
| 400 | 未配置健康检查路径或端口 |  |
| 503 | 健康探测器不可用 |  |

<details><summary>响应示例</summary>

```json
{
  "healthy": false,
  "project_id": 0
}
```

</details>

```bash
fpctl api POST '/api/projects/1/health-check' --json
```

### GET `/api/projects/{id}/links`

项目的联链列表

- 认证：Bearer JWT
- 权限：`project:read`
- operationId：`list_project_links`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 项目 ID |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 联链列表（不含连接串） | ProjectDbLinkDto[] |
| 404 | 项目不存在 |  |
| 503 | 联链组件未启用 |  |

<details><summary>响应示例</summary>

```json
[
  {
    "db_instance_id": 0,
    "db_instance_name": "string",
    "db_type": "string",
    "env_name": "string",
    "id": 0,
    "port": 0,
    "project_id": 0
  }
]
```

</details>

```bash
fpctl api GET '/api/projects/1/links' --json
```

### POST `/api/projects/{id}/links`

关联数据库（联链成功即在建库侧幂等创建业务库；连接串在启动时注入）。

- 认证：Bearer JWT
- 权限：`project:update`
- operationId：`link_project_database`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 项目 ID |

**请求体**（`application/json`，必填）：LinkDatabaseRequest

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `db_instance_id` | integer(int64) | 是 | 数据库实例 ID（/api/databases 返回的实例） |
| `db_name` | string? | 否 | 业务库名（留空取项目名；redis 忽略） |
| `env_name` | string? | 否 | 注入的环境变量名（留空按 db_type 推导默认值） |

```json
{
  "db_instance_id": 0,
  "db_name": "string",
  "env_name": "string"
}
```

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 关联成功 | ProjectDbLinkDto |
| 400 | 参数校验失败 / 实例未运行 |  |
| 404 | 项目或数据库实例不存在 |  |
| 409 | 环境变量名冲突或已关联该实例 |  |
| 503 | 联链组件未启用 |  |

<details><summary>响应示例</summary>

```json
{
  "db_instance_id": 0,
  "db_instance_name": "string",
  "db_type": "string",
  "env_name": "string",
  "id": 0,
  "port": 0,
  "project_id": 0
}
```

</details>

```bash
fpctl api POST '/api/projects/1/links' -d '{"db_instance_id":0,"db_name":"string","env_name":"string"}' --json
```

### GET `/api/projects/{id}/links/available`

可关联的数据库实例清单（联链弹窗数据源；仅返回 running 实例）。

- 认证：Bearer JWT
- 权限：`project:read`
- operationId：`list_linkable_databases`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 项目 ID |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 可关联实例清单 | LinkableDatabaseDto[] |
| 404 | 项目不存在 |  |

<details><summary>响应示例</summary>

```json
[
  {
    "db_type": "string",
    "default_env_name": "string",
    "id": 0,
    "name": "string",
    "port": 0,
    "status": "string"
  }
]
```

</details>

```bash
fpctl api GET '/api/projects/1/links/available' --json
```

### DELETE `/api/projects/{id}/links/{link_id}`

解除联链

- 认证：Bearer JWT
- 权限：`project:delete`
- operationId：`unlink_project_database`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 项目 ID |
| `link_id` | path | integer(int64) | 是 | 联链 ID |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 解除成功 | string (`text/plain`) |
| 404 | 项目或联链不存在 |  |
| 503 | 联链组件未启用 |  |

```bash
fpctl api DELETE '/api/projects/1/links/1' --json
```

### GET `/api/projects/{id}/logs`

最近日志（按执行面分流：容器模式 docker logs / 原生模式守护器日志）

- 认证：Bearer JWT
- 权限：`project:read`
- operationId：`project_logs`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 项目 ID |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 日志列表（容器模式取 docker logs） | LogLine[] |

<details><summary>响应示例</summary>

```json
[
  {
    "message": "string",
    "timestamp": "string"
  }
]
```

</details>

```bash
fpctl api GET '/api/projects/1/logs' --json
```

### GET `/api/projects/{id}/procfile`

查看项目 Procfile 进程声明

- 认证：Bearer JWT
- 权限：`project:read`
- operationId：`get_project_procfile`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 项目 ID |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 进程声明（无 Procfile 为空表） | ProcfileDto |
| 404 | 项目不存在 |  |

<details><summary>响应示例</summary>

```json
{
  "processes": [
    {
      "command": "string",
      "proc_type": "string"
    }
  ],
  "project_id": 0
}
```

</details>

```bash
fpctl api GET '/api/projects/1/procfile' --json
```

### POST `/api/projects/{id}/restart`

重启项目

- 认证：Bearer JWT
- 权限：`database:start`
- operationId：`restart_project`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 项目 ID |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 重启成功 | ProjectDto |

<details><summary>响应示例</summary>

```json
{
  "auto_deploy": false,
  "build_type": "string",
  "created_at": "2026-01-01T00:00:00Z",
  "deploy_path": "string",
  "deploy_strategy": "string",
  "deploy_type": "string",
  "domain": "string",
  "has_procfile": false,
  "has_webhook_token": false,
  "health_check_path": "string",
  "id": 0,
  "kind": "string",
  "name": "string",
  "port": 0,
  "process_unit": "string",
  "run_command": "string",
  "runtime_id": 0,
  "source": "string",
  "source_url": "string",
  "status": "string",
  "updated_at": "2026-01-01T00:00:00Z"
}
```

</details>

```bash
fpctl api POST '/api/projects/1/restart' --json
```

### POST `/api/projects/{id}/start`

启动项目

- 认证：Bearer JWT
- 权限：`database:start`
- operationId：`start_project`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 项目 ID |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 启动成功 | ProjectDto |
| 400 | 未配置启动命令 |  |

<details><summary>响应示例</summary>

```json
{
  "auto_deploy": false,
  "build_type": "string",
  "created_at": "2026-01-01T00:00:00Z",
  "deploy_path": "string",
  "deploy_strategy": "string",
  "deploy_type": "string",
  "domain": "string",
  "has_procfile": false,
  "has_webhook_token": false,
  "health_check_path": "string",
  "id": 0,
  "kind": "string",
  "name": "string",
  "port": 0,
  "process_unit": "string",
  "run_command": "string",
  "runtime_id": 0,
  "source": "string",
  "source_url": "string",
  "status": "string",
  "updated_at": "2026-01-01T00:00:00Z"
}
```

</details>

```bash
fpctl api POST '/api/projects/1/start' --json
```

### GET `/api/projects/{id}/status`

实时进程状态（按执行面分流：容器模式 docker inspect / 原生模式守护器探测）

- 认证：Bearer JWT
- 权限：`database:read`
- operationId：`project_process_status`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 项目 ID |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 进程状态（容器模式自容器探测；两路同构） | ProcessStatus |
| 404 | 项目不存在 |  |

<details><summary>响应示例</summary>

```json
{
  "exit_code": 0,
  "restarts": 0,
  "state": "string"
}
```

</details>

```bash
fpctl api GET '/api/projects/1/status' --json
```

### POST `/api/projects/{id}/stop`

停止项目

- 认证：Bearer JWT
- 权限：`database:stop`
- operationId：`stop_project`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 项目 ID |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 停止成功 | ProjectDto |

<details><summary>响应示例</summary>

```json
{
  "auto_deploy": false,
  "build_type": "string",
  "created_at": "2026-01-01T00:00:00Z",
  "deploy_path": "string",
  "deploy_strategy": "string",
  "deploy_type": "string",
  "domain": "string",
  "has_procfile": false,
  "has_webhook_token": false,
  "health_check_path": "string",
  "id": 0,
  "kind": "string",
  "name": "string",
  "port": 0,
  "process_unit": "string",
  "run_command": "string",
  "runtime_id": 0,
  "source": "string",
  "source_url": "string",
  "status": "string",
  "updated_at": "2026-01-01T00:00:00Z"
}
```

</details>

```bash
fpctl api POST '/api/projects/1/stop' --json
```

### POST `/api/projects/{id}/webhook`

Webhook push 触发部署（免 JWT，令牌经中间件校验；
兼容 GitHub / GitLab / Gitee / Gitea / 通用 JSON——服务端不解析平台语义，
任何 POST 命中即视为一次 push 部署请求）

- 认证：无
- 权限：公开（无需登录）
- operationId：`project_webhook_deploy`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 项目 ID |

**请求体**（`application/json`）：any

```json
null
```

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 部署已触发 | ProjectDto |
| 401 | 令牌缺失/无效 |  |
| 404 | 项目未启用 webhook |  |
| 409 | 同项目操作互斥 |  |

<details><summary>响应示例</summary>

```json
{
  "auto_deploy": false,
  "build_type": "string",
  "created_at": "2026-01-01T00:00:00Z",
  "deploy_path": "string",
  "deploy_strategy": "string",
  "deploy_type": "string",
  "domain": "string",
  "has_procfile": false,
  "has_webhook_token": false,
  "health_check_path": "string",
  "id": 0,
  "kind": "string",
  "name": "string",
  "port": 0,
  "process_unit": "string",
  "run_command": "string",
  "runtime_id": 0,
  "source": "string",
  "source_url": "string",
  "status": "string",
  "updated_at": "2026-01-01T00:00:00Z"
}
```

</details>

### POST `/api/projects/{id}/webhook/disable`

关闭项目 webhook 部署

- 认证：Bearer JWT
- 权限：`project:update`
- operationId：`disable_project_webhook`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 项目 ID |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 已关闭 | ProjectDto |
| 404 | 项目不存在 |  |

<details><summary>响应示例</summary>

```json
{
  "auto_deploy": false,
  "build_type": "string",
  "created_at": "2026-01-01T00:00:00Z",
  "deploy_path": "string",
  "deploy_strategy": "string",
  "deploy_type": "string",
  "domain": "string",
  "has_procfile": false,
  "has_webhook_token": false,
  "health_check_path": "string",
  "id": 0,
  "kind": "string",
  "name": "string",
  "port": 0,
  "process_unit": "string",
  "run_command": "string",
  "runtime_id": 0,
  "source": "string",
  "source_url": "string",
  "status": "string",
  "updated_at": "2026-01-01T00:00:00Z"
}
```

</details>

```bash
fpctl api POST '/api/projects/1/webhook/disable' --json
```

### POST `/api/projects/{id}/webhook/enable`

开启/重置项目 webhook 部署（返回一次性部署令牌）

- 认证：Bearer JWT
- 权限：`project:update`
- operationId：`enable_project_webhook`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 项目 ID |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 已开启（重复调用轮换令牌） | WebhookDeployDto |
| 400 | 非 git 来源项目 |  |
| 404 | 项目不存在 |  |

<details><summary>响应示例</summary>

```json
{
  "project": {
    "auto_deploy": false,
    "build_type": "string",
    "created_at": "2026-01-01T00:00:00Z",
    "deploy_path": "string",
    "deploy_strategy": "string",
    "deploy_type": "string",
    "domain": "string",
    "has_procfile": false,
    "has_webhook_token": false,
    "health_check_path": "string",
    "id": 0,
    "kind": "string",
    "name": "string",
    "port": 0,
    "process_unit": "string",
    "run_command": "string",
    "runtime_id": 0,
    "source": "string",
    "source_url": "string",
    "status": "string",
    "updated_at": "2026-01-01T00:00:00Z"
  },
  "token": "string",
  "webhook_url": "string"
}
```

</details>

```bash
fpctl api POST '/api/projects/1/webhook/enable' --json
```
