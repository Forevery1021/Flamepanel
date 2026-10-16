<!-- 由 scripts/gen-api-docs.py 从 frontend/openapi.json 生成，请勿手改；修改 utoipa 注解后运行 just api-docs -->

# API · app_store

> 26 个端点 · 通用约定见 [README](./README.md)

| 方法 | 路径 | 摘要 | 权限 |
|---|---|---|---|
| GET | `/api/app-store/installed` | 已安装应用列表 | `app_store:read` |
| GET | `/api/app-store/installed/{id}` | 已安装应用详情 | `app_store:read` |
| POST | `/api/app-store/installed/{id}/launch` | 记录应用启动次数（常用应用排序） 记录一次启动（常用应用排序） | `app_store:read` |
| GET | `/api/app-store/installed/{id}/logs` | 容器日志尾部（`{logs}`；原生模式为空） | `app_store:read` |
| POST | `/api/app-store/installed/{id}/native-restart` | 重启原生已安装应用服务 重启已安装应用服务（`{success, id, status}`） | `app_store:update` |
| POST | `/api/app-store/installed/{id}/native-start` | 启动原生已安装应用服务 启动已安装应用服务（`{success, id, status}`） | `app_store:update` |
| GET | `/api/app-store/installed/{id}/native-status` | 原生已安装应用状态（running/stopped） 运行状态（`{id, status}`，running / stopped） | `app_store:read` |
| POST | `/api/app-store/installed/{id}/native-stop` | 停止原生已安装应用服务 停止已安装应用服务（`{success, id, status}`） | `app_store:update` |
| POST | `/api/app-store/installed/{id}/uninstall` | 卸载应用（`{success: true}`） | `app_store:delete` |
| POST | `/api/app-store/installed/{id}/upgrade` | 升级应用（拒绝降级；同版本幂等）。容器模式对 compose 做三方合并保留用户修改， 冲突时默认 409，可用 `conflict_strategy=local\|upstream` 指定处理方式。 | `app_store:update` |
| GET | `/api/app-store/installed/{id}/upgrade-preview` | 升级预览：渲染目标版本 compose 并与本地做三方合并，返回冲突 / 保留的本地修改 / 生效的上游修改与合并结果；不改动任何文件。 | `app_store:read` |
| GET | `/api/app-store/packages` | 应用商店包列表 | `app_store:read` |
| POST | `/api/app-store/packages/batch-import` | 批量导入本地目录应用包（Phase A2：`create_many` 事务语义，全成或全败） | `app_store:create` |
| POST | `/api/app-store/packages/import` | 导入服务器本地目录为应用包（Flame / 1Panel / 宝塔格式自动识别；key 须为安全路径段，不跟随符号链接） | `app_store:create` |
| GET | `/api/app-store/packages/{key}` | 应用包元数据 | `app_store:read` |
| POST | `/api/app-store/packages/{key}/install` | 安装应用（容器 / 原生 / WASM；高风险或含宿主机脚本需显式确认） | `app_store:create` |
| GET | `/api/app-store/packages/{key}/versions/{version}` | 指定版本详情（compose 模板、表单字段、钩子、原生安装参数） | `app_store:read` |
| GET | `/api/app-store/security` | 商店安全配置（验签策略 + 镜像仓库白名单） | `app_store:read` |
| PUT | `/api/app-store/security` | 更新商店安全配置（验签策略 / 镜像仓库白名单） | `app_store:update` |
| GET | `/api/app-store/sources` | 远程源列表 | `app_store:read` |
| POST | `/api/app-store/sources` | 添加远程源 | `app_store:create` |
| PUT | `/api/app-store/sources/{id}` | 更新远程源（name / url / enabled / public_key） | `app_store:update` |
| DELETE | `/api/app-store/sources/{id}` | 删除远程源 | `app_store:delete` |
| GET | `/api/app-store/sources/{id}/packages` | 按源过滤应用包（复用现有包列表，返回包元数据） | `app_store:read` |
| POST | `/api/app-store/sources/{id}/sync` | 手动同步远程源索引（同步任务经统一任务中心跟踪） | `app_store:update` |
| GET | `/api/app-store/wasm-builtins` | 内置 WASM 应用 | `app_store:read` |

