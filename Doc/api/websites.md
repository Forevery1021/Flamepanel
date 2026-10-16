<!-- 由 scripts/gen-api-docs.py 从 frontend/openapi.json 生成，请勿手改；修改 utoipa 注解后运行 just api-docs -->

# API · websites

> 11 个端点 · 通用约定见 [README](./README.md)

| 方法 | 路径 | 摘要 | 权限 |
|---|---|---|---|
| GET | `/api/websites` | 网站列表（分页） | `website:read` |
| POST | `/api/websites` | 创建网站 | `website:create` |
| GET | `/api/websites/{id}` | 获取网站详情 | `website:read` |
| PUT | `/api/websites/{id}` | 更新网站 | `website:update` |
| DELETE | `/api/websites/{id}` | 删除网站 | `website:delete` |
| POST | `/api/websites/{id}/deploy` | 手动部署站点配置（Doc/20 §5.5 站点配置流水线）：按当前库中状态渲染完整配置 （含已绑定证书的 HTTPS），写入面板自有站点目录 → 配置校验 → reload，失败整体回滚。 | `website:update` |
| GET | `/api/websites/{id}/rewrite` | 查询站点伪静态套用状态（当前模板 + 该引擎语法规则文本 + 可选模板清单） | `website:read` |
| POST | `/api/websites/{id}/rewrite` | 一键套用伪静态规则模板——落库 + 写 vhost + 校验 + reload（「套用即生效」）。 | `website:update` |
| DELETE | `/api/websites/{id}/rewrite` | 清除站点伪静态套用（回落引擎默认路由；同样落库 + 写 vhost）。 | `website:update` |
| POST | `/api/websites/{id}/runtime` | 绑定/解绑运行时（解绑传 site_type=static 且不传 id） | `website:update` |
| POST | `/api/websites/{id}/switch-engine` | 切换网站的 Web 服务器引擎（重写配置并重载） | `website:update` |

### GET `/api/websites`

网站列表（分页）

- 认证：Bearer JWT
- 权限：`website:read`
- operationId：`list_websites`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `page` | query | integer(int64)? | 否 | 页码（从 1 开始） |
| `page_size` | query | integer(int64)? | 否 | 每页条数（1~200） |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 网站列表 | PaginatedResponse<Website>_Website |
| 401 | 未认证 |  |

<details><summary>响应示例</summary>

```json
{
  "data": [
    {
      "created_at": "2026-01-01T00:00:00Z",
      "domain": "string",
      "engine": "string",
      "id": 0,
      "name": "string",
      "node_id": 0,
      "project_id": 0,
      "proxy_enabled": false,
      "proxy_pass": "string",
      "resource_version": 0,
      "rewrite_preset": "string",
      "root_path": "string",
      "runtime_id": 0,
      "site_type": "string",
      "ssl_enabled": false,
      "status": "string"
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
fpctl api GET '/api/websites' --json
```

### POST `/api/websites`

创建网站

- 认证：Bearer JWT
- 权限：`website:create`
- operationId：`create_website`

**请求体**（`application/json`，必填）：CreateWebsiteRequest

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `website` | Website | 是 |  |

```json
{
  "website": {
    "created_at": "2026-01-01T00:00:00Z",
    "domain": "string",
    "engine": "string",
    "id": 0,
    "name": "string",
    "node_id": 0,
    "project_id": 0,
    "proxy_enabled": false,
    "proxy_pass": "string",
    "resource_version": 0,
    "rewrite_preset": "string",
    "root_path": "string",
    "runtime_id": 0,
    "site_type": "string",
    "ssl_enabled": false,
    "status": "string"
  }
}
```

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 创建成功，返回网站 ID | integer(int64) (`text/plain`) |
| 400 | 参数错误 |  |

```bash
fpctl api POST '/api/websites' -d '{"website":{"created_at":"2026-01-01T00:00:00Z","domain":"string","engine":"string","id":0,"name":"string","node_id":0,"project_id":0,"proxy_enabled":false,"proxy_pass":"string","resource_version":0,"rewrite_preset":"string","root_path":"string","runtime_id":0,"site_type":"string","ssl_enabled":false,"status":"string"}}' --json
```

### GET `/api/websites/{id}`

获取网站详情

- 认证：Bearer JWT
- 权限：`website:read`
- operationId：`get_website`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 网站 ID |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 网站详情 | Website |
| 404 | 网站不存在 |  |

<details><summary>响应示例</summary>

