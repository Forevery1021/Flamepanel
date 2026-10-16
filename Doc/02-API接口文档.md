# 02 · API 接口文档

> FlamePanel 全部 REST / WebSocket 接口说明，含认证、错误码、分页与请求/响应示例

## 1. 约定

### 1.1 基础信息

| 项 | 值 |
|----|----|
| Base URL | `http://<host>:<port>/api`（生产经 Nginx，端口 80） |
| 数据格式 | JSON（`Content-Type: application/json`） |
| 认证方式 | `Authorization: Bearer <JWT>` |
| WebSocket | `ws://<host>/ws/metrics`、`/ws/logs`、`/ws/terminal` |

### 1.2 认证流程

```mermaid
sequenceDiagram
    participant C as 客户端
    participant S as 服务端
    C->>S: POST /api/auth/login {username, password}
    S-->>C: 200 {token, username, role} + Set-Cookie: flame_refresh=<jwt>; HttpOnly
    Note over C: Access token 存 localStorage；refresh token 由浏览器自动管理（httpOnly Cookie）
    C->>S: 后续请求带 Authorization: Bearer <token>
    S-->>C: 401 表示 token 无效/过期（前端自动调用 /auth/refresh 重放）
```

### 1.3 统一错误格式

所有错误（含中间件、404、JSON 解析失败）均返回：

```json
{
  "code": 400,
  "error": "BAD_REQUEST",
  "message": "Bad request: Invalid JSON body: ..."
}
```

### 1.4 错误码对照表

| `error` 错误码 | HTTP | 含义 |
|----------------|------|------|
| `AUTH_UNAUTHORIZED` | 401 | 未登录 / 令牌无效 / 令牌过期 |
| `AUTH_FORBIDDEN` | 403 | 无操作权限（RBAC 拒绝） |
| `PASSWORD_CHANGE_REQUIRED` | 403 | 需先修改初始密码（新装面板首次登录） |
| `TWO_FACTOR_SETUP_REQUIRED` | 403 | 面板强制两步验证（设置 `two_factor_enabled=true`）而当前用户尚未绑定；仅 `/api/auth/2fa*`、`me`、`refresh`、`logout`、`change-password` 放行 |
| `NOT_FOUND` | 404 | 资源或路由不存在 |
| `BAD_REQUEST` | 400 | 参数错误 / JSON 解析失败 |
| `VALIDATION_ERROR` | 400 | 业务校验失败 |
| `CONFLICT` | 409 | 资源冲突（用户名重复、端口占用） |
| `RATE_LIMITED` | 429 | 请求频率超限（分级限流：Login 5/min、Api 120/min、Health 600/min） |
| `SERVICE_UNAVAILABLE` | 503 | 依赖服务不可用 |
| `INTERNAL_ERROR` | 500 | 内部错误（完整错误链仅记日志） |

### 1.5 分页

所有列表接口支持统一分页参数：

| 参数 | 类型 | 默认 | 说明 |
|------|------|------|------|
| `page` | int | 1 | 页码（从 1 开始） |
| `page_size` | int | 20 | 每页条数（最大 200） |

响应格式：

```json
{
  "data": [],
  "page": 1,
  "page_size": 20,
  "total": 0,
  "total_pages": 0
}
```

### 1.6 字段裁剪 `?fields=`

任何返回 JSON 的 `GET /api/...` 接口都可附加 `fields=a,b,c`，只返回所列字段（通用中间件
`api/field_selection.rs`，处理器无需改动）：

- 响应是**数组** → 每个对象元素只保留所列字段；
- 响应是**对象**且含 `items` / `data` / `list` / `entries` / `records` 数组（分页信封）→ 只裁剪这些数组的元素，`total` / `page` 等元数据原样保留；
- 其它对象（单资源详情）→ 裁剪顶层字段。

约束：字段名 `[A-Za-z0-9_]{1,64}`、最多 32 个、可重复传参（`fields=a&fields=b`）；非法 → 400。只裁剪
2xx JSON 响应（≤ 16 MiB），已裁剪的响应带 `X-Fields-Applied: 1` 并去掉 `ETag`。鉴权在裁剪之前执行，
401 / 403 不受影响。例：`GET /api/toolbox/processes?fields=pid,name,cpu`。

## 2. 健康检查

### `GET /health`

无需认证。返回 `OK`（纯文本）。

**响应**：`200 OK`

### `GET /api/health`

无需认证。详细健康检查（依赖探测 + 版本 + 运行时长）：

```json
{
  "status": "ok",
  "version": "0.1.0",
  "uptime_secs": 86400,
  "checks": {
    "database": { "status": "ok", "detail": null },
    "docker": { "status": "ok", "detail": "5 containers" },
    "disk": { "status": "ok", "detail": "10240 MB free" }
  }
}
```

> `status` 为 `ok`/`degraded`；`docker` 不可用时为 `degraded`（面板仍可用）；`disk` 目录不存在时为 `unknown`（不判失败）。

## 3. 认证模块 `/api/auth`

### `POST /api/auth/login`

用户登录，获取 JWT。

**请求体**：

```json
{ "username": "admin", "password": "admin123" }
```

**响应 200**：

```json
{
  "token": "eyJhbGciOiJIUzI1NiIs...",
  "username": "admin",
  "role": "admin",
  "must_change_password": true
}
```

> `must_change_password=true` 表示首次登录需先改密（改密前除 `/api/auth/*` 外一律 `403 PASSWORD_CHANGE_REQUIRED`）。

**错误**：`401 AUTH_UNAUTHORIZED`（凭据错误）、`403 AUTH_FORBIDDEN`（登录失败锁定，5 次/5 分钟）

**两步验证（TOTP）**：用户已启用 2FA 时，密码通过后**不签发会话**，响应为：

```json
{ "token": "", "refresh_token": "", "username": "alice", "role": "operator",
  "must_change_password": false, "two_factor_required": true, "challenge": "<5 分钟挑战 JWT>" }
```

此时不下发 refresh Cookie，登录失败计数也不清零（第二步通过才清零）。另有 `must_setup_two_factor=true` 表示面板强制 2FA 而该用户尚未绑定，前端登录后跳转 `/two-factor-setup`。

### `POST /api/auth/login/2fa`

登录第二步（公开端点）。请求体 `{ "challenge": "<第一步返回>", "code": "123456" }`，`code` 可为 6 位 TOTP 验证码或一次性恢复码（`XXXXX-XXXXX`，大小写 / 连字符不敏感）。成功响应与 `login` 相同（签发 access token + refresh Cookie）。

- 挑战令牌类型为 `mfa`（与 access / refresh 互不通用），5 分钟有效；签发后若用户改过密码即失效。
- 错误验证码计入与密码登录相同的「用户名 + IP」锁定计数；TOTP 命中步号原子推进（同一验证码不能用两次，含启用时用过的那一个），恢复码原子消费。
- 审计：`LOGIN_2FA_CHALLENGE` / `LOGIN_2FA_FAILED` / `LOGIN_2FA_RECOVERY_CODE_USED` / `LOGIN_SUCCESS`。

**错误**：`401`（挑战无效 / 过期 / 验证码错误）、`429`（锁定）

### 两步验证自助管理 `/api/auth/2fa`

均只需登录（作用于调用者本人，不做资源级 RBAC）。密码 / 验证码错误返回 **400**（而非 401，避免前端误判会话失效）并计入登录锁定计数。

| 方法 | 路径 | 请求体 | 说明 |
|------|------|--------|------|
| GET | `/api/auth/2fa` | — | `{enabled, pending, recovery_codes_remaining, enabled_at, required}`；`required` 即面板是否强制 |
| POST | `/api/auth/2fa/setup` | `{password}` | 生成待绑定密钥：`{secret, otpauth_uri}`。已启用时旧密钥在新密钥验证通过前继续有效（更换认证器） |
| POST | `/api/auth/2fa/enable` | `{code}` | 用待绑定密钥的验证码确认；返回 `{recovery_codes: [10 个]}`，**明文仅此一次** |
| POST | `/api/auth/2fa/disable` | `{password, code}` | `code` 可为验证码或恢复码；面板强制时 `409` |
| POST | `/api/auth/2fa/recovery-codes` | `{password, code}` | 重新生成恢复码（`code` 必须是 TOTP），旧码全部作废 |

参数：SHA-1 / 6 位 / 30 秒步长，容忍前后各 1 步时钟偏差。密钥以 `SecretCipher`（`enc:v1:`）加密落库（表 `user_two_factor`，Doc/03），恢复码只存 SHA-256。管理员重置见 §4 `PUT /api/users/{id}/2fa/reset`；离线兜底 `fpctl reset-2fa`（Doc/10 §7）。

### `POST /api/auth/refresh`

刷新 JWT（滑动过期：剩余寿命 <12h 时重置为 24h，否则原样返回）。前端 401 自动调用并重放原请求。

**请求头**：无特殊要求——refresh token 通过 httpOnly Cookie `flame_refresh` 自动携带（兼容历史 Bearer 方式）。

**响应 200**：同 login（`token`/`username`/`role`/`must_change_password`），新的 refresh token 经 Set-Cookie 下发

**错误**：`401 AUTH_UNAUTHORIZED`（token 无效/缺失）

### `GET /api/auth/me`

获取当前登录用户信息（前端刷新页面恢复身份）。

**响应 200**：

```json
{ "id": 1, "username": "admin", "role": "admin", "must_change_password": false }
```

### `POST /api/auth/change-password`

修改当前用户密码（需登录）。修改成功后自动清除 `must_change_password` 标志。

**请求体**：

```json
{ "old_password": "admin123", "new_password": "NewP@ss123" }
```

**响应 200**：`{}`

## 4. 用户模块 `/api/users`

权限前缀：`user:*`

| 方法 | 路径 | 权限 | 说明 |
|------|------|------|------|
| GET | `/api/users` | user:read | 用户列表（分页） |
| POST | `/api/users` | user:create | 创建用户 |
| PUT | `/api/users/:id` | user:update | 更新用户 |
| DELETE | `/api/users/:id` | user:delete | 删除用户（同时清除其两步验证记录） |
| PUT | `/api/users/:id/2fa/reset` | user:update **且 admin 角色** | 重置该用户的两步验证（丢失认证器且恢复码用尽时）；响应 `{reset: bool}`，审计 `2FA_RESET`。operator 在种子里也有 `user:update`，但不能摘除第二因子 |

**POST /api/users 请求体**：

```json
{
  "username": "ops",
  "password_hash": "<bcrypt hash>",
  "role": "operator"
}
```

> 注意：`password_hash` 需为 bcrypt 加密后的哈希值（可在服务端用 bcrypt 生成）。

**PUT /api/users/:id 请求体**（`password_hash` 可选，不传则不修改密码）：

```json
{
  "username": "ops2",
  "password_hash": "<新 bcrypt hash>",
  "role": "admin"
}
```

**User 实体字段**：`id`、`username`、`password_hash`、`role`、`created_at`、`must_change_password`

## 5. 节点模块 `/api/nodes`

权限前缀：`node:*`

| 方法 | 路径 | 权限 | 说明 |
|------|------|------|------|
| GET | `/api/nodes` | node:read | 节点列表（分页） |
| POST | `/api/nodes` | node:create | 注册节点（兼容 Agent 平铺格式） |
| PUT | `/api/nodes/:id` | node:update | 更新节点 |
| DELETE | `/api/nodes/:id` | node:delete | 删除节点 |
| POST | `/api/nodes/heartbeat/:id` | 白名单 | Agent 心跳上报（校验 Agent token） |
| GET | `/api/nodes/:id/status` | node:read | 在线状态 |
| GET | `/api/nodes/:id/metrics` | node:read | 指标快照 |
| GET | `/api/nodes/:id/metrics/history` | node:read | 指标历史（内存窗口，见 §5.1） |

**请求体结构**（POST/PUT，兼容两种格式）：

```json
// 面板/测试格式（嵌套 node）
{ "node": { "name": "web-01", "hostname": "web-01.example.com", "ip_address": "192.168.1.10", "status": "online" } }

// Agent 平铺格式
{ "name": "web-01", "host": "192.168.1.10", "agent_port": 9527, "auth_token": "agent-secret" }
```

**ServerNode 字段**：`id`、`name`、`hostname`、`ip_address`、`status`、`created_at`、`last_heartbeat_at`、`metrics_json`、`auth_token`

### 5.1 节点心跳与在线状态

| 方法 | 路径 | 权限 | 说明 |
|------|------|------|------|
| POST | `/api/nodes/heartbeat/:id` | 白名单（免 JWT，校验 Agent token） | Agent 心跳上报 |
| GET | `/api/nodes/:id/status` | node:read | 在线状态（>30s 无心跳判定 offline） |
| GET | `/api/nodes/:id/metrics` | node:read | 最近指标快照（完整 `MetricsSnapshot`） |
| GET | `/api/nodes/:id/metrics/history` | node:read | 指标历史：`MetricsSnapshot[]`，时间升序；节点不存在 404 |

**heartbeat 请求体**（前 4 项为旧协议，其余可选、缺省记 0——旧 Agent 照常工作）：

```json
{ "cpu_usage": 12.3, "memory_usage_percent": 45.6, "disk_usage_percent": 67.8, "load_one": 0.5,
  "load_five": 0.4, "load_fifteen": 0.3, "cpu_cores": 8, "memory_total_mb": 16000, "memory_used_mb": 7300,
  "disk_total_gb": 100.0, "disk_used_gb": 67.8, "network_rx_mbps": 1.2, "network_tx_mbps": 0.3 }
```

**节点指标模型（Doc/30 M-P）**：面板把每次心跳规整为与本机同构的 `MetricsSnapshot`——百分比钳制到 0–100、非有限值 / 负值记 0、`cpu_cores` 上限 4096、已用内存不超过总量、`timestamp` 取面板接收时刻（毫秒，不信任节点时钟）——写入 `metrics_json`，并发布到该节点独立的内存 `MetricsHub`（120 点 ≈ 20 分钟，面板重启后重新积累；删除节点即释放）。订阅 `/ws/metrics?node_id=<id>` 得到与本机相同的 `init` / `tick` 帧（见 §23.1）。

**请求头**：`Authorization: Bearer <agent-token>`（与注册时 `auth_token` 一致；旧 Agent 无 token 时放行并告警）

**响应 200**：`{ "id": 3, "status": "ok", "last_heartbeat_at": "..." }`

**错误**：`401 AUTH_UNAUTHORIZED`（Agent token 不匹配）

**status 响应**：`{ "id": 3, "status": "online" }`

## 6. 网站模块 `/api/websites`

权限前缀：`website:*`