### GET `/api/app-store/installed`

已安装应用列表

- 认证：Bearer JWT
- 权限：`app_store:read`
- operationId：`list_installed`

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 已安装应用列表 | InstalledApp[] |
| 401 | 未认证 |  |

<details><summary>响应示例</summary>

```json
[
  {
    "access_url": "string",
    "container_name": "string",
    "created_at": "2026-01-01T00:00:00Z",
    "id": 0,
    "install_path": "string",
    "launch_count": 0,
    "mode": "string",
    "name": "string",
    "package_key": "string",
    "params_json": "string",
    "port": 0,
    "status": "string",
    "updated_at": "2026-01-01T00:00:00Z",
    "version": "string"
  }
]
```

</details>

```bash
fpctl api GET '/api/app-store/installed' --json
```

### GET `/api/app-store/installed/{id}`

已安装应用详情

- 认证：Bearer JWT
- 权限：`app_store:read`
- operationId：`get_installed`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 已安装应用 ID |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 详情 | InstalledAppResponse |
| 401 | 未认证 |  |
| 403 | 缺少 app_store:read 权限 |  |
| 404 | 已安装应用不存在 |  |

<details><summary>响应示例</summary>

```json
{
  "app": {
    "access_url": "string",
    "container_name": "string",
    "created_at": "2026-01-01T00:00:00Z",
    "id": 0,
    "install_path": "string",
    "launch_count": 0,
    "mode": "string",
    "name": "string",
    "package_key": "string",
    "params_json": "string",
    "port": 0,
    "status": "string",
    "updated_at": "2026-01-01T00:00:00Z",
    "version": "string"
  },
  "logs": "string"
}
```

</details>

```bash
fpctl api GET '/api/app-store/installed/1' --json
```

### POST `/api/app-store/installed/{id}/launch`

记录应用启动次数（常用应用排序）
记录一次启动（常用应用排序）

- 认证：Bearer JWT
- 权限：`app_store:read`
- operationId：`launch`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 已安装应用 ID |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 更新后的应用 | InstalledApp |
| 401 | 未认证 |  |
| 403 | 缺少 app_store:read 权限 |  |
| 404 | 已安装应用不存在 |  |

<details><summary>响应示例</summary>

```json
{
  "access_url": "string",
  "container_name": "string",
  "created_at": "2026-01-01T00:00:00Z",
  "id": 0,
  "install_path": "string",
  "launch_count": 0,
  "mode": "string",
  "name": "string",
  "package_key": "string",
  "params_json": "string",
  "port": 0,
  "status": "string",
  "updated_at": "2026-01-01T00:00:00Z",
  "version": "string"
}
```

</details>

```bash
fpctl api POST '/api/app-store/installed/1/launch' --json
```

### GET `/api/app-store/installed/{id}/logs`

容器日志尾部（`{logs}`；原生模式为空）

- 认证：Bearer JWT
- 权限：`app_store:read`
- operationId：`get_logs`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 已安装应用 ID |
| `tail` | query | integer | 否 | 尾行数，缺省 200，钳制到 1–5000 |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 日志 |  |
| 401 | 未认证 |  |
| 403 | 缺少 app_store:read 权限 |  |
| 404 | 已安装应用不存在 |  |

```bash
fpctl api GET '/api/app-store/installed/1/logs' --json
```

### POST `/api/app-store/installed/{id}/native-restart`

重启原生已安装应用服务
重启已安装应用服务（`{success, id, status}`）

- 认证：Bearer JWT
- 权限：`app_store:update`
- operationId：`native_restart`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 已安装应用 ID |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 已执行 |  |
| 401 | 未认证 |  |
| 403 | 缺少 app_store:update 权限 |  |
| 404 | 已安装应用不存在 |  |

```bash
fpctl api POST '/api/app-store/installed/1/native-restart' --json
```

### POST `/api/app-store/installed/{id}/native-start`