```json
{
  "created_at": "2026-01-01T00:00:00Z",
  "domain": "string",
  "engine": "string",
  "id": 0,
  "name": "string",
  "node_id": 0,
  "project_id": 0,
  "proxy_enabled": false,
  "proxy_pass": "string",
  "resource_version": 0,
  "rewrite_preset": "string",
  "root_path": "string",
  "runtime_id": 0,
  "site_type": "string",
  "ssl_enabled": false,
  "status": "string"
}
```

</details>

```bash
fpctl api GET '/api/websites/1' --json
```

### PUT `/api/websites/{id}`

更新网站

- 认证：Bearer JWT
- 权限：`website:update`
- operationId：`update_website`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 网站 ID |

**请求体**（`application/json`，必填）：CreateWebsiteRequest

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `website` | Website | 是 |  |

```json
{
  "website": {
    "created_at": "2026-01-01T00:00:00Z",
    "domain": "string",
    "engine": "string",
    "id": 0,
    "name": "string",
    "node_id": 0,
    "project_id": 0,
    "proxy_enabled": false,
    "proxy_pass": "string",
    "resource_version": 0,
    "rewrite_preset": "string",
    "root_path": "string",
    "runtime_id": 0,
    "site_type": "string",
    "ssl_enabled": false,
    "status": "string"
  }
}
```

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 更新成功 | Website |
| 404 | 网站不存在 |  |

<details><summary>响应示例</summary>

```json
{
  "created_at": "2026-01-01T00:00:00Z",
  "domain": "string",
  "engine": "string",
  "id": 0,
  "name": "string",
  "node_id": 0,
  "project_id": 0,
  "proxy_enabled": false,
  "proxy_pass": "string",
  "resource_version": 0,
  "rewrite_preset": "string",
  "root_path": "string",
  "runtime_id": 0,
  "site_type": "string",
  "ssl_enabled": false,
  "status": "string"
}
```

</details>

```bash
fpctl api PUT '/api/websites/1' -d '{"website":{"created_at":"2026-01-01T00:00:00Z","domain":"string","engine":"string","id":0,"name":"string","node_id":0,"project_id":0,"proxy_enabled":false,"proxy_pass":"string","resource_version":0,"rewrite_preset":"string","root_path":"string","runtime_id":0,"site_type":"string","ssl_enabled":false,"status":"string"}}' --json
```

### DELETE `/api/websites/{id}`

删除网站

- 认证：Bearer JWT
- 权限：`website:delete`
- operationId：`delete_website`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 网站 ID |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 删除成功 |  |
| 404 | 网站不存在 |  |

```bash
fpctl api DELETE '/api/websites/1' --json
```

### POST `/api/websites/{id}/deploy`

手动部署站点配置（Doc/20 §5.5 站点配置流水线）：按当前库中状态渲染完整配置
（含已绑定证书的 HTTPS），写入面板自有站点目录 → 配置校验 → reload，失败整体回滚。

严格模式：引擎未安装 / 配置校验失败返回 400，同名非托管 OLS 虚拟主机返回 409。

- 认证：Bearer JWT
- 权限：`website:update`
- operationId：`deploy`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 网站 ID |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 部署结果 | SiteDeployment |
| 400 | 引擎未安装或配置校验失败 |  |
| 401 | 未认证 |  |
| 403 | 无权限 |  |
| 404 | 网站不存在 |  |
| 409 | 存在同名非托管虚拟主机 |  |

<details><summary>响应示例</summary>

```json
{
  "enabled": false,
  "engine": "string",
  "path": "string",
  "reloaded": false
}
```

</details>

```bash
fpctl api POST '/api/websites/1/deploy' --json
```

### GET `/api/websites/{id}/rewrite`

查询站点伪静态套用状态（当前模板 + 该引擎语法规则文本 + 可选模板清单）

- 认证：Bearer JWT
- 权限：`website:read`
- operationId：`get_website_rewrite`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 网站 ID |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 套用状态 | SiteRewriteStatus |
| 404 | 网站不存在 |  |

<details><summary>响应示例</summary>

```json
{
  "preset": "string",
  "rules": "string",
  "supported": [
    "string"
  ]
}
```

</details>

```bash
fpctl api GET '/api/websites/1/rewrite' --json
```

### POST `/api/websites/{id}/rewrite`

一键套用伪静态规则模板——落库 + 写 vhost + 校验 + reload（「套用即生效」）。

响应保持既有契约（生成的规则文本，按站点引擎语法）；写盘失败返回错误并回滚套用。

