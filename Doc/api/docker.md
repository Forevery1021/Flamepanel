<!-- 由 scripts/gen-api-docs.py 从 frontend/openapi.json 生成，请勿手改；修改 utoipa 注解后运行 just api-docs -->

# API · docker

> 35 个端点 · 通用约定见 [README](./README.md)

| 方法 | 路径 | 摘要 | 权限 |
|---|---|---|---|
| GET | `/api/docker/compose` | Compose 项目列表（含已停止项目；`{name, status, config_files}`） | `docker:read` |
| POST | `/api/docker/compose/convert-run` | `docker run` 命令 → Compose YAML（纯文本转换，不执行命令、不接触 Docker） | `docker:create` |
| POST | `/api/docker/compose/deploy` | 部署 Compose 项目（等价于宿主机 root 能力，故归 `docker:create`） | `docker:create` |
| POST | `/api/docker/compose/{project_name}/down` | 停止并移除 Compose 项目容器与网络（保留配置文件与数据） | `docker:stop` |
| POST | `/api/docker/compose/{project_name}/up` | 启动 Compose 项目（使用项目记录的配置文件） | `docker:start` |
| GET | `/api/docker/containers` | 容器列表 | `docker:read` |
| POST | `/api/docker/containers/prune` | 清理已停止容器 | `docker:delete` |
| GET | `/api/docker/containers/{id}` | 单个容器（按 ID 或名称匹配） | `docker:read` |
| GET | `/api/docker/containers/{id}/compose` | 既有容器 → Compose YAML（收编：导出后停止原容器，再以该 YAML 部署） | `docker:read` |
| GET | `/api/docker/containers/{id}/inspect` | 容器详情（Engine API inspect 原样） | `docker:read` |
| POST | `/api/docker/containers/{id}/kill` | 强制终止容器 | `docker:start` |
| GET | `/api/docker/containers/{id}/logs` | 容器日志（stdout + stderr，输出上限 1 MiB） | `docker:read` |
| POST | `/api/docker/containers/{id}/pause` | 暂停容器 | `docker:start` |
| POST | `/api/docker/containers/{id}/remove` | 删除容器（非强制） | `docker:delete` |
| POST | `/api/docker/containers/{id}/rename` | 重命名容器 | `docker:update` |
| POST | `/api/docker/containers/{id}/restart` | 重启容器（10 s 超时） | `docker:start` |
| POST | `/api/docker/containers/{id}/start` | 启动容器 | `docker:start` |
| GET | `/api/docker/containers/{id}/stats` | 容器资源统计（Engine API 原样） | `docker:read` |
| POST | `/api/docker/containers/{id}/stop` | 停止容器（10 s 超时） | `docker:stop` |
| POST | `/api/docker/containers/{id}/unpause` | 恢复容器 | `docker:start` |
| GET | `/api/docker/images` | 镜像列表 | `docker:read` |
| POST | `/api/docker/images/prune` | 清理悬空镜像 | `docker:delete` |
| POST | `/api/docker/images/pull` | 拉取镜像（`{image, message}`） | `docker:start` |
| POST | `/api/docker/images/{id}/remove` | 删除镜像 | `docker:delete` |
| POST | `/api/docker/images/{id}/tag` | 为镜像打标签 | `docker:update` |
| GET | `/api/docker/networks` | 网络列表 | `docker:read` |
| POST | `/api/docker/networks` | 创建网络 | `docker:create` |
| POST | `/api/docker/networks/prune` | 清理未使用网络 | `docker:delete` |
| DELETE | `/api/docker/networks/{id}` | 删除网络 | `docker:delete` |
| POST | `/api/docker/networks/{id}/connect` | 把容器接入网络 | `docker:update` |
| POST | `/api/docker/networks/{id}/disconnect` | 把容器移出网络（`force` 可强制） | `docker:update` |
| GET | `/api/docker/volumes` | 卷列表 | `docker:read` |
| POST | `/api/docker/volumes` | 创建卷 | `docker:create` |
| POST | `/api/docker/volumes/prune` | 清理未使用卷 | `docker:delete` |
| DELETE | `/api/docker/volumes/{name}` | 删除卷 | `docker:delete` |

### GET `/api/docker/compose`