启动原生已安装应用服务
启动已安装应用服务（`{success, id, status}`）

- 认证：Bearer JWT
- 权限：`app_store:update`
- operationId：`native_start`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 已安装应用 ID |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 已执行 |  |
| 401 | 未认证 |  |
| 403 | 缺少 app_store:update 权限 |  |
| 404 | 已安装应用不存在 |  |

```bash
fpctl api POST '/api/app-store/installed/1/native-start' --json
```

### GET `/api/app-store/installed/{id}/native-status`

原生已安装应用状态（running/stopped）
运行状态（`{id, status}`，running / stopped）

- 认证：Bearer JWT
- 权限：`app_store:read`
- operationId：`native_status`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 已安装应用 ID |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 状态 |  |
| 401 | 未认证 |  |
| 403 | 缺少 app_store:read 权限 |  |
| 404 | 已安装应用不存在 |  |

```bash
fpctl api GET '/api/app-store/installed/1/native-status' --json
```

### POST `/api/app-store/installed/{id}/native-stop`

停止原生已安装应用服务
停止已安装应用服务（`{success, id, status}`）

- 认证：Bearer JWT
- 权限：`app_store:update`
- operationId：`native_stop`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 已安装应用 ID |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 已执行 |  |
| 401 | 未认证 |  |
| 403 | 缺少 app_store:update 权限 |  |
| 404 | 已安装应用不存在 |  |

```bash
fpctl api POST '/api/app-store/installed/1/native-stop' --json
```

### POST `/api/app-store/installed/{id}/uninstall`

卸载应用（`{success: true}`）

- 认证：Bearer JWT
- 权限：`app_store:delete`
- operationId：`uninstall`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 已安装应用 ID |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 已卸载 |  |
| 401 | 未认证 |  |
| 403 | 缺少 app_store:delete 权限 |  |
| 404 | 已安装应用不存在 |  |

```bash
fpctl api POST '/api/app-store/installed/1/uninstall' --json
```

### POST `/api/app-store/installed/{id}/upgrade`

升级应用（拒绝降级；同版本幂等）。容器模式对 compose 做三方合并保留用户修改，
冲突时默认 409，可用 `conflict_strategy=local|upstream` 指定处理方式。

- 认证：Bearer JWT
- 权限：`app_store:update`
- operationId：`upgrade`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 已安装应用 ID |
| `target_version` | query | string | 否 | 目标版本，缺省 latest |
| `conflict_strategy` | query | string | 否 | compose 冲突处理：abort（默认，409）/ local（保留本地）/ upstream（采用新版本） |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 升级后的应用 | InstalledApp |
| 400 | 降级 / 新版本无 compose 模板 / 未知冲突策略 |  |
| 401 | 未认证 |  |
| 403 | 缺少 app_store:update 权限 |  |
| 404 | 已安装应用不存在 |  |
| 409 | 本地 compose 与新版本冲突（abort 策略） |  |

<details><summary>响应示例</summary>

```json
{
  "access_url": "string",
  "container_name": "string",
  "created_at": "2026-01-01T00:00:00Z",
  "id": 0,
  "install_path": "string",
  "launch_count": 0,
  "mode": "string",
  "name": "string",
  "package_key": "string",
  "params_json": "string",
  "port": 0,
  "status": "string",
  "updated_at": "2026-01-01T00:00:00Z",
  "version": "string"
}
```

</details>

```bash
fpctl api POST '/api/app-store/installed/1/upgrade' --json
```

### GET `/api/app-store/installed/{id}/upgrade-preview`

升级预览：渲染目标版本 compose 并与本地做三方合并，返回冲突 / 保留的本地修改 /
生效的上游修改与合并结果；不改动任何文件。

- 认证：Bearer JWT
- 权限：`app_store:read`
- operationId：`upgrade_preview`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 已安装应用 ID |
| `target_version` | query | string | 否 | 目标版本，缺省 latest |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 升级预览 | UpgradePreview |
| 400 | 降级 / compose 不是合法 YAML |  |
| 401 | 未认证 |  |
| 403 | 缺少 app_store:read 权限 |  |
| 404 | 已安装应用或版本不存在 |  |

