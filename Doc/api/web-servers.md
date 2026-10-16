<!-- 由 scripts/gen-api-docs.py 从 frontend/openapi.json 生成，请勿手改；修改 utoipa 注解后运行 just api-docs -->

# API · web-servers

> 23 个端点 · 通用约定见 [README](./README.md)

| 方法 | 路径 | 摘要 | 权限 |
|---|---|---|---|
| GET | `/api/web-servers` | 面板登记的 Web 服务器实例（分页） | `web_server:read` |
| POST | `/api/web-servers` | 登记实例（不安装软件；安装见 native/install） | `web_server:create` |
| GET | `/api/web-servers/capabilities` | 查询引擎支持的配套能力（PHP-FPM / Lua / Redis / 扩展模块 / 自动 HTTPS）。 引擎可选配套能力（PHP-FPM / Lua / 缓存 / 扩展模块 / 自动 HTTPS）及推荐系统包 | `web_server:read` |
| POST | `/api/web-servers/capabilities/install` | 批量安装引擎的配套依赖。 批量安装配套能力 | `web_server:update` |
| GET | `/api/web-servers/engines` | 支持的 Web 引擎及能力 | `web_server:read` |
| POST | `/api/web-servers/native/autostart` | 按引擎设置 systemd 开机自启（未登记实例也可用） | `web_server:update` |
| GET | `/api/web-servers/native/detect` | 探测系统已安装的 Web 引擎（包管理器 / PATH / 候选路径 + systemd 状态 + 监听端口） | `web_server:read` |
| POST | `/api/web-servers/native/install` | 经系统包管理器安装引擎并登记实例 | `web_server:create` |
| POST | `/api/web-servers/native/uninstall` | 卸载系统引擎并删除对应实例登记 | `web_server:delete` |
| GET | `/api/web-servers/presets` | 性能预设列表（按本机资源标注推荐项） | `web_server:read` |
| GET | `/api/web-servers/{id}` | 单个实例 | `web_server:read` |
| PUT | `/api/web-servers/{id}` | 更新实例字段（`engine` / `version` / `config_path` / `binary_path` / `port`，可带 `resource_version` 做乐观并发） | `web_server:update` |
| DELETE | `/api/web-servers/{id}` | 删除实例登记（不卸载软件） | `web_server:delete` |
| POST | `/api/web-servers/{id}/autostart` | 设置实例对应 systemd 服务的开机自启 | `web_server:update` |
| GET | `/api/web-servers/{id}/config` | 读取实例主配置文件（仅限引擎配置根下；不存在时 content 为提示文本） | `web_server:read` |
| POST | `/api/web-servers/{id}/configtest` | 校验引擎配置（`nginx -t` 等） | `web_server:configtest` |
| GET | `/api/web-servers/{id}/native-status` | 实例对应引擎的原生状态 | `web_server:read` |
| POST | `/api/web-servers/{id}/preset` | 应用性能预设：生成全局配置 → 原子写 → 校验 → reload，失败回滚 | `web_server:update` |
| POST | `/api/web-servers/{id}/reload` | 平滑重载配置 | `web_server:reload` |
| POST | `/api/web-servers/{id}/restart` | 重启引擎 | `web_server:start` |
| POST | `/api/web-servers/{id}/start` | 启动引擎 | `web_server:start` |
| POST | `/api/web-servers/{id}/stop` | 停止引擎 | `web_server:stop` |
| POST | `/api/web-servers/{id}/switch-engine` | 切换实例引擎（按新引擎默认路径重写登记） | `web_server:update` |

### GET `/api/web-servers`

面板登记的 Web 服务器实例（分页）

- 认证：Bearer JWT
- 权限：`web_server:read`
- operationId：`list`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `page` | query | integer(int64)? | 否 | 页码（从 1 开始） |
| `page_size` | query | integer(int64)? | 否 | 每页条数（1~200） |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 实例分页 | PaginatedResponse<WebServerResponse>_WebServerResponse |
| 401 | 未认证 |  |
| 403 | 缺少 web_server:read 权限 |  |

<details><summary>响应示例</summary>