| 方法 | 路径 | 权限 | 说明 |
|------|------|------|------|
| GET | `/api/websites` | website:read | 网站列表（分页） |
| POST | `/api/websites` | website:create | 创建网站 |
| GET | `/api/websites/:id` | website:read | 获取网站详情 |
| PUT | `/api/websites/:id` | website:update | 更新网站 |
| DELETE | `/api/websites/:id` | website:delete | 删除网站 |
| POST | `/api/websites/:id/switch-engine` | website:update | 切换网站 Web 引擎（目标引擎须已安装且可写；新引擎部署成功后才清理旧引擎配置，失败恢复原记录） |
| POST | `/api/websites/:id/deploy` | website:update | 手动部署站点配置（严格：引擎未安装 / 无写权限 / 配置校验失败 → 400，同名非托管 OLS 虚拟主机 → 409）；返回 `SiteDeployment { engine, path, enabled, reloaded }` |
| POST | `/api/websites/:id/runtime` | website:update | 绑定/解绑运行时或项目（M-B Stage A-6 · Doc/20 §5.5） |
| POST | `/api/websites/:id/rewrite` | website:update | 一键套用伪静态规则模板（落库 + 写 vhost + 校验 + reload；返回规则文本，写盘失败回滚套用） |
| GET | `/api/websites/:id/rewrite` | website:read | 伪静态套用状态（当前模板名 + 按站点引擎语法生成的规则文本 + 可选模板清单） |
| DELETE | `/api/websites/:id/rewrite` | website:update | 清除伪静态套用（回落引擎默认路由；同样落库 + 写 vhost，失败回滚） |
| GET | `/api/runtimes/:id/sites` | website:read | 按运行时列出绑定站点 |

**POST 请求体**：

```json
{
  "website": {
    "name": "my-blog",
    "domain": "blog.example.com",
    "root_path": "/var/www/blog",
    "status": "active",
    "node_id": 1,
    "engine": "nginx",
    "ssl_enabled": false,
    "proxy_enabled": true,
    "proxy_pass": "http://127.0.0.1:3000",
    "site_type": "static"
  }
}
```

**switch-engine 请求体**：`{ "engine": "caddy" }`

**站点配置联动**（Doc/20 §12.1）：创建 / 更新 / 绑定运行时 / 切换引擎 / 删除会同步写入或移除 Web 引擎站点配置——
引擎已安装且面板可写时执行（写入 → 配置校验 → reload，失败整体回滚并恢复库中记录），否则仅落库并记日志；
`status = "stopped"` 的站点以停用态部署（`<file>.disabled`）。`reloaded = false` 表示引擎当前未运行，配置已通过校验、启动后生效。

**bind-runtime 请求体**：`{ "site_type": "runtime_php", "runtime_id": 3, "project_id": null }`
（`site_type` 白名单：`runtime_php` / `reverse_proxy` / `static` / `container_app`；
`runtime_php` 须绑定已安装 php 运行时，`reverse_proxy` 可绑定配置了端口的项目；
runtime 与 project 互斥；解绑传 `{ "site_type": "static" }`）

**rewrite 请求体**：`{ "preset": "wordpress" }`
（可用模板：`wordpress` / `discuz` / `typecho` / `laravel` / `thinkphp` / `springboot`）

**Website 字段**：`id`、`name`、`domain`、`root_path`、`status`、`node_id`、`engine`、`ssl_enabled`、`proxy_enabled`、`proxy_pass`、`site_type`、`runtime_id`、`project_id`、`created_at`

## 7. Docker 模块 `/api/docker`

权限前缀：`docker:*`（动作：read / create / start / stop / update / delete）

> 引用校验（Doc/33 P-12）：容器 / 网络 / 卷的 ID 或名称须为 `[A-Za-z0-9][A-Za-z0-9_.-]*`（≤ 255），镜像引用允许 `/ : @` 但不得含 `..`、`//`、`?`、空白；tag 为 `[A-Za-z0-9_][A-Za-z0-9_.-]{0,127}`；驱动名小写；子网须为 CIDR。不合规一律 400。

### 7.1 容器

| 方法 | 路径 | 权限 | 说明 |
|------|------|------|------|
| GET | `/api/docker/containers` | docker:read | 容器列表（`?node_id=`） |
| GET | `/api/docker/containers/:id` | docker:read | 容器详情（id 或 name） |
| POST | `/api/docker/containers/:id/start` | docker:start | 启动容器 |
| POST | `/api/docker/containers/:id/stop` | docker:stop | 停止容器 |
| POST | `/api/docker/containers/:id/restart` | docker:start | 重启容器 |
| POST | `/api/docker/containers/:id/remove` | docker:delete | 删除容器 |
| GET | `/api/docker/containers/:id/logs` | docker:read | 容器日志（`?tail=100`，钳制 1–10000；输出上限 1 MiB） |
| GET | `/api/docker/containers/:id/stats` | docker:read | 容器实时统计 |
| GET | `/api/docker/containers/:id/inspect` | docker:read | 容器完整配置详情（JSON） |
| GET | `/api/docker/containers/:id/compose` | docker:read | 导出为 Compose（收编；结合镜像默认值剔除非用户设置项，返回结构同 convert-run，不改动原容器） |
| POST | `/api/docker/containers/:id/rename` | docker:update | 重命名容器（`{new_name}`） |
| POST | `/api/docker/containers/:id/pause` | docker:start | 暂停容器 |
| POST | `/api/docker/containers/:id/unpause` | docker:start | 恢复容器 |
| POST | `/api/docker/containers/:id/kill` | docker:start | 强杀容器（SIGKILL） |
| POST | `/api/docker/containers/prune` | docker:delete | 清理已停止容器 |

### 7.2 镜像

| 方法 | 路径 | 权限 | 说明 |
|------|------|------|------|
| GET | `/api/docker/images` | docker:read | 镜像列表 |
| POST | `/api/docker/images/pull` | docker:start | 拉取镜像（`{image}`，如 `nginx:latest`） |
| POST | `/api/docker/images/:id/remove` | docker:delete | 删除镜像 |
| POST | `/api/docker/images/:id/tag` | docker:update | 打标签（`{repo, tag}`） |
| POST | `/api/docker/images/prune` | docker:delete | 清理悬空镜像 |

### 7.3 网络

| 方法 | 路径 | 权限 | 说明 |
|------|------|------|------|
| GET | `/api/docker/networks` | docker:read | 网络列表（含已连接容器与 IP） |
| POST | `/api/docker/networks` | docker:create | 创建网络（`{name, driver?, subnet?}`） |
| POST | `/api/docker/networks/prune` | docker:delete | 清理未使用网络 |
| DELETE | `/api/docker/networks/:id` | docker:delete | 删除网络 |
| POST | `/api/docker/networks/:id/connect` | docker:update | 连接容器（`{container_id}`） |
| POST | `/api/docker/networks/:id/disconnect` | docker:update | 断开容器（`{container_id, force?}`） |

### 7.4 卷

| 方法 | 路径 | 权限 | 说明 |
|------|------|------|------|
| GET | `/api/docker/volumes` | docker:read | 卷列表 |
| POST | `/api/docker/volumes` | docker:create | 创建卷（`{name, driver?}`） |
| POST | `/api/docker/volumes/prune` | docker:delete | 清理未使用卷 |
| DELETE | `/api/docker/volumes/:name` | docker:delete | 删除卷（`?force=true`） |

### 7.5 Compose

| 方法 | 路径 | 权限 | 说明 |
|------|------|------|------|
| GET | `/api/docker/compose` | docker:read | Compose 项目列表（`docker compose ls -a`，含已停止项目） |
| POST | `/api/docker/compose/convert-run` | docker:create | `docker run` 命令 → Compose YAML（`{command}` ≤ 16 KiB；只转换不执行，返回 `{project_name, service_name, compose_yaml, warnings}`；含管道 / 重定向 / 命令替换或未知参数时 400） |
| POST | `/api/docker/compose/deploy` | **docker:create** | Compose 部署（YAML ≤ 256 KiB；v1.1 前为 docker:start） |
| POST | `/api/docker/compose/:project/up` | docker:start | Compose 启动（使用项目记录的配置文件） |
| POST | `/api/docker/compose/:project/down` | docker:stop | Compose 停止（保留配置文件与数据） |

> 项目目录：面板直接部署的项目持久化在 `data/compose/<project>/docker-compose.yml`（可用 `FLAME_COMPOSE_DIR` 覆盖；目录 0700、文件 0600），应用商店安装的项目以安装目录为项目目录。相对挂载（`./data:/data`）据此解析。部署任意 YAML 等价于宿主机 root 能力（`privileged`、挂载 `/`），请仅授予可信角色 `docker:create`。

**compose/deploy 请求体**：

```json
{
  "project_name": "myapp",
  "compose_yaml": "version: '3'\nservices:\n  app:\n    image: nginx"
}
```

**响应**：`{ "project_name": "myapp", "status": "deployed", "message": "..." }`

## 8. 插件模块 `/api/plugins`

权限前缀：`plugin:*`

| 方法 | 路径 | 权限 | 说明 |
|------|------|------|------|
| GET | `/api/plugins` | plugin:read | 插件列表 |
| POST | `/api/plugins` | plugin:create | 加载 WASM 插件（同 ID 已存在 409） |
| GET | `/api/plugins/market` | plugin:read | 插件市场目录（内置 + 远程索引） |
| POST | `/api/plugins/market/:id/install` | plugin:create | 从市场安装（申请能力须 `accept_capabilities`） |
| PUT | `/api/plugins/:id/capabilities` | plugin:create | 整体替换宿主能力授予 |
| GET | `/api/plugins/:id` | plugin:read | 插件详情 |
| POST | `/api/plugins/:id` | plugin:delete | 卸载插件（同时删除持久化记录） |
| POST | `/api/plugins/:id/enable` | plugin:create | 启用插件 |
| POST | `/api/plugins/:id/disable` | plugin:create | 禁用插件 |
| POST | `/api/plugins/:id/execute/:function` | plugin:execute | 执行插件函数 |
| POST | `/api/plugins/:id/reload` | plugin:create | 热重载插件 |
| GET | `/api/plugins/:id/metrics` | plugin:read | 插件指标 |
| DELETE | `/api/plugins/:id/metrics` | plugin:config | 重置指标 |
| GET | `/api/plugins/:id/settings` | plugin:config | 插件设置列表 |
| POST | `/api/plugins/:id/settings` | plugin:config | 保存插件设置 |
| GET | `/api/plugins/:id/settings/:key` | plugin:config | 读取单个设置 |

> 约束（Doc/33 P-10）：`id` 为 1–64 位 `[A-Za-z0-9._-]` 且不以 `.` 开头；`memory_limit_bytes` 1 MiB–256 MiB（缺省 64 MiB）、`timeout_ms` 1–300000（缺省 30000），越界 400；设置键同字符集 ≤ 128 位、值 ≤ 4 KiB（设置持久化，重启后恢复）。`id` 不能为保留字 `market`。`capabilities` 每项为 `fs:read:<绝对目录>` / `fs:write:<绝对目录>` / `metrics:read` / `settings:write`，≤ 16 项；系统敏感目录不可授予（规则见 Doc/21 §5.7），否则 400。

**POST /api/plugins（加载插件）请求体**：

```json
{
  "id": "my-plugin",
  "name": "My Plugin",
  "version": "1.0.0",
  "author": "dev",
  "description": "示例插件",
  "wasm_base64": "<WASM 字节码 base64>",
  "memory_limit_bytes": 1048576,
  "timeout_ms": 100,
  "capabilities": ["fs:read:/var/log", "metrics:read"]
}
```

**POST /api/plugins/:id/execute/:function 请求体**：

```json
{ "input": "Bob" }
```

`input`（≤ 1 MiB）由插件经宿主 API `flame.input_read` 读取；旧字段 `args` 仍被接受但不再使用。`timeout_ms` 按墙钟强制，超时返回 500 `WASM execution timed out after N ms`。

**执行响应**（插件调用 `flame.output` 时 `output` 为其字节串，否则为返回值 i32 的小端字节）：

```json
{
  "output": [72, 101, 108, 108, 111, 44, 32, 66, 111, 98, 33],
  "output_hex": "48656c6c6f2c20426f6221",
  "output_string": "Hello, Bob!",
  "execution_ms": 1,
  "logs": ["[INFO] hello plugin invoked"]
}
```

**GET /api/plugins/market 响应**（远程索引地址取面板设置 `plugin_market_url`；拉取失败不影响内置目录）：

```json
{
  "plugins": [
    {
      "id": "flame-log-viewer", "name": "Log Viewer", "version": "1.0.0", "author": "FlamePanel",
      "description": "…", "capabilities": ["fs:read:/var/log"], "tags": ["example", "logs"],
      "entry": "run", "wasm_sha256": "<64 hex>", "source": "builtin",
      "installed": false, "installed_version": null
    }
  ],
  "remote_url": null,
  "remote_error": null
}
```

**POST /api/plugins/market/:id/install 请求体**：`{ "accept_capabilities": true }`。插件申请了能力而未同意返回 400，同 ID 已安装 409，市场中不存在 404，远程包体哈希不符 400，远程不可达 503。

## 9. Web 服务器模块 `/api/web-servers`

权限前缀：`web_server:*`

### 9.1 实例管理

| 方法 | 路径 | 权限 | 说明 |
|------|------|------|------|
| GET | `/api/web-servers/engines` | web_server:read | 支持的引擎列表 |
| GET | `/api/web-servers` | web_server:read | 服务器实例列表（分页） |
| POST | `/api/web-servers` | web_server:create | 创建实例 |
| GET | `/api/web-servers/:id` | web_server:read | 实例详情 |
| PUT | `/api/web-servers/:id` | web_server:update | 更新实例 |
| DELETE | `/api/web-servers/:id` | web_server:delete | 删除实例 |
| POST | `/api/web-servers/:id/start` | web_server:start | 启动 |
| POST | `/api/web-servers/:id/stop` | web_server:stop | 停止 |
| POST | `/api/web-servers/:id/restart` | web_server:start | 重启 |
| POST | `/api/web-servers/:id/reload` | web_server:reload | 重载配置 |
| POST | `/api/web-servers/:id/configtest` | web_server:configtest | 配置测试 |
| GET | `/api/web-servers/:id/config` | web_server:read | 读取主配置（`{id, config_path, content}`；仅限引擎配置根下，否则 403） |
| POST | `/api/web-servers/:id/switch-engine` | web_server:update | 切换引擎 |
| POST | `/api/web-servers/:id/preset` | web_server:update | 应用性能预设 |
| GET | `/api/web-servers/presets` | web_server:read | 预设列表（含推荐） |

**实例字段约束**（创建 / 更新，v1.1 起）：`engine` 必须是已知引擎；`config_path` 必须是规范绝对路径且位于 Web 引擎配置根之下（`/etc/nginx`、`/etc/apache2`、`/etc/httpd`、`/etc/caddy`、`/usr/local/lsws/conf`、`/usr/local/openresty/nginx/conf`，与特权助手同一份校验）；`binary_path` 只接受该引擎的已知二进制（裸程序名，或 `/usr/sbin`、`/usr/bin`、`/usr/local/{s,}bin`、`/{s,}bin` 下同名文件，及引擎候选路径）。不满足 → 400 `VALIDATION_ERROR`。启动 / 停止 / 性能预设 / 读取配置时对存量实例再校验一次。原 `POST /api/web-servers/:id/config`（裸写任意 `config_path`，绕过助手与配置校验）已移除——全局配置改动走性能预设，站点配置走网站部署流水线。