- 认证：Bearer JWT
- 权限：`website:update`
- operationId：`apply_website_rewrite`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 网站 ID |

**请求体**（`application/json`，必填）：ApplyRewriteRequest

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `preset` | string | 是 | 模板名：wordpress \| discuz \| typecho \| laravel \| thinkphp \| springboot |

```json
{
  "preset": "string"
}
```

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 套用成功，返回规则文本 | string (`text/plain`) |
| 400 | 未知模板 |  |
| 404 | 网站不存在 |  |
| 500 | vhost 写盘/校验/reload 失败（套用已回滚） |  |

```bash
fpctl api POST '/api/websites/1/rewrite' -d '{"preset":"string"}' --json
```

### DELETE `/api/websites/{id}/rewrite`

清除站点伪静态套用（回落引擎默认路由；同样落库 + 写 vhost）。

- 认证：Bearer JWT
- 权限：`website:update`
- operationId：`clear_website_rewrite`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 网站 ID |

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 已清除 | SiteRewriteStatus |
| 404 | 网站不存在 |  |
| 500 | vhost 写盘/校验/reload 失败（清除已回滚） |  |

<details><summary>响应示例</summary>

```json
{
  "preset": "string",
  "rules": "string",
  "supported": [
    "string"
  ]
}
```

</details>

```bash
fpctl api DELETE '/api/websites/1/rewrite' --json
```

### POST `/api/websites/{id}/runtime`

绑定/解绑运行时（解绑传 site_type=static 且不传 id）

- 认证：Bearer JWT
- 权限：`website:update`
- operationId：`bind_website_runtime`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 网站 ID |

**请求体**（`application/json`，必填）：BindRuntimeRequest

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `project_id` | integer(int64)? | 否 | 绑定的项目 ID（reverse_proxy + 项目场景） |
| `runtime_id` | integer(int64)? | 否 | 绑定的运行时 ID（runtime_php 场景） |
| `site_type` | string | 是 | 站点类型：runtime_php \| reverse_proxy \| static \| container_app |

```json
{
  "project_id": 0,
  "runtime_id": 0,
  "site_type": "string"
}
```

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 绑定成功 | Website |
| 400 | 参数错误/运行时或项目校验失败 |  |
| 404 | 网站不存在 |  |

<details><summary>响应示例</summary>

```json
{
  "created_at": "2026-01-01T00:00:00Z",
  "domain": "string",
  "engine": "string",
  "id": 0,
  "name": "string",
  "node_id": 0,
  "project_id": 0,
  "proxy_enabled": false,
  "proxy_pass": "string",
  "resource_version": 0,
  "rewrite_preset": "string",
  "root_path": "string",
  "runtime_id": 0,
  "site_type": "string",
  "ssl_enabled": false,
  "status": "string"
}
```

</details>

```bash
fpctl api POST '/api/websites/1/runtime' -d '{"project_id":0,"runtime_id":0,"site_type":"string"}' --json
```

### POST `/api/websites/{id}/switch-engine`

切换网站的 Web 服务器引擎（重写配置并重载）

- 认证：Bearer JWT
- 权限：`website:update`
- operationId：`switch_engine`

| 参数 | 位置 | 类型 | 必填 | 说明 |
|---|---|---|---|---|
| `id` | path | integer(int64) | 是 | 网站 ID |

**请求体**（`application/json`，必填）：SwitchWebsiteEngineRequest

| 字段 | 类型 | 必填 | 说明 |
|---|---|---|---|
| `engine` | string | 是 | 目标引擎：`nginx` \| `openresty` \| `caddy` \| `apache` \| `openlitespeed` |

```json
{
  "engine": "string"
}
```

| 状态 | 说明 | 响应体 |
|---|---|---|
| 200 | 切换后的网站 | Website |
| 400 | 目标引擎不可用 |  |
| 401 | 未认证 |  |
| 403 | 无权限 |  |
| 404 | 网站不存在 |  |

<details><summary>响应示例</summary>

```json
{
  "created_at": "2026-01-01T00:00:00Z",
  "domain": "string",
  "engine": "string",
  "id": 0,
  "name": "string",
  "node_id": 0,
  "project_id": 0,
  "proxy_enabled": false,
  "proxy_pass": "string",
  "resource_version": 0,
  "rewrite_preset": "string",
  "root_path": "string",
  "runtime_id": 0,
  "site_type": "string",
  "ssl_enabled": false,
  "status": "string"
}
```

</details>

```bash
fpctl api POST '/api/websites/1/switch-engine' -d '{"engine":"string"}' --json
```