```json
{
  "data": [
    {
      "binary_path": "string",
      "config_path": "string",
      "created_at": "string",
      "engine": "string",
      "id": 0,
      "port": 0,
      "resource_version": 0,
      "status": "string",
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
fpctl api GET '/api/web-servers' --json
```

### POST `/api/web-servers`

登记实例（不安装软件；安装见 native/install）

- 认证：Bearer JWT
- 权限：`web_server:create`
- operationId：`create`

**请求体**（`application/json`，必填）：CreateWebServerInstanceRequest

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `binary_path` | string? | 否 | 缺省按发行版取默认；只接受该引擎的已知二进制（裸程序名或系统 bin 目录下同名文件） |
| `config_path` | string? | 否 | 缺省取引擎默认主配置；必须位于 Web 引擎配置根之下（`/etc/nginx`、`/etc/apache2`、`/etc/httpd`、`/etc/caddy`、`/usr/local/lsws/conf`、`/usr/local/openresty/nginx/conf`） |
| `engine` | string | 是 | `nginx` / `apache` / `openlitespeed` / `openresty` / `caddy` |
| `port` | integer(int32)? | 否 |  |
| `version` | string? | 否 |  |

```json
{
  "binary_path": "string",
  "config_path": "string",
  "engine": "string",
  "port": 0,
  "version": "string"
}
```

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 已登记 | WebServerResponse |
| 400 | 未知引擎 / config_path 不在引擎配置根下 / binary_path 不是该引擎的已知二进制 |  |
| 401 | 未认证 |  |
| 403 | 缺少 web_server:create 权限 |  |

<details><summary>响应示例</summary>

```json
{
  "binary_path": "string",
  "config_path": "string",
  "created_at": "string",
  "engine": "string",
  "id": 0,
  "port": 0,
  "resource_version": 0,
  "status": "string",
  "version": "string"
}
```

</details>

```bash
fpctl api POST '/api/web-servers' -d '{"binary_path":"string","config_path":"string","engine":"string","port":0,"version":"string"}' --json
```

### GET `/api/web-servers/capabilities`

查询引擎支持的配套能力（PHP-FPM / Lua / Redis / 扩展模块 / 自动 HTTPS）。
引擎可选配套能力（PHP-FPM / Lua / 缓存 / 扩展模块 / 自动 HTTPS）及推荐系统包

- 认证：Bearer JWT
- 权限：`web_server:read`
- operationId：`engine_capabilities`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `engine` | query | string | 是 |  |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | `{engine, capabilities: CapabilityInfo[]}` | object |
| 400 | 未知引擎 |  |
| 401 | 未认证 |  |
| 403 | 缺少 web_server:read 权限 |  |

```bash
fpctl api GET '/api/web-servers/capabilities' --json
```

### POST `/api/web-servers/capabilities/install`

批量安装引擎的配套依赖。
批量安装配套能力

- 认证：Bearer JWT
- 权限：`web_server:update`
- operationId：`install_capabilities`

**请求体**（`application/json`，必填）：InstallCapabilitiesRequest

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `capabilities` | string[] | 是 |  |
| `engine` | string | 是 |  |

```json
{
  "capabilities": [
    "string"
  ],
  "engine": "string"
}
```

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | `{success, installed[], failed[]}` | object |
| 400 | 未知引擎 / 未知能力（php / lua / cache / extra_modules / auto_https） |  |
| 401 | 未认证 |  |
| 403 | 缺少 web_server:update 权限 |  |

```bash
fpctl api POST '/api/web-servers/capabilities/install' -d '{"capabilities":["string"],"engine":"string"}' --json
```

### GET `/api/web-servers/engines`

支持的 Web 引擎及能力

- 认证：Bearer JWT
- 权限：`web_server:read`
- operationId：`list_engines`

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 引擎列表 | EngineInfo[] |
| 401 | 未认证 |  |
| 403 | 缺少 web_server:read 权限 |  |

<details><summary>响应示例</summary>

```json
[
  {
    "default_port": 0,
    "default_ssl_port": 0,
    "description": "string",
    "name": "string",
    "supports_load_balancing": false,
    "supports_reverse_proxy": false,
    "supports_rewrite": false,
    "supports_ssl": false
  }
]
```