<details><summary>响应示例</summary>

```json
{
  "base_available": false,
  "from_version": "string",
  "merge": {
    "conflicts": [],
    "local_changes": [
      "string"
    ],
    "merged": "string",
    "reformatted": false,
    "upstream_changes": [
      "string"
    ]
  },
  "mode": "string",
  "to_version": "string",
  "up_to_date": false
}
```

</details>

```bash
fpctl api GET '/api/app-store/installed/1/upgrade-preview' --json
```

### GET `/api/app-store/packages`

应用商店包列表

- 认证：Bearer JWT
- 权限：`app_store:read`
- operationId：`list_packages`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `category` | query | string | 否 | 分类过滤 |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 包列表 | AppStoreListResponse |
| 401 | 未认证 |  |

<details><summary>响应示例</summary>

```json
{
  "packages": [
    {
      "architectures": [
        "string"
      ],
      "category": "string",
      "default_version": "string",
      "format": "onepanel",
      "key": "string",
      "logo": "string",
      "min_memory_mb": 0,
      "modes": [],
      "name": "string",
      "readme": "string",
      "recommended": false,
      "short_desc_en": "string",
      "short_desc_zh": "string",
      "tags": [
        "string"
      ],
      "versions": [
        "string"
      ]
    }
  ],
  "total": 0
}
```

</details>

```bash
fpctl api GET '/api/app-store/packages' --json
```

### POST `/api/app-store/packages/batch-import`

批量导入本地目录应用包（Phase A2：`create_many` 事务语义，全成或全败）

- 认证：Bearer JWT
- 权限：`app_store:create`
- operationId：`batch_import_packages`

**请求体**（`application/json`，必填）：BatchImportPackagesRequest

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `paths` | string[] | 是 |  |

```json
{
  "paths": [
    "string"
  ]
}
```

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 批量导入结果 | BatchImportPackagesResponse |
| 400 | 非法路径 / 空列表 / 已存在 |  |
| 401 | 未认证 |  |

<details><summary>响应示例</summary>

```json
{
  "count": 0,
  "imported": [
    {
      "architectures": [
        "string"
      ],
      "category": "string",
      "default_version": "string",
      "format": "onepanel",
      "key": "string",
      "logo": "string",
      "min_memory_mb": 0,
      "modes": [],
      "name": "string",
      "readme": "string",
      "recommended": false,
      "short_desc_en": "string",
      "short_desc_zh": "string",
      "tags": [
        "string"
      ],
      "versions": [
        "string"
      ]
    }
  ]
}
```

</details>

```bash
fpctl api POST '/api/app-store/packages/batch-import' -d '{"paths":["string"]}' --json
```

### POST `/api/app-store/packages/import`

导入服务器本地目录为应用包（Flame / 1Panel / 宝塔格式自动识别；key 须为安全路径段，不跟随符号链接）

- 认证：Bearer JWT
- 权限：`app_store:create`
- operationId：`import_package`

**请求体**（`application/json`，必填）：ImportPackageRequest

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `path` | string | 是 |  |

```json
{
  "path": "string"
}
```

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 导入的元数据 | AppMetadata |
| 400 | 不是目录 / 格式无法识别 / key 非法 / 已存在 / 验签失败 / 源与目标目录互相包含 |  |
| 401 | 未认证 |  |
| 403 | 缺少 app_store:create 权限 |  |

<details><summary>响应示例</summary>

```json
{
  "architectures": [
    "string"
  ],
  "category": "string",
  "default_version": "string",
  "format": "onepanel",
  "key": "string",
  "logo": "string",
  "min_memory_mb": 0,
  "modes": [
    "container"
  ],
  "name": "string",
  "readme": "string",
  "recommended": false,
  "short_desc_en": "string",
  "short_desc_zh": "string",
  "tags": [
    "string"
  ],
  "versions": [
    "string"
  ]
}
```

</details>