Compose 项目列表（含已停止项目；`{name, status, config_files}`）

- 认证：Bearer JWT
- 权限：`docker:read`
- operationId：`compose_ls`

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 项目 | any[] |
| 401 | 未认证 |  |
| 403 | 缺少 docker:read 权限 |  |

```bash
fpctl api GET '/api/docker/compose' --json
```

### POST `/api/docker/compose/convert-run`

`docker run` 命令 → Compose YAML（纯文本转换，不执行命令、不接触 Docker）

- 认证：Bearer JWT
- 权限：`docker:create`
- operationId：`compose_convert_run`

**请求体**（`application/json`，必填）：ConvertRunRequest

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `command` | string | 是 | 完整的 `docker run …` 命令（可含 `\` 续行，≤ 16 KiB） |

```json
{
  "command": "string"
}
```

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 转换结果；`warnings` 列出无法等价转换的参数 | ComposeConversion |
| 400 | 不是 docker run 命令 / 含管道等 shell 结构 / 不支持的参数 / 超过 16 KiB |  |
| 401 | 未认证 |  |
| 403 | 缺少 docker:create 权限 |  |

<details><summary>响应示例</summary>

```json
{
  "compose_yaml": "string",
  "project_name": "string",
  "service_name": "string",
  "warnings": [
    "string"
  ]
}
```

</details>

```bash
fpctl api POST '/api/docker/compose/convert-run' -d '{"command":"string"}' --json
```

### POST `/api/docker/compose/deploy`

部署 Compose 项目（等价于宿主机 root 能力，故归 `docker:create`）

- 认证：Bearer JWT
- 权限：`docker:create`
- operationId：`compose_deploy`

**请求体**（`application/json`，必填）：ComposeDeployRequest

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `compose_yaml` | string | 是 | ≤ 256 KiB；文件持久化到 `data/compose/<project>/docker-compose.yml` |
| `project_name` | string | 是 | `[A-Za-z0-9][A-Za-z0-9_.-]*`，≤ 63 |

```json
{
  "compose_yaml": "string",
  "project_name": "string"
}
```

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | `{project_name, path, status}` |  |
| 400 | 项目名非法 / YAML 为空或超过 256 KiB |  |
| 401 | 未认证 |  |
| 403 | 缺少 docker:create 权限 |  |
| 500 | docker compose up 失败 |  |

```bash
fpctl api POST '/api/docker/compose/deploy' -d '{"compose_yaml":"string","project_name":"string"}' --json
```

### POST `/api/docker/compose/{project_name}/down`

停止并移除 Compose 项目容器与网络（保留配置文件与数据）

- 认证：Bearer JWT
- 权限：`docker:stop`
- operationId：`compose_down`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `project_name` | path | string | 是 | Compose 项目名 |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 成功（`null`） |  |
| 400 | 项目名非法 |  |
| 401 | 未认证 |  |
| 403 | 缺少 docker:stop 权限 |  |

```bash
fpctl api POST '/api/docker/compose/1/down' --json
```

### POST `/api/docker/compose/{project_name}/up`

启动 Compose 项目（使用项目记录的配置文件）

- 认证：Bearer JWT
- 权限：`docker:start`
- operationId：`compose_up`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `project_name` | path | string | 是 | Compose 项目名 |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 成功（`null`） |  |
| 400 | 项目名非法 |  |
| 401 | 未认证 |  |
| 403 | 缺少 docker:start 权限 |  |

```bash
fpctl api POST '/api/docker/compose/1/up' --json
```

### GET `/api/docker/containers`

容器列表

- 认证：Bearer JWT
- 权限：`docker:read`
- operationId：`list`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `node_id` | query | integer(int64) | 否 | 节点 ID，缺省 0 = 本机 |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 容器 | DockerContainer[] |
| 401 | 未认证 |  |
| 403 | 缺少 docker:read 权限 |  |

<details><summary>响应示例</summary>

```json
[
  {
    "created_at": "2026-01-01T00:00:00Z",
    "id": "string",
    "image": "string",
    "name": "string",
    "node_id": 0,
    "status": "string"
  }
]
```

</details>

```bash
fpctl api GET '/api/docker/containers' --json
```

### POST `/api/docker/containers/prune`

清理已停止容器

- 认证：Bearer JWT
- 权限：`docker:delete`
- operationId：`prune_containers`

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 清理结果 |  |
| 401 | 未认证 |  |
| 403 | 缺少 docker:delete 权限 |  |

```bash
fpctl api POST '/api/docker/containers/prune' --json
```

### GET `/api/docker/containers/{id}`

单个容器（按 ID 或名称匹配）

- 认证：Bearer JWT
- 权限：`docker:read`
- operationId：`get`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | string | 是 | 容器 ID 或名称（`[A-Za-z0-9][A-Za-z0-9_.-]*`） |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 容器 | DockerContainer |
| 401 | 未认证 |  |
| 403 | 缺少 docker:read 权限 |  |
| 404 | 容器不存在 |  |

<details><summary>响应示例</summary>

```json
{
  "created_at": "2026-01-01T00:00:00Z",
  "id": "string",
  "image": "string",
  "name": "string",
  "node_id": 0,
  "status": "string"
}
```

</details>

```bash
fpctl api GET '/api/docker/containers/1' --json
```

### GET `/api/docker/containers/{id}/compose`

既有容器 → Compose YAML（收编：导出后停止原容器，再以该 YAML 部署）

- 认证：Bearer JWT
- 权限：`docker:read`
- operationId：`container_compose`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | string | 是 | 容器 ID 或名称（`[A-Za-z0-9][A-Za-z0-9_.-]*`） |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 转换结果（YAML 仅供审阅，不会部署） | ComposeConversion |
| 400 | 引用含非法字符 / 容器信息缺少镜像 |  |
| 401 | 未认证 |  |
| 403 | 缺少 docker:read 权限 |  |

<details><summary>响应示例</summary>

```json
{
  "compose_yaml": "string",
  "project_name": "string",
  "service_name": "string",
  "warnings": [
    "string"
  ]
}
```

</details>

```bash
fpctl api GET '/api/docker/containers/1/compose' --json
```

### GET `/api/docker/containers/{id}/inspect`

容器详情（Engine API inspect 原样）

- 认证：Bearer JWT
- 权限：`docker:read`
- operationId：`inspect`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | string | 是 | 容器 ID 或名称（`[A-Za-z0-9][A-Za-z0-9_.-]*`） |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 详情 |  |
| 400 | 引用含非法字符（`/`、`..`、`?` 等，Doc/33 P-12） |  |
| 401 | 未认证 |  |
| 403 | 缺少 docker:read 权限 |  |

```bash
fpctl api GET '/api/docker/containers/1/inspect' --json
```

### POST `/api/docker/containers/{id}/kill`

强制终止容器

- 认证：Bearer JWT
- 权限：`docker:start`
- operationId：`kill`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | string | 是 | 容器 ID 或名称（`[A-Za-z0-9][A-Za-z0-9_.-]*`） |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 成功（`null`） |  |
| 400 | 引用含非法字符（`/`、`..`、`?` 等，Doc/33 P-12） |  |
| 401 | 未认证 |  |
| 403 | 缺少 docker:start 权限 |  |

```bash
fpctl api POST '/api/docker/containers/1/kill' --json
```

### GET `/api/docker/containers/{id}/logs`

容器日志（stdout + stderr，输出上限 1 MiB）

- 认证：Bearer JWT
- 权限：`docker:read`
- operationId：`logs`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | string | 是 | 容器 ID 或名称（`[A-Za-z0-9][A-Za-z0-9_.-]*`） |
| `tail` | query | integer | 否 | 尾行数，缺省 100，钳制到 1–10000 |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 日志文本 | string (`text/plain`) |
| 400 | 引用含非法字符（`/`、`..`、`?` 等，Doc/33 P-12） |  |
| 401 | 未认证 |  |
| 403 | 缺少 docker:read 权限 |  |

```bash
fpctl api GET '/api/docker/containers/1/logs' --json
```

### POST `/api/docker/containers/{id}/pause`

暂停容器

- 认证：Bearer JWT
- 权限：`docker:start`
- operationId：`pause`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | string | 是 | 容器 ID 或名称（`[A-Za-z0-9][A-Za-z0-9_.-]*`） |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 成功（`null`） |  |
| 400 | 引用含非法字符（`/`、`..`、`?` 等，Doc/33 P-12） |  |
| 401 | 未认证 |  |
| 403 | 缺少 docker:start 权限 |  |

```bash
fpctl api POST '/api/docker/containers/1/pause' --json
```

### POST `/api/docker/containers/{id}/remove`

删除容器（非强制）

- 认证：Bearer JWT
- 权限：`docker:delete`
- operationId：`remove`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | string | 是 | 容器 ID 或名称（`[A-Za-z0-9][A-Za-z0-9_.-]*`） |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 成功（`null`） |  |
| 400 | 引用含非法字符（`/`、`..`、`?` 等，Doc/33 P-12） |  |
| 401 | 未认证 |  |
| 403 | 缺少 docker:delete 权限 |  |

```bash
fpctl api POST '/api/docker/containers/1/remove' --json
```

### POST `/api/docker/containers/{id}/rename`

重命名容器

- 认证：Bearer JWT
- 权限：`docker:update`
- operationId：`rename`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | string | 是 | 容器 ID 或名称（`[A-Za-z0-9][A-Za-z0-9_.-]*`） |

**请求体**（`application/json`，必填）：DockerRenameRequest

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `new_name` | string | 是 |  |

```json
{
  "new_name": "string"
}
```

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 成功（`null`） |  |
| 400 | ID 或新名称非法 |  |
| 401 | 未认证 |  |
| 403 | 缺少 docker:update 权限 |  |

```bash
fpctl api POST '/api/docker/containers/1/rename' -d '{"new_name":"string"}' --json
```

### POST `/api/docker/containers/{id}/restart`

重启容器（10 s 超时）

- 认证：Bearer JWT
- 权限：`docker:start`
- operationId：`restart`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | string | 是 | 容器 ID 或名称（`[A-Za-z0-9][A-Za-z0-9_.-]*`） |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 成功（`null`） |  |
| 400 | 引用含非法字符（`/`、`..`、`?` 等，Doc/33 P-12） |  |
| 401 | 未认证 |  |
| 403 | 缺少 docker:start 权限 |  |

```bash
fpctl api POST '/api/docker/containers/1/restart' --json
```

### POST `/api/docker/containers/{id}/start`

启动容器

- 认证：Bearer JWT
- 权限：`docker:start`
- operationId：`start`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | string | 是 | 容器 ID 或名称（`[A-Za-z0-9][A-Za-z0-9_.-]*`） |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 成功（`null`） |  |
| 400 | 引用含非法字符（`/`、`..`、`?` 等，Doc/33 P-12） |  |
| 401 | 未认证 |  |
| 403 | 缺少 docker:start 权限 |  |

```bash
fpctl api POST '/api/docker/containers/1/start' --json
```

### GET `/api/docker/containers/{id}/stats`

容器资源统计（Engine API 原样）

- 认证：Bearer JWT
- 权限：`docker:read`
- operationId：`stats`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | string | 是 | 容器 ID 或名称（`[A-Za-z0-9][A-Za-z0-9_.-]*`） |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 统计 |  |
| 400 | 引用含非法字符（`/`、`..`、`?` 等，Doc/33 P-12） |  |
| 401 | 未认证 |  |
| 403 | 缺少 docker:read 权限 |  |

```bash
fpctl api GET '/api/docker/containers/1/stats' --json
```

### POST `/api/docker/containers/{id}/stop`

停止容器（10 s 超时）

- 认证：Bearer JWT
- 权限：`docker:stop`
- operationId：`stop`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | string | 是 | 容器 ID 或名称（`[A-Za-z0-9][A-Za-z0-9_.-]*`） |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 成功（`null`） |  |
| 400 | 引用含非法字符（`/`、`..`、`?` 等，Doc/33 P-12） |  |
| 401 | 未认证 |  |
| 403 | 缺少 docker:stop 权限 |  |

```bash
fpctl api POST '/api/docker/containers/1/stop' --json
```

### POST `/api/docker/containers/{id}/unpause`

恢复容器

- 认证：Bearer JWT
- 权限：`docker:start`
- operationId：`unpause`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | string | 是 | 容器 ID 或名称（`[A-Za-z0-9][A-Za-z0-9_.-]*`） |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 成功（`null`） |  |
| 400 | 引用含非法字符（`/`、`..`、`?` 等，Doc/33 P-12） |  |
| 401 | 未认证 |  |
| 403 | 缺少 docker:start 权限 |  |

```bash
fpctl api POST '/api/docker/containers/1/unpause' --json
```

### GET `/api/docker/images`

镜像列表

- 认证：Bearer JWT
- 权限：`docker:read`
- operationId：`list_images`

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 镜像 | any[] |
| 401 | 未认证 |  |
| 403 | 缺少 docker:read 权限 |  |

```bash
fpctl api GET '/api/docker/images' --json
```

### POST `/api/docker/images/prune`

清理悬空镜像

- 认证：Bearer JWT
- 权限：`docker:delete`
- operationId：`prune_images`

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 清理结果 |  |
| 401 | 未认证 |  |
| 403 | 缺少 docker:delete 权限 |  |

```bash
fpctl api POST '/api/docker/images/prune' --json
```

### POST `/api/docker/images/pull`

拉取镜像（`{image, message}`）

- 认证：Bearer JWT
- 权限：`docker:start`
- operationId：`pull_image`

**请求体**（`application/json`，必填）：PullImageRequest

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `image` | string | 是 |  |

```json
{
  "image": "string"
}
```

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 拉取结果 |  |
| 400 | 镜像引用非法 |  |
| 401 | 未认证 |  |
| 403 | 缺少 docker:start 权限 |  |

```bash
fpctl api POST '/api/docker/images/pull' -d '{"image":"string"}' --json
```

### POST `/api/docker/images/{id}/remove`

删除镜像

- 认证：Bearer JWT
- 权限：`docker:delete`
- operationId：`remove_image`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | string | 是 | 镜像 ID 或引用 |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 成功（`null`） |  |
| 400 | 引用含非法字符（`/`、`..`、`?` 等，Doc/33 P-12） |  |
| 401 | 未认证 |  |
| 403 | 缺少 docker:delete 权限 |  |

```bash
fpctl api POST '/api/docker/images/1/remove' --json
```

### POST `/api/docker/images/{id}/tag`

为镜像打标签

- 认证：Bearer JWT
- 权限：`docker:update`
- operationId：`tag_image`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | string | 是 | 镜像 ID 或引用 |

**请求体**（`application/json`，必填）：TagImageRequest

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `repo` | string | 是 | 仓库名（不含 tag / digest），如 `registry.local:5000/team/app` |
| `tag` | string | 是 | `[A-Za-z0-9_][A-Za-z0-9_.-]{0,127}` |

```json
{
  "repo": "string",
  "tag": "string"
}
```

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 成功（`null`） |  |
| 400 | 镜像 / 仓库 / tag 非法；repo 含 tag 或 digest |  |
| 401 | 未认证 |  |
| 403 | 缺少 docker:update 权限 |  |

```bash
fpctl api POST '/api/docker/images/1/tag' -d '{"repo":"string","tag":"string"}' --json
```

### GET `/api/docker/networks`

网络列表

- 认证：Bearer JWT
- 权限：`docker:read`
- operationId：`list_networks`

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 网络 | any[] |
| 401 | 未认证 |  |
| 403 | 缺少 docker:read 权限 |  |

```bash
fpctl api GET '/api/docker/networks' --json
```

### POST `/api/docker/networks`

创建网络

- 认证：Bearer JWT
- 权限：`docker:create`
- operationId：`create_network`

**请求体**（`application/json`，必填）：CreateNetworkRequest

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `driver` | string | 否 |  |
| `name` | string | 是 |  |
| `subnet` | string? | 否 |  |

```json
{
  "driver": "string",
  "name": "string",
  "subnet": "string"
}
```

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 创建结果 |  |
| 400 | 名称 / 驱动 / 子网（CIDR）非法 |  |
| 401 | 未认证 |  |
| 403 | 缺少 docker:create 权限 |  |

```bash
fpctl api POST '/api/docker/networks' -d '{"driver":"string","name":"string","subnet":"string"}' --json
```

### POST `/api/docker/networks/prune`

清理未使用网络

- 认证：Bearer JWT
- 权限：`docker:delete`
- operationId：`prune_networks`

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 清理结果 |  |
| 401 | 未认证 |  |
| 403 | 缺少 docker:delete 权限 |  |

```bash
fpctl api POST '/api/docker/networks/prune' --json
```

### DELETE `/api/docker/networks/{id}`

删除网络

- 认证：Bearer JWT
- 权限：`docker:delete`
- operationId：`remove_network`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | string | 是 | 网络 ID 或名称 |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 成功（`null`） |  |
| 400 | 引用含非法字符（`/`、`..`、`?` 等，Doc/33 P-12） |  |
| 401 | 未认证 |  |
| 403 | 缺少 docker:delete 权限 |  |

```bash
fpctl api DELETE '/api/docker/networks/1' --json
```

### POST `/api/docker/networks/{id}/connect`

把容器接入网络

- 认证：Bearer JWT
- 权限：`docker:update`
- operationId：`connect_network`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | string | 是 | 网络 ID 或名称 |

**请求体**（`application/json`，必填）：ConnectNetworkRequest

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `container_id` | string | 是 |  |
| `force` | boolean | 否 |  |

```json
{
  "container_id": "string",
  "force": false
}
```

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 成功（`null`） |  |
| 400 | 引用含非法字符（`/`、`..`、`?` 等，Doc/33 P-12） |  |
| 401 | 未认证 |  |
| 403 | 缺少 docker:update 权限 |  |

```bash
fpctl api POST '/api/docker/networks/1/connect' -d '{"container_id":"string","force":false}' --json
```

### POST `/api/docker/networks/{id}/disconnect`

把容器移出网络（`force` 可强制）

- 认证：Bearer JWT
- 权限：`docker:update`
- operationId：`disconnect_network`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | string | 是 | 网络 ID 或名称 |

**请求体**（`application/json`，必填）：ConnectNetworkRequest

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `container_id` | string | 是 |  |
| `force` | boolean | 否 |  |

```json
{
  "container_id": "string",
  "force": false
}
```

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 成功（`null`） |  |
| 400 | 引用含非法字符（`/`、`..`、`?` 等，Doc/33 P-12） |  |
| 401 | 未认证 |  |
| 403 | 缺少 docker:update 权限 |  |

```bash
fpctl api POST '/api/docker/networks/1/disconnect' -d '{"container_id":"string","force":false}' --json
```

### GET `/api/docker/volumes`

卷列表

- 认证：Bearer JWT
- 权限：`docker:read`
- operationId：`list_volumes`

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 卷 | any[] |
| 401 | 未认证 |  |
| 403 | 缺少 docker:read 权限 |  |

```bash
fpctl api GET '/api/docker/volumes' --json
```

### POST `/api/docker/volumes`

创建卷

- 认证：Bearer JWT
- 权限：`docker:create`
- operationId：`create_volume`

**请求体**（`application/json`，必填）：CreateVolumeRequest

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `driver` | string | 否 |  |
| `name` | string | 是 |  |

```json
{
  "driver": "string",
  "name": "string"
}
```

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 创建结果 |  |
| 400 | 名称 / 驱动非法 |  |
| 401 | 未认证 |  |
| 403 | 缺少 docker:create 权限 |  |

```bash
fpctl api POST '/api/docker/volumes' -d '{"driver":"string","name":"string"}' --json
```

### POST `/api/docker/volumes/prune`

清理未使用卷

- 认证：Bearer JWT
- 权限：`docker:delete`
- operationId：`prune_volumes`

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 清理结果 |  |
| 401 | 未认证 |  |
| 403 | 缺少 docker:delete 权限 |  |

```bash
fpctl api POST '/api/docker/volumes/prune' --json
```

### DELETE `/api/docker/volumes/{name}`

删除卷

- 认证：Bearer JWT
- 权限：`docker:delete`
- operationId：`remove_volume`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `name` | path | string | 是 | 卷名 |
| `force` | query | boolean | 否 | 强制删除 |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 成功（`null`） |  |
| 400 | 引用含非法字符（`/`、`..`、`?` 等，Doc/33 P-12） |  |
| 401 | 未认证 |  |
| 403 | 缺少 docker:delete 权限 |  |

```bash
fpctl api DELETE '/api/docker/volumes/1' --json
```