</details>

```bash
fpctl api GET '/api/web-servers/engines' --json
```

### POST `/api/web-servers/native/autostart`

按引擎设置 systemd 开机自启（未登记实例也可用）

- 认证：Bearer JWT
- 权限：`web_server:update`
- operationId：`native_autostart`

**请求体**（`application/json`，必填）：NativeAutostartRequest

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `enabled` | boolean | 是 |  |
| `engine` | string | 是 |  |

```json
{
  "enabled": false,
  "engine": "string"
}
```

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | `{engine, autostart}` | object |
| 400 | 未知引擎 |  |
| 401 | 未认证 |  |
| 403 | 缺少 web_server:update 权限 |  |

```bash
fpctl api POST '/api/web-servers/native/autostart' -d '{"enabled":false,"engine":"string"}' --json
```

### GET `/api/web-servers/native/detect`

探测系统已安装的 Web 引擎（包管理器 / PATH / 候选路径 + systemd 状态 + 监听端口）

- 认证：Bearer JWT
- 权限：`web_server:read`
- operationId：`native_detect`

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 各引擎检测结果 | NativeWebServerInfo[] |
| 401 | 未认证 |  |
| 403 | 缺少 web_server:read 权限 |  |

<details><summary>响应示例</summary>

```json
[
  {
    "binary_path": "string",
    "config_path": "string",
    "default_port": 0,
    "description": "string",
    "enabled": false,
    "engine": "string",
    "installed": false,
    "listening_ports": [
      0
    ],
    "package_installed": false,
    "running": false,
    "service_name": "string",
    "version": "string"
  }
]
```

</details>

```bash
fpctl api GET '/api/web-servers/native/detect' --json
```

### POST `/api/web-servers/native/install`

经系统包管理器安装引擎并登记实例

- 认证：Bearer JWT
- 权限：`web_server:create`
- operationId：`native_install`

**请求体**（`application/json`，必填）：NativeInstallRequest

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `engine` | string | 是 |  |
| `version` | string? | 否 |  |

```json
{
  "engine": "string",
  "version": "string"
}
```

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 已安装并登记 | WebServerResponse |
| 400 | 未知引擎 |  |
| 401 | 未认证 |  |
| 403 | 缺少 web_server:create 权限 |  |
| 500 | 包管理器安装失败 |  |

<details><summary>响应示例</summary>

```json
{
  "binary_path": "string",
  "config_path": "string",
  "created_at": "string",
  "engine": "string",
  "id": 0,
  "port": 0,
  "resource_version": 0,
  "status": "string",
  "version": "string"
}
```

</details>

```bash
fpctl api POST '/api/web-servers/native/install' -d '{"engine":"string","version":"string"}' --json
```

### POST `/api/web-servers/native/uninstall`

卸载系统引擎并删除对应实例登记

- 认证：Bearer JWT
- 权限：`web_server:delete`
- operationId：`native_uninstall`

**请求体**（`application/json`，必填）：NativeUninstallRequest

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `engine` | string | 是 |  |

```json
{
  "engine": "string"
}
```

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | `{deleted: true, engine}` | object |
| 400 | 未知引擎 |  |
| 401 | 未认证 |  |
| 403 | 缺少 web_server:delete 权限 |  |

```bash
fpctl api POST '/api/web-servers/native/uninstall' -d '{"engine":"string"}' --json
```

### GET `/api/web-servers/presets`

性能预设列表（按本机资源标注推荐项）

- 认证：Bearer JWT
- 权限：`web_server:read`
- operationId：`list_presets`

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | `[{name, description, recommended, worker_processes}]` | object[] |
| 401 | 未认证 |  |
| 403 | 缺少 web_server:read 权限 |  |

<details><summary>响应示例</summary>

```json
[
  {}
]
```

</details>

```bash
fpctl api GET '/api/web-servers/presets' --json
```

### GET `/api/web-servers/{id}`

单个实例