### 9.2 原生控制（系统包 + systemd）

| 方法 | 路径 | 权限 | 说明 |
|------|------|------|------|
| GET | `/api/web-servers/native/detect` | web_server:read | 检测 5 引擎原生状态（安装/版本/服务/端口） |
| POST | `/api/web-servers/native/install` | web_server:create | 包管理器安装（`{engine, version?}`）并注册实例 |
| POST | `/api/web-servers/native/uninstall` | web_server:delete | 卸载（`{engine}`，含系统包与注册实例） |
| POST | `/api/web-servers/native/autostart` | web_server:update | 开机自启（`{engine, enabled}`） |
| POST | `/api/web-servers/:id/autostart` | web_server:update | 按实例设置开机自启（`{enabled}`） |
| GET | `/api/web-servers/:id/native-status` | web_server:read | 实例原生状态详情 |

**native/detect 响应**（每引擎一项）：

```json
{
  "engine": "nginx",
  "description": "Nginx - 高性能 HTTP 和反向代理服务器",
  "installed": true,
  "package_installed": true,
  "version": "1.27.0",
  "service_name": "nginx",
  "running": true,
  "enabled": true,
  "binary_path": "/usr/sbin/nginx",
  "config_path": "/etc/nginx/nginx.conf",
  "default_port": 80,
  "listening_ports": [80]
}
```

**支持的引擎**：`nginx`、`apache`、`openlitespeed`、`openresty`、`caddy`

### 9.3 配套能力（安装时"各取所需"）

权限前缀：`web_server:*`

| 方法 | 路径 | 权限 | 说明 |
|------|------|------|------|
| GET | `/api/web-servers/capabilities?engine=nginx` | web_server:read | 查询引擎的配套能力清单（含推荐包与配置提示） |
| POST | `/api/web-servers/capabilities/install` | web_server:update | 批量安装配套依赖（`{engine, capabilities: ["php", ...]}`） |

**GET /api/web-servers/capabilities 响应**：

```json
{
  "engine": "nginx",
  "capabilities": [
    {
      "key": "php",
      "label_zh": "PHP 运行时（PHP-FPM）",
      "label_en": "PHP runtime (PHP-FPM)",
      "packages": ["php-fpm", "php-cli", "php-mysql"],
      "hint": "Nginx + PHP-FPM：站点 server 块需添加 fastcgi_pass ..."
    }
  ]
}
```

**支持的配套能力 key**：`php`（PHP-FPM）、`lua`（Lua 模块，仅 OpenResty）、`cache`（Redis 缓存）、`extra_modules`（扩展模块，仅 Nginx/Apache）、`auto_https`（自动 HTTPS）。

> 推荐包按 引擎 + 发行版 差异化路由：Debian 系 `php-fpm`/`php-mysql`、RHEL 系 `php-mysqlnd`、Alpine `php-fpm`；OpenLiteSpeed 的 PHP 走官方仓库 `lsphp82`；Caddy 自动 HTTPS 内置无需额外安装。单包失败仅告警不阻断整体，响应返回 `{installed, failed, success}`。

> OpenLiteSpeed 原生控制：检测走绝对路径探测（`/usr/local/lsws/bin/lshttpd`，PATH 探测不到），安装走 LiteSpeed 官方仓库（Debian 系写 APT 源 / RHEL 系装 RPM 源后 `openlitespeed` 包），启停/重载/配置测试走 `lswsctrl` 控制脚本。

> Apache 原生控制（发行版感知）：Debian/Ubuntu 系走包 `apache2` + 服务 `apache2` + 控制工具 `apache2ctl`（configtest/graceful）+ `a2enmod/a2dismod` 模块管理 + `/etc/apache2/sites-{available,enabled}` 布局；RHEL 系走包/服务 `httpd` + `httpd -t`/`httpd -k graceful` + `/etc/httpd/conf.d`。安装/检测/启停/配置测试/站点启用全部按发行版自动路由，未知发行版回退 RHEL 风格。

> Caddy 原生控制：安装走官方 Cloudsmith stable 仓库（Debian 系写 `caddy-stable.list` APT 源 / RHEL 系写 `caddy-stable.repo` YUM 源；Alpine 直接用 `apk add caddy`）。配置校验走 `caddy validate --config /etc/caddy/Caddyfile`，reload 走 `caddy reload --config /etc/caddy/Caddyfile` 热加载（非 systemd restart）；实例启动/停止走 `caddy start/stop --config`（`caddy -c` 参数不存在，直接 `caddy run` 会前台阻塞），运行状态经 systemd `is-active caddy` 判断，并支持 `caddy fmt --overwrite` 配置格式化。

**POST /api/web-servers 请求体**：

```json
{
  "engine": "nginx",
  "version": "1.27",
  "port": 80,
  "config_path": "/etc/nginx/nginx.conf",
  "binary_path": "/usr/sbin/nginx"
}
```

**性能预设**：`low` / `medium` / `high` / `ultra`（按资源自动推荐）

## 10. 设置模块 `/api/settings`

权限前缀：`settings:*`

| 方法 | 路径 | 权限 | 说明 |
|------|------|------|------|
| GET | `/api/settings` | settings:read | 设置列表（分页） |
| GET | `/api/settings/:key` | settings:read | 读取单个设置 |
| PUT | `/api/settings` | settings:update | 更新单个设置 `{key, value}` |
| PATCH | `/api/settings/batch` | settings:update | 批量更新 `{settings: [[key, value], …]}`，任一项校验失败整体 400 |
| GET | `/api/settings/access` | settings:read | 访问来源与生效策略：`client_ip` / `host` / `local`（本机直连）/ `allowed_ips` / `bind_domains` / `session_timeout_minutes` / `login_max_failures` / `login_lockout_minutes`（设置页据此提示「你的 IP」，防止误锁） |

**PUT 请求体**：

```json
{ "key": "panel_name", "value": "MyPanel" }
```

**已登记设置键**（W4-a 起按类型校验并规范化，非法值 400）：

| 键 | 类型 / 范围 | 说明 |
|----|------------|------|
| `panel_name` | 文本 ≤ 64 | 面板名称 |
| `language` | `zh-CN` / `en-US` / `ja-JP` | 默认语言 |
| `theme` | `light` / `dark` / `auto` | 默认主题模式 |
| `panel_port` | 1–65535 | 需重启；推荐用 `fpctl port set` |
| `server_address` | 文本 ≤ 255 | 面板对外地址（生成链接用） |
| `session_timeout_minutes` | 5–43200 | 会话有效期（refresh token TTL） |
| `login_max_failures` / `login_lockout_minutes` | 3–20 / 1–1440 | 登录失败锁定 |
| `allowed_ips` | IP / CIDR 列表 ≤ 64 条 | 授权 IP，空 = 不限 |
| `bind_domains` | 域名列表（可 `*.`）≤ 64 条 | 绑定域名，空 = 不限 |
| `log_level` | trace…error | 日志级别 |
| `log_retention_days` / `audit_log_retention_days` | 1–3650 / 7–3650 | 日志保留 |
| `auto_backup_enabled` / `auto_backup_interval_hours` / `backup_retention` | 布尔 / 1–720 / 1–365 | 自动备份 |
| `two_factor_enabled`、`menu_accordion`、`menu_collapsed` | 布尔 | |
| `hide_menu` | JSON 字符串数组 | 隐藏菜单分组 |
| `app_background` / `login_background` | 图片 data URL ≤ 1 MiB | 背景图 |
| `app_signature_policy` | `optional` / `required` | 应用包验签策略 |
| `appstore_registry_whitelist` | 文本 ≤ 4096 | 镜像仓库白名单 |

未登记键兼容写入（键名 `[a-z0-9_.-]` ≤ 64，值 ≤ 64 KiB）。**保留键** `jwt_secret`、`jwt_secret_persisted`、`waf.ingest.*` 不出现在列表、读取 404、写入 400；签名密钥轮换走 `POST /api/auth/rotate-secret`。

访问控制：违反授权 IP / 绑定域名的请求返回 403 `{"error":"ACCESS_DENIED"}`；本机回环且无转发头的请求始终放行（见 Doc/10 §7.4.1）。

## 11. 数据库模块 `/api/databases`

权限前缀：`database:*`

| 方法 | 路径 | 权限 | 说明 |
|------|------|------|------|
| GET | `/api/databases` | database:read | 实例列表（分页） |
| GET | `/api/databases/:id` | database:read | 实例详情 |
| DELETE | `/api/databases/:id` | database:delete | 删除实例 |
| POST | `/api/databases/mysql/install` | database:create | 安装 MySQL |
| POST | `/api/databases/redis/install` | database:create | 安装 Redis |
| POST | `/api/databases/postgres/install` | database:create | 安装 PostgreSQL（Doc/20 §6.4） |
| POST | `/api/databases/mongodb/install` | database:create | 安装 MongoDB（官方仓库 `version` 8.0 默认 / 9.0，`port` 默认 27017；`password` ≥ 8 位时创建 `admin` 并开启鉴权，留空仅本机免鉴权） |
| POST | `/api/databases/:id/start` | database:start | 启动 |
| POST | `/api/databases/:id/stop` | database:stop | 停止 |
| POST | `/api/databases/:id/restart` | database:start | 重启 |
| GET | `/api/databases/:id/status` | database:read | 状态检查 |
| GET | `/api/databases/:id/databases` | database:read | 数据库列表 |
| POST | `/api/databases/:id/databases` | database:create | 创建数据库 |
| DELETE | `/api/databases/:id/databases/:name` | database:delete | 删除数据库 |
| POST | `/api/databases/:id/users` | database:update | 创建数据库用户 |
| DELETE | `/api/databases/:id/users/:username` | database:update | 删除数据库用户 |
| POST | `/api/databases/:id/uninstall` | database:delete | 卸载数据库 |
| PATCH | `/api/databases/batch-status` | database:update | 批量更新实例状态（`updates: [[id, "running"\|"stopped"\|"error"]]`，≤ 200 项） |

> 保护（Doc/33 P-11）：系统库（MySQL `mysql` / `information_schema` / `performance_schema` / `sys`，PostgreSQL `postgres` / `template0` / `template1`）与内置超级用户（`root`、`mysql.sys` 等、`postgres`）不可删除，返回 400。v1.1 前实例详情与子库列表实际未做 `database:read` 校验（Doc/33 P-8），已修复。

**install/mysql 请求体**：

```json
{
  "name": "mysql-main",
  "version": "8.0",
  "port": 3306,
  "root_password": "Root@123"
}
```

**install/redis 请求体**：

```json
{
  "name": "redis-cache",
  "version": "7.4",
  "port": 6379,
  "password": "redis-pass"
}
```

**install/postgres 请求体**：

```json
{
  "name": "pg-main",
  "version": "16",
  "port": 5432,
  "password": "pg-pass"
}
```

**create database 请求体**：`{ "name": "appdb", "charset": "utf8mb4" }`

**create user 请求体**：`{ "username": "app", "password": "pass", "host": "localhost" }`

## 12. 应用商店模块 `/api/app-store`

权限前缀：`app_store:*`

| 方法 | 路径 | 权限 | 说明 |
|------|------|------|------|
| GET | `/api/app-store/packages` | app_store:read | 应用包列表（`?category=`） |
| POST | `/api/app-store/packages/import` | app_store:create | 导入本地应用包 |
| GET | `/api/app-store/packages/:key` | app_store:read | 应用详情 |
| GET | `/api/app-store/packages/:key/versions/:version` | app_store:read | 版本详情（含表单字段） |
| POST | `/api/app-store/packages/:key/install` | app_store:create | 安装应用 |
| GET | `/api/app-store/installed` | app_store:read | 已安装应用列表 |
| GET | `/api/app-store/installed/:id` | app_store:read | 已安装应用详情 |
| GET | `/api/app-store/installed/:id/upgrade-preview` | app_store:read | 升级预览：compose 三方合并结果（冲突 / 保留的本地修改 / 生效的上游修改），不改文件（Doc/20 §3.1.2） |
| POST | `/api/app-store/installed/:id/upgrade` | app_store:update | 升级应用（`?target_version=`；`?conflict_strategy=abort\|local\|upstream`，默认 abort：冲突返回 409） |
| POST | `/api/app-store/installed/:id/uninstall` | app_store:delete | 卸载应用 |
| GET | `/api/app-store/installed/:id/logs` | app_store:read | 应用日志（`?tail=200`，钳制 1–5000） |
| GET | `/api/app-store/installed/:id/native-status` | app_store:read | 原生应用运行状态（running/stopped） |
| POST | `/api/app-store/installed/:id/native-start` | app_store:update | 启动原生应用服务 |
| POST | `/api/app-store/installed/:id/native-stop` | app_store:update | 停止原生应用服务 |
| POST | `/api/app-store/installed/:id/native-restart` | app_store:update | 重启原生应用服务 |
| GET | `/api/app-store/wasm-builtins` | app_store:read | WASM 内置工具 |
| GET | `/api/app-store/sources` | app_store:read | 远程应用源列表（M-A Issue 1.2） |
| POST | `/api/app-store/sources` | app_store:create | 添加远程源（git/http） |
| PUT | `/api/app-store/sources/:id` | app_store:update | 更新远程源（name/url/enabled/public_key） |
| DELETE | `/api/app-store/sources/:id` | app_store:delete | 删除远程源 |
| POST | `/api/app-store/sources/:id/sync` | app_store:update | 手动同步源索引（统一任务中心跟踪，同源互斥） |
| GET | `/api/app-store/sources/:id/packages` | app_store:read | 按源过滤应用包列表 |
| GET | `/api/app-store/security` | app_store:read | 商店安全配置（Doc/20 §4.2 收口）：验签策略 / 内置白名单 / 追加项 / 生效清单 / 未签名源计数 |
| PUT | `/api/app-store/security` | app_store:update | 更新商店安全配置（`{signature_policy?, registry_whitelist?}`，白名单全量替换） |

> 远程包懒加载安装（M-A Issue 1.4）：http 源包同步阶段仅落索引（无包体），安装时经 `ensure_remote_package_fetched` 自动下载包体到 `data/apps/remote/<key>/` 缓存并回写 `source_path`（外部下载走 Circuit Breaker；源已停用或已删除则拒绝安装）。git 源包复用 clone 缓存，无二次下载。安装请求与响应结构不变，无新增端点。

> 导入约束（Doc/33 P-9）：包 `key` 须为 `[A-Za-z0-9._-]{1,128}` 且不以 `.` 开头；已存在的 key 在复制前即拒绝（不覆盖）；不跟随符号链接；源目录与目标目录（商店目录下的 `key` 子目录）不得互相包含；批量导入 ≤ 50 个、批次内 key 不得重复，任一失败回滚已复制文件。安装端点路径 `:key` 须与请求体 `package_key` 一致，否则 400。