```bash
fpctl api POST '/api/app-store/packages/import' -d '{"path":"string"}' --json
```

### GET `/api/app-store/packages/{key}`

应用包元数据

- 认证：Bearer JWT
- 权限：`app_store:read`
- operationId：`get_package`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `key` | path | string | 是 | 应用包 key |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 元数据 | AppMetadata |
| 401 | 未认证 |  |
| 403 | 缺少 app_store:read 权限 |  |
| 404 | 应用包不存在 |  |

<details><summary>响应示例</summary>

```json
{
  "architectures": [
    "string"
  ],
  "category": "string",
  "default_version": "string",
  "format": "onepanel",
  "key": "string",
  "logo": "string",
  "min_memory_mb": 0,
  "modes": [
    "container"
  ],
  "name": "string",
  "readme": "string",
  "recommended": false,
  "short_desc_en": "string",
  "short_desc_zh": "string",
  "tags": [
    "string"
  ],
  "versions": [
    "string"
  ]
}
```

</details>

```bash
fpctl api GET '/api/app-store/packages/1' --json
```

### POST `/api/app-store/packages/{key}/install`

安装应用（容器 / 原生 / WASM；高风险或含宿主机脚本需显式确认）

- 认证：Bearer JWT
- 权限：`app_store:create`
- operationId：`install`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `key` | path | string | 是 | 应用包 key |

**请求体**（`application/json`，必填）：InstallAppRequest

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `acknowledge_scripts` | boolean | 否 | 安装包含原生脚本时需显式确认执行第三方脚本。 |
| `confirm_risky` | boolean | 否 |  |
| `container_name` | string? | 否 |  |
| `mode` | string? | 否 |  |
| `name` | string? | 否 |  |
| `package_key` | string | 是 |  |
| `port` | integer(int32)? | 否 |  |
| `values` | map<string, string>? | 否 |  |
| `version` | string? | 否 |  |

```json
{
  "acknowledge_scripts": false,
  "confirm_risky": false,
  "container_name": "string",
  "mode": "string",
  "name": "string",
  "package_key": "string",
  "port": 0,
  "values": {},
  "version": "string"
}
```

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 已安装应用 | InstalledApp |
| 400 | package_key 与路径不一致 / 端口或变量非法 / 未确认风险或脚本 / 验签失败 |  |
| 401 | 未认证 |  |
| 403 | 缺少 app_store:create 权限 |  |
| 404 | 应用包不存在 |  |

<details><summary>响应示例</summary>

```json
{
  "access_url": "string",
  "container_name": "string",
  "created_at": "2026-01-01T00:00:00Z",
  "id": 0,
  "install_path": "string",
  "launch_count": 0,
  "mode": "string",
  "name": "string",
  "package_key": "string",
  "params_json": "string",
  "port": 0,
  "status": "string",
  "updated_at": "2026-01-01T00:00:00Z",
  "version": "string"
}
```

</details>

```bash
fpctl api POST '/api/app-store/packages/1/install' -d '{"acknowledge_scripts":false,"confirm_risky":false,"container_name":"string","mode":"string","name":"string","package_key":"string","port":0,"values":{},"version":"string"}' --json
```

### GET `/api/app-store/packages/{key}/versions/{version}`

指定版本详情（compose 模板、表单字段、钩子、原生安装参数）

- 认证：Bearer JWT
- 权限：`app_store:read`
- operationId：`list_versions`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `key` | path | string | 是 | 应用包 key |
| `version` | path | string | 是 | 版本号 |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 版本详情（AppVersionInfo） |  |
| 401 | 未认证 |  |
| 403 | 缺少 app_store:read 权限 |  |
| 404 | 应用包或版本不存在 |  |

```bash
fpctl api GET '/api/app-store/packages/1/versions/1' --json
```

### GET `/api/app-store/security`

商店安全配置（验签策略 + 镜像仓库白名单）

- 认证：Bearer JWT
- 权限：`app_store:read`
- operationId：`get_store_security`

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 商店安全配置 | StoreSecurityConfig |
| 401 | 未认证 |  |