- 认证：Bearer JWT
- 权限：`web_server:read`
- operationId：`get`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 实例 ID |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 实例 | WebServerResponse |
| 401 | 未认证 |  |
| 403 | 缺少 web_server:read 权限 |  |
| 404 | 实例不存在 |  |

<details><summary>响应示例</summary>

```json
{
  "binary_path": "string",
  "config_path": "string",
  "created_at": "string",
  "engine": "string",
  "id": 0,
  "port": 0,
  "resource_version": 0,
  "status": "string",
  "version": "string"
}
```

</details>

```bash
fpctl api GET '/api/web-servers/1' --json
```

### PUT `/api/web-servers/{id}`

更新实例字段（`engine` / `version` / `config_path` / `binary_path` / `port`，可带 `resource_version` 做乐观并发）

- 认证：Bearer JWT
- 权限：`web_server:update`
- operationId：`update`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 实例 ID |

**请求体**（`application/json`，必填）：object

```json
{}
```

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 已更新 | WebServerResponse |
| 400 | 未知引擎 / config_path 不在引擎配置根下 / binary_path 不是该引擎的已知二进制 |  |
| 401 | 未认证 |  |
| 403 | 缺少 web_server:update 权限 |  |
| 404 | 实例不存在 |  |
| 409 | resource_version 冲突 |  |

<details><summary>响应示例</summary>

```json
{
  "binary_path": "string",
  "config_path": "string",
  "created_at": "string",
  "engine": "string",
  "id": 0,
  "port": 0,
  "resource_version": 0,
  "status": "string",
  "version": "string"
}
```

</details>

```bash
fpctl api PUT '/api/web-servers/1' -d '{}' --json
```

### DELETE `/api/web-servers/{id}`

删除实例登记（不卸载软件）

- 认证：Bearer JWT
- 权限：`web_server:delete`
- operationId：`delete`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 实例 ID |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | `{deleted: true, id}` | object |
| 401 | 未认证 |  |
| 403 | 缺少 web_server:delete 权限 |  |
| 404 | 实例不存在 |  |

```bash
fpctl api DELETE '/api/web-servers/1' --json
```

### POST `/api/web-servers/{id}/autostart`

设置实例对应 systemd 服务的开机自启

- 认证：Bearer JWT
- 权限：`web_server:update`
- operationId：`set_autostart`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 实例 ID |

**请求体**（`application/json`，必填）：AutostartRequest

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `enabled` | boolean | 是 |  |

```json
{
  "enabled": false
}
```

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | `{id, engine, autostart}` | object |
| 401 | 未认证 |  |
| 403 | 缺少 web_server:update 权限 |  |
| 404 | 实例不存在 |  |

```bash
fpctl api POST '/api/web-servers/1/autostart' -d '{"enabled":false}' --json
```

### GET `/api/web-servers/{id}/config`

读取实例主配置文件（仅限引擎配置根下；不存在时 content 为提示文本）

- 认证：Bearer JWT
- 权限：`web_server:read`
- operationId：`get_config`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 实例 ID |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | `{id, config_path, content}` | object |
| 401 | 未认证 |  |
| 403 | 缺少 web_server:read 权限 |  |
| 404 | 实例不存在 |  |

```bash
fpctl api GET '/api/web-servers/1/config' --json
```

### POST `/api/web-servers/{id}/configtest`

校验引擎配置（`nginx -t` 等）

- 认证：Bearer JWT
- 权限：`web_server:configtest`
- operationId：`config_test`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 实例 ID |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | `{output, id}` | object |
| 401 | 未认证 |  |
| 403 | 缺少 web_server:configtest 权限 |  |
| 404 | 实例不存在 |  |

```bash
fpctl api POST '/api/web-servers/1/configtest' --json
```

### GET `/api/web-servers/{id}/native-status`

实例对应引擎的原生状态

- 认证：Bearer JWT
- 权限：`web_server:read`
- operationId：`native_status`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 实例 ID |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 检测结果 | NativeWebServerInfo |
| 401 | 未认证 |  |
| 403 | 缺少 web_server:read 权限 |  |
| 404 | 实例不存在 |  |

<details><summary>响应示例</summary>