> 包签名验签（M-A Issue 1.3）：本地导入（`POST /api/app-store/packages/import`、`/batch-import`）时，包根 `app.json` 携带 `signature` 字段即强制验签（`public_key` 随包提供）；远程包（git/http 源）在同步与安装阶段按源 `public_key` 验证包根 `signature.sig`，settings `app_signature_policy=required` 时未配公钥的源拒绝同步与安装。
>
> **商店安全配置与未签名源门禁（§4.2 收口）**：`POST /api/app-store/sources` 与 `PUT /api/app-store/sources/:id` 新增可选字段 `confirm_unsigned`（默认 false）——未配置验签公钥的源须显式确认；签名源**清空公钥**（降级为未签名源）同样须确认（防绕过创建期校验卸载信任锚）；策略 `required` 下无论是否确认，未配公钥的源一律拒绝。配置读写端点 `GET/PUT /api/app-store/security` 见上表；白名单条目仅接受 `host[:port]` 形态（拒绝 scheme/路径），最多 50 项，归一（小写 + trim）后去重且不与内置白名单重复。

**install 请求体**：

```json
{
  "package_key": "wordpress",
  "version": "6.7",
  "mode": "container",
  "name": "my-wp",
  "port": 8081,
  "container_name": "wp-01",
  "values": { "DB_PASSWORD": "secret" },
  "confirm_risky": false
}
```

> `mode`：`container` / `native` / `wasm`；`confirm_risky` 为 true 时允许高风险安全项。

**import 请求体**：`{ "path": "/path/to/app-package" }`

**远程源 create 请求体**：`{ "name": "官方源", "source_type": "git", "url": "https://github.com/1Panel-dev/appstore", "public_key": null }`

> 远程源协议（M-A Issue 1.2，Doc/20 §4.1）：`git` 类型对仓库 `git clone --depth 1` 懒加载，同步遍历一级子目录经适配器解析索引；`http` 类型拉取 JSON manifest（`{"packages":[{"key","name","category","description","format","download_url","versions","logo"}]}`），同步只落索引，安装时才按需下载包体到 `data/apps/remote/` 缓存。同步任务经统一任务中心跟踪（`GET /api/tasks`），同源并发同步互斥（409）。

**sync 响应体**：`{ "added": 12, "skipped": 0, "packages": ["wordpress", "gitea"] }`

## 13. 文件模块 `/api/files`

权限前缀：`file:*`。所有路径按 chroot 语义解释：`/` 即 `OP_FILE_ROOT`（默认 `data/files`），越界、`..` 穿越或经符号链接逃逸一律 403；列表返回的 `path` 为根内相对路径，可原样回传。根下的 `.fp-recycle/`（回收站）与 `.fp-part-*`（上传 / 压缩临时文件）对文件 API 不可见、不可访问。

| 方法 | 路径 | 权限 | 说明 |
|------|------|------|------|
| GET | `/api/files` | file:read | 目录列表（`?path=/`；目录在前，含 `is_symlink`） |
| GET | `/api/files/read` | file:read | 读取文本文件（`?path=`；≤ 10 MiB，非 UTF-8 → 400） |
| POST | `/api/files/write` | file:write | 保存文件（原子替换，保留权限 / 属主；`expected_modified` 不符 → 409） |
| POST | `/api/files/create-file` | file:write | 创建空文件（已存在 → 409） |
| POST | `/api/files/create-dir` | file:write | 创建目录（已存在 → 409） |
| DELETE | `/api/files/delete` | file:write | 删除（`?path=&recursive=&permanent=`；默认移入回收站，`permanent=true` 直接删除） |
| POST | `/api/files/rename` | file:write | 重命名 / 移动（目标已存在 → 409） |
| POST | `/api/files/copy` | file:write | 批量复制到目录（逐项结果，见下） |
| POST | `/api/files/move` | file:write | 批量移动到目录（逐项结果，见下） |
| POST | `/api/files/chmod` | file:write | 修改权限（八进制，≤ `7777`） |
| POST | `/api/files/upload` | file:upload | 上传（`?path=&name=&overwrite=`，body 为文件原始字节，流式落盘，≤ 2 GiB；同名且未 `overwrite` → 409） |
| GET | `/api/files/download` | file:upload | 下载（`?path=`，流式；`Content-Disposition` 带 RFC 5987 文件名） |
| POST | `/api/files/compress` | file:write | 压缩（`.zip` / `.tar.gz` / `.tgz`，由 `dest` 扩展名决定） |
| POST | `/api/files/extract` | file:write | 解压（zip / tar / tar.gz / tgz / tar.xz / txz / gz） |
| GET | `/api/files/recycle` | file:read | 回收站列表（新删除的在前；过期 30 天的条目在列出时清理） |
| POST | `/api/files/recycle/{id}/restore` | file:write | 还原到原位置（原位置已存在 → 409；缺失的父目录自动重建） |
| DELETE | `/api/files/recycle/{id}` | file:write | 彻底删除单个条目 |
| DELETE | `/api/files/recycle` | file:write | 清空回收站（返回 `{ "purged": n }`） |

**write 请求体**：`{ "path": "/conf/app.ini", "content": "…", "expected_modified": "2026-09-27T10:00:00.123456789+00:00" }` → `{ "modified_at": "…", "size": 12 }`。`expected_modified` 为打开文件时列表 / 上次保存返回的 `modified_at`；省略即强制覆盖。

**rename 请求体**：`{ "old_path": "/a.txt", "new_path": "/b.txt" }`

**copy / move 请求体**：`{ "sources": ["/a.txt", "/logs"], "dest_dir": "/backup", "overwrite": false }` → `200 { "succeeded": 1, "failed": 1, "items": [{ "source", "ok", "target"?, "error"?, "error_kind"? }] }`。

- 请求级错误：`sources` 为空或 > 500 项 → 400；`dest_dir` 不是目录 → 400；越出文件根或指向回收站 → 403；不存在 → 404。
- 逐项错误 `error_kind`：`conflict`（同名已存在且未 `overwrite`）/ `not_found` / `forbidden` / `invalid`（移动到原目录、目录放进自身）/ `internal`；单项失败不影响其它项。
- `overwrite=true` 时旧目标先移入回收站（可还原）；复制到源所在目录自动命名为 `名称 (副本)`、`名称 (副本 2)`……；源路径去重；复制保留权限位，拒绝复制后会越出文件根的相对符号链接；跨文件系统移动回退为「复制 + 删除」。

**chmod 请求体**：`{ "path": "/a.txt", "mode": "644" }`

**compress 请求体**：`{ "paths": ["/site/css", "/site/index.html"], "dest": "/site.zip", "overwrite": false }` → `{ "path": "/site.zip", "size": 1024, "entries": 5, "skipped": 0 }`。`paths` 须位于同一目录（≤ 1000 项）；zip 不收录符号链接（计入 `skipped`），tar.gz 原样保留链接；先写同目录临时文件再改名，失败不留半成品。

**extract 请求体**：`{ "path": "/site.zip", "dest": "/www", "overwrite": false }` → `{ "entries": 5, "bytes": 2048, "top_level": ["site"], "replaced": [], "skipped": 0 }`。`dest` 缺省为压缩包所在目录。先解到目标目录下的临时目录，校验全部通过后逐个移入：

- 路径穿越（`..` / 绝对路径）、指向包外的符号链接、硬链接、名为 `.fp-recycle` 的顶层条目 → 400（`VALIDATION_ERROR`），目标目录不受影响；
- 解压总量 > 16 GiB 或条目 > 200 000 → 400（`VALIDATION_ERROR`，防解压炸弹）；设备 / FIFO 等特殊文件跳过（计入 `skipped`）；
- 目标已有同名顶层条目：未指定 `overwrite` → 409 且不做任何改动；`overwrite=true` → 旧条目移入回收站后替换（名称列于 `replaced`）。

**回收站条目**：`{ "id", "name", "original_path", "is_dir", "size", "size_truncated", "deleted_at", "deleted_by", "expires_at" }`。回收站位于 `<OP_FILE_ROOT>/.fp-recycle/`，与数据同一文件系统，移入 / 还原都是 `rename`（不复制）；若被删条目在另一文件系统（挂载点）上则返回 400，需改用 `permanent=true`。

## 14. 防火墙模块 `/api/firewall`

权限前缀：`firewall:*`

| 方法 | 路径 | 权限 | 说明 |
|------|------|------|------|
| GET | `/api/firewall/rules` | firewall:read | 规则列表（分页） |
| POST | `/api/firewall/rules` | firewall:create | 创建规则 |
| GET | `/api/firewall/rules/:id` | firewall:read | 规则详情 |
| PUT | `/api/firewall/rules/:id` | firewall:update | 更新规则 |
| DELETE | `/api/firewall/rules/:id` | firewall:delete | 删除规则 |
| POST | `/api/firewall/rules/:id/toggle` | firewall:enable | 启用/禁用规则 |
| POST | `/api/firewall/apply` | firewall:apply | 应用全部规则 |
| GET | `/api/firewall/status` | firewall:read | 后端状态（ufw/firewalld/iptables） |
| POST | `/api/firewall/enable` | firewall:enable | 启用防火墙 |
| POST | `/api/firewall/disable` | firewall:enable | 禁用防火墙 |
| POST | `/api/firewall/reorder` | firewall:update | 规则排序 |

**create 请求体**：

```json
{
  "name": "允许 8080",
  "protocol": "tcp",
  "port": "8080",
  "source": "0.0.0.0/0",
  "action": "allow",
  "priority": 50,
  "direction": "in"
}
```

**字段校验与规范化（S-13，create / update 共用，`domain/firewall.rs` 纯函数）**：违反任一条返回 400 `VALIDATION_ERROR`，`message` 一次列出全部字段错误（`name: required; port: invalid port '70000'; …`），不落库、不下发系统命令。

| 字段 | 规则 | 规范化 |
|------|------|--------|
| `name` | 1–64 字符，不含控制字符 | 去首尾空白 |
| `description` | ≤ 255 字符 | 空白 → `null` |
| `protocol` | `tcp` / `udp` / `icmp` / `any` | 小写 |
| `port` | 单端口 / 端口段（`8000-9000`，也接受 `8000:9000`）/ 逗号分隔；1–65535；≤ 15 槽位（端口段计 2，对齐 iptables multiport）；**仅 `tcp` / `udp` 可带端口** | `80,443,8000-9000`（去重）；空串 → `null`（= 所有端口，update 传 `""` 可清空） |
| `source` | IPv4 / IPv6 地址或 CIDR；空 / `any` / `0.0.0.0/0` / `::/0` = 任意；拒绝 `/0` 与主机名 | 主机位归零（`192.168.1.10/24` → `192.168.1.0/24`），`/32`、`/128` 省略前缀；任意 → `0.0.0.0/0` |
| `destination` | 同 `source` | 任意 → `null` |
| `action` / `direction` | `allow` / `deny` / `reject`；`in` / `out` | 小写 |
| `priority` | 1–999 | — |

**各后端渲染**（`infrastructure/firewall_render.rs`，参数数组、不拼 shell；严格白名单同时杜绝 firewalld rich rule 字符串注入）：

| | ufw | firewalld | iptables |
|---|---|---|---|
| 语法 | `ufw [delete] ACTION in\|out [proto P] from A to B [port …]` | 每个端口项一条 `--add-rich-rule` / `--remove-rich-rule` | `-A` / `-D INPUT\|OUTPUT` |
| 端口段 / 多端口 | `8000:9000`、`80,443` | `port="8000-9000"` | `--dport 8000:9000`；多项 `-m multiport --dports` |
| 远端地址 | 入站 `from X`，出站 `to X` | `family="ipv4\|ipv6" source address="X"` | 入站 `-s`，出站 `-d`；IPv6 → `ip6tables` |
| 动作 | allow / deny / reject | accept / drop / reject | ACCEPT / DROP / REJECT |
| 限制 | 不支持 ICMP（400） | rich rule 仅入站（出站 400） | — |

历史遗留的非法规则无法渲染（400 `…; edit and save it again`）：**删除**直接删库，**编辑**跳过旧规则的系统移除、以新值写入，便于修复。firewalld 删除 S-13 之前以 `--add-port` 写入的放行规则时回落旧形态移除。

**toggle 请求体**：`{ "enabled": false }`

**reorder 请求体**：`{ "ids": [3, 2, 1] }`

## 15. 日志模块

### 操作日志 `/api/operation-logs`

权限前缀：`operation_log:*`

| 方法 | 路径 | 权限 | 说明 |
|------|------|------|------|
| GET | `/api/operation-logs` | operation_log:read | 审计日志（分页，`?action=` 按前缀过滤） |
| DELETE | `/api/operation-logs/:id` | operation_log:delete | 删除日志 |

> **审计机制（v0.6.0）**：所有写操作（POST/PUT/DELETE）经中间件自动落库，`action` 格式为 `{METHOD} {path}`（如 `POST /api/users`）；登录成功/失败显式记录为 `LOGIN_SUCCESS` / `LOGIN_FAILED`。示例：`GET /api/operation-logs?action=LOGIN` 仅查登录审计。

### 系统日志 `/api/logs`

权限前缀：`log:*`

| 方法 | 路径 | 权限 | 说明 |
|------|------|------|------|
| GET | `/api/logs` | log:read | 系统日志（分页） |
| DELETE | `/api/logs/:id` | log:delete | 删除日志 |

### 内核日志 `/api/kernel-logs`（W0 · Doc/32 §4，v1.0.0）

> 内核自身的运行日志（`tracing` 事件）落盘为 **JSON Lines**：`<OP_LOG_DIR>/flamepanel.YYYY-MM-DD.log`，按天切分、单文件超 `OP_LOG_MAX_FILE_MB` 滚动为 `.log.N`、保留 `OP_LOG_RETENTION_DAYS` 天；panic 另写 `crash-YYYYMMDD-HHMMSS.log`。写入走独立线程 + 有界队列（8192 行），磁盘慢时丢行计数而**不阻塞请求**。五级：`trace` < `debug` < `info` < `warn` < `error`。

| 方法 | 路径 | 权限 | 说明 |
|------|------|------|------|
| GET | `/api/kernel-logs` | log:read | 概览 `{enabled, dir, level, retention_days, dropped_lines, files[{name,size,modified,current}]}`；文件按「当前在前、其余按修改时间倒序」 |
| GET | `/api/kernel-logs/entries` | log:read | 倒序分页（最新在前）。参数：`file`（缺省当前文件）、`levels=warn,error`、`q`（不区分大小写，匹配整行——含 span 中的 `request_id` / `username`）、`target`（模块前缀）、`limit`（缺省 200，1–1000）、`before`（上一页 `next_before`）。单次最多扫描 32 MB，`next_before=null` 表示到头 |
| GET | `/api/kernel-logs/stats?file=` | log:read | 五级计数 `{counts:{trace,debug,info,warn,error,total}, truncated}`；超过 256 MB 只统计前 256 MB 并置 `truncated` |
| GET | `/api/kernel-logs/files/{name}/download` | log:read | 下载原文（`text/plain`，附件头；≤ 512 MB） |
| DELETE | `/api/kernel-logs/files/{name}` | log:delete | 删除历史文件；**当前正在写入的文件 → 400** |
| PUT | `/api/kernel-logs/level` | settings:update | `{ "level": "debug" }` 运行时调级，立即生效，重启回到 `OP_LOG_LEVEL`；日志系统未初始化（嵌入 / 测试进程）→ 409 |