<details><summary>响应示例</summary>

```json
{
  "builtin_registries": [
    "string"
  ],
  "effective_registries": [
    "string"
  ],
  "registry_whitelist": [
    "string"
  ],
  "signature_policy": "string",
  "unsigned_source_count": 0
}
```

</details>

```bash
fpctl api GET '/api/app-store/security' --json
```

### PUT `/api/app-store/security`

更新商店安全配置（验签策略 / 镜像仓库白名单）

- 认证：Bearer JWT
- 权限：`app_store:update`
- operationId：`update_store_security`

**请求体**（`application/json`，必填）：UpdateStoreSecurityRequest

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `registry_whitelist` | string[]? | 否 | 镜像仓库白名单追加项（全量替换；与内置白名单合并去重） |
| `signature_policy` | string? | 否 | 验签策略：optional（源配公钥才验签）\| required（远程包强制验签） |

```json
{
  "registry_whitelist": [
    "string"
  ],
  "signature_policy": "string"
}
```

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 更新后的商店安全配置 | StoreSecurityConfig |
| 400 | 无更新项 / 参数非法 |  |
| 401 | 未认证 |  |
| 503 | 设置服务未装配 |  |

<details><summary>响应示例</summary>

```json
{
  "builtin_registries": [
    "string"
  ],
  "effective_registries": [
    "string"
  ],
  "registry_whitelist": [
    "string"
  ],
  "signature_policy": "string",
  "unsigned_source_count": 0
}
```

</details>

```bash
fpctl api PUT '/api/app-store/security' -d '{"registry_whitelist":["string"],"signature_policy":"string"}' --json
```

### GET `/api/app-store/sources`

远程源列表

- 认证：Bearer JWT
- 权限：`app_store:read`
- operationId：`list_sources`

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 远程源列表 | SourcesResponse |
| 401 | 未认证 |  |

<details><summary>响应示例</summary>

```json
{
  "sources": [
    {
      "created_at": "2026-01-01T00:00:00Z",
      "enabled": false,
      "id": 0,
      "last_synced_at": "2026-01-01T00:00:00Z",
      "name": "string",
      "public_key": "string",
      "source_type": "string",
      "updated_at": "2026-01-01T00:00:00Z",
      "url": "string"
    }
  ],
  "total": 0
}
```

</details>

```bash
fpctl api GET '/api/app-store/sources' --json
```

### POST `/api/app-store/sources`

添加远程源

- 认证：Bearer JWT
- 权限：`app_store:create`
- operationId：`create_source`

**请求体**（`application/json`，必填）：CreateSourceRequest

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `confirm_unsigned` | boolean | 否 | 未配置公钥的源须显式确认（Doc/20 §4.2：无签名第三方源二次确认） |
| `name` | string | 是 |  |
| `public_key` | string? | 否 | Ed25519 公钥（可选，验签用） |
| `source_type` | string | 是 | git \| http |
| `url` | string | 是 |  |

```json
{
  "confirm_unsigned": false,
  "name": "string",
  "public_key": "string",
  "source_type": "string",
  "url": "string"
}
```

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 新建的远程源 | AppSource |
| 400 | 参数非法 / url 重复 |  |
| 401 | 未认证 |  |

<details><summary>响应示例</summary>

```json
{
  "created_at": "2026-01-01T00:00:00Z",
  "enabled": false,
  "id": 0,
  "last_synced_at": "2026-01-01T00:00:00Z",
  "name": "string",
  "public_key": "string",
  "source_type": "string",
  "updated_at": "2026-01-01T00:00:00Z",
  "url": "string"
}
```

</details>

```bash
fpctl api POST '/api/app-store/sources' -d '{"confirm_unsigned":false,"name":"string","public_key":"string","source_type":"string","url":"string"}' --json
```

### PUT `/api/app-store/sources/{id}`

更新远程源（name / url / enabled / public_key）

- 认证：Bearer JWT
- 权限：`app_store:update`
- operationId：`update_source`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 源 id |

