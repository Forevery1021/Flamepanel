# 前端 API 模块说明

> FlamePanel 前端 HTTP 层规范：一模块一文件，统一走 `client.ts`（Axios 实例），
> 错误码归一。后端 OpenAPI 就绪后，`generated/` 类型自动生成，api 层只包一层。

## 目录

- [client.ts](#clientts--统一实例与鉴权)
- [模块清单](#模块清单)
- [错误处理与错误码对照表](#错误处理与错误码对照表)
- [数据层约定（vue-query）](#数据层约定vue-query)

---

## client.ts — 统一实例与鉴权

| 能力 | 说明 |
|------|------|
| baseURL | `/api`，`timeout 15000ms` |
| 鉴权 | 请求拦截器自动附加 `Authorization: Bearer <token>` |
| 401 刷新 | 单飞去重刷新 `refresh_token` 后重放原请求；刷新失败清空登录态跳 `/login` |
| 强改密 | 后端返回 `PASSWORD_CHANGE_REQUIRED` 时强制跳转改密页 |
| `ApiError` | 后端统一错误响应 `{ code, message, status }`（见 `flame-kernel/src/core/error.rs`） |
| `isNetworkError` | 断网 / 超时 / CORS 等非 HTTP 错误的判定工具 |

**约定**：业务模块一律 `import api from './client'`，不直接 `import axios`。

---

## 模块清单

### `auth.ts`
| 函数 | 方法 | 路径 | 说明 |
|------|------|------|------|
| `login` | POST | `/auth/login` | 登录，返回 access+refresh |
| `refreshToken` | POST | `/auth/refresh` | 刷新令牌 |
| `fetchMe` | GET | `/auth/me` | 当前用户信息 |
| `changePassword` | PUT | `/auth/password` | 修改密码 |

### `users.ts`
| 函数 | 方法 | 路径 | 说明 |
|------|------|------|------|
| `listUsers` | GET | `/users` | 分页列表 `?page=&page_size=` |
| `createUser` | POST | `/users` | 创建用户 |
| `updateUser` | PUT | `/users/{id}` | 更新用户（可改密/角色） |
| `deleteUser` | DELETE | `/users/{id}` | 删除用户 |

### `nodes.ts`
| 函数 | 方法 | 路径 | 说明 |
|------|------|------|------|
| `listNodes` | GET | `/nodes` | 分页节点列表 |
| `createNode` | POST | `/nodes` | 新增节点 |
| `updateNode` | PUT | `/nodes/{id}` | 更新节点 |
| `deleteNode` | DELETE | `/nodes/{id}` | 删除节点 |
| `nodeStatus` | GET | `/nodes/{id}/status` | 节点状态 |
| `nodeMetrics` | GET | `/nodes/{id}/metrics` | 节点指标 |
| `remoteExecute` | POST | `/nodes/{id}/exec` | 远程执行命令 |
| `remoteAction` | POST | `/nodes/{id}/action` | Agent 动作枚举（白名单安全） |
| `batchExecute` | POST | `/nodes/batch-exec` | 批量执行 |
| `remoteListFiles` | GET | `/nodes/{id}/files` | 远程文件列表 |
| `remoteDownloadFile` | GET | `/nodes/{id}/files/download` | 远程下载 |
| `remoteUploadFile` | POST | `/nodes/{id}/files/upload` | 远程上传 |
| `registerNode` | POST | `/nodes/register` | Agent 注册 |

### `files.ts`
| 函数 | 方法 | 路径 | 说明 |
|------|------|------|------|
| `listFiles` | GET | `/files?path=` | 目录列表 |
| `readFile` | GET | `/files/read` | 读文件 |
| `writeFile` | POST | `/files/write` | 写文件 |
| `createFile` / `createDir` | POST | `/files/create` | 新建文件/目录 |
| `deleteFile` | DELETE | `/files` | 删除（可递归） |
| `renameFile` | PUT | `/files/rename` | 重命名 |
| `chmodFile` | POST | `/files/chmod` | 改权限 |
| `uploadFile` | POST | `/files/upload` | 上传 |
| `downloadFile` | GET | `/files/download` | 下载 |

### `docker.ts`（容器/镜像/网络/卷/Compose）
| 函数 | 方法 | 路径 | 说明 |
|------|------|------|------|
| `listContainers` | GET | `/docker/containers` | 容器列表 |
| `getContainer` / `inspectContainer` | GET | `/docker/containers/{id}` | 容器详情 |
| `startContainer`…`killContainer` | POST | `/docker/containers/{id}/…` | 启停/重启/暂停/强杀 |
| `removeContainer` / `pruneContainers` | DELETE | `/docker/containers` | 移除/清理 |
| `containerLogs` | GET | `/docker/containers/{id}/logs` | 容器日志 |
| `containerStats` | GET | `/docker/containers/{id}/stats` | 实时占用 |
| `listImages` / `pullImage` / `removeImage` / `tagImage` / `pruneImages` | — | `/docker/images` | 镜像管理 |
| `listNetworks` / `createNetwork` / `removeNetwork` / `connectNetwork` / `disconnectNetwork` / `pruneNetworks` | — | `/docker/networks` | 网络管理 |
| `listVolumes` / `createVolume` / `removeVolume` / `pruneVolumes` | — | `/docker/volumes` | 卷管理 |
| `listComposeProjects` | GET | `/docker/compose` | Compose 项目 |

### `websites.ts` / `webServers.ts`
| 模块 | 函数 | 说明 |
|------|------|------|
| websites | `listWebsites` `createWebsite` `updateWebsite` `deleteWebsite` `switchWebsiteEngine` | 网站 CRUD + 引擎切换 |
| webServers | `listEngines` `listWebServers` `getWebServer` `createWebServer` `updateWebServer` `deleteWebServer` `startWebServer` `stopWebServer` `restartWebServer` `reloadWebServer` `configtestWebServer` `getWebServerConfig` `updateWebServerConfig` `switchWebServerEngine` `applyWebServerPreset` `listPresets` `detectNativeWebServers` `nativeInstallWebServer` `nativeUninstallWebServer` `nativeAutostartWebServer` `setWebServerAutostart` `nativeStatusWebServer` | Web 服务器与引擎管理 |

### `databases.ts`
| 函数 | 说明 |
|------|------|
| `listDatabases` `getDatabase` `deleteDatabase` | 数据库实例 CRUD |
| `installMysql` `installRedis` `uninstallDatabase` | 安装/卸载 |
| `startDatabase` `stopDatabase` `restartDatabase` `checkDatabaseStatus` | 启停/状态 |
| `listInternalDatabases` `createInternalDatabase` `dropInternalDatabase` | 内部库管理 |
| `createDatabaseUser` `dropDatabaseUser` | 用户管理 |

### `appStore.ts`（应用商店与插件）
| 函数 | 说明 |
|------|------|
| `listPackages` `getPackage` `getPackageVersion` | 应用市场检索 |
| `installApp` `importPackage` `batchImportPackages` `uninstallApp` `upgradeApp` | 安装/批量导入/卸载/升级 |
| `listInstalledApps` `getInstalledApp` `launchApp` `getAppLogs` | 已装应用 |
| `listWasmBuiltins` | WASM 内置插件 |

### `tasks.ts`（统一任务进度，Phase B1）
| 函数 | 方法 | 路径 | 说明 |
|------|------|------|------|
| `listTasks` | GET | `/tasks` | 任务列表（可按 state 过滤） |
| `getTask` | GET | `/tasks/{id}` | 单个任务详情 |
| `cancelTask` | POST | `/tasks/{id}/cancel` | 取消任务（Pending/Running→Cancelled） |
| `pruneTasks` | POST | `/tasks/prune` | 清理全部终态任务 |

### `plugins.ts`
| 函数 | 说明 |
|------|------|
| `listPlugins` `getPlugin` | 插件列表/详情 |
| `loadPlugin` `unloadPlugin` `reloadPlugin` | 加载/卸载/重载 |
| `enablePlugin` `disablePlugin` | 启停 |
| `executePlugin` | 调用插件函数 |
| `getPluginMetrics` `resetPluginMetrics` | 指标 |
| `listPluginSettings` `setPluginSetting` `getPluginSetting` | 配置 |

### `logs.ts` / `health.ts` / `metrics.ts`
| 函数 | 说明 |
|------|------|
| `listOperationLogs` | GET `/operation-logs`（审计日志，分页+action 过滤） |
| `listSystemLogs` | GET `/system-logs`（系统日志） |
| `fetchHealthDetail` | GET `/health`（健康详情） |
| `listTopProcesses` | GET `/metrics/top-processes`（进程 TOP） |

### `scheduledTasks.ts` / `backups.ts` / `settings.ts` / `memos.ts` / `firewall.ts`
| 模块 | 说明 |
|------|------|
| scheduledTasks | 定时任务 CRUD + 立即执行/启停 |
| backups | 备份 列表/创建/下载/恢复/删除 |
| settings | 键值设置 列表/读取/更新 |
| memos | 备忘录 列表/创建/更新/删除 |
| firewall | 防火墙规则 CRUD + 启停/排序/状态 |

### `queryKeys.ts`
统一 `@tanstack/vue-query` 查询键定义（模块级前缀），供缓存失效/写缓存共用。
约定：同一查询键共享缓存，写操作通过 `invalidates` 或 `setQueryData` 更新。

---

## 错误处理与错误码对照表

### 统一处理

`client.ts` 的响应拦截器做 401 刷新重放；业务层捕获后：

1. 优先调用 `getErrorMessage(err, fallback)`（`src/utils/error.ts`）：
   - 后端返回 `code` 时，优先查 i18n `common.error.<code>` 本地化文案；
   - 无映射则回退后端 `message`；
   - 断网/超时（无 HTTP 响应）→ `common.error.NETWORK_ERROR`；
   - 最后回退调用方 fallback。
2. 交互反馈用 `FpToast`（`useFpToast()`）。
3. 列表页三态（loading/error/empty）用 `FpStatePanel`。

### 错误码对照表（前端 `locales` ↔ 后端 `ErrorCode`）

前端本地化位于 `src/locales/*.ts` 的 `common.error` 字段；后端定义见
`flame-kernel/src/core/error.rs`。**新增后端错误码时，三端（后端、前端 locales、本表）同步维护。**

| 错误码 | HTTP | 后端枚举 | 前端 i18n | 说明 |
|--------|------|----------|-----------|------|
| `AUTH_UNAUTHORIZED` | 401 | `AuthUnauthorized` | 未登录或登录已过期 | 未认证/令牌失效 |
| `AUTH_FORBIDDEN` | 403 | `AuthForbidden` | 没有操作权限 | 无权限访问 |
| `PASSWORD_CHANGE_REQUIRED` | 403 | `PasswordChangeRequired` | 首次登录需改密 | 强制改密（前端拦截跳转） |
| `NOT_FOUND` | 404 | `NotFound` | 请求的资源不存在 | 资源不存在 |
| `BAD_REQUEST` | 400 | `BadRequest` | 请求参数错误 | 请求错误 |
| `VALIDATION_ERROR` | 400 | `ValidationError` | 参数校验失败 | 字段校验失败 |
| `CONFLICT` | 409 | `Conflict` | 资源冲突 | 唯一冲突/状态冲突 |
| `SERVICE_UNAVAILABLE` | 503 | `ServiceUnavailable` | 服务暂不可用 | 依赖服务不可用 |
| `INTERNAL_ERROR` | 500 | `Internal` | 服务器内部错误 | 未预期错误 |

**前端附加码（非后端返回，前端拦截器合成）：**

| 错误码 | 场景 | 前端 i18n |
|--------|------|-----------|
| `NETWORK_ERROR` | 断网 / CORS / 服务未启动 | 网络不可用，请检查网络连接 |
| `TIMEOUT` | 请求超时 | 请求超时，请稍后重试 |

---

## 数据层约定（vue-query）

- 只读接口统一走 `useApiQuery`（`src/composables/useApiQuery.ts`）。
- 查询键用 `queryKeys.<module>.list(...)`，同屏共享缓存、分页 `keepPreviousData` 切页不闪空。
- 写操作统一走 `useApiMutation` + `invalidates` 对应 key。
- WebSocket 推送优先 `setQueryData` 写缓存，其次 `invalidateQueries`，避免双源轮询。
- 列表页统一范式：工具栏（主操作 + 筛选）+ `FpTable`（loading/empty/error 三态）+ `FpPagination`。