- 文件名白名单 `[A-Za-z0-9._-]` 且以 `.log` / `.log.N` 结尾，含 `/`、`..` 或其他扩展名一律 400（不可借此读取目录内其他文件）。
- 文件日志未启用（`OP_LOG_FILE=false` 或目录不可写）：概览 `enabled=false`，其余读端点 404。
- **错误反馈闭环**：所有响应带 `X-Request-Id`；5xx 在内核日志中以 `error` 级记录真实原因（响应体只返回通用文案），前端错误提示附 `ID: <request_id>`，在面板「日志中心 → 内核日志」搜索该编号或访问 `/system-logs?q=<id>` 即可定位。

## 16. 备份模块 `/api/backups`

权限前缀：`backup:*`

| 方法 | 路径 | 权限 | 说明 |
|------|------|------|------|
| GET | `/api/backups` | backup:read | 备份列表 |
| POST | `/api/backups` | backup:create | 创建备份 |
| GET | `/api/backups/:filename` | backup:read | 下载备份文件 |
| DELETE | `/api/backups/:filename` | backup:delete | 删除备份 |
| POST | `/api/backups/:filename/restore` | backup:create | 恢复备份（`{filename}`） |

### 16.1 备份账号 `/api/backup-accounts`（M-A 收尾 Stage 1 · Doc/20 §6.5）

| 方法 | 路径 | 权限 | 说明 |
|------|------|------|------|
| GET | `/api/backup-accounts` | backup:read | 备份账号列表 |
| POST | `/api/backup-accounts` | backup:create | 创建账号（`{name, account_type: s3\|sftp\|local, config_json, enabled}`） |
| PUT | `/api/backup-accounts/:id` | backup:update | 更新账号（部分字段可选，测试历史不重置） |
| DELETE | `/api/backup-accounts/:id` | backup:delete | 删除账号 |
| POST | `/api/backup-accounts/:id/test` | backup:update | 连通性测试（真实探测：连接/认证/写探针，结果落库） |
| GET | `/api/backup-accounts/:id/objects` | backup:read | 列出云端账号的备份对象 |
| POST | `/api/backup-accounts/:id/objects/upload` | backup:create | 上传本地备份到云端账号（`{local_filename, remote_key?}`） |
| DELETE | `/api/backup-accounts/:id/objects?key=` | backup:delete | 删除云端备份对象（幂等） |

> 响应不含 `config_json`（防敏感连接信息泄漏）；`last_test_status`/`last_test_at` 记录最近一次连通性测试结果。
> `config_json` 落库前统一 AES-256-GCM 加密（`enc:v1:` 密文，密钥见 Doc/03 §3.14a），请求仍传明文 JSON，历史明文账号读取静默兼容。
> Stage 2（Doc/20 §6.5）：`test` 端点按账号类型构造真实后端探测（S3 兼容 / SFTP / 本地目录），
> 探测失败仍返回 200（`last_test_status=failed` + `test_message` 带失败原因）。
> 本地目录账号 `config_json`：`{"root": "/path"}`；SFTP：`{"host","port","username","password"|"private_key_path","passphrase?","remote_root"}`；
> S3 兼容：`{"endpoint","bucket","access_key","secret_key","region?","prefix?"}`（AWS S3/MinIO/OSS/COS/R2）。

### 16.2 备份任务 `/api/backup-jobs`（M-A 收尾 Stage 3 · Doc/20 §6.5）

计划备份任务：定义「备份什么（scope）→ 何时（cron）→ 备到哪（目标账号）→ 保留几份」，
由后台调度器每 30 秒 tick 驱动，同任务防重叠执行（F-10 同款语义）。

| 方法 | 路径 | 权限 | 说明 |
|------|------|------|------|
| GET | `/api/backup-jobs` | backup:read | 任务列表 |
| POST | `/api/backup-jobs` | backup:create | 创建任务（`{name, scope, scope_key?, schedule, target_account_id, retention?, enabled?}`） |
| PUT | `/api/backup-jobs/:id` | backup:update | 更新任务（执行历史字段不重置） |
| DELETE | `/api/backup-jobs/:id` | backup:delete | 删除任务 |
| PUT | `/api/backup-jobs/:id/enabled` | backup:update | 启停（`{enabled}`） |
| POST | `/api/backup-jobs/:id/run` | backup:update | 立即执行一次 |
| POST | `/api/backup-jobs/restore-cloud` | backup:create | 云端恢复（`{account_id, remote_key}`，按 key 前缀分流，响应 `{scope, filename, message}`） |

- `scope`：`panel`（面板库，复用 VACUUM INTO 一致性快照）/ `app`（应用目录 tar.gz 打包，`scope_key` 为应用 key，安装目录名与 key 一致性校验防穿越）；
- `retention`：远端同前缀按 key 字典序（时间戳命名，天然有序）保留最近 N 份，超出删最旧；
- 执行结果落库（`last_status`/`last_message`/`last_remote_key`/`last_run_at`/`next_run_at`），
  成功发 `BackupSucceeded`、失败发 `BackupFailed` Outbox 通知（邮件渠道渲染见通知模块）；
- 云端恢复：远端 key 校验（禁穿越/反斜杠），按 key 前缀**分流**：
  - `panel/**/*.db` → 面板库：先拉取到本地备份目录再复用现有恢复流程（含恢复前二次备份保护，需重启面板生效）；
  - `app/<key>/*.tar.gz` → 应用数据：定位该 key 对应已安装应用的安装目录（布局守卫：
    `<apps>/<key>` 或 `<apps>/<key>/<container>`，其余路径拒绝），拉取备份包后解包到目标同级
    暂存目录并**交换就位**（旧目录先改名保留，成功清理、失败回滚）——还原前不删现有数据，
    应用数据不会出现半包状态；解包**整包拒绝**绝对路径 / `..` 穿越 / 符号链接 / 硬链接 /
    特殊文件（设备、fifo）条目，并受解包体积上限约束（防解压炸弹）。应用需先安装（否返回 404）。

## 16a. 运行环境与项目模块 `/api/runtimes` `/api/projects`（M-B 主线二 Stage A-3 · Doc/20 §5.6）

Runtime（多版本语言运行时）+ Project（用户项目）双层模型；进程守护走 systemd 模板单元
（`flame-proj-<id>.service`），零新增依赖；宿主机无 systemd（容器面板）时返回 503。

> **Stage B-2 交付（Doc/20 §5.3，契约零变更）**：安装来源按语言分流——
> php/java 走发行版包，node 走官方 `tar.xz` 目录式安装，python 走官方源码
> 包编译安装，go 走官方 `tar.gz`，tomcat 走官方 `tar.gz`（新增 `tomcat` kind）。
> 项目侧语言约定：`kind=python` 部署后自动创建 `<deploy_path>/.venv` 并按
> `requirements.txt` 摘要安装依赖；`kind=war` 要求绑定 `tomcat` 运行时，部署时
> 探测构建产物（项目根 / `target` / `build/libs`，同名 `<context>.war` 优先）
> 幂等落位 `<install_path>/webapps/<context>.war`（内容摘要一致则跳过，
> 变化原子替换；未探测到产物不阻断部署，落告警提示）。以上均不改动任何
> REST 请求/响应结构。
>
> **M-C 收尾（Python / Go / Java，契约零变更，仅声明版本更新）**：
> - 声明版本：java `8 / 11 / 17 / 21 / 25`（Eclipse Temurin，经 Adoptium API 解析到最新 GA 并校验
>   SHA-256，目录式安装到 `<install_path>`，不再占用发行版包）、python `3.10`–`3.14`（python.org 源码
>   编译，候选补丁版回落，装后自检 `ssl / sqlite3 / zlib / ctypes / bz2 / lzma`）、go `1.25 / 1.26 / 1.27`
>   （go.dev 索引解析到最新补丁版并校验 SHA-256）。旧布局（`<install>/go/bin`、系统 JDK）保持可用。
> - 部署约定：`kind=go` 且有 `go.mod` → `go build -trimpath -o .fp-bin/app .`；`kind=jar` 且带
>   `mvnw` / `gradlew` → 跳过测试构建，随后在 `target/` → `build/libs/` → 项目根中取最大的可运行 jar；
>   未配置启动命令（也无 Procfile）时按约定回填：Go `.fp-bin/app`、Java `java -jar <jar>`、Python
>   Django → gunicorn、FastAPI → uvicorn、Flask → gunicorn、兜底 `python app.py|main.py`；jar 项目
>   既无产物也无 wrapper 时部署返回 400 并说明原因。
> - 守护单元：Python 裸命令按 `.venv/bin` 实际入口补全，注入 `VIRTUAL_ENV` / `PYTHONUNBUFFERED=1` /
>   `UVICORN_HOST` / `UVICORN_PORT`；Java 补全 `java` 并注入 `JAVA_HOME` / `SERVER_PORT`；Go 补全 `go`。
>   联链变量同名时以联链为准。

### 16a.1 运行时 `/api/runtimes`（权限前缀 `runtime:*`）

| 方法 | 路径 | 权限 | 说明 |
|------|------|------|------|
| GET | `/api/runtimes` | runtime:read | 已登记运行时列表 |
| GET | `/api/runtimes/available?kind=php` | runtime:read | 可安装版本清单（声明版本 × 系统包/目录探测）。`kind` 支持 php/java/node/python/go/tomcat |
| GET | `/api/runtimes/kind/:kind` | runtime:read | 按语言过滤 |
| GET | `/api/runtimes/:id` | runtime:read | 运行时详情 |
| POST | `/api/runtimes/install` | runtime:create | 安装（`{kind, version}`；状态机 installing→installed，失败落 error 可重试，已安装 409） |
| POST | `/api/runtimes/:id/default` | runtime:update | 设为同语言唯一默认（要求已安装） |
| DELETE | `/api/runtimes/:id` | runtime:delete | 卸载（清理文件 + 删记录） |

### 16a.2 项目 `/api/projects`（权限前缀 `project:*`）

| 方法 | 路径 | 权限 | 说明 |
|------|------|------|------|
| GET | `/api/projects` | project:read | 项目列表 |
| GET | `/api/projects/:id` | project:read | 项目详情 |
| POST | `/api/projects` | project:create | 创建（`{name, kind, source, source_url?, deploy_path?, run_command?, port?, domain?}`；deploy_path 留空自动取 `/opt/flamepanel/projects/<name>`） |
| PUT | `/api/projects/:id` | project:update | 更新（运行中禁止改启动命令 409） |
| DELETE | `/api/projects/:id` | project:delete | 删除（运行中自动停止；部署目录保留——零锁定纪律） |
| POST | `/api/projects/:id/deploy` | project:create | 部署（local_path 校验目录在部署根内 / git clone --depth 1；`kind=war` 项目部署时探测构建产物并幂等落位 Tomcat `webapps/<context>.war`；部署 ≠ 启动，成功回落 stopped） |
| POST | `/api/projects/:id/start` | project:update | 启动（要求已配置启动命令；ensure_unit + systemctl start） |
| POST | `/api/projects/:id/stop` | project:update | 停止 |
| POST | `/api/projects/:id/restart` | project:update | 重启 |
| GET | `/api/projects/:id/status` | project:read | 实时进程状态（running/stopped/failed + 退出码 + 重启计数；容器模式自 `docker inspect`，原生模式守护器探测） |
| GET | `/api/projects/:id/logs` | project:read | 最近日志（原生模式 journalctl / 容器模式 `docker logs --tail`，最多 200 条） |
| POST | `/api/projects/:id/webhook` | 免 JWT（部署令牌守卫） | **Webhook push 触发部署**（`?token=` 或 `X-Webhook-Token` 头；兼容 GitHub/GitLab/Gitee/Gitea/通用 JSON，命中即视为一次 push） |
| POST | `/api/projects/:id/webhook/enable` | project:update | 开启/重置 webhook 部署（返回一次性令牌与 webhook_url；重复调用轮换令牌） |
| POST | `/api/projects/:id/webhook/disable` | project:update | 关闭 webhook 部署（回落 manual 并清空令牌） |
| POST | `/api/projects/:id/health-check` | project:update | 手动健康探测（轮询 `127.0.0.1:{port}{health_check_path}`，返回 `{healthy}`；未配置路径/端口 400） |
| GET | `/api/projects/:id/builder` | project:read | 构建器决策（dockerfile / buildpacks / runtime + 容器镜像名） |
| GET | `/api/projects/:id/links` | project:read | 项目数据库联链列表（不含连接串） |
| GET | `/api/projects/:id/links/available` | project:read | 可关联的数据库实例清单（仅 running 实例 + 推荐环境变量名） |
| POST | `/api/projects/:id/links` | project:update | 关联数据库（按需幂等建库；redis 跳过建库） |
| DELETE | `/api/projects/:id/links/:linkId` | project:update | 解除联链 |

安全约束：项目名仅字母数字与 `-/_`（1–64）；`source_url` 仅 http(s)/git@ 且禁 shell 元字符；
启动命令禁 shell 元字符并按「程序+参数」切分，可执行文件限定部署目录与 `/usr/bin`、`/usr/local/bin`
前缀内；同项目 deploy/start/stop 互斥（409）。

零停机切换（Stage A-9 · Doc/20 §5.8 ④）：项目增 `health_check_path`（站内路径，禁空白字符，
长度 ≤512）与 `deploy_strategy`（restart | rolling）两字段；rolling + 已配置健康检查端点时
restart 编排为「基线探测 → 重启 → 轮询新实例健康 → 通过切换 / 失败回滚旧实例 +
`ProjectZeroDowntimeFailed` 事件告警」，与 Dokku app.json checks 语义对齐。

Webhook 部署通道（Stage A-7 · Doc/20 §5.8）：项目增 `deploy_type`（manual | webhook）/
`auto_deploy`/`webhook_token` 三字段（响应不回显令牌明文，仅 `has_webhook_token` 布尔）。
webhook 通道仅支持 git 来源项目且必须开启 auto_deploy；webhook 端点免 JWT 但经
`auth_middleware` 集中守卫部署令牌（常量时间比较防时序侧信道）；开启时返回一次性令牌，
重复开启轮换令牌；push 部署语义——部署前项目在运行则拉取新代码后自动重启。

构建器自动协商（Stage A-8 · Doc/20 §5.8 ②）：部署时按顺序探测——项目根 `Dockerfile` →
容器模式（`docker build` 本地镜像 `flame-proj/<name>:<id>` + `docker run` 回环端口映射，
Dockerfile CMD 即启动命令）；否则 `pack-cli` 可用 → buildpacks 自动检测构建；否则回退
Runtime 模式（原生运行时 + run_command/Procfile）。结果落库 `build_type` 并随项目 DTO
透出；`GET /:id/builder` 查看当前决策；容器模式启停/重启/删除走 Docker 执行面（不生成
systemd unit）。