**请求体**（`application/json`，必填）：UpdateSourceRequest

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `confirm_unsigned` | boolean | 否 | 清空公钥（签名源降级为未签名源）须显式确认 |
| `enabled` | boolean? | 否 |  |
| `name` | string? | 否 |  |
| `public_key` | string? | 否 |  |
| `url` | string? | 否 |  |

```json
{
  "confirm_unsigned": false,
  "enabled": false,
  "name": "string",
  "public_key": "string",
  "url": "string"
}
```

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 更新后的远程源 | AppSource |
| 401 | 未认证 |  |
| 404 | 源不存在 |  |

<details><summary>响应示例</summary>

```json
{
  "created_at": "2026-01-01T00:00:00Z",
  "enabled": false,
  "id": 0,
  "last_synced_at": "2026-01-01T00:00:00Z",
  "name": "string",
  "public_key": "string",
  "source_type": "string",
  "updated_at": "2026-01-01T00:00:00Z",
  "url": "string"
}
```

</details>

```bash
fpctl api PUT '/api/app-store/sources/1' -d '{"confirm_unsigned":false,"enabled":false,"name":"string","public_key":"string","url":"string"}' --json
```

### DELETE `/api/app-store/sources/{id}`

删除远程源

- 认证：Bearer JWT
- 权限：`app_store:delete`
- operationId：`delete_source`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 源 id |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 删除成功 |  |
| 401 | 未认证 |  |
| 404 | 源不存在 |  |

```bash
fpctl api DELETE '/api/app-store/sources/1' --json
```

### GET `/api/app-store/sources/{id}/packages`

按源过滤应用包（复用现有包列表，返回包元数据）

- 认证：Bearer JWT
- 权限：`app_store:read`
- operationId：`list_source_packages`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 源 id |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 该源下的应用包列表 | AppStoreListResponse |
| 401 | 未认证 |  |
| 404 | 源不存在 |  |

<details><summary>响应示例</summary>

```json
{
  "packages": [
    {
      "architectures": [
        "string"
      ],
      "category": "string",
      "default_version": "string",
      "format": "onepanel",
      "key": "string",
      "logo": "string",
      "min_memory_mb": 0,
      "modes": [],
      "name": "string",
      "readme": "string",
      "recommended": false,
      "short_desc_en": "string",
      "short_desc_zh": "string",
      "tags": [
        "string"
      ],
      "versions": [
        "string"
      ]
    }
  ],
  "total": 0
}
```

</details>

```bash
fpctl api GET '/api/app-store/sources/1/packages' --json
```

### POST `/api/app-store/sources/{id}/sync`

手动同步远程源索引（同步任务经统一任务中心跟踪）

- 认证：Bearer JWT
- 权限：`app_store:update`
- operationId：`sync_source`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 源 id |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 同步结果 | SyncSourceResponse |
| 401 | 未认证 |  |
| 404 | 源不存在 |  |
| 409 | 该源正在同步中 |  |

<details><summary>响应示例</summary>

```json
{
  "added": 0,
  "packages": [
    "string"
  ],
  "skipped": 0
}
```

</details>

```bash
fpctl api POST '/api/app-store/sources/1/sync' --json
```

### GET `/api/app-store/wasm-builtins`

内置 WASM 应用

- 认证：Bearer JWT
- 权限：`app_store:read`
- operationId：`list_wasm_builtins`

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 元数据列表 | AppMetadata[] |
| 401 | 未认证 |  |
| 403 | 缺少 app_store:read 权限 |  |

<details><summary>响应示例</summary>

```json
[
  {
    "architectures": [
      "string"
    ],
    "category": "string",
    "default_version": "string",
    "format": "onepanel",
    "key": "string",
    "logo": "string",
    "min_memory_mb": 0,
    "modes": [
      "container"
    ],
    "name": "string",
    "readme": "string",
    "recommended": false,
    "short_desc_en": "string",
    "short_desc_zh": "string",
    "tags": [
      "string"
    ],
    "versions": [
      "string"
    ]
  }
]
```

</details>

```bash
fpctl api GET '/api/app-store/wasm-builtins' --json
```