```json
{
  "binary_path": "string",
  "config_path": "string",
  "default_port": 0,
  "description": "string",
  "enabled": false,
  "engine": "string",
  "installed": false,
  "listening_ports": [
    0
  ],
  "package_installed": false,
  "running": false,
  "service_name": "string",
  "version": "string"
}
```

</details>

```bash
fpctl api GET '/api/web-servers/1/native-status' --json
```

### POST `/api/web-servers/{id}/preset`

应用性能预设：生成全局配置 → 原子写 → 校验 → reload，失败回滚

- 认证：Bearer JWT
- 权限：`web_server:update`
- operationId：`apply_preset`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 实例 ID |

**请求体**（`application/json`，必填）：ApplyPresetRequest

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `preset` | string | 是 |  |

```json
{
  "preset": "string"
}
```

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | `{id, engine, preset, worker_processes, keepalive_timeout}` | object |
| 400 | 未知预设 / 配置校验失败 |  |
| 401 | 未认证 |  |
| 403 | 缺少 web_server:update 权限 |  |
| 404 | 实例不存在 |  |

```bash
fpctl api POST '/api/web-servers/1/preset' -d '{"preset":"string"}' --json
```

### POST `/api/web-servers/{id}/reload`

平滑重载配置

- 认证：Bearer JWT
- 权限：`web_server:reload`
- operationId：`reload`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 实例 ID |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | `{message, id}` | object |
| 401 | 未认证 |  |
| 403 | 缺少 web_server:reload 权限 |  |
| 404 | 实例不存在 |  |
| 500 | 引擎命令执行失败 |  |

```bash
fpctl api POST '/api/web-servers/1/reload' --json
```

### POST `/api/web-servers/{id}/restart`

重启引擎

- 认证：Bearer JWT
- 权限：`web_server:start`
- operationId：`restart`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 实例 ID |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | `{message, id}` | object |
| 401 | 未认证 |  |
| 403 | 缺少 web_server:start 权限 |  |
| 404 | 实例不存在 |  |
| 500 | 引擎命令执行失败 |  |

```bash
fpctl api POST '/api/web-servers/1/restart' --json
```

### POST `/api/web-servers/{id}/start`

启动引擎

- 认证：Bearer JWT
- 权限：`web_server:start`
- operationId：`start`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 实例 ID |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | `{message, id}` | object |
| 401 | 未认证 |  |
| 403 | 缺少 web_server:start 权限 |  |
| 404 | 实例不存在 |  |
| 500 | 引擎命令执行失败 |  |

```bash
fpctl api POST '/api/web-servers/1/start' --json
```

### POST `/api/web-servers/{id}/stop`

停止引擎

- 认证：Bearer JWT
- 权限：`web_server:stop`
- operationId：`stop`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 实例 ID |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | `{message, id}` | object |
| 401 | 未认证 |  |
| 403 | 缺少 web_server:stop 权限 |  |
| 404 | 实例不存在 |  |
| 500 | 引擎命令执行失败 |  |

```bash
fpctl api POST '/api/web-servers/1/stop' --json
```

### POST `/api/web-servers/{id}/switch-engine`

切换实例引擎（按新引擎默认路径重写登记）

- 认证：Bearer JWT
- 权限：`web_server:update`
- operationId：`switch_engine`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 实例 ID |

**请求体**（`application/json`，必填）：SwitchEngineRequest

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `engine` | string | 是 |  |

```json
{
  "engine": "string"
}
```

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 切换后的实例 | WebServerInstance |
| 400 | 未知引擎 |  |
| 401 | 未认证 |  |
| 403 | 缺少 web_server:update 权限 |  |
| 404 | 实例不存在 |  |

<details><summary>响应示例</summary>

```json
{
  "binary_path": "string",
  "config_path": "string",
  "created_at": "2026-01-01T00:00:00Z",
  "engine": "string",
  "id": 0,
  "port": 0,
  "resource_version": 0,
  "status": "string",
  "version": "string"
}
```

</details>

```bash
fpctl api POST '/api/web-servers/1/switch-engine' -d '{"engine":"string"}' --json
```