执行面分流（容器模式状态与日志收口）：`GET /api/projects/:id/status` 与
`GET /api/projects/:id/logs` 按 `build_type` 自动选择执行面——**容器模式**（dockerfile）分别
经 `docker inspect`（`Running`/`ExitCode`/`RestartCount`）与 `docker logs --tail <n>` 取数，
容器不存在时状态回落 `stopped`、日志返回空表（不报错）；**原生模式**（runtime/buildpacks）
维持守护器探测（systemd `show` / `journalctl -n`）。两路返回同构的 `ProcessStatus`
（state/exit_code/restarts）与日志行结构，API 契约不变；容器已退出且退出码非 0 时状态为
`failed`（`--restart on-failure` 重试耗尽的终态）。

服务联链（Stage A-10 · Doc/20 §5.8 ⑤，Dokku link 模式）：项目一键「关联数据库」——
关联时校验实例存在且运行中、同项目内环境变量名与实例均唯一；mysql/mariadb/postgres
幂等创建业务库（默认库名 = 项目名，redis 无库级语义跳过）。连接串在项目**启动与部署时**
现场生成并注入（原生模式 systemd unit `Environment=` / 容器模式 `docker run -e`）：
`mysql://user:pass@127.0.0.1:port/db`、`postgres://…`、`redis://:pass@127.0.0.1:port/0`。
安全收敛：凭据经运行时环境变量（`MYSQL_ROOT_PASSWORD` / `POSTGRES_SUPER_PASSWORD` /
`REDIS_PASSWORD`）参与拼接，仅内存与进程环境流转，**不落日志、不落库**；URL 特殊字符
percent-encode；主机恒为回环地址（数据库默认不暴露公网）；联链的实例被删除或停止时注入
空连接串并告警（应用启动失败可见，不静默缺失）；项目删除级联清理联链记录。

## 17. 定时任务模块 `/api/scheduled-tasks`

权限前缀：`scheduled_task:*`

| 方法 | 路径 | 权限 | 说明 |
|------|------|------|------|
| GET | `/api/scheduled-tasks` | scheduled_task:read | 任务列表 |
| POST | `/api/scheduled-tasks` | scheduled_task:create | 创建任务 |
| GET | `/api/scheduled-tasks/:id` | scheduled_task:read | 任务详情 |
| PUT | `/api/scheduled-tasks/:id` | scheduled_task:update | 更新任务 |
| DELETE | `/api/scheduled-tasks/:id` | scheduled_task:delete | 删除任务 |
| POST | `/api/scheduled-tasks/:id/run` | scheduled_task:execute | 立即执行 |
| POST | `/api/scheduled-tasks/:id/toggle` | scheduled_task:update | 启用/禁用 |

**创建任务请求体**：

```json
{
  "name": "每日备份",
  "command": "sqlite3 /opt/flamepanel/data/flamepanel.db \".backup '/opt/flamepanel/backups/db-$(date +%Y%m%d).db'\"",
  "schedule": "0 3 * * *",
  "enabled": true
}
```

> `schedule` 为标准 5 字段 cron 表达式（分 时 日 月 周），后端每 30 秒检查一次到期任务。

## 18. 备忘录模块 `/api/memos`

权限前缀：`memo:*`（read/create/update/delete）

| 方法 | 路径 | 权限 | 说明 |
|------|------|------|------|
| GET | `/api/memos` | memo:read | 列表（`?kind=memo\|todo`、`?done=true\|false`） |
| POST | `/api/memos` | memo:create | 创建 `{content, kind}` |
| PUT | `/api/memos/:id` | memo:update | 更新 `{content?, done?}` |
| DELETE | `/api/memos/:id` | memo:delete | 删除 |

## 19. 进程与系统指标 `/api/metrics`

| 方法 | 路径 | 权限 | 说明 |
|------|------|------|------|
| GET | `/api/metrics/processes` | 免认证 | 按 CPU 排序的进程 TOP 5 |
| GET | `/api/metrics/history` | node:read | 监控历史降采样曲线：`?range=1h\|6h\|24h\|7d\|30d`（缺省 `24h`） |

#### 监控历史 `GET /api/metrics/history`（S-07 · Doc/30 M-P）

> `MetricsHub` 只保留 120 点（6 min）内存历史；S-07 让后台写入器作为 Hub 的一个 broadcast 订阅者，把 3 s 快照在内存里聚合成 **1 min 点**（均值 / 峰值）落库，并在每个整点把上一小时的 1 min 点汇总成 **1 h 点**。单表 `metrics_history`，主键 `(resolution, bucket_start)`；1 min 点保留 7 天、1 h 点保留 90 天（随整点 rollup 一起裁剪）。写放大：每分钟一行 + 每小时一行。

| `range` | 分辨率 | `step_secs` | 理论点数 |
|---|---|---|---|
| `1h` / `6h` / `24h` | 1 min 点 | 60 | 60 / 360 / 1440 |
| `7d` / `30d` | 1 h 点 | 3600 | 168 / 720 |

响应示例（`range=1h`）：

```json
{
  "range": "1h",
  "step_secs": 60,
  "from": 1758700800,
  "to": 1758704400,
  "points": [
    { "bucket_start": 1758700800, "samples": 20, "cpu_avg": 12.4, "cpu_max": 31.0,
      "memory_avg": 46.1, "memory_max": 47.0, "disk_avg": 33.0, "load_one_avg": 0.52,
      "net_rx_avg_mbps": 0.01, "net_tx_avg_mbps": 0.02 }
  ]
}
```

- `to` 为当前桶终点（不含），`from = to - 窗口长度`；刚启动或历史不足时 `points` 少于理论点数，前端按 `bucket_start` 绘制即可（不要按下标对齐）。
- 1 min 视图会把**当前未落库的活动桶**也带上（右端不空一格）；1 h 视图只含已完成整点的 rollup。
- `range` 非法返回 `400 BAD_REQUEST`（错误信息列出合法取值）。
- 前端：`api/metrics.ts` `getMetricsHistory(range)` + `queryKeys.metrics.history(range)`；仪表盘时间范围切换（实时 6 min / 24 h / 7 d）作为后续切片消费此端点。

### 19a. 顶栏概览聚合 `/api/overview`（S-06 · Doc/30 M-P）

> 顶栏（AppHeader）原先每 10 s 并发三个 REST 轮询（节点列表 / 容器列表 / 操作日志）；现折叠为一个聚合端点，前端 30 s 轮询一次，服务端 10 s TTL 缓存 + 单飞（多页签并发只触发一次真实聚合，Docker daemon 每 TTL 至多被查询一次）。

| 方法 | 路径 | 权限 | 说明 |
|------|------|------|------|
| GET | `/api/overview/badges` | 仅需登录（分组按角色裁剪） | 节点 / 容器 / 任务 / 告警计数一次返回 |

响应示例：

```json
{
  "nodes": { "online": 2, "total": 3 },
  "containers": { "running": 4, "total": 6 },
  "tasks": { "running": 1, "pending": 0, "failed": 2 },
  "alerts": { "nodes_offline": 1, "certificates_expiring": 0, "tasks_failed": 2, "total": 3 },
  "generated_at": "2026-09-24T08:00:00Z",
  "cache_age_secs": 4
}
```

- 分组裁剪：`nodes` 需 `node:read`、`containers` 需 `docker:read`、`tasks` 需 `task:read`、`alerts.certificates_expiring` 需 `certificate:read`；无权限的分组为 `null`（前端据此隐藏对应徽标），与 `v-permission` 口径一致。缺少角色上下文时按全部不可见处理（fail closed）。
- 来源不可用也为 `null`：Docker 未安装 / daemon 超时（2 s）时 `containers` 为 `null`，其余分组照常返回。
- `alerts.total` 只汇总可见分项；`certificates_expiring` 统计 14 天内到期（含已过期）的证书。
- `cache_age_secs > 0` 表示命中服务端缓存；写操作后服务端可调用 `OverviewService::invalidate()` 让下次请求立即重算。

### 19b. 系统体检 `/api/inspection`（S-17 · Doc/30 M-C）

> 安装后（及任意时刻）对宿主机做一次体检：**内核信息** + **风险 / 漏洞分析**。判定逻辑全部是纯函数（`domain/inspection.rs`），采集在 `infrastructure/system_probe.rs`（文件优先；外部命令经 `SharedCommandRunner`，单条 3 s 超时、整体 15 s 预算，超时项呈现为 `unknown` 而不是让请求失败）。

| 方法 | 路径 | 权限 | 说明 |
|------|------|------|------|
| GET | `/api/inspection` | toolbox:read | 最近一次报告（服务端 10 min 缓存 + 单飞；无缓存时同步执行一次） |
| POST | `/api/inspection/run` | toolbox:execute | 忽略缓存立即重新体检，返回新报告 |

响应示例（节选）：

```json
{
  "generated_at": "2026-09-24T08:00:00Z",
  "duration_ms": 412,
  "cache_age_secs": 0,
  "data_version": "2026-09",
  "kernel": {
    "release": "6.1.0-25-amd64", "version": "Linux version 6.1.0-25-amd64 (…)", "arch": "x86_64",
    "os_name": "Debian GNU/Linux 12 (bookworm)", "os_id": "debian", "os_version_id": "12",
    "hostname": "web-01", "uptime_secs": 86400, "virtualization": "kvm",
    "reboot_required": false, "newest_installed_kernel": "6.1.0-25-amd64",
    "series": "6.1", "lts": true, "series_eol": "2033-12-31"
  },
  "summary": { "score": 82, "grade": "B", "total": 41, "passed": 34, "failed": 4, "unknown": 3,
               "critical": 0, "high": 1, "medium": 2, "low": 1 },
  "findings": [
    { "code": "sshd_root_login", "subject": null, "category": "access", "severity": "high",
      "status": "fail", "evidence": "PermitRootLogin yes", "heuristic": false },
    { "code": "exposed_service", "subject": "6379/tcp", "category": "network", "severity": "critical",
      "status": "fail", "evidence": "redis listening on 0.0.0.0:6379", "heuristic": false },
    { "code": "kernel_eol", "subject": null, "category": "kernel", "severity": "medium",
      "status": "pass", "evidence": "6.1 LTS until 2033-12-31", "heuristic": true }
  ]
}
```

- `findings[].code` 为稳定编码（前端据此映射三语言标题与建议），`evidence` 为语言无关的原始取证，`subject` 区分同一 `code` 下的多条（漏洞名 / 端口 / 账号 / sysctl 键）。
- `status`：`pass` 未发现风险 / `fail` 存在风险 / `unknown` 无法判定（文件不可读、命令不可用或超时）；`summary.score = 100 − Σ fail 项严重度权重`（critical 25 / high 15 / medium 8 / low 3 / info 0，下限 0），等级 A ≥ 90 / B ≥ 75 / C ≥ 60 / D ≥ 40 / F。
- `heuristic = true` 的结论（`kernel_eol` / `os_eol`）依赖内置生命周期表（`data_version`），发行版厂商内核（RHEL / Ubuntu 回移补丁）会降级为信息项；以厂商公告为准。
- 检查项分类：`kernel`（系列 EOL / 发行版 EOL / 待重启 / MAC）、`cpu`（`/sys/devices/system/cpu/vulnerabilities/*`）、`hardening`（13 个 sysctl 基线）、`access`（sshd 5 项、UID 0 / 空口令账户、`/etc/shadow` `/etc/passwd` `docker.sock` 权限）、`network`（防火墙、0.0.0.0 上的数据库 / 缓存 / 远程管理服务、公网监听总览）、`updates`（待装安全更新、无人值守升级、时间同步）、`storage`（根分区、swap）、`panel`（会话超时、`/metrics` 令牌、运行用户）。
- 权限：`toolbox:read` / `toolbox:execute` 为本切片新增的内置权限（viewer 含 read）；老库升级时 `seed_rbac` 只对**新增**的内置权限按角色默认集补授，不恢复管理员已显式收回的既有权限。

### 19c. 系统工具箱 · 进程管理 `/api/toolbox/processes`（S-12 · Doc/30 M-C）

> 进程表来自 `MetricsHub::process_table`（与仪表盘进程 TOP 共用同一个 `sysinfo::System`，2 s TTL + 单飞：多标签页 / 多用户并发只扫一次 `/proc`）；用户态线程已剔除，内核线程默认隐藏。过滤 / 排序 / 保护判定为纯函数（`domain/toolbox.rs`），结束进程经 `SharedCommandRunner` 执行 `kill -s <SIG> <pid>`（参数数组，不拼 shell）。

| 方法 | 路径 | 权限 | 说明 |
|------|------|------|------|
| GET | `/api/toolbox/processes` | toolbox:read | 进程列表；查询参数 `q`（pid 精确 / 名称·用户·命令行包含，不区分大小写）、`user`、`sort=cpu\|memory\|pid\|name\|started`（缺省 cpu）、`order=desc\|asc`、`limit`（缺省 100，1–500）、`include_kernel`；`sort` / `order` 非法 → 400 |
| POST | `/api/toolbox/processes/{pid}/kill` | toolbox:execute | 发送信号；请求体 `{ "signal": "TERM\|KILL\|HUP\|INT"（缺省 TERM）, "started_at": 1727164800 }`（字段均可省略） |
| GET | `/api/toolbox/host` | toolbox:read | 主机详情只读汇总：`system`（主机名 / 发行版 / 内核 / 架构 / 运行时长 / 开机时刻）、`cpu`（型号 / 逻辑与物理核数 / 1·5·15 分钟负载）、`memory`（内存与 Swap）、`time`（IANA 时区 / UTC 偏移 / 服务器当前时间）、`dns`（`/etc/resolv.conf`）、`hosts`（`/etc/hosts`，最多 200 条 + `hosts_truncated`）、`interfaces`（物理网卡在前，`virtual_iface` 标记 lo / veth / docker / br- 等，最多 64）、`disks`（真实文件系统、挂载点去重，最多 64）、`panel`（版本 / pid / uid / 是否 root）。只读文件与 sysinfo，不执行命令；任何事实不可探测时对应字段为空而不是报错 |

列表响应示例（节选）：

```json
{
  "items": [
    { "pid": 812, "ppid": 1, "name": "redis-server", "user": "redis", "uid": 110,
      "command": "/usr/bin/redis-server 127.0.0.1:6379 --requirepass ***",
      "cpu": 0.7, "memory_bytes": 9437184, "status": "Sleep", "started_at": 1727164800,
      "kernel_thread": false, "protected": null },
    { "pid": 1, "ppid": null, "name": "systemd", "user": "root", "uid": 0, "command": "/sbin/init",
      "cpu": 0.0, "memory_bytes": 12582912, "status": "Sleep", "started_at": 1727100000,
      "kernel_thread": false, "protected": "init" }
  ],
  "total": 143, "matched": 2, "total_memory_bytes": 1288490188, "panel_pid": 2210, "sampled_at": 1727168400123
}
```

- `protected`：不可结束的原因，`null` 表示可以结束——`init`（pid ≤ 1）/ `kernel_thread`（kthreadd 及其子线程）/ `panel`（面板自身）/ `panel_ancestor`（面板祖先进程，如容器 shim / 启动 shell，结束会连带面板退出）。`kill` 端点对这些目标一律 400。
- **命令行脱敏**：采集时即把 `--password=x`、`DB_TOKEN=x`、`--requirepass x`、`scheme://user:pass@host` 等敏感值替换为 `***`（viewer 也持有 `toolbox:read`），缓存中不保留明文。
- **防 pid 复用**：`kill` 前重新扫描进程表；带了 `started_at` 且与当前进程不一致 → 409，不发信号。进程已不存在 → 404。
- **提权**：目标进程与面板同属主时直接发信号；否则 `prefer_root`（非 root 面板自动 `sudo -n`）。系统层权限不足（`EPERM` / `sudo` 需要密码）返回 403 `AUTH_FORBIDDEN`，`message` 含 `insufficient privilege` 与配置指引（5xx 的 message 会被统一脱敏，故不用 500）；其余命令失败为 500。
- 响应 `KillProcessResponse { pid, name, signal, elevated }`；写操作由审计中间件自动记入操作日志。

## 20. 应用启动记录

| 方法 | 路径 | 权限 | 说明 |
|------|------|------|------|
| POST | `/api/app-store/installed/:id/launch` | app_store:read | 记录应用启动次数（常用应用排序） |

## 21. 统一任务接口 `/api/tasks`

> 统一 Task 状态机接口（Phase B1 扩展）。长耗时操作（应用安装、Web 引擎切换、批量节点操作）通过统一任务状态机追踪进度，前端可在此轮询/查询进度并取消任务。

权限前缀：`task:*`

| 方法 | 路径 | 权限 | 说明 |
|------|------|------|------|
| GET | `/api/tasks` | task:read | 任务列表（`?state=pending\|running\|success\|failed\|cancelled` 过滤） |
| GET | `/api/tasks/:id` | task:read | 任务详情 |
| POST | `/api/tasks/:id/cancel` | task:execute | 取消任务（Pending / Running → Cancelled） |
| POST | `/api/tasks/prune` | task:delete | 清理全部终态任务 |

> **B02b 持久化语义**：查询只展示已经确认落库的快照。取消和清理会等待存储确认；写入失败返回 HTTP 500 / `INTERNAL_ERROR`（详情只记日志），不返回虚假的成功状态或清理数量。清理逐条提交，中途失败时此前成功删除的记录不回滚，客户端应重新查询列表。原有 401 / 403、取消的 404 / 409 及成功响应格式不变。`Cancelled` 仍是任务记录状态，不代表真实执行进程已经停止；安装、引擎切换或节点操作也不会因最终记录落库失败而自动回滚/重放。详见 [Doc/24 §8](./24-事件与通知系统.md#8-统一任务中心确认式持久化b02b)。

**任务状态机（五态）**：`pending → running → success | failed | cancelled`（`pending` 亦可直接取消）

```json
{
  "id": 42,
  "kind": "install",
  "name": "安装 Nginx",
  "state": "running",
  "progress": 60,
  "message": "正在拉取镜像…",
  "created_at": "2025-08-14T10:00:00Z",
  "updated_at": "2025-08-14T10:01:00Z"
}
```

`kind` 取值：`install` / `engine_switch` / `batch_node` / `generic`；`state` 取值：`pending` / `running` / `success` / `failed` / `cancelled`；`progress` 为 0–100 整数。

## 22. Setup 初始化模块 `/api/setup`

> 首次部署 Setup 向导接口（Phase 9，见 [06-部署运维指南.md](./10-部署运维指南.md)）。初始化完成前面板不存在可认证用户，故两路由**均为公开路由**（不进 RBAC 权限表）：`status` 挂健康检查限流档（600 次/分钟/IP），`initialize` 挂登录限流档（5 次/分钟/IP）。初始化唯一判据为 settings 表 `setup_completed_at`；老库（已有用户但缺该键）视为已完成，启动期自动补写。

### `GET /api/setup/status`

初始化状态 + 向导所需环境信息（公开，无需认证）：

```json
{
  "status": "in_progress",
  "theme": "flame",
  "language": "zh-CN",
  "docker": true,
  "nginx": false
}
```

- `status`：`in_progress`（新装，待向导）/ `completed`（已初始化）/ `unattended`（无人值守模式，配置了 `OP_ADMIN_PASSWORD`）
- `docker` / `nginx`：尽力探测（bollard ping / `nginx -v`），探测失败静默返回 `null`，不返回 500
- `theme` 仅返回 4 个预设值之一（`flame` / `aurora` / `infinity` / `custom`），默认 `flame`

### `POST /api/setup/initialize`

两阶段初始化（公开，Login 档限流）。请求体：

```json
{
  "step": "database",
  "database": {
    "db_type": "sqlite",
    "host": "",
    "port": 3306,
    "name": "",
    "user": "",
    "password": "",
    "mysql_root_password": ""
  },
  "theme": "flame",
  "language": "zh-CN"
}
```

- `step`：`database` | `admin`
  - **database 步**：校验 `db_type`（`sqlite` / `mysql` / `mariadb`，非法返回 400）；SQLite 直接落库 `db_type` 设置；MySQL/MariaDB 尽力用 `mysql_root_password` 创建数据库与用户（失败不 500，向向导返回原因）。
  - **admin 步**（终态）：携带 `{ "step": "admin", "admin": { "username", "password" }, "theme", "language" }` —— 校验密码 ≥ 8 位，事务内并发兜底重查（重复初始化或已有用户返回 409），创建管理员、写入 `setup_completed_at` / `theme` / `language`、以访问域名（Host 头）为 SAN 签发自签证书（`data/certs/panel.{crt,key}`）、发布 `SetupCompleted` 事件并签发登录令牌。

成功响应（admin 步）：

```json
{
  "status": "completed",
  "message": "Setup completed",
  "token": "<jwt>",
  "username": "admin",
  "role": "admin"
}
```

> 响应头同时通过 `Set-Cookie: flame_refresh=<jwt>; HttpOnly` 下发 refresh token（前端不再读取响应体中的 refresh_token 字段）。

错误码：`400`（未知 step / 非法 db_type / 弱密码 / 缺 admin 段）、`409`（已初始化 / 已有用户 / 无人值守模式拒绝向导）。前端需在向导提交前查询 `status` 引导用户，避免直接触发 409。

## 23. Outbox 事件接口 `/api/outbox-events`

> 事件落库（Outbox）查询接口（Stage 9，事件驱动一致性，见 [12-事件与通知系统.md](./24-事件与通知系统.md)）。业务事件先写本地 Outbox 表持久化，再异步分发；该接口用于审计与排障查询。

权限：`outbox:read`

| 方法 | 路径 | 权限 | 说明 |
|------|------|------|------|
| GET | `/api/outbox-events` | outbox:read | 分页查询事件落库（`?type=AppInstalled` 按事件类型过滤） |

**OutboxEvent 字段**：`id`、`event_type`（如 `AppInstalled` / `UserLoggedIn`）、`payload`（JSON 结构化载荷）、`published`（是否已送达通知渠道）、`created_at`

```json
{
  "items": [
    {
      "id": 1,
      "event_type": "AppInstalled",
      "payload": "{\"key\":\"nginx\",\"version\":\"1.27\"}",
      "published": false,
      "created_at": "2025-08-14T10:00:00Z"
    }
  ],
  "total": 1
}
```

## 24. WebSocket 接口

| 路径 | 说明 | 消息格式 |
|------|------|----------|
| `/ws/metrics` | 系统指标实时推送 | 见下方 |
| `/ws/logs` | 系统日志实时推送 | 见下方 |
| `/ws/terminal` | Web 终端（`?node_id=` 为节点远程终端） | 见下方 |
| `/ws/agent/edge` | Agent Edge 反向通道（Agent 专用，`hello` 帧以节点 `auth_token` 鉴权，不走用户 JWT） | Doc/23 §8 |

> **WS 鉴权（首帧鉴权）**：所有 WebSocket 连接建立后，客户端必须在首帧发送 `{"type":"auth","token":"<access_token>"}` 完成鉴权。JWT 校验通过后还需通过 RBAC 权限检查（`/ws/terminal`→`terminal:execute`、`/ws/logs`→`log:read`、`/ws/metrics`→`node:read`）。鉴权失败立即关闭连接，不产生任何数据。前端 `utils/ws.ts` 封装自动发送首帧鉴权。

> **会话控制关闭码（Doc/33「WS 4001」）**：服务端以应用私有关闭码结束连接，客户端据此决定是否重连——
>
> | 关闭码 | 原因（Close reason） | 触发 | 客户端行为（`utils/ws.ts`） |
> |--------|---------------------|------|-----------------------------|
> | `4001` | `session expired` | 首帧缺失 / 非法；令牌过期、被吊销、改密后签发早于改密、用户被删除；续期帧换成其他用户的令牌 | 强制刷新 access token 后重连（保留退避）；刷新失败即清凭证回登录页，不再重连 |
> | `4003` | `forbidden` | 已认证但失去该通道权限（如角色被降级） | 停止重连，`onDenied` 通知页面；共享连接注册表移除该 URL，重新进入页面时可重建 |
>
> 连接期间服务端每 **60 s** 复核一次当前令牌（`/ws/metrics`、`/ws/logs` 的推送循环与 `/ws/terminal` 发送循环）。**带内续期**：客户端可随时再发一帧 `{"type":"auth","token":"<新 access token>"}`，同一用户且校验通过即替换当前令牌，长会话（终端）不会因 15 分钟的 access token 轮换被踢；续期失败按上表关闭。前端每 60 s 检查一次（`tokenForAuth()`，临期先刷新），令牌有变化才发续期帧。

> 浏览器 WebSocket API 无法在握手时携带自定义 Header，故 access token 不再通过 URL query 传递（避免泄露到浏览器历史/访问日志）。

> **背压与重同步（S-08）**：`/ws/metrics`、`/ws/logs` 为服务端 `broadcast` 扇出（容量分别为 64 / 256 帧）。客户端消费过慢导致 `RecvError::Lagged` 时，偶发滞后只记数并继续推送；**连续 3 次**滞后（期间无任何成功送达）判定为慢消费者，服务端发送控制帧 `{"type":"resync","reason":"lagged"}` 后主动关闭连接。客户端收到 `resync` 应立即重连（前端 `useSharedWebSocket` 注册表已内置：跳过退避直接 `reconnectNow()`，且不把该帧当作数据分发），重连后的 `init` 帧携带完整历史窗口即可补齐曲线。滞后计数通过 `GET /metrics` 暴露：`flamepanel_ws_lag_events_total{stream}`（滞后事件数）、`flamepanel_ws_lag_dropped_frames_total{stream}`（累计丢帧数）、`flamepanel_ws_slow_consumer_disconnects_total{stream}`（慢消费者断开次数），`stream` 取值 `metrics` / `logs`。

### 23.1 `/ws/metrics`

**服务端 → 客户端**（仅推送，无需发送消息）：

```json
{ "type": "init", "data": [ { "timestamp": 1750000000, "cpu_usage": 12.3, "memory_usage_percent": 45.6, ... } ] }
{ "type": "tick", "data": { "timestamp": 1750000060, "cpu_usage": 15.2, ... } }
```

`init` 为连接时的历史快照（最多 60 条，来自 `MetricsHistory` 环形缓冲），`tick` 为每 3 秒一次的实时快照（`spawn_metrics_collector` 采集间隔 3 秒）。

**MetricsSnapshot 字段**：`timestamp`(毫秒)、`cpu_usage`、`cpu_cores`、`memory_usage_percent`、`memory_total_mb`、`memory_used_mb`、`disk_usage_percent`、`disk_total_gb`、`disk_used_gb`、`load_one`、`load_five`、`load_fifteen`

> 前端订阅方式：`new WebSocket('/ws/metrics')`，监听 `message` 事件后 `JSON.parse`，按 `type` 分支处理 `init`/`tick`。

**节点指标（Doc/30 M-P）**：`/ws/metrics?node_id=<id>` 订阅远程节点（Agent 心跳并入的每节点 `MetricsHub`），帧格式同上，`tick` 随心跳每 10 秒一帧，`init` 最多 120 条。权限同为 `node:read`。`node_id` 非整数时握手直接 400；节点不存在在**首帧鉴权通过之后**以关闭码 **4004** 关闭（未认证连接无法借此探测节点 ID），前端收到 4004 与 4003 一样停止重连。节点被删除时其 Hub 释放、广播关闭，已连接的订阅随之结束。

### 23.2 `/ws/logs`

**服务端 → 客户端**：

```json
{ "type": "init", "data": [ { "id": 1, "source": "system", "level": "info", "message": "...", "metadata": null, "created_at": "..." } ] }
{ "type": "tick", "data": { "id": 2, "source": "system", "level": "warn", "message": "...", "metadata": null, "created_at": "..." } }
```

`init` 为连接时最近的日志列表，`tick` 为实时新增日志（`log_tx` broadcast 通道）。

### 23.3 `/ws/terminal`

**客户端 → 服务端**：

```json
{ "type": "input", "data": "ls -la\r" }
{ "type": "resize", "cols": 120, "rows": 30 }
```

**服务端 → 客户端**：

```json
{ "type": "output", "data": "total 64\r\n..." }
```

- `input`：写入终端（`data` 为字符串，含 `\r` 换行）；`resize`：调整 PTY 尺寸（缺省 80×24）。
- 连接建立时后端创建独立 `TerminalSession`，客户端断开后自动关闭会话并清理。

**节点远程终端（Agent Edge，batch-13）**：`/ws/terminal?node_id=<id>`，消息协议同上，shell 运行在节点上（经 Agent Edge 通道的 PTY）。

- 权限：首帧鉴权的 `terminal:execute` 之外还需 `node:execute`（admin 豁免），缺失以 **4003** 关闭；节点不存在 **4004**；`node_id` 非整数握手 400。
- 额外服务端帧：`{"type":"exit","code":0}`（shell 退出，会话结束）；`{"type":"error","code":"TERMINAL_DISABLED"|"TOO_MANY_TERMINALS"|"EDGE_OFFLINE","message":"…"}`。前端对前两者停止自动重连，`EDGE_OFFLINE` 退避重连等待 Agent 上线。
- 开 / 关审计 `OPEN_REMOTE_TERMINAL` / `CLOSE_REMOTE_TERMINAL`（target `nodes/<id>/terminal/<sid>`）。详见 Doc/23 §8.4。

## 24a. SSL 证书模块 `/api/certificates`（M-B 主线三 §6.1 · Stage 2）

权限前缀 `certificate:*`（admin 全部 / operator 除 delete / viewer 只读）。

| 方法 | 路径 | 权限 | 说明 |
|------|------|------|------|
| GET | `/api/certificates` | certificate:read | 证书列表（SAN/来源/到期/关联网站） |
| GET | `/api/certificates/{id}` | certificate:read | 证书详情 |
| PUT | `/api/certificates/{id}` | certificate:update | 更新 `auto_renew` / `website_id` |
| DELETE | `/api/certificates/{id}` | certificate:delete | 删除记录（不删落盘文件） |
| POST | `/api/certificates/issue/acme` | certificate:update | ACME 签发（HTTP-01/DNS-01） |
| POST | `/api/certificates/issue/self-signed` | certificate:update | 自签（多 SAN，内网场景） |
| POST | `/api/certificates` | certificate:update | 上传第三方证书（PEM 全链 + 私钥） |
| POST | `/api/certificates/{id}/bind` | certificate:update | 绑定/解绑网站（SAN 覆盖校验 + 443 配置原子改写） |
| POST | `/api/certificates/{id}/renew` | certificate:update | 手动续签（ACME 来源；自签/上传返回 400，签发失败 502） |

**ACME 签发请求**：`{ "domains": ["example.com", "*.example.com"], "directory_url?": "...", "dns01?": bool, "webroot?": "...", "website_id?": number, "dns_provider_account_id?": number }`
—— 泛域名必须 `dns01: true` 且已在服务端配置 `DnsProvider`（否则 400）；指定 `dns_provider_account_id` 时按账号解析厂商类型与凭据（解密在编排层完成），声明了厂商但无可用账号返回 400；无有效 ACME 账户/验证失败返回 502/400 语义错误。签发成功记录落 `data/certs/<主域名>/`（fullchain.pem + privkey.pem 0600），同域名集合重复签发走续签回写（幂等），发布 `CertificateIssued` 事件。

**自动续签（Stage 3 + Stage 5）**：后台 `cert-renew-tick` 每 6 小时扫描一次——`auto_renew = true` 且距到期 ≤30 天的 ACME 证书自动续签（HTTP-01 优先用绑定站点 webroot，泛域名或证书声明了 DNS 厂商时走 DNS-01），按证书 `dns_provider` 声明解析启用厂商账号（Stage 5，支持泛域名无人值守续签），复用同域名集合幂等回写；`CertificateExpiring` 事件按 30 天级（>7 天）/7 天级（≤7 天）两级发布，同一自然日同级别去重，经事件总线 → Outbox → 邮件通知。自签/上传证书不支持自动续签（手动续签返回 400）。

## 24b. DNS 厂商账号 `/api/dns-providers`（主线三 §6.1 · Stage 4）

权限前缀 `dns_provider:*`（admin 全部 / operator 除 delete / viewer 只读）。
DNS-01 验证的厂商 API 凭据托管；`credentials_json` 落库前经 SecretCipher
（AES-256-GCM）加密，响应一律不回显。

| 方法 | 路径 | 权限 | 说明 |
|------|------|------|------|
| GET | `/api/dns-providers` | dns_provider:read | 账号列表（不含凭据） |
| POST | `/api/dns-providers` | dns_provider:create | 创建账号（name + provider_type + credentials_json） |
| PUT | `/api/dns-providers/{id}` | dns_provider:update | 更新账号（同明文保留既有密文） |
| DELETE | `/api/dns-providers/{id}` | dns_provider:delete | 删除账号 |
| POST | `/api/dns-providers/{id}/test` | dns_provider:update | 连通性测试（四家厂商均走真实厂商 API 零副作用探测；结果落库） |

**provider_type 白名单**：`cloudflare`（api_token）/ `aliyun`（access_key_id + access_key_secret）/ `tencent`（secret_id + secret_key）/ `huawei`（access_key + secret_key）。测试失败返回 200 + `last_test_status: "failed"` + `test_message`（业务可预期结果）。

**真实厂商 API（Stage 5 + Step 3 全覆盖）**：`infrastructure/acme/dns_real.rs` 四家
DNS-01 Provider 全部交付：
- Cloudflare（API Token Bearer，v4 REST，`/user/tokens/verify` 探测）
- 阿里云（RPC 签名 HMAC-SHA1，AddDomainRecord 等，`DescribeDomains` 探测）
- 腾讯云（TC3-HMAC-SHA256 签名，CreateRecord/DescribeRecordList/DeleteRecord，
  `DescribeDomainList` 探测）
- 华为云（IAM project token 认证 + DNS v2 recordsets，token 缓存 1 小时，
  token 签发即探测）

通用语义：TXT 记录幂等写入/清理（同名同值复用，缺失不报错）、泛域名
`*.example.com` 归一化、凭据不落日志；签发与续签编排按证书 `dns_provider`
声明在启用账号中解析真实凭据（`build_provider` 组合根路由）。前端设置页新增
「DNS 厂商」Tab 管理账号，网站 ACME 签发表单 DNS-01 模式支持选择厂商账号。

## 24c. WAF 基础防护 `/api/waf/*`（主线三 §6.2）

权限前缀 `waf:*`（admin 全部 / operator 除 delete / viewer 只读）。
规则评估为面板进程内引擎（`webserver/waf.rs`），规则变更即时重建生效；并编译下发为 Web 引擎原生配置（Nginx / OpenResty / Apache / Caddy / OpenLiteSpeed，`webserver/waf_engines.rs`，Doc/20 §12.3 / §12.3a）。
拦截日志只记来源 IP / 请求行摘要 / UA（请求头与 body 不落库）。

| 方法 | 路径 | 权限 | 说明 |
|------|------|------|------|
| GET | `/api/waf/rules` | waf:read | 规则列表（`?website_id=` 过滤，0 = 全部） |
| POST | `/api/waf/rules` | waf:create | 创建规则（name + rule_type + pattern + config_json + action） |
| GET | `/api/waf/rules/{id}` | waf:read | 规则详情 |
| PUT | `/api/waf/rules/{id}` | waf:update | 更新规则（变更后引擎即时重建） |
| DELETE | `/api/waf/rules/{id}` | waf:delete | 删除规则（拦截日志保留，规则名快照仍可读） |
| GET | `/api/waf/logs` | waf:read | 拦截日志分页（`?page=&page_size=&website_id=`） |
| POST | `/api/waf/logs/clear` | waf:update | 清理日志（body `{ "website_id": 0 }`，0 = 全部） |
| GET | `/api/waf/status` | waf:read | 引擎诊断：`loaded_rules`（进程内启用规则数）/ `enforcement`（是否配置 Web 引擎下发）/ `ingest`（各引擎命中日志回灌：偏移、导入条数、最近时间、错误；OLS 无命中日志不列出） |
| POST | `/api/waf/apply` | waf:update | 下发规则到本机 Nginx / OpenResty / Apache / Caddy / OpenLiteSpeed（逐引擎报告 `WafApplyReport`，引擎不支持的规则列入 `skipped` 并附原因——如 Apache / Caddy / OLS 的 CC、OLS 的仅告警规则；未配置下发 503；规则变更时也会自动尽力下发，见 Doc/20 §12.3 / §12.3a） |

**拦截通知（`WafBlocked` 事件）**：阻断（action=block）命中发布领域事件
`WafBlocked`——经 Outbox 落库存档（`GET /api/outbox-events?type=WafBlocked` 可查）
并触发邮件通知（SMTP 未配置时静默跳过）；**按规则 300s 节流聚合**，事件携带的
`hit_count` 为本次通知覆盖的窗口累计拦截次数。`log`（仅告警）动作不触发通知。

**rule_type 白名单**：`cc`（固定窗口频率限制，config_json：`{"threshold":120,"window_secs":60}`）/
`sql_injection` / `xss`（内置 OWASP CRS 精选特征，pattern 可空）/
`ip_blacklist` / `ip_whitelist`（pattern 支持 CIDR 与逗号分隔多值）/
`ua_blacklist` / `ua_whitelist`（UA 子串，大小写不敏感）/ `path_block`（路径前缀）。

**评估语义**：白名单最高优先级（命中直接放行）→ 黑名单/特征/路径 → CC 限频；
默认动作 `log` 仅告警不阻断（观察期防误拦截），`block` 返回阻断结论。
`website_id = 0` 为全局规则；白名单优先级最高（§6.2 风险对策）。

## 24d. MCP 接入 `/api/mcp`（M-AI-1 / M-AI-2 · Doc/34）

**协议端点**（AI 客户端使用，`Authorization: Bearer fpk_…`，JWT 无效）：`POST /api/mcp`（JSON-RPC 2.0 · Streamable HTTP，
`initialize` 响应头下发 `Mcp-Session-Id`；通知 202；批量 ≤ 32）、`DELETE /api/mcp`（结束会话，204）、`GET /api/mcp`（405）。
协议细节、工具目录与客户端配置见 Doc/34。

**管理端点**（面板登录用户，前端「AI 接入中心」使用）：

| 方法 | 路径 | 权限 | 说明 |
|------|------|------|------|
| GET | `/api/mcp/tools` | mcp:read | 工具目录（含 `permission`、`risk`（read / write / destructive）与当前用户 `allowed`） |
| GET | `/api/mcp/prompts` | mcp:read | 运维技能库：`[{name, title, description, arguments:[{name, description, required}]}]`（与 MCP `prompts/list` 一致） |
| GET | `/api/mcp/prompts/{name}` | mcp:read | 渲染技能，查询参数即技能参数 → `{name, title, text}`；缺必填参数 / 未声明参数 400，未知技能 404 |
| GET | `/api/mcp/principals` | mcp:read | 凭证列表（`active` / `owner`） |
| POST | `/api/mcp/principals` | mcp:create | 签发（`{name, expires_in_days?, allow_write?}` → 201 `{principal, token, endpoint}`，token 仅此一次；`allow_write` 缺省 false） |
| POST | `/api/mcp/principals/{id}/revoke` | mcp:create | 吊销（同时关闭会话；吊销他人凭证另需 `mcp:delete`，否则 403） |
| GET | `/api/mcp/sessions` | mcp:read | 会话（`?principal_id&limit`） |
| GET | `/api/mcp/calls` | mcp:read | 调用审计分页（`?principal_id&session_id&tool&page&page_size`，最新在前） |
| GET | `/api/mcp/approvals` | mcp:read | 写操作审批单分页（`?status&principal_id&page&page_size`；`status` 非法 400） |
| GET | `/api/mcp/approvals/{id}` | mcp:read | 审批单详情（`preview` / 脱敏 `arguments` / `result` / `reason`；原始参数永不返回） |
| POST | `/api/mcp/approvals/{id}/approve` | mcp:update | 批准并立即执行（≤ 120 s），返回终态 `succeeded` / `failed`；批准者缺工具本身权限或高危工具缺 `mcp:delete` → 403；非待审批 / 已过期 / 凭证失效 / 签发者权限已变 → 409 |
| POST | `/api/mcp/approvals/{id}/reject` | mcp:update | 拒绝（可选 `{reason}`，≤ 500 字节，AI 经 `approval_get` 可见）；非待审批 → 409 |

**审批状态机**（M-AI-2）：`pending` →（批准，CAS）`executing` → `succeeded` / `failed`；`pending` → `rejected` / `expired`（30 分钟）。
每凭证同时最多 20 个待审批（超出时工具返回 `Rate limited`）。

## 24e. 面板 AI 助手 `/api/assistant`（M-AI-3 · Doc/32 §7）

默认关闭；未启用时 `POST /chat` 返回 503。

| 方法 | 路径 | 权限 | 说明 |
|---|---|---|---|
| GET | `/api/assistant/config` | mcp:read | 配置：`enabled` / `provider`（`openai` / `ollama`）/ `base_url` / `effective_base_url` / `model` / `has_api_key` / `temperature` / `max_steps` / `timeout_secs`；**API Key 永不返回** |
| PUT | `/api/assistant/config` | settings:update | 部分更新；`api_key` 缺省 = 保持原值，`""` = 清除；启用时必须填写 `model`；`base_url` 须 http(s)、不得内嵌 `user:pass@`；`max_steps` 1–12 |
| POST | `/api/assistant/test` | settings:update | 用已保存配置列出模型 → `{models, model_found}`；连接 / 鉴权失败 503（错误信息已脱敏） |
| POST | `/api/assistant/chat` | mcp:create | `{messages:[{role,content}], conversation_id?}` → `{conversation_id, reply, steps[], step_limit_reached, approval_ids[]}`；历史须 user / assistant 交替并以 user 结尾（≤ 40 条、单条 ≤ 32 KiB，否则 400）；同一用户并发对话 429；上游模型不可用 / 鉴权失败 / 响应异常 503 |

`steps[]` 每项 `{tool, arguments, ok, output, approval_id}`；写工具不执行，只生成审批单（见 §24d 审批接口）。

## 24f. IaC 只读配置导出 `/api/iac/export`（M-E 首片，#185）

| 方法 | 路径 | 入口权限 | 响应 |
|------|------|----------|------|
| GET | `/api/iac/export` | `settings:read` | `application/yaml; charset=utf-8`，附件 `flamepanel-config.yaml`，`Cache-Control: no-store` |

- `api_version: flamepanel/v1alpha1`、`kind: ConfigurationExport`；无生成时间，区块内按名称 / 源 ID 稳定排序。
- 三个区块分别检查 `website:read`、`database:read`、`app_store:read`，无权限时**完全省略**，不返回数量；有权限但没有数据时为 `[]`。仅有入口权限时返回版本 / kind 元数据。
- 网站白名单：`name`、`domain`、`root_path`、`engine`、`site_type`、`ssl_enabled`、`proxy_enabled`、`rewrite_preset`。
- 数据库白名单：`name`、`db_type`、`version`、`port`。应用白名单：`name`、`package_key`、`version`、`mode`、`port`。
- **不导出**代理目标、应用参数 / 环境变量 / 访问 URL、管理用户、原始配置文件、设置、证书 / 密钥、账号凭据、关联 ID、运行状态与审计时间。
- 这是**配置清单**，不是完整备份或可直接重建的部署声明；不包含导入 / 部署 / 启停行为。关联和遗漏配置需人工补齐；跨仓储读取不承诺事务快照。
- 未认证 401；没有入口权限 403。完整结构以生成的 [IaC API 文档](./api/iac.md) 为准。

```bash
# FP_TOKEN 是面板 JWT（不是 CNB 访问令牌）；PANEL_URL 是已部署面板地址。
curl -fsS -H "Authorization: Bearer $FP_TOKEN" "$PANEL_URL/api/iac/export" -o flamepanel-config.yaml
```

## 25. 快速调试示例

```bash
# 登录获取 token
TOKEN=$(curl -s -X POST http://localhost:8080/api/auth/login \
  -H 'Content-Type: application/json' \
  -d '{"username":"admin","password":"admin123"}' | jq -r .token)

# 用户列表
curl -s http://localhost:8080/api/users?page=1&page_size=10 \
  -H "Authorization: Bearer $TOKEN"

# 健康检查（无需认证）
curl -s http://localhost:8080/health
```

## 26. 权限速查

| 资源 | 动作 |
|------|------|
| user / node / website / docker / plugin / web_server / database / file / firewall / app_store / settings / memo / operation_log / log / backup / scheduled_task / project / runtime / certificate | read / create / update / delete（部分含 start / stop / reload / execute / config / enable / apply 等细粒度动作） |
| outbox | read |
| waf | read / create / update（清理日志）/ delete |
| task | read / execute / delete |
| mcp | read / create / update（审批 AI 写操作）/ delete（吊销他人凭证、批准高危操作） |

角色：`admin`（全部）、`operator`（除 delete 外全部）、`viewer`（仅 read）。

> 详细权限映射见 [07-权限体系设计.md](./07-权限体系设计.md)。
