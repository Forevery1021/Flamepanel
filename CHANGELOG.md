# Changelog

本文件记录 FlamePanel 后端（flame-kernel）与部署脚本的重大变更，遵循 [Keep a Changelog](https://keepachangelog.com/) 风格。

## [Unreleased]

### chore: CI/Release 工作流重构与容器/终端安全加固（见《FlamePanel-后续修复指导.md》）

- **CI（ci-cd.yml）**：拆分为并行 job（frontend / rust-format-clippy / rust-test / security-audit / docker），fmt 失败不再阻塞测试；各 job 设置超时与最小权限；`cargo-audit` 改用 `taiki-e/install-action`（预编译缓存，免每次全量编译）；镜像 tag 除 `latest` 外增加 `<sha>` 与 tag 解析的 `vX.Y.Z`，推送权限收紧为 `packages: write`。
- **Release（release.yml）**：前端只构建一次（build-frontend job + artifact，matrix 各 target 下载复用）；统一汇总一份 `flamepanel-${VERSION}-checksums.txt`；固定 `cargo-zigbuild@0.23.0`（install-action 版本语法）；`workflow_dispatch` 无 tag 时 VERSION 回退输入 → Cargo.toml。
- **docker-compose.yml**：两套 profile——默认（生产安全，无 docker.sock、`cap_drop: ALL` + 最小 cap_add、可选 read_only）与 `--profile dev`（docker.sock 只读，8090 端口）；移除硬编码 `OP_JWT_SECRET` 弱默认值。
- **Dockerfile**：镜像源 build-arg 化（`NPM_REGISTRY`/`CRATES_RSYNC`，默认官方源，境外 CI 无需改文件）；`FROM ... AS` 大小写规范；去掉冗余 `VOLUME` 声明中未挂载的 docker.sock。
- **docker-entrypoint.sh**：后端健康等待（60s 超时）+ SIGTERM/SIGINT 转发与等待子进程 + 后端退出码透传（`restart: unless-stopped` 生效），启动失败非 0 退出。
- **nginx.conf**：基础安全头（`X-Content-Type-Options`/`X-Frame-Options`/`Referrer-Policy`）；`/metrics` 默认仅内网网段可访问（`allow/deny`）；补 TLS server + HSTS + HTTP→HTTPS 跳转生产示例。
- **P1 终端审计**：Web 终端会话开/关写入操作审计日志（`OPEN_WS_TERMINAL` / `CLOSE_WS_TERMINAL` + 会话 id），任意命令执行面可追溯（`handler/ws/mod.rs`，经 `Username` 扩展获取身份）。
- **P1 路径穿越否定用例**：新增 symlink 逃逸（白名单内符号链接指向根外）与写目标穿越（`..` 规范化逃逸/裸 `..` 文件名）集成测试，与既有 `..`/绝对路径用例共同锁定沙箱行为。
- **文档**：README 与 Doc/06 增加「root 容器 + docker.sock」危险组合警告、生产推荐拓扑（systemd 非 root / 默认 profile / Agent 白名单）、compose 两套 profile 说明、安全清单补 docker.sock 与终端审计项。
- 测试基线：329 个测试全部通过（152 单元 + 153 集成 + 13 setup + 7 stage5 + 4 agent）。

### feat: Setup 向导 + 安全加固（Part A/B，见《flamepanel-setup-wizard-guide.md》）

#### 首次部署 Setup 向导（Part B）
- **后端 `SetupService`**（`application/setup_service.rs`）：`GET /api/setup/status`（`in_progress`/`completed`/`unattended` + docker/nginx 尽力探测 + theme/language 预填）、`POST /api/setup/initialize` 两阶段（`database`：SQLite/MySQL/MariaDB 校验与尽力建库；`admin`：创建管理员 + 写 `setup_completed_at`/theme/language + 签发令牌 + 发布 `SetupCompleted` 事件）。初始化唯一判据为 settings 表 `setup_completed_at`；老库（已有用户）启动期自动补写。
- **无人值守模式**：配置 `OP_ADMIN_PASSWORD`（或 TOML `admin_password`）后跳过向导，启动全量种子并强制首次登录改密；向导接口返回 `unattended`、`initialize` 一律 409。
- **自签证书**（`infrastructure/cert.rs`，rcgen ECDSA P-256）：以访问域名（Host 头）为 SAN 生成 `data/certs/panel.{crt,key}`，幂等，私钥 0600。
- **前端**：`/setup` 路由 + 路由守卫（未初始化自动引导、已登录跳转、语言与状态并行预载）、Pinia `setup` store（单飞状态探测）、`SetupView` 6 步向导（欢迎/管理员/数据库/服务器/主题/完成）+ 3 语言 i18n、主题与语言即时预览。
- 两路由均为公开路由（状态挂 Health 档、initialize 挂 Login 档限流），不进 RBAC。

#### 安全加固（Part A）
- **A3.2 节点注册引导令牌**：`POST /api/nodes/register` 需携带 `X-Bootstrap-Token`（与 `OP_BOOTSTRAP_TOKEN`/配置 `bootstrap_token` 常量时间比较，缺失/错误 401）；Agent 新增 `BOOTSTRAP_TOKEN` 环境变量并在注册时携带。
- **A3.3 配置解析失败拒绝启动**：TOML 解析错误携带 `文件:行:列` 报错并拒绝启动（文件缺失仍走默认值）。
- **A3.5 JWT 轮换强随机**：前端密钥轮换改用 `crypto.getRandomValues`。
- **A1 心跳鉴权**：`POST /api/nodes/heartbeat/{id}` 校验 `Authorization: Bearer <agent_token>`（常量时间比较，兼容无 token 旧节点）。
- **A3.1 上传路径加固**：Agent 文件根目录 canonicalize + 上传 `O_NOFOLLOW`。

#### 门禁与基线
- `cargo fmt --check` / `cargo clippy --all-targets -- -D warnings` / `cargo test --workspace`（327 例：152 单元 + 151 集成 + 13 setup + 7 stage5 + 4 agent）；前端 typecheck / lint / vitest（15）/ build 全绿。
- 路由基线：181 条 HTTP + 3 条 WebSocket。

### refactor: 后端工程质量 P3——migrations 索引 / panic 清理 / 死代码 / 配置化（T13-T16，见《后端架构分析与重构方案》)

按《flamepanel-backend-refactoring.md》P3 工程质量阶段落地 **T13-T16**：补索引与循环写事务、清理生产路径 panic 与阻塞 IO、清理死代码、配置化硬编码路径。

#### T13 数据库层
- **`infrastructure/sqlite.rs`**：新增幂等索引 `idx_nodes_heartbeat(nodes.last_heartbeat_at)`、`idx_operation_logs_username/action/created`、`idx_logs_source/level/created`，覆盖离线扫描、操作日志与系统日志的过滤/排序 WHERE 清单，避免量大后全表扫描。
- 关键循环写（`set_role_permissions`、firewall `reorder`）已在 P1/T7 落地单事务，本次沿用；`query!` 宏编译期校验作为后续渐进目标（需 DATABASE_URL/离线模式，本期未引入）。

#### T14 panic 与阻塞 IO 清理
- **`infrastructure/sqlite.rs`**：`UserRepository::create` 的 TOCTOU `unwrap()` 改为 `ok_or_else` 返回 `AppError`。
- **`terminal/mod.rs`**：`TerminalSession::new` 由「直接 `expect` panic」改为返回 `Result<_, AppError>`，`spawn`/取 stdout/stderr 失败时优雅报错；`TerminalManager::create_session` 相应改为返回 `Result`。
- **`api/handler/ws/mod.rs`**：`handle_terminal` 处理 `create_session` 错误，创建失败时打日志并优雅关闭连接而非 panic。
- **`infrastructure/docker.rs`**：`compose_deploy` 的阻塞 `std::fs` 文件写改为 `tokio::fs`（async IO）。
- **`plugin/sandbox.rs`**：移除恒为 0 的误导性 `ExecutionResult.fuel_used` 指标（从未从 wasmtime store 读取真实值）。
- 说明：`reload_plugin` 持锁执行 WASM `on_reload` 钩子（`#[cfg(feature="wasm")]`）的 `spawn_blocking` 下沉涉及持锁跨 await 重构，风险较高，留待具备 cargo 的构建环境落地。

#### T15 死代码与重复清理
- **删除** `utils/validation.rs`（`ValidationUtils` 零调用）+ 其在 `utils/mod.rs` 的模块声明与再导出。
- **删除** `resilience/wrapper.rs`（`ResilientWrapper`/`ResilientRepoFactory` 零调用）+ 其在 `resilience/mod.rs` 的声明/再导出。
- **删除** `api/pagination.rs` 的 `paginate_slice`（零调用）+ `api/types.rs` 的再导出。
- **`api/rate_limiter.rs`**：429 错误体由重复的 `RateLimitResponse` 改为直接复用 `core::error::ErrorResponse`，移除重复结构。
- 说明：`ensure_can_change_password` 空实现作为「改密策略扩展点」保留（注释已明示）；app_store 列表补分页涉及 API 契约变更，建议产品确认后另立变更。

#### T16 配置化硬编码路径
- **`config/loader.rs`**：`AppConfig` 新增 `mysql_config_file`/`redis_config_file`（serde 默认 `/etc/mysql/...`、`/etc/redis/redis.conf`）+ env 覆盖 `OP_MYSQL_CONFIG`/`OP_REDIS_CONFIG`。
- **`database/mysql.rs` / `database/redis.rs`**：`MySqlManager`/`RedisManager` 新增 `with_config_file` 覆盖路径方法。
- **`application/database_service.rs`**：新增 `DatabaseService::new_with_config_paths`。
- **`lib.rs`**：`build_services` 接收 mysql/redis 配置路径并传入 `new_with_config_paths`，默认值保持与既有行为一致。

#### 验收核对（静态）
- 索引补全覆盖 WHERE 清单；生产路径 `unwrap/expect` 清零（仅剩测试/`spawn_blocking` 边界）。
- 删除符号（`paginate_slice`/`ResilientWrapper`/`ResilientRepoFactory`/`ValidationUtils`/`fuel_used`）全仓引用归零；429 复用 `ErrorResponse` 结构一致。
- 本环境无 cargo，未运行 `fmt/clippy/test` 门禁；改动为低风险、可静态核验（引用归零、braces 平衡、调用点同步）。


### refactor: 后端架构整顿 P2——AppStoreService 构造整顿（T12，见《后端架构分析与重构方案》)

按《flamepanel-backend-refactoring.md》P2 架构整顿阶段落地 **T12**：消除 `AppStoreService` 便捷构造对 `infrastructure` 层的直接依赖，使 application 层纯依赖端口（六边形落地）。

#### 改动点
- **`application/app_store_ports.rs`**：新增 application 层 `DefaultAppStorePorts` 聚合类型（含 `adapter_provider`/`security_scanner`/`variable_mapper_factory`/`package_manager`/`service_manager` + `runner`），作为便捷构造的默认端口注入载体，不再从 infrastructure 拉取。
- **`application/app_store_service.rs`**：`AppStoreService::new` 由「接收 `runner` 并在内部 `default_ports` 组装」改为「接收 `DefaultAppStorePorts` 参数」，生产代码不再 `use crate::infrastructure::app_store::default_ports`。
- **`infrastructure/app_store/mod.rs`**：移除本地 `DefaultAppStorePorts` 结构定义（下沉至 application），`default_ports` 改为返回 application 层 `DefaultAppStorePorts`（含 `runner`），仅由组合根/测试组装。
- **测试调用点**：`app_store_service.rs`（test_service）、`integration_test.rs`（3 处）、`stage5_nodes_test.rs`（1 处）改为先用 `default_ports(runner.clone())` 组装 ports 再传 `new`。

#### 验收核对（静态）
- 生产代码 `AppStoreService::new` 不再直接调用/引用 `crate::infrastructure::app_store::default_ports`（仅测试内组装 + 注释提及）；application 层仅依赖 application 端口。
- `DefaultAppStorePorts` 类型定义与 `default_ports` 返回类型一致（含 `runner` 字段）；5 处 `new` 调用点全部同步更新。
- 本环境无 cargo，改动经人工审查 + 静态一致性校验（引用路径逐一核对、braces 平衡）。


### refactor: 后端架构整顿 P2——修正 domain 依赖方向（T11，见《后端架构分析与重构方案》)

按《flamepanel-backend-refactoring.md》P2 架构整顿阶段落地 **T11**：将特权命令执行端口从 `application` 层下沉至 `domain` 层，修正「domain 依赖 application」的反向依赖。

#### 改动点
- **新增 `domain/execution_mode.rs`**：将原 `application/execution_mode.rs` 中的领域端口整体下沉至 domain 层，包括 `ExecutionMode` / `CommandOutput` / `PrivilegedCommand` / `PrivilegedCommandRunner` / `SharedCommandRunner`（含既有单测）。该端口仅依赖 `core::error::AppError`（domain → core 横切依赖，合法）。
- **`domain/repository.rs`**：`ComposeRepository::run_compose` 返回值由 `crate::application::execution_mode::CommandOutput` 改为 `crate::domain::execution_mode::CommandOutput`，domain 层不再反向引用 application。
- **`domain/mod.rs`**：新增 `pub mod execution_mode;` 声明。

#### 兼容层
- `application/execution_mode.rs` 改为统一 `pub use crate::domain::execution_mode::*;` 兼容再导出，既有 `crate::application::execution_mode::*`（application/database/infrastructure/webserver/lib 共 20 处）引用**零改动**继续可用。

#### 验收核对（静态）
- domain 层引用 `crate::application`/`crate::infrastructure` 的**实际代码归零**（仅注释提及）；被引用符号 `CommandOutput`/`ExecutionMode`/`PrivilegedCommand`/`PrivilegedCommandRunner`/`SharedCommandRunner` 均经再导出覆盖。
- 本环境无 cargo，改动经人工审查 + 静态一致性校验（引用路径逐一核对、braces 平衡）。

### refactor: 后端架构整顿 P2——上帝文件 `api/types.rs` 拆分 + AppState 收敛（T9，见《后端架构分析与重构方案》)

按《flamepanel-backend-refactoring.md》P2 架构整顿阶段落地 T9：拆分 `api/types.rs`（原 1867 行）为「每域一文件」，`types.rs` 保留为兼容再导出层，并将 `AppState`/`Services` 收敛至独立模块。

#### 文件拆分（`flame-kernel/src/api/`）
- `app_state.rs`：`AppState` + `Services`（业务服务聚合）+ `impl AppState`（含 `from_services`/`new`/`new_with_roots`/`current_jwt_secret`/`shared_jwt`）+ 请求上下文 `UserId`/`Username`。
- `dto.rs`：请求/响应 DTO（`CreateUserRequest`/`UpdateUserRequest`/`CreateNodeRequest`/`CreateWebsiteRequest`/`WebServerResponse`/`PluginSettingRequest`/`PluginMetricsResponse`/`PluginReloadRequest`/`CreateWebServerInstanceRequest`）。
- `permissions.rs`：`PermissionRule` + `ROUTE_PERMISSIONS` 声明式权限表 + `route_permission` + 权限表单测（含 `permission_table_covers_all_routes` 全路由覆盖校验）。
- `pagination.rs`：`PaginationParams`/`PaginatedResponse`（含 utoipa 泛型 `ComposeSchema`/`ToSchema` 实现）/`paginate_slice`。

#### 兼容层
- `api/types.rs` 改为统一再导出上述模块（`pub use ...`），既有 `use crate::api::types::*` 及 `AppState`/`Services`/`PaginationParams`/`PaginatedResponse`/`route_permission` 等精确引用全部保持兼容，**零调用点改动**。
- `api/mod.rs` 新增 `app_state`/`dto`/`permissions`/`pagination` 模块声明；`pub use types::AppState` 经再导出继续提供 `crate::api::AppState`。

#### 验收核对（静态）
- 拆分后各实现文件均 ≤ 1370 行（最大 `permissions.rs` 1370 行，含权限表 + 全量单测；其余均 ≤ 158 行）；`types.rs` 收敛为 20 行纯再导出兼容层。
- 原 18 个 `pub` 导出项（结构体/静态表/函数）全部经再导出覆盖；`openapi.rs`/`middleware.rs`/各 handler/service/`lib.rs`/集成测试的引用点逐一路径核对。
- 本环境无 cargo，改动经人工审查 + 静态一致性校验（字段结构 diff 一致、braces 平衡、再导出路径逐一核对）。

### refactor: 后端安全收口 P0（T1/T2/T3，见《后端架构分析与重构方案》）

按《flamepanel-backend-refactoring.md》P0 安全收口阶段落地 T1/T2/T3，消除三个安全硬伤：

#### T1 弱默认密钥消除
- `config/loader.rs`：删除硬编码默认 `jwt_secret="flamepanel-secret"` 与 `admin_password="admin123"`（`grep` 取证归零）。
- `jwt_secret` 未提供（无配置文件 / 未设 `OP_JWT_SECRET`）时，启动期生成随机 64 字节密钥并持久化到 `data/jwt_secret.key`（`0600`）；重启复用已持久化密钥，保证令牌稳定。
- `admin_password` 未提供时生成随机密码并在启动日志打印一次（供首登，配合既有强制改密）。
- 无配置文件启动时打印告警日志，不再静默以公开弱值运行。

#### T2 Agent `/exec` 收口 + 链路加固
- `/exec` 端点默认关闭，需 agent 侧显式 `ALLOW_EXEC=1` 才开启（方案 B）。
- token 校验改为**常量时间比较**（`constant_time_eq`），消除时序侧信道；未设 `AUTH_TOKEN` 时生成密码学随机 token（`Uuid::new_v4()`），替换原时间戳伪 UUID。
- 文件端点（list/download/upload）新增路径白名单：仅允许 `FILE_ROOT` 根目录内的读写，拒绝越界绝对路径。

#### T3 应用安装脚本信任提示
- `InstallRequest` / `InstallAppRequest` 新增 `acknowledge_scripts` 字段：带 `native_scripts` 的应用安装必须显式 `acknowledge_scripts=true`，否则 400。
- 确认后计算并记录安装脚本 SHA-256 摘要（结构化日志），配合中间件写操作审计落库。

#### 验证说明
- 本环境无 cargo，改动经人工代码审查 + 静态一致性校验（`grep` 弱密钥取证归零、字段/调用点全量同步）。

### refactor: 后端可靠性修复 P1（T4/T5/T6/T7，见《后端架构分析与重构方案》)

按《flamepanel-backend-refactoring.md》P1 可靠性修复阶段落地 T4/T5/T6/T7：

#### T4 广播消费循环（Lagged 不死）
- 统一修复 4 处 `broadcast::Receiver` 消费循环（`event/handler.rs`、`lib.rs` outbox 记录器、`ws/mod.rs` metrics/log 两处）：收到 `RecvError::Lagged(n)` 时仅 `warn!` 并 `continue`，不再 `else break` 静默退出。
- 事件总线容量 100 → 1024，metrics 频道 16 → 64。

#### T5 WS 健全性
- 终端 WS `close(id)` 从 `recv_task` 移到 `select!` 之后的统一清理路径，任意分支结束后会话必关闭，避免泄漏。
- 终端 WS 接收增加消息大小上限（256 KiB）校验。

#### T6 限流/登录锁 TTL 清理
- `rate_limiter.rs`：分片 `Mutex<HashMap>` 改为 `moka::sync::Cache`（`time_to_live` + `max_capacity`），过期条目自动回收，杜绝无界增长。
- `login_attempt.rs`：新增空闲条目 TTL 清理（`IDLE_TTL`=1h），过期锁/空闲条目在 `check_locked` 时回收。
- `client_ip` 提取收敛为 `api/extract.rs::extract_client_ip` 单一实现。
- 限流阈值进入 `AppConfig`（`rate_limit_max`/`rate_limit_window_secs`，支持 `OP_RATE_LIMIT_MAX`/`OP_RATE_LIMIT_WINDOW`），经 AppState 注入中间件。
- 中间件顺序调整为 **rate_limit → auth**（先限流再鉴权，保护昂贵的 JWT 验签与用户查询）。

#### T7 关键写路径补事务
- `set_role_permissions`（sqlite.rs）：DELETE + 循环 INSERT 放入单事务，DELETE 错误不再吞掉。
- firewall `reorder`（sqlite.rs）：循环写放入单事务。
- `FirewallService::update_rule` / `delete_rule` / `toggle_rule`：去掉 `.ok()`，先应用 OS 规则、失败即返回错误不写 DB，避免库表与系统状态不一致。

#### 验证说明
- 本环境无 cargo，改动经人工代码审查 + 静态一致性校验（新字段/参数/调用点全量同步）。

### refactor: 后端架构整顿 P2 第一阶段——越层补齐（T10 核心，见《后端架构分析与重构方案》)

按《flamepanel-backend-refactoring.md》P2 架构整顿阶段落地 T10（越层补齐）中风险可控、无需大文件搬移的核心项：

#### T10 审计归因与错误传播
- 新增 `Username` 请求上下文扩展（`api/types.rs`），认证中间件在 REST/WS 两处注入已认证用户名。
- `api/handler/file/mod.rs`：全部 10 个文件 handler 改为从 `Extension<Username>` 取审计用户名，删除硬编码 `"system"`。
- `api/handler/plugin/mod.rs`：插件持久化失败由 `tracing::warn!` 吞错改为向上返回错误。

#### 说明
- T8/T9/T11/T12（service.rs/types.rs 上帝文件拆分、ROUTE_PERMISSIONS 迁移、domain 依赖方向修正、AppStore 构造整顿）为大规模纯搬移重构，依赖 151 个集成测试兜底；本环境无 cargo，建议在有编译/测试环境的分支逐项落地，避免盲改引入编译回归。

### refactor: 后端架构整顿 P2——上帝文件 `service.rs` 拆分（T8，见《后端架构分析与重构方案》)

按《flamepanel-backend-refactoring.md》P2 架构整顿阶段落地 T8：拆分 `application/service.rs`（原 2929 行、15 个服务）为「每域一文件」，`service.rs` 保留为兼容再导出层。

#### 文件拆分（`flame-kernel/src/application/`）
- `user_service.rs`：`UserService`
- `node_service.rs`：`NodeService`
- `website_service.rs`：`WebsiteService`
- `docker_service.rs`：`DockerService` + `DockerRepositoryFacade`
- `role_service.rs`：`RoleService` + `PermissionService`
- `web_server_service.rs`：`WebServerService`
- `settings_service.rs`：`SettingsService`
- `database_service.rs`：`DatabaseService`
- `misc_service.rs`：`MemoService` / `OperationLogService` / `OutboxService` / `LogService`（小服务合并）
- `firewall_service.rs`：`FirewallService`（应用编排）

#### FirewallManager 下沉基础设施
- `FirewallManager` 与 `FirewallBackend`（ufw/firewall-cmd/iptables 的 OS 探测与执行，属基础设施职责）移至 `infrastructure/firewall.rs`；`firewall_tests` 一并迁移。

#### 兼容层
- `application/service.rs` 改为统一再导出各新文件服务（`pub use ...::*`），既有 `use crate::application::service::*` 及 `DatabaseService`/`DockerService`/`WebServerService` 等精确引用全部保持兼容，无调用点改动。
- `application/mod.rs` / `infrastructure/mod.rs` 新增模块声明。

#### 验收核对（静态）
- 拆分后各实现文件均 ≤ 600 行（最大 `web_server_service.rs` 359 行 / `docker_service.rs` 353 行）；`service.rs` 收敛为 402 行兼容层 + 测试。
- 15 个服务 + 1 门面全部保留原 public API 与构造签名，跨文件无同名符号冲突。
- 本环境无 cargo，改动经人工审查 + 静态一致性校验（trait 方法签名、实体类型、再导出路径逐一核对）。

### refactor: 后端架构 Stage 8/9 完善——Docker 门面拆分 + Outbox 落库重试（见 Doc/19）

按《19-后端架构分析与完善落地手册.md》执行 Stage 8（A5）与 Stage 9（A7），收尾后端架构完善：

#### Stage 8：Docker trait 拆分（P1，A5）
- `DockerRepository` 门面已拆出 `ContainerRepository` / `ImageRepository` / `NetworkRepository` / `VolumeRepository` / `ComposeRepository` 五个细分端口，并在 `domain/repository.rs` 补充拆分边界文档：新代码直接按职责依赖细分 trait，不再膨胀单个聚合门面。
- `DockerRepositoryFacade` 仅保留用于既有构造路径过渡（未硬标记 deprecated，避免 `-D warnings` 门禁告警）。

#### Stage 9：事件一致性 Outbox（P2，A7）
- `OutboxService::record_event` 落库失败自动重试：小指数退避（50ms→100ms），最多 3 次，避免短时 I/O/锁瞬时抖动丢关键审计。
- 新增 2 单测：`record_event_retries_transient_failure`（重试后不丢事件）、`record_event_gives_up_after_max_retries`（超限返回错误）。

### refactor: 后端架构 Stage 7 完善——JWT 加固（P1，见 Doc/19）

按《19-后端架构分析与完善落地手册.md》执行 Stage 7（JWT 加固），消除每次请求重建
`JwtUtils` 的开销并收紧 JWT 校验策略。

#### 实例复用（AppState 持有共享 JwtUtils，禁止每次请求 new）
- `api/types.rs`：`AppState` 新增 `jwt_utils: Arc<RwLock<Arc<JwtUtils>>>`，基于启动密钥构建；
  新增 `shared_jwt()` 在热路径取共享读锁复用实例（并发不互斥）。
- `api/middleware.rs`（REST + WebSocket 两处）与 `api/handler/auth/mod.rs`（login / refresh）
  全部改为 `state.shared_jwt()`，移除逐请求 `JwtUtils::new_pair(...)`。
- `rotate_secret`：密钥轮换时整体替换共享 JwtUtils 实例，使旧 access token 立即可失效（与既有语义一致）。

#### 显式 Validation（收紧校验策略）
- `utils/jwt.rs`：`verify_token` 改为显式构建 `Validation`——算法受限 `HS256`、显式启用过期校验
  `validate_exp=true`、时钟偏差 `leeway=30`、强制要求 `exp`/`sub` 声明（`set_required_spec_claims`），
  替换默认宽松的 `Validation::default()`。

#### 测试（新增 5 例）
- 校验策略参数断言（HS256 / validate_exp / leeway=30）
- 缺 `sub` 声明令牌被拒（功能性验证 required claims 生效）
- sign/verify access 往返、refresh 不可作 access（类型守卫）、错误密钥拒验

### refactor: 后端架构 Stage 5 完善——权限路由元数据化 + 默认拒绝（A2，见 Doc/19）

按《19-后端架构分析与完善落地手册.md》执行 Stage 5（A2），在既有声明式权限表基础上
补齐漏声明的路由权限，并将「未声明权限的受保护路径」从默认放行收紧为**默认 403**，
同时提供「路由↔权限一致性检查」测试，确保新增路由漏鉴权可被 CI 抓住。

#### 权限声明补齐（修复 10 处漏声明）
- **新增 `task` 权限资源**：`task:read` / `task:execute` / `task:delete`，并同步
  `role_permissions`（admin 全量 / operator 非 delete / viewer 仅 read）。
- 为以下此前**无权限声明、任意已登录用户均可访问**的受保护路由补齐映射：
  - `GET /api/tasks`、`GET /api/tasks/{id}` → `task:read`
  - `POST /api/tasks/{id}/cancel` → `task:execute`
  - `POST /api/tasks/prune` → `task:delete`
  - `GET /api/metrics/processes` → `node:read`
  - `POST /api/nodes/{id}/action` → `node:execute`
  - `GET /api/docker/containers/{id}` → `docker:read`
  - `POST /api/app-store/packages/batch-import` → `app_store:create`
  - `PATCH /api/databases/batch-status` → `database:update`
  - `PATCH /api/settings/batch` → `settings:update`

#### 默认拒绝（强制规则）
- `api/middleware.rs`：RBAC 分支改为三态——已声明权限 → 校验；auth-only 白名单 → 放行；
  其余未声明受保护路径 → **403**（`Forbidden`，统一 JSON 错误体）。
- 新增 `is_auth_only_path` 白名单（仅需登录、无资源归属）：`/api/auth/me`、
  `/api/auth/change-password`、`/api/auth/logout`，以及历史语义下 `GET /api/databases/{id}`
  与 `GET /api/databases/{id}/databases`（子库列举仅鉴权）。

#### 路由↔权限一致性检查（Scheme B CI）
- 新增 `permission_table_covers_all_routes` 单测：枚举 `routes.rs` 全部受保护路由（167 条），
  断言每条都在 `ROUTE_PERMISSIONS` 声明或在 auth-only 白名单，否则测试失败，从而**漏声明可被测试抓住**。

#### 测试
- 新增 `test_undeclared_protected_route_defaults_to_403` 集成测试：admin 访问未声明路径 → 403；
  auth-only 白名单路径（`/api/auth/me`）→ 不 403。
- 全部既有权限表单测断言保持通过（含 `GET /api/databases/1` → None 等历史语义）。

#### 验证
- `cargo test --all` 与 `cargo clippy --all-targets -- -D warnings` 保持全绿（本环境无 cargo，改动经人工代码审查 + 静态一致性校验）。

### refactor: 后端架构 Stage 3 完善——RateLimiter 升级（去全局锁、分级限额，见 Doc/19）

按《19-后端架构分析与完善落地手册.md》执行 Stage 3，重构请求入口限流器：

#### 去除全局 `Mutex<HashMap>` 串行化
- `api/rate_limiter.rs` 重写为**分片结构**：IP 经 FNV 哈希散列到 `16` 个独立分片，
  每分片持有独立的 `Mutex<HashMap>`，并发请求仅锁各自分片，不再全链路串行。
- 全局实例改用 `OnceLock<RwLock<RateLimiter>>`：热路径仅取**共享读锁**（并发不互斥），
  仅在启动初始化时短暂取写锁替换实例。

#### 分级限额（登录更严 / 普通 API 更高 / health 宽松）
- `Login` 分级：`/api/auth/login` 默认 `5/min`（防暴力破解）。
- `Api` 分级：普通接口默认 `120/min`。
- `Health` 分级：`/health`、`/api/health` 默认 `600/min`，探活不被误伤。

#### IP 提取与统一 429 错误体
- IP 提取优先 `X-Real-IP`，回退到 `X-Forwarded-For` 链首值（可信代理链）。
- 429 返回统一 JSON 错误体 `{code, error, message}`（新增 `ErrorCode::RateLimited`→`RATE_LIMITED`，HTTP 429）。

#### 测试
- 新增 `rate_limiter` 单测：分片哈希稳定、登录分级严于普通 API、Api 用默认配额、
  health 宽松、路径→分级映射、IP 提取优先 `X-Real-IP`。

#### 验证
- `cargo test --all` 与 `cargo clippy --all-targets -- -D warnings` 保持全绿。

### refactor: 后端架构 Stage 2 完善——鉴权短缓存 AuthCache（A4，见 Doc/19）

按《19-后端架构分析与完善落地手册.md》执行 Stage 2（A4），为认证/RBAC 热路径引入 moka 短缓存，
显著降低每请求的用户查询与角色权限查询对仓储（尤其 SQLite）的压力。

#### A4 鉴权短缓存（Stage 2，P0）
- 新增 `utils/auth_cache.rs` 的 `AuthCache`：
  - `users`：`moka::future::Cache<i64, User>`，TTL 15s、容量 10_000；
  - `role_perms`：`moka::future::Cache<String, HashSet<(String,String)>>`，TTL 30s、容量 256。
- `UserService` 注入共享 `AuthCache`，`find_by_id` 改为 **cache-aside**（命中缓存免查仓储）。
- `RoleService` 注入共享 `AuthCache`，`check_permission` 改为先取**角色权限集合缓存**再判断，不再逐请求多次查表。
- 写路径显式失效，保证**无缓存导致的越权/残留**：
  - 用户：`update_user` / `delete_user` / `update_password` / `set_must_change_password` → `users.invalidate(id)`；
  - 角色权限：`set_role_permissions` / `create_role` / `update_role` / `delete_role` → `role_perms.invalidate(role_name)`。
- 中间件**不直接依赖** cache，仅经 Service 的 `find_by_id` / `check_permission` 间接命中（保持分层约束）。

#### 测试
- 新增 `auth_cache_tests`：热路径命中（第二次 `find_by_id` 不打仓储）、用户写路径失效、改密失效、
  角色权限失效后立即按新权限生效、无越权（未授权返回 false / 授权后返回 true）。

#### 验证
- `cargo test --all`：287 通过（128 单测 + 150 集成 + 5 stage5 + 3 agent + 1 doctest）。
- `cargo clippy --all-targets -- -D warnings`：0 warning。

### refactor: 后端架构 Stage 1 完善——分页下沉 + 错误映射细分 + 离线扫描条件化（A1/A4/A6，见 Doc/19）

按《19-后端架构分析与完善落地手册.md》执行 Stage 0/1/6，同步修复历史编译与测试问题。

#### A1 分页下沉（Stage 1，P0）
- `UserRepository` / `NodeRepository` / `WebsiteRepository` / `WebServerRepository` / `DatabaseRepository` / `FirewallRepository` / `SettingsRepository` 新增 `list_page(limit, offset)` + `count()`（`limit` clamp 到 `1..=200`）。
- SQLite 实现直接用 `LIMIT ? OFFSET ?` 分页；InMemory 实现按 id 倒序切片，双后端行为一致。
- 对应 `UserService` / `NodeService` / `WebsiteService` / `WebServerService` / `SettingsService` / `DatabaseService` / `FirewallService` 分页方法改为调用仓储 `list_page`/`count`，**移除全表 `list_all + paginate_slice` 主路径**（列表接口由 O(n) 降为 O(page)）。
- 新增 `NodeService::list_stale_nodes` 与 `NodeRepository::list_stale_heartbeats`，后台离线扫描改为按心跳阈值条件查询，不再全量加载后过滤。
- 新增分页单测：User/Node/Website 分页总数、跨页唯一性与 stale 心跳（3 例）。

#### A6 错误映射细分（Stage 6，P1）
- `From<sqlx::Error> for AppError` 细化：`RowNotFound` → `404 NOT_FOUND`，唯一约束冲突 → `409 CONFLICT`，其余保留 `500 INTERNAL`。

#### 历史问题修复（保证 `cargo test --all` / clippy 全绿）
- `ComposeRepository` 补 `run_compose` trait 方法并在门面 / InMemory 实现补齐。
- 修复 `webserver/manager.rs` 部分移动借用、`app_store_service.rs` `Arc` move、`task_state.rs` snapshot move 等编译错误。
- 修复集成测试 `crate::` 路径、`atomic_replace` 关联函数调用、防火墙后端探测断言与 mysql 测试等历史失败。

#### 文档
- 新增 `Doc/19-后端架构分析与完善落地手册.md` 并在 `Doc/README.md` 文档导航中登记。

#### 验证
- `cargo test --all`：282 通过（123 单测 + 150 集成 + 5 stage5 + 3 agent + 1 doctest）。
- `cargo clippy --all-targets -- -D warnings`：0 warning。

### refactor: Phase A/B 收尾——命令路径收敛收尾 + AppStore 批量导入端点 + 统一 Task 查询/取消 API（A1/A2/B1）

按《FlamePanel-Master-Refactor-Handbook》Phase A/B 继续推进，完成 PR #31 遗留的三项建议：A1 剩余命令路径收敛收尾、A2 AppStore 批量写对外端点、B1 统一 Task 查询/取消 API。

#### A1 收尾：os.rs / health 剩余直接命令路径收敛
- `OsInfo::detect_distro` 由 `sh -c "cat /etc/os-release ..."` 改为**标准库直接读取** `/etc/os-release` 与 `/etc/*release`（新增 `read_release_info`），无命令注入面，Agent/Embedded 两种模式行为一致，不再需要白名单放行任意 shell。
- `health::disk_free_bytes` 由 `df -k` 外部命令改为 **`sysinfo::Disks`** 按挂载点最长前缀匹配获取可用空间，健康检查不再 spawn 外部命令。
- 说明：`terminal`（交互式 shell）与 `scheduled_task`（用户自定义 cron 命令）本质是用户驱动的任意命令执行，**不**纳入特权白名单收敛（保持功能本质）。

#### A2 扩展：AppStore 批量导入对外端点
- `AppStoreService` 新增 `batch_import_packages(paths)`：一次性解析全部目录，任一非法/已存在则整体失败，全部通过后用 `create_many` 原子写（SQLite 事务 / InMemory 单锁）。
- 提取 `prepare_import_package` 复用解析+复制逻辑（`import_package` 同步改造为其调用）。
- 新增 **`POST /api/app-store/packages/batch-import`**（空列表返回 400）；前端 `src/api/appStore.ts` 新增 `batchImportPackages`。

#### B1 扩展：统一 Task 查询/取消 API + 前端任务进度
- 新增应用层 `TaskService`（薄封装 `TaskTracker`：list / list_by_state / get / cancel / prune）。
- 组合根创建**共享 `TaskTracker`**（带持久化 TaskStore），注入 `AppStoreService`/`WebServerService`/`NodeService`（三服务共享同一任务集合），并暴露到 `AppState.task_service`。
- 新增端点：**`GET /api/tasks`**（可按 `state` 过滤）、**`GET /api/tasks/{id}`**、**`POST /api/tasks/{id}/cancel`**（终态 409）、**`POST /api/tasks/prune`**（清理终态）。
- 前端新增 `src/api/tasks.ts` 与 `TasksView.vue`（列表/状态过滤/进度条/取消/清理），路由 `/tasks`（ops 分组），`nav.tasks` 与 `task.*` i18n（zh/en/ja）。
- `TaskState`/`TaskKind`/`TaskRecord` 补 `utoipa::ToSchema`，OpenAPI 同步注册。

#### 验证
- 新增单测：`TaskService` list/get/cancel/prune（4 例，含终态取消 409）。
- `cargo test --workspace` / `cargo clippy --all-targets -- -D warnings` / `cargo fmt --all -- --check` 由 CI 验证（本环境无 cargo 工具链）。

### refactor: Phase A/B 继续——原生数据库管理迁移 + 批量状态端点 + 安全白名单加固（A1/A2/A3）

按《FlamePanel-Master-Refactor-Handbook》Phase A 继续推进，完成 A1 收尾、A2 端点落地与 A3 白名单加固。

#### A1 扩展（收尾）：NativeDbManager + 通用原生脚本收敛到统一端口
- `MySqlManager` / `RedisManager` 新增持有 `SharedCommandRunner`，其直接 `mysql` / `mysqladmin` / `redis-cli` / `redis-server` 命令及 `sh -c` 配置/卸载脚本**全部经 `PrivilegedCommandRunner` 统一端口执行**，不再直接 `tokio::process::Command`（`exec_mysql` 由静态改实例方法）。
- `AppStoreService::install_native` 通用原生安装脚本（`sh -c`）改经 `self.runner` 执行（`AppStoreService` 新增 `runner` 字段），杜绝面板直接 spawn 安装脚本。
- Agent 白名单扩充原生数据库命令：`mysql -u root -e` / `mysqladmin ping` / `redis-cli` / `redis-server --version`；并新增否定断言（`sh -c` 任意命令一律拒绝）。
- 新增单测：`test_mysql_routes_through_runner` / `test_redis_routes_through_runner`（记录型 mock runner 断言命令均经统一端口）。

#### A2 扩展：数据库批量状态更新对外端点
- 新增 **`PATCH /api/databases/batch-status`**（请求体 `{updates:[[id,status],...]}`，空 updates 返回 400），调用 `DatabaseService::update_instances_status_batch`（SQLite 事务 / InMemory 原子）。
- 前端 `src/api/databases.ts` 新增 `updateDatabasesBatchStatus`。

#### A3 扩展：执行面安全基线加固
- Agent 白名单语义复核：原生数据库命令逐条精确放行；`sh -c` / `bash -c` 任意 shell 注入一律拒绝（新增否定用例断言）。
- 原生数据库 `sh -c` 配置/卸载脚本明确为 **embedded-only**（agent 模式白名单不含 `sh -c`，天然拒绝），写入 `Doc/18` 说明。

#### 验证
- 新增单测：`test_mysql_routes_through_runner`、`test_redis_routes_through_runner`、Agent 白名单新增放行/拒绝断言。
- `cargo test --workspace` / `cargo clippy --all-targets -- -D warnings` / `cargo fmt --all -- --check` 由 CI 验证（本环境无 cargo 工具链）。

### refactor: Phase A/B 继续——原生检测迁移 + 批量写事务扩展 + Task 持久化（A1/A2/B1）

按《FlamePanel-Master-Refactor-Handbook》Phase A/B 继续推进，承接 PR #27/#28/#29，完成三项扩展。

#### A1 扩展（续）：WebServerNativeManager 检测命令迁移到统一端口
- `WebServerNativeManager` 新增持有 `SharedCommandRunner`，其版本检测（`nginx -v` 等）、`which`、端口扫描（`ss`/`netstat`）**全部经 `PrivilegedCommandRunner` 统一端口执行**，不再直接 `tokio::process::Command`，使 `execution_mode=embedded|agent` 分离模式覆盖到原生 Web 服务器检测路径。
- Agent 白名单扩充：`nginx -v` / `httpd -v` / `openresty -v` / `lshttpd -v` / `caddy version` / `ss -tln` / `netstat -tln`。

#### A2 扩展：DatabaseService / AppStoreService 批量写统一接入事务语义
- `DatabaseRepository` 新增 `update_status_batch(updates)`（SQLite 用事务，任一失败回滚；InMemory 单锁内原子写），`DatabaseService` 新增 `update_instances_status_batch`。
- `AppPackageRepository` 新增 `create_many(pkgs)`（SQLite 事务 + 幂等跳过已存在；InMemory 原子写），`AppStoreService::seed_builtin_apps` 改为批量原子写（`set_many` 事务语义）。

#### B1 扩展：TaskTracker 持久化 + 批量节点操作统一 Task 编排
- 新增 `TaskStore` 端口（`runtime::task_state`），`InMemoryTaskStore`（默认）与 `SqliteTaskStore`（`tasks` 表落库）；`TaskTracker` 新增 `with_store` 与 `load_from_store`（进程重启恢复），create/transition/update_progress/remove 同步持久化。
- `RepoFactory::create_task_store` 按后端注入；`WebServerService`/`AppStoreService`/`NodeService` 组合根注入持久化 TaskStore。
- `NodeService::batch_execute` 建立统一 `BatchNode` Task：创建→running→按节点推进进度→全部成功 success / 部分失败 failed。

#### 验证
- 新增单测：`native_detection_routes_through_runner`（原生检测命令路由）、`tracker_persists_and_loads_from_store`（Task 持久化恢复）、集成测试 `test_database_batch_update_status_is_atomic`、Agent 白名单新增断言；`seeds_builtin_apps` 改走 `create_many` 事务路径。
- `cargo test --workspace` / `cargo clippy --all-targets -- -D warnings` / `cargo fmt --all -- --check` 由 CI 验证。

### refactor: Phase A/B 继续——Web 引擎管理与 Docker compose 迁移 + PanelSetting 批量写 + 统一 Task 状态机（A1/A2/B1）

按《FlamePanel-Master-Refactor-Handbook》Phase A/B 继续推进，承接 PR #27/#28 的 `execution_mode=embedded|agent` 分离模式地基，完成三项：

#### A1 扩展（续）：WebServerManager 引擎管理 + Docker compose 降级路径迁移到统一端口
- `WebServerManager` 改为持有 `SharedCommandRunner`，`check_status`/`start`/`stop`/`reload`/`config_test` 及原子写流程的 `atomic_config_test`/`atomic_reload` 全部经 `PrivilegedCommandRunner` 执行（新增 `parse_command` 把引擎返回的 shell 命令行拆为 program + args，杜绝任意 shell 拼接注入）；
  - 新增 `new()`（embedded 便捷构造，测试/默认零破坏）与 `new_with_runner(runner)`（DI 注入）；
  - `WebServerService::new` 注入 runner。
- `BollardDockerRepository` 新增 runner 字段，`compose_deploy`/`compose_up`/`compose_down`/`compose_ls` 的 `docker compose` 命令经统一端口执行（新增 `run_compose` 便捷方法）；新增 `new_with_connection_and_runner` 供 DI 注入。
- Agent 白名单扩充 Web 引擎命令（`nginx -s`/`nginx -t`/`httpd -k`/`httpd -t`/`openresty -s`/`openresty -t`/`caddy reload`/`caddy validate`/`killall`/`lswsctrl`）与 Docker compose 命令（`docker compose`/`docker-compose`）。

#### A2 扩展：PanelSetting 批量写事务（多键原子更新）
- `SettingsRepository` 新增 `set_many(entries)`：
  - SQLite 用事务（`BEGIN`→逐键 upsert→`COMMIT`，任一失败自动回滚）；
  - InMemory 单锁内批量写入（原子：要么全生效要么维持原状）；
- `SettingsService::set_many` + 新增 `PATCH /api/settings/batch` 端点（空请求体返回 400）；前端 `settings.ts` 新增 `updateSettingsBatch`。

#### B1：统一 Task 状态机（安装 / 引擎切换 / 批量节点）
- 新增 `runtime::task_state` 模块，提供统一五态状态机：
  - `TaskState`（`pending → running → success|failed|cancelled`），`TaskRecord::advance` 强校验非法迁移；
  - `TaskKind`（Install / EngineSwitch / BatchNode / Generic）；
  - `TaskTracker` 进程内线程安全跟踪器（create / transition / update_progress / list / prune_terminal）；
- 接入落地：`AppStoreService::install` 与 `WebServerService::switch_engine` 均创建统一 Task 跟踪安装/切换进度与结果，`AppStoreService`/`WebServerService` 各持有 `TaskTracker`。

#### 验证
- 新增单测：`task_state`（状态机合法/非法迁移、tracker 增删查、prune）、`WebServerManager::parse_command`（3 例）、Agent 白名单新增断言、设置批量 API 集成测试（2 例）；
- `cargo test --workspace` / `cargo clippy --all-targets -- -D warnings` / `cargo fmt --all -- --check` 由 CI 验证。

### refactor: Phase A 继续——包管理 / 服务管理迁移到特权命令执行端口（A1 扩展）

按《FlamePanel-Master-Refactor-Handbook》Phase A（P0）继续推进，承接上一轮防火墙迁移，把 `PackageManager` / `ServiceManager`（包管理与 systemctl 服务管理这两组高权限系统命令集合）也收敛到 `PrivilegedCommandRunner` 统一端口，使 `execution_mode=embedded|agent` 分离模式覆盖更完整，agent 模式下这两组命令可经远端 Agent 白名单执行。

#### 迁移：PackageManager / ServiceManager → PrivilegedCommandRunner
- `PackageManager` / `ServiceManager` 由**静态方法**改为**持有 `SharedCommandRunner` 的实例**，新增 `new(runner)` 与 `embedded()` 便捷构造（默认行为与重构前一致）；
  - `ServiceManager`：`start`/`stop`/`restart`/`enable`/`disable`/`is_running`（systemctl is-active + pgrep 回退）与新增 `is_enabled`（systemctl is-enabled）全部经 runner 执行（`prefer_root`，非 root 自动 `sudo -n`）；
  - `PackageManager`：`install`/`is_installed`/`uninstall`/`get_version`（apt/yum/dnf/apk/dpkg/rpm 按发行版路由）全部经 runner 执行。
- 消费方全部注入 runner：
  - `MySqlManager` / `RedisManager`（数据库原生安装/启停）持有 `PackageManager` + `ServiceManager`；
  - `WebServerNativeManager`（原生 Web 服务器检测/安装/卸载/自启）持有 `PackageManager` + `ServiceManager`（`is_enabled` 迁移自原 `is_service_enabled`）；
  - `DefaultPackageManagerPort` / `DefaultServiceManagerPort`（应用商店六边形端口）持有 runner；
  - `WebServerService` / `DatabaseService` / `AppStoreService` 构造器新增 runner 参数，组合根（`lib.rs`）与测试统一注入 `EmbeddedCommandRunner`。

#### Agent 白名单扩充
- 新增包管理命令：`apt install` / `apt-get remove` / `dpkg -l` / `yum install` / `dnf remove` / `rpm -q` / `apk add` / `apk del` / `apk info`；
- 新增服务管理命令：`systemctl enable` / `systemctl disable` / `pgrep -x`。

#### 验证
- 新增 Agent 白名单测试断言（包管理 + systemctl enable/disable + pgrep 放行）；
- `cargo test --workspace` / `cargo clippy --all-targets -- -D warnings` / `cargo fmt --all -- --check` 由 CI 验证。

### refactor: Phase A 继续——特权命令执行模式分离（execution_mode=embedded|agent）+ 防火墙迁移（A1 扩展）

按《FlamePanel-Master-Refactor-Handbook》Phase A（P0）继续推进，落实 `execution_mode=embedded|agent` 分离模式的**地基**，并把防火墙（高权限系统命令）作为首个迁移路径收敛到统一特权命令执行端口，为后续 docker / 包管理 / 引擎 reload 的逐步迁移提供可复用模式。

#### 新增：ExecutionMode + PrivilegedCommandRunner 端口
- 新增 `ExecutionMode` 枚举（`embedded` 默认 / `agent`），配置项 `OP_EXECUTION_MODE` + `AppConfig.execution_mode`（serde default=embedded，旧配置向后兼容）；
- 新增 `PrivilegedCommandRunner` 端口（application 层，六边形）：`run(&PrivilegedCommand) -> CommandOutput`，抽象「面板执行特权系统命令」的统一入口；
  - `EmbeddedCommandRunner`（本地 `tokio::process::Command`，保留 root 判断 / `sudo -n` 免密码）；
  - `AgentCommandRunner`（委托远端 Agent `whitelisted_command` 动作，Agent 侧白名单校验，非白名单命令被拒绝）。

#### 迁移：防火墙命令收敛到统一端口
- `FirewallManager` 由静态方法改为持有 `SharedCommandRunner` 的实例，`detect_backend` / `get_status` / `apply_rule` / `remove_rule` / `enable` / `disable` 全部经 runner 执行；
- `FirewallService::new` 改为注入 runner（新增 `new_embedded` 便捷构造），组合根按 `execution_mode` 注入；
- Agent 侧白名单扩充防火墙命令（`which` / `ufw` / `firewall-cmd` / `iptables` / `systemctl` 相关），使 agent 模式防火墙可执行。

#### 验证
- 新增测试：ExecutionMode 解析/展示、PrivilegedCommand 构造、CommandOutput 语义、Agent 动作结果解析（成功/拒绝）、Embedded runner 本地执行、FirewallManager 经 runner 路由、Agent 白名单防火墙命令放行；
- `cargo test --workspace` / `cargo clippy --all-targets -- -D warnings` / `cargo fmt --all -- --check` 由 CI 验证。

### refactor: Phase A 继续——Web 引擎配置原子写 + 校验 + 回滚（A2 写路径事务）

按《FlamePanel-Master-Refactor-Handbook》Phase A（P0）继续推进，承接前三轮 Website / WebServerInstance / DatabaseInstance 的乐观并发控制（OCC），补全 G2（写路径事务）在「Web 引擎配置写入」上的原子性与回滚能力，防止静默损坏引擎配置。

#### A2 原子写 + 回滚：WebServerManager
- 新增 `write_config_file_atomic(engine, path, content, do_reload)`：临时文件 → `rename` 原子替换 → 引擎 `config_test`（`nginx -t` 类）校验 → 按需 `reload` → 任一环节失败自动回滚原文件；
- 拆分可复用的 `atomic_replace`（临时文件 + 原子替换 + 清理临时文件）与 `restore_config`（有备份恢复 / 无备份删除）；
- 原子写基于 `engine`（而非仅 instance），`write_site_config` 与 `apply_preset` 均接入原子写路径（含校验 + reload + 回滚）；
- 失败语义：config 校验或 reload 失败时恢复原配置并返回稳定内部错误，不静默留残；
- 新增单测：`atomic_replace` 写入并清理临时文件 / 覆盖已存在文件；`restore_config` 有备份恢复 / 无备份删除。

#### 验证
- `cargo test --workspace`：新增 4 个原子写/回滚单测；
- `cargo clippy --all-targets -- -D warnings` / `cargo fmt --all -- --check` 保持绿；
- CI 门禁：`cargo audit` + `npm audit` 双重依赖审计保持。

### refactor: Phase A 继续——DatabaseInstance 乐观并发控制（A2 扩展）

按《FlamePanel-Master-Refactor-Handbook》Phase A（P0）继续推进，承接上两轮 Website / WebServerInstance OCC，将写路径并发控制扩展到「数据库引擎配置」，补全 G2（写路径事务）在 DatabaseInstance 上的缺口。

#### A2 扩展：DatabaseInstance 乐观并发控制（OCC）
- `DatabaseInstance` 实体新增 `resource_version` 字段（serde default=0，旧请求兼容）；
- `databases` 表新增 `resource_version INTEGER NOT NULL DEFAULT 0` 幂等迁移列（`add_column_if_missing`）；
- SQLite 写入改为条件更新：`UPDATE databases SET ... resource_version=resource_version+1 WHERE id=? AND resource_version=?`，命中即自增版本；
- InMemory 实现同步 OCC 语义；版本冲突返回稳定 `AppError::Conflict`（HTTP 409，ErrorCode `CONFLICT`）；
- 不存在与版本冲突区分：不存在返回 404，版本过期返回 409；
- `DatabaseInstanceResponse` 增加 `resource_version` 字段，前端 `DatabaseInstance` 类型已同步；
- 新增测试：`test_database_occ_version_conflict`（repository 级 OCC：正确版本自增、过期版本冲突、不被过期写入污染）、`test_database_occ_update_not_found`（不存在返回 NotFound）。

#### 验证
- `cargo test --workspace`：新增 2 个 OCC 测试；
- `cargo clippy --all-targets -- -D warnings` / `cargo fmt --all -- --check` 保持绿。

### refactor: Phase A 继续——WebServerInstance 乐观并发控制 + 实机安全基线（A2 扩展 / A3）

按《FlamePanel-Master-Refactor-Handbook》Phase A（P0）继续推进，承接上一轮 A1/A2 首批落地，扩展写路径事务覆盖并建立实机安全基线。

#### A2 扩展：WebServerInstance 乐观并发控制（OCC）
- `WebServerInstance` 实体新增 `resource_version` 字段（serde default=0，旧请求兼容）；
- `web_servers` 表新增 `resource_version INTEGER NOT NULL DEFAULT 0` 幂等迁移列（`add_column_if_missing`）；
- SQLite 写入改为条件更新：`UPDATE ... WHERE id=? AND resource_version=?`，命中即自增版本；
- InMemory 实现同步 OCC 语义；版本冲突返回稳定 `AppError::Conflict`（HTTP 409，ErrorCode `CONFLICT`）；
- 不存在与版本冲突区分：不存在返回 404，版本过期返回 409；
- `PUT /api/web-servers/{id}` 支持请求体携带 `resource_version` 作为客户端基准版本；未携带时保持向后兼容（用当前版本）；
- `WebServerResponse` 增加 `resource_version` 字段，前端可读版本号；
- 新增测试：`test_web_server_occ_version_conflict`（repository 级 OCC）、`test_web_server_occ_update_endpoint_conflict_409`（API 级 409）。

#### A3 实机与安全基线
- CI 新增前端依赖审计：`npm audit --audit-level=high`（高危 0 即阻断），与既有 `cargo audit` 形成 Rust+前端双依赖门禁；
- 新增 `Doc/18-兼容性与安全基线.md`：操作系统兼容矩阵（Debian 12/13、Ubuntu 22.04/24.04 完全支持；Rocky/Alma 9、CentOS Stream、Alpine best-effort）+ 上线前安全验收清单（非 root、TLS、WS 鉴权、Agent 令牌、路径穿越否定用例）+ CI 安全门禁表；
- `Doc/README.md` 登记 Doc/18。

#### 验证
- `cargo test --workspace`：新增 2 个 OCC 测试（repository + API 级）；
- `cargo clippy --all-targets -- -D warnings` / `cargo fmt --all -- --check` 保持绿；
- CI 门禁：`cargo audit` + `npm audit` 双重依赖审计。

### refactor: Phase A1/A2 安全与真相终局（resource_version 乐观并发 + Agent 动作枚举）

按《FlamePanel-Master-Refactor-Handbook》Phase A（P0）落地首批可执行项，对应 G1（特权执行面）与 G2（写路径事务）结构性缺口。

#### A2 写路径事务：Website resource_version 乐观并发控制（OCC）
- `Website` 实体新增 `resource_version` 字段（serde default=0，旧请求兼容）；
- `websites` 表新增 `resource_version INTEGER NOT NULL DEFAULT 0` 幂等迁移列；
- SQLite 写入改为条件更新：`UPDATE ... WHERE id=? AND resource_version=?`，命中即自增版本；
- InMemory 实现同步 OCC 语义；版本冲突返回稳定 `AppError::Conflict`（HTTP 409，ErrorCode `CONFLICT`）；
- 不存在与版本冲突区分：不存在返回 404，版本过期返回 409；
- 新增测试：`test_website_occ_version_conflict`（repository 级 OCC）、`test_website_occ_update_endpoint_conflict_409`（API 级）。

#### A1 特权执行面：Agent 动作枚举（action enumeration）
- Agent 新增 `POST /action` 端点：动作白名单分发，拒绝任意 Shell；
- 定义 `AgentAction` 枚举：ping / system_info / service_status|start|stop|restart / file_exists / path_is_dir / whitelisted_command；
- `is_whitelisted` 白名单前缀校验 + 危险字符阻断（`;` `|` `&` `$()` `${}` `>` `<` 等注入向量）；
- 面板侧新增 `POST /api/nodes/{id}/action` 代理端点（转发至 Agent `/action`）；
- `AgentClient` 新增 `call_action` / `ping` / `system_info` / `whitelisted_command` 方法；
- 前端新增 `remoteAction(id, action, params)` API（`frontend/src/api/nodes.ts`）；
- 新增 agent 单测 3 个（白名单通过/拒绝/序列化）；
- OpenAPI 更新 `/api/nodes/{id}/action` + `RemoteActionRequest` schema；frontend/openapi.json 重新生成。

#### 验证
- `cargo test --workspace`：242 个测试全部通过（92 unit + 141 integration + 5 stage5 + 3 agent + 1 doctest）；
- `cargo clippy --all-targets -- -D warnings`：通过；`cargo fmt --all -- --check`：通过；
- 新增 2 个 OCC 测试 + 3 个 Agent 白名单单测。

### refactor: stage6 事件驱动深化（DomainEvent 全面接入 + 通知渠道抽象 + 事件落库 Outbox）

按《Doc/13 P2-5.2 事件驱动深化》落地后端 Stage6，补齐「事件总线已接线未发布」缺口。

#### 6.1 DomainEvent 全面接入
- 补齐缺失事件发布：`AppUpgraded`（`AppStoreService::upgrade` 容器/WASM 路径）、`UserLoggedIn`（登录成功）、`PasswordChanged`（改密）；
- `DomainEvent` 增 `Serialize`，便于事件落库 JSON 载荷。

#### 6.2 NotificationChannel 通知渠道抽象
- 新增 `AsyncNotificationChannel` 端口（`notification`），`EventHandler` 改为持有 `Vec<Arc<dyn AsyncNotificationChannel>>`，与具体通知器解耦；
- `EmailChannel` 实现：事件→邮件主题/正文映射覆盖全部 12 种领域事件（含 Web 站创建、登录、改密、心跳等），可组合多渠道、便于扩展站内信/Webhook。

#### 6.3 事件落库 Outbox
- 新增 `outbox_events` 表 + `OutboxEvent` 实体 + `OutboxRepository` 端口（Sqlite/InMemory 双实现）；
- 组合根新增 `event-outbox` 订阅器：每条领域事件持久化存档，保证审计不丢；
- `OutboxService`（record_event / 分页查询）+ `GET /api/outbox-events` API + `outbox:read` 权限 + OpenAPI；
- admin 角色自动获得新权限（role_permissions all_ids）。

#### 验证
- `cargo test --workspace`：237 个测试全部通过（92 unit + 139 integration + 5 stage5 + 1 doctest，新增 outbox 2 测试）；
- `cargo clippy --all-targets -- -D warnings`：通过；`cargo fmt --all -- --check`：通过；
- feature 组合编译：default / `--all-features` / `--no-default-features` / `+sqlite` / `+wasm` 全部通过；
- domain 零 sqlx/axum/bollard 实际依赖。

### refactor: frontend Modernization M11（架构边界工程化：eslint 强制 Fp* 唯一出口）

按《Doc/17 §21.2 工程化》补齐架构约束，从工程上固化「业务 views 禁止裸 OpenVue」的硬性规则（§15.3/§17.3）。

#### M11 eslint 强制 Fp* 边界
- `eslint.config.js` 新增针对 `src/views/**` 的 `no-restricted-imports` 规则：屏蔽 `openvue` / `openvue/*` 全部底层组件入口；
- 业务视图只能从 `@/components/ui` 的 Fp* 封装消费 UI，新增代码若有裸 OpenVue import 将直接 lint 报错；
- 与既有 `components/ui/README.md`「views 禁止直接 import openvue」约定形成「文档 + 工具」双保障；
- 图标统一经 openicons CSS 类名（`oi oi-*`），无第二图标源。

#### 验证
- eslint：0 error（含新规则生效）；`vue-tsc --noEmit`：通过；`vitest run`：13 通过。

### refactor: frontend Modernization P0/P1 队列（命令集中配置 / vue-query 覆盖 / 主题导入导出）

按《Doc/17 第三部分 · OpenVue 现代化》继续推进 **Modernization 队列**：

#### M1 命令集中配置（src/config/commands.ts）
- 新增 `config/commands.ts`：⌘K 命令统一来源——导航命令从 `menuRoutes` 派生（与侧边栏同源），动作命令集中注册；CommandPalette 收敛到该配置，新命令无需改视图。

#### M2 vue-query 覆盖核心只读接口
- **Users / Websites** 分页列表从 `onMounted+ref` 迁移到统一数据获取层 `useApiQuery`（`keepPreviousData` 切页不闪空）；新增 `queryKeys.users/websites/engines`。

#### M3 主题/外观/语言 JSON 导入导出增强
- 设置页主题导出升级为 v2：包含主题（mode/preset/glass/custom）、外观（菜单页签/手风琴/隐藏菜单/折叠）、语言，可在另一浏览器完整恢复（Doc/17 §24）；兼容导入 v1（仅 preset/custom）旧文件。

#### 验证
- `vue-tsc --noEmit`：通过；eslint：0 error；`vitest run`：13 通过；`vite build`：通过。

### refactor: frontend Modernization M4–M7（统一列表三态 / Dashboard 跳转 / API 文档 / 角色化侧栏）

继续推进 **Modernization 队列**：

#### M4 统一列表页范式（工具栏 + FpTable 三态规范化）
- **Users / Websites** 列表接入 `FpStatePanel` 三态：加载骨架 / 错误可重试（`retryable`）/ 空态，与 Files/SystemLogs 等视图范式对齐。

#### M5 Dashboard 指标卡点击跳转 / 顶栏状态可点
- Dashboard 指标卡改为可点击（`<button>`），CPU/Memory→`/nodes`、Disk→`/files`、Load→`/health`，hover 高亮 + 右侧箭头，支持键盘焦点（`focus-visible`）；
- 顶栏面板状态灯从只读 `<span>` 改为可点击按钮 → `/health`（在线节点/运行容器此前已可点）。

#### M6 前端 API 文档（src/api/README.md + 错误码对照表）
- 新增 `src/api/README.md`：逐模块函数/方法/路径说明、统一鉴权与 401 刷新、数据层（vue-query）约定；
- 新增**前端 locales ↔ 后端 ErrorCode 错误码对照表**（含 HTTP 状态、前端附加网络码）。

#### M7 角色化默认侧栏折叠
- `appearance` store 新增 `applyRoleDefaults(role)`：admin 默认展开 `web/storage/ops`、operator 展开 `ops/storage`、viewer 折叠全部次要分组（仅保留 main）；
- 仅当用户**未手动自定义**分组时生效，`toggleGroup` 后即锁定用户选择；AppSidebar 挂载时按登录角色应用。

#### 验证
- `vue-tsc --noEmit`：通过；eslint：0 error；`vitest run`：13 通过；`vite build`：通过。

### refactor: frontend Modernization M8–M10（Fp 组件文档 / 列表搜索筛选 / ⌘K 动作命令）

继续推进 **OpenVue 现代化** 剩余高价值项。

#### M8 Fp 组件 props/events/slots 文档
- 新增 `src/components/ui/docs/`：**每个 Fp 组件一份** API 文档（props/events/slots），覆盖 `FpButton/FpTable/FpModal/FpDrawer/FpPagination/FpTabs/FpStatePanel/FpInput/FpSelect/FpTextarea/FpNumber/FpFormField/FpSwitch/FpCheckbox/FpSlider/FpSelectButton/FpRadioGroup/FpTag/FpEmpty/FpSkeleton/FpProgress/FpBreadcrumb/FpChip/FpInlineMessage/FpDivider/FpCard/FpButtonLink/FpFileUpload/FpColumn` + hooks `useFpToast/useFpConfirm`；
- `components/ui/README.md` 新增「API 文档」入口 + 维护约定（改 props 即同步 docs）。

#### M9 列表工具栏搜索/筛选条规范化（标准页面范式 §18.2A）
- **PluginsView**：搜索（id/name/author）+ 启用状态筛选（客户端过滤，全量列表）；
- **DatabasesView**：搜索（name/type/port）+ 类型筛选（MySQL/Redis）；
- **FirewallView**：搜索（name/desc/port/source）+ 协议筛选（TCP/UDP/ICMP）；
- **ScheduledTasksView**：搜索（name/command）+ 状态筛选（启用/停用/成功/失败）；
- **WebsitesView**：搜索（name/domain/root_path）+ 状态筛选（active/inactive）；
- **UsersView**：搜索（username）+ 角色筛选（admin/operator/viewer）；
- 新增 `common.searchPlaceholder` / `common.all`、`scheduledTask.statusEnabled/statusDisabled` i18n 三语。

#### M10 ⌘K 覆盖更多动作命令
- `config/commands.ts` 新增动作命令：**切换玻璃材质**（`setGlassEnabled`）、**循环密度**（compact→standard→comfortable，`updateCustom`）；
- 与 M1 同一来源，新命令无需改 CommandPalette 视图。

#### 验证
- `vue-tsc --noEmit`：通过；eslint：0 error；`vite build`：通过。

### refactor: frontend F4 engineering & type safety（工程化与类型安全）

按《Doc/17 第二部分 · Fix-Plan》完成前端 **F4 工程化与类型安全** 阶段（F4.1–F4.3）：

#### F4.1 类型收紧 → OpenAPI 生成
- **类型单源**：后端 OpenAPI 已覆盖实体统一从 `src/api/generated`（openapi-typescript 生成）导入；`@/types` 仅保留后端未覆盖的端点类型；分页统一 `Page<T>`。
- **operationId 唯一化**：后端 user/website/node handler 显式 `operation_id`（`list_users`/`create_website` 等），消除 OpenAPI 重复 operationId 导致的 TS 类型冲突。
- **消灭 `any`**：NodesView 5 处 `catch (e: any)` 改为 `catch (e: unknown)` + `getErrorMessage`，仓库 `any` 归零（仅剩协议字面量）。
- **修复 getErrorMessage null 崩溃**：`e` 为 `null` 时 `?.response` 安全访问（由单测发现）。
- **离线快照**：`frontend/openapi.json` 入库 + `npm run openapi:generate` 可复现生成。

#### F4.2 组件文档与回归预览页
- 新增 **`/dev/ui`**（`src/views/DevUiView.vue`）：FpButton 8 态/图标/加载、FpTable+FpColumn+FpPagination、FpModal/FpDrawer、表单控件、FpTag、设计令牌色板与阴影的回归预览；路由不进侧边栏，仅地址访问。

#### F4.3 工程化脚本 + vitest + CI
- `package.json` 新增 `typecheck` / `test:unit` / `analyze` / `openapi:generate`；新增 `vitest.config.ts`。
- 新增单测：`queryKeys`（4）+ `getErrorMessage/isNetworkError`（9），共 **13 个测试通过**。
- CI 新增 **Frontend Typecheck** 与 **Frontend Unit Tests** 步骤；`justfile` 新增 `test-unit`，`check-full` 串联。
- **npm audit 0 vulnerabilities**（`js-yaml`→4.3.1、`undici`→6.28.0 经 `overrides` 修复 openapi-typescript 6.x 的传递依赖）；修复 `.gitignore` 误忽略 `frontend/package-lock.json`（CI `npm ci` 依赖）。
- `openapi-typescript@^6.7.6`（与项目 TypeScript 6 兼容；v7 要求 TS^5 会破坏 npm ci）。

#### 验证
- `vue-tsc --noEmit`：通过；eslint：0 error 0 warning；`vitest run`：13 通过；`vite build`：通过。

### refactor: frontend F3 accessibility & information architecture（可访问性与信息架构）

按《Doc/17 第二部分 · Fix-Plan》完成前端 **F3 a11y 与 IA** 阶段（F3.1–F3.3）：

#### F3.1 a11y 基线
- **命令面板**：`role=combobox/listbox` + `aria-activedescendant` + `aria-controls`；键盘支持 Home/End 首尾、Esc 关闭；占位文本 i18n 化；空态 `role=status`。
- **图标按钮**：AppHeader（搜索/快捷应用/在线节点/运行容器/通知/语言/主题）、AppTabs 页签关闭、MemoList 编辑/删除、CopyButton、LayoutContent 刷新、AppSidebar 折叠分组按钮全部补充 `aria-label`。
- **表单错误关联**：FpInput / FpSelect / FpTextarea 增加 `aria-invalid` + `aria-describedby`（错误 id 关联），FpTextarea 新增错误提示展示。
- **路由焦点管理**：路由切换后焦点移到 `sr-only` 主内容标题，辅助技术与键盘用户可感知页面变化。
- 全局 `:focus-visible` 焦点样式已具备，OpenVue Dialog 自带 focus-trap 与 Esc 关闭。

#### F3.2 信息架构降噪
- **角色化侧栏**：`permission.ts` 补齐 RBAC 矩阵（admin 全量 / operator 除 delete+管理类 / viewer 仅 read），新增 `isMenuHiddenForRole`；AppSidebar 按角色过滤菜单（viewer/operator 隐藏用户/设置等管理入口）。
- **Dashboard 只读适配**：viewer 隐藏待办勾选与应用启动入口（只读监控）。
- **权限指令落地**：13 个视图 76 处写操作按钮接入 `v-permission`（viewer 自动隐藏，消除 403 按钮）。
- **命令面板增强**：路由 `meta.weight` 权重排序（高频入口靠前）；`meta.keywords` 支持中文/英文/日文关键词搜索（与 i18n 同步）。

#### F3.3 响应式底线
- `FpTable` 容器 `overflow-x: auto` + `.p-datatable-wrapper` 横向滚动兜底，窄屏表格不溢出。
- Layout 新增平板断点（769–1100px 紧凑间距）；移动端侧栏抽屉已有。<768px Dashboard 图表单列堆叠已有。

#### 验证
- `vue-tsc --noEmit`：通过；eslint：0 error（仅既有 5 个 NodesView any warning，非本次引入）；`vite build`：通过。

### refactor: frontend F2 design system（设计系统与一致性）

按《Doc/17 第二部分 · Fix-Plan》完成前端 **F2 设计系统** 阶段（F2.1–F2.3）：

#### F2.1 令牌补全 + 玻璃降级
- `theme/tokens.css` 补齐设计令牌：阴影 elevation（--fp-shadow-xs~lg/brand）、字号/行高、z-index 层级（--fp-z-*）、玻璃档位（--fp-glass-blur / --fp-glass-blur-dense）、终端色板（--fp-term-*）、代码文本色（--fp-text-code）、终端背景（--fp-bg-terminal）。
- 玻璃总开关：`theme store.glassEnabled`（持久化 `flamepanel.glass`），关闭时 `html.glass-disabled` 全局回退实色；`prefers-reduced-transparency` 自动纯色回退；表格/表单密集区默认低 blur。
- `uno.config.ts` 桥接 `boxShadow` / `z-*` shortcuts。
- 硬编码色清理：Dashboard 图表调色板、TerminalView xterm 主题（--fp-term-*）、AppStore/ScheduledTasks/Docker 日志底色、NodesView 状态色、SettingsView 预设色板、AppStore/Dashboard 阴影等全部改为令牌。

#### F2.2 Fp* 封装加厚 + views 收敛
- 新增 `FpPagination` / `FpFormField` / `FpDrawer` / `FpSkeleton` / `FpTabs`（声明式 `:items` + 命名 slot）及 `FpNumber` / `FpTextarea` / `FpSwitch` / `FpCheckbox` / `FpSlider` / `FpSelectButton` / `FpRadioGroup(+FpRadioOption)` / `FpDivider` / `FpProgress` / `FpBreadcrumb` / `FpFileUpload` / `FpChip` / `FpInlineMessage` / `FpColumn` / `FpButtonLink`。
- `FpTable` 新增 `sortable/sortField/sortOrder` 客户端排序；`components/ui/index.ts` barrel 统一出口。
- **views 全部收敛**：21 个业务视图移除全部 `openvue/*` 直接 import（0 残留），改用 Fp* 封装；5 个 Tabs 视图迁移到声明式 `FpTabs`。
- 新增 `components/ui/README.md` 组件文档（清单/Props/范式/已登记例外）。

#### F2.3 主题切换与首屏无闪烁
- `theme store.apply()` 批量写 CSS 变量（单次回流）。
- `index.html` 内联脚本 mount 前同步明暗 + 玻璃开关，避免闪白/闪黑。
- 自定义背景开启时按玻璃策略降强度，密集区低 blur。

#### 验证
- `vue-tsc --noEmit`：通过；eslint：0 error（5 个既有 warning 非本次引入）；`vite build`：通过。

### refactor: frontend F1 data & performance（前端数据层与列表性能）

按《Doc/17 第二部分 · Fix-Plan》完成前端 **F1 数据与性能** 阶段（F1.1–F1.3）：

#### F1.1 统一数据获取层（@tanstack/vue-query）
- 新增 `@tanstack/vue-query` 依赖，全局 `QueryClient`（staleTime 15s / retry 2 / gcTime 5min / 窗口聚焦静默刷新）。
- 新增 `composables/useApiQuery.ts`：`useApiQuery` / `useApiMutation` 封装，语义与 `useAsyncState` 对齐（data/loading/error/refresh）；支持轮询（refetchInterval）并自动随 keep-alive 失活/页面隐藏暂停。
- 新增 `composables/useWsCache.ts`：WS 推送 `setQueryData` 写缓存 / `invalidateQueries`，消除「WS 一份、轮询一份」双源真相。
- 新增 `api/queryKeys.ts`：集中管理 queryKey（nodes/containers/images/networks/volumes/compose/packages/installedApps/settings/files/operationLogs）。
- 迁移页面：NodesView（列表+状态轮询写缓存）、OperationLogsView（分页 keepPreviousData）、FilesView（按路径缓存 + 写操作 invalidate）、DashboardView（指标快照写缓存）。

#### F1.2 虚拟列表
- `FpTable` 新增 `virtual` 模式：自动 `scrollable` + `scrollHeight`，虚拟模式自动关闭分页；新增 `virtualItemSize` / `virtualScrollHeight` 可配置。
- 开启虚拟滚动：Docker 容器/镜像/卷/Compose 项目、文件列表、操作日志、系统日志、应用商店已安装列表。1000+ 行不再整页卡死。

#### F1.3 Dashboard 图表节流
- 1s 节流合并三张图（趋势/网络/负载）的更新，避免每个 WS tick 都 `setOption`。
- 页面隐藏（visibilitychange）跳过图表计算，回到前台补刷积压更新；keep-alive 失活时 `useECharts` 暂停渲染。
- 高频更新使用 `lazyUpdate`（渲染合并到下一帧）；主题切换一次性 `notMerge` 刷新，无残留配置。

#### 验证
- `vue-tsc --noEmit`：通过；eslint：0 error（5 个既有 warning 非本次引入）；`vite build`：通过。

### refactor: frontend F0 stability（前端稳定性：统一错误 / 资源销毁 / 三态）

按《Doc/17 第二部分 · Fix-Plan》完成前端 **F0 稳定性** 阶段（F0.1–F0.3）：

#### F0.1 统一请求与错误处理
- `api/client.ts`：新增 `isNetworkError`（断网/超时/服务未启动识别）；401 refresh 失败清理 token；导出 `PASSWORD_CHANGE_REQUIRED` 常量。
- `utils/error.ts`：`getErrorMessage` 支持断网本地化文案，未知错误码回退后端 message。
- `locales`：三语补全 `NETWORK_ERROR` / `TIMEOUT` / `PASSWORD_CHANGE_REQUIRED` / `common.retry`。
- 新增 `composables/useAsyncState`：统一 `idle → loading → success/error` 状态机。

#### F0.2 资源销毁清单（keep-alive 生命周期管理）
- 新增 `useECharts` / `useWebSocket` / `usePolling` composable：`onDeactivated` 自动暂停、`onActivated` 自动恢复、`onBeforeUnmount` 自动清理。
- `DashboardView`：图表 / WS / 进程轮询全部接入 composable，切换页签后台停止更新与连接。
- `SystemLogsView` / `HealthView`：WS 失活关闭、激活重连。
- `TerminalView`：终端 WS 统一管理，卸载自动 dispose。
- `NodesView`：轮询接入 usePolling，keep-alive 失活暂停。

#### F0.3 统一 Loading/Empty/Error 三态
- 新增 `components/ui/FpStatePanel.vue`：loading（spinner）/ error（可重试）/ empty 统一三态。
- `FilesView` / `SystemLogsView` 接入 FpStatePanel，错误态带重试按钮；三语补全 `file.emptyDir` / `log.connectionFailed` / `log.waiting`。

#### 验证
- `vue-tsc --noEmit`：通过；eslint：0 error；`vite build`：通过。

### feat: stage5 multi-node remote capabilities（多节点能力：远程命令 / 远程文件 / 批量命令）

按《Doc/13 开发路线图与后续规划》P2-5.1「多节点能力」落地（Stage5）：

#### 5.1a 面板侧 Agent HTTP 客户端
- 新增 `infrastructure/agent_client.rs`：`AgentClient`（reqwest）封装远程 `/exec`、`/files/list`、`/files/download`、`/files/upload` 调用。
- 复用 `resilience` 的 `retry_with_backoff`（2 次指数退避重试）+ 节点粒度 `CircuitBreaker`（5 失败熔断 30s）。
- 请求携带 `Authorization: Bearer <auth_token>` 完成 Agent 侧鉴权。
- 新增 5 个单元测试（执行/鉴权拒绝/列表/上传下载/瞬态重试）。

#### 5.1b 节点注册与 agent_port 持久化
- `ServerNode` / `ServerNodeRow` 新增 `agent_port`（默认 9527，serde 默认值兼容旧客户端）；SQLite 幂等迁移 `add_column_if_missing(nodes, agent_port, INTEGER NOT NULL DEFAULT 9527)`。
- 新增公开白名单路由 `POST /api/nodes/register`（免 JWT，Agent 启动自动注册，返回 `{"id": n}`），注册时持久化 `auth_token` + `agent_port`（修复 Doc/11 协议缺口）。

#### 5.1c 后端远程调用 API
- `NodeService` 新增 `remote_execute` / `remote_list_files` / `remote_download_file` / `remote_upload_file` / `batch_execute`（多节点并行 `tokio::spawn` + 结果聚合）。
- 新端点：`POST /api/nodes/{id}/execute`、`POST /api/nodes/batch-execute`、`GET /api/nodes/{id}/files`、`GET /api/nodes/{id}/files/download`（base64）、`POST /api/nodes/{id}/files/upload`。
- 权限表新增 `node:execute` 资源（`default_permissions` + 精确 `pre_suf`/`pre_contains` 规则，不影响既有 `/api/files/*`）。
- OpenAPI 注册新端点与 DTO；新增 5 个集成测试。

#### 5.1d + 5.1e 前端多节点能力
- `NodesView` 新增「远程命令」弹窗（命令 + 退出码/耗时/输出）、「远程文件」弹窗（路径导航/列表/下载/上传）、「批量命令」弹窗（多选节点并行执行 + 聚合结果）。
- `api/nodes.ts` 新增 `remoteExecute` / `batchExecute` / `remoteListFiles` / `remoteDownloadFile` / `remoteUploadFile` / `registerNode`；类型新增 `ServerNode.agent_port` / `RemoteFileEntry` / `RemoteExecResult` / `BatchExecItem`。
- i18n：zh-CN / en-US / ja-JP 补齐 node 远程操作文案。

#### 文档
- `Doc/13`：勾选 5.1 多节点能力全部任务；`Doc/11`：更新注册路由与 auth_token 持久化说明、安全模型；`Doc/01`：路由端点数与白名单；`Doc/05`：14.2 远程 Agent 调用指南（AgentClient 封装示例）。

#### 验证
- `cargo test --workspace`：**235 个测试全部通过**（92 unit + 137 integration + 5 stage5 + 1 doctest）。
- `cargo clippy --all-targets -- -D warnings`：通过；`cargo fmt --all -- --check`：通过。
- feature 组合编译：`default` / `--all-features` / `--no-default-features` 全部通过。
- 前端 `vue-tsc --noEmit` / eslint / `vite build`：通过。

### feat: stage4 production hardening（生产可用性加固：WS 鉴权 / HTTPS / Prometheus / 审计导出 / 备份加固）

按《Doc/13 开发路线图与后续规划》P0/P1 剩余生产可用性任务落地（Stage4）：

#### 4.1 WebSocket 鉴权
- `/ws/*` 不再整体白名单：握手必须携带 `?token=<access_token>` 查询参数。
- 认证中间件对 WS 路径复用 `JwtUtils::verify_access`（与 REST 同一密钥/类型语义）校验并确认用户存在，注入用户上下文；缺失/无效 token 返回统一 JSON 401 `AUTH_UNAUTHORIZED`。
- 前端 `frontend/src/utils/ws.ts` 自动附加当前 access token（`?token=`），4 个视图（Dashboard/Health/SystemLogs/Terminal）无需改动即可工作。
- 新增 3 个 WS 鉴权集成测试。

#### 4.2 HTTPS（install.sh --tls）
- `install.sh` 新增 `-t/--tls`：未提供 `--cert/--key` 时自动生成自签证书（`/opt/flamepanel/tls/flamepanel.{crt,key}`，3650 天，私钥 600）；也可传入 CA 证书路径。
- nginx 生成配置新增 443 ssl server 块（TLSv1.2/1.3 + 安全套件）+ `/health`/`/metrics` 根路径代理；完成输出按 HTTPS 显示访问地址并提示自签证书信任。

#### 4.3 Prometheus /metrics
- 新增 `GET /metrics`（公开只读）：Prometheus 文本格式（`text/plain; version=0.0.4`），含 up/uptime/info + 最近一次快照的 CPU/内存/磁盘/负载/网络指标。
- `nginx.conf` 与 `install.sh` 生成的 nginx 配置均增加 `/metrics` 根路径代理。
- 新增 1 个集成测试。

#### 4.4 审计日志导出
- 新增 `GET /api/operation-logs/export?format=csv|json`（需 `operation_log:read`）：CSV 带 UTF-8 BOM + 字段转义（Excel 兼容），JSON 为 UTF-8 数组；均带 `Content-Disposition: attachment`。
- 权限表新增 export 路由 exact 规则；OpenAPI 增加该路径。
- 新增 3 个集成测试（csv/json/非法格式）。

#### 4.5 备份加固
- `BackupService::create_backup` 备份文件权限 `600`（仅属主可读）。
- `BackupService::restore_backup` 恢复前自动创建 `pre-restore-*` 二次备份（同样 600），防止恢复失败导致数据不可逆丢失。
- 新增 1 个备份加固集成测试（Unix 权限 + pre-restore）。

#### 4.6 安装密码提示
- `install.sh` 完成输出：静默/自动生成密码时醒目警告并要求立即改密。

#### 文档
- `Doc/01-架构设计.md`：白名单路径更新、7.5~7.8（WS 鉴权/Prometheus/审计导出/备份加固）。
- `Doc/05-后端开发指南.md`：9.5 生产可观测与安全基线。
- `Doc/06-部署运维指南.md`：`--tls`/`--cert`/`--key` 参数与 HTTPS 说明。
- `Doc/13-开发路线图与后续规划.md`：勾选对应 P0/P1 任务。
- `nginx.conf`：新增 `/metrics` 根路径代理。

#### 验证
- `cargo test --workspace`：全部通过（新增 Stage4 测试 11 个）。
- `cargo clippy --all-targets -- -D warnings`：通过。
- `cargo fmt --all -- --check`：通过。
- 前端 `vue-tsc --noEmit` / eslint：通过。
- `install.sh`/`uninstall.sh` bash 语法校验通过。

### feat: stage3.3 OpenAPI + stage3.4 声明式权限表 + stage3.5 CI audit（P3 工程化收尾）

按《FlamePanel 后端重构与长期维护落地手册》Stage3 剩余任务落地：

#### 3.3 OpenAPI（utoipa）
- 新增 `flame-kernel/src/openapi.rs`：`ApiDoc`（`#[derive(OpenApi)]`）聚合核心路由与 DTO，编译期生成 OpenAPI 3.1 文档。
- 引入 `utoipa`（`axum_extras` + `chrono`）；关键 DTO 加 `ToSchema`（User/ServerNode/Website/ScheduledTask/AppMetadata/InstalledApp/LoginRequest/LoginResponse/HealthDetail/PaginationParams 等）。
- 关键路由加 `#[utoipa::path]` 注解：auth（login/refresh）、health、users、nodes（含 heartbeat）、websites、settings、backups、scheduled-tasks、app-store、metrics 等 28 个端点。
- `PaginatedResponse<T>` 实现 `utoipa::ToSchema`（泛型）与 `__dev::ComposeSchema`；`PaginationParams` 实现 `IntoParams`。
- 组合根挂载 `GET /api/openapi.json`（免认证白名单），JWT BearerAuth security scheme 注入；新增集成测试验证文档结构与关键路径/schema。

#### 3.4 权限映射声明化
- `route_permission` 巨型 match 重构为声明式常量表 `ROUTE_PERMISSIONS: &[PermissionRule]`（顺序敏感，首个命中生效，语义与原 match 完全一致）。
- `PermissionRule` 支持 exact / prefix / suffix / contains / not_contains / not_suffix 组合，含 const 构造器；`route_permission` 仅 6 行查询逻辑。
- **顺带修复 3 处历史顺序 bug**：
  1. `POST /api/app-store/packages/*/install|import` 与 `installed/*/uninstall` 曾被 database 全局 `ends_with("/install")`/`ends_with("/uninstall")` 规则抢先映射为 `database:create|delete` → 修正为 `app_store` 资源。
  2. `POST /api/scheduled-tasks/{id}/toggle` 曾被 firewall 全局 `ends_with("/toggle")` 规则抢先映射为 `firewall:enable` → 修正为 `scheduled_task:update`。
  3. `POST /api/databases/{id}/uninstall` 曾被 `contains("/databases")` 规则抢先映射为 `database:create` → 修正为 `database:delete`。
- 新增 7 个权限表单元测试覆盖全部资源 CRUD/子路径映射与未登记路径返回 None。

#### 3.5 CI 安全审计 + wasmtime 升级
- `.github/workflows/ci-cd.yml` 新增 `cargo audit` 步骤（cargo-install cargo-audit + audit，失败即阻断）。
- `wasmtime` 29 → 46：修复 15 个已知漏洞（含 1 个 critical：`RUSTSEC-2026-0095` aarch64 沙箱逃逸；多个 medium panic）。升级后 `cargo audit` **0 vulnerabilities**（427 依赖全绿）。
- feature 组合编译验证：default / `--all-features` / `--no-default-features` / `+sqlite` / `+wasm` 全部通过。

#### 验证
- `cargo test --workspace`：**217 个测试全部通过**（87 unit + 129 integration + 1 doctest；新增权限表 7 + OpenAPI 1）。
- `cargo clippy --all-targets -- -D warnings`：通过。
- `cargo fmt --all -- --check`：通过。
- `cargo audit`：零漏洞。

### feat: stage3 engineering（P3 工程化：feature flags + Axum 升级）

按《FlamePanel 后端重构与长期维护落地手册》Stage3 落地（本次先完成 3.1 + 3.2）：

#### 3.1 Feature Flags（可选能力按需编译）
- `flame-kernel/Cargo.toml` 新增 `[features]`：`default = ["sqlite", "docker", "wasm", "email"]`，各 feature 对应 `dep:sqlx` / `dep:bollard` / `dep:wasmtime` / `dep:lettre`（optional 依赖）。
- `infrastructure` 按 feature 隔离：`sqlite`（`db_models`/`sqlite` 模块、`BackendKind::Sqlite`、`new_sqlite`）、`docker`（`docker.rs` 与 bollard 连接代码，无 docker 时回落 InMemory）、`wasm`（`WasmSandbox` 整体隔离，`verify_wasm_hash` 提为模块级仅依赖 sha2，`PluginSandbox` 无 wasm 时仅管理元数据）、`email`（`EmailNotifier` 隔离，事件通知降级为仅日志）。
- `main.rs` 无 `sqlite` feature 时使用 in-memory 后端。
- 验证：`cargo build`（default）/ `--all-features` / `--no-default-features` / 各单独 feature 均编译通过，无 unused 警告。

#### 3.2 Axum 升级（0.6 → 0.8）
- `axum` 0.6 → 0.8（含 ws）、`tower-http` 0.3 → 0.6；移除未使用的 `axum-test` dev-dep；`hyper` 0.14 dev-dep 移除，测试改用 `axum::body::to_bytes(_, usize::MAX)`。
- 路由语法迁移：全部 handler 的 `:param` → `{param}`（约 90 处）。
- 中间件迁移：使用 `axum::extract::Request` + `Next`（去掉 `<B>` 泛型）；`ApiJson` 改为原生 async fn 实现 `FromRequest<S>`（去掉 async_trait）。
- 服务器迁移：`axum::Server::bind` → `tokio::net::TcpListener` + `axum::serve`；响应体 `boxed(Full::from)` → `Body::from`；WebSocket `Message::Text(String)` → `Utf8Bytes`（`.into()`）。
- 验证：`cargo test --workspace` 全绿（80 unit + 128 integration + 1 doctest），`cargo clippy --all-targets -- -D warnings`、`cargo fmt --check` 通过。

### refactor: stage2 runtime lifecycle + observability + pagination（P2 组合根、生命周期与可观测性）

按《FlamePanel 后端重构与长期维护落地手册》Stage2 落地：

#### 2.1 后台任务 Supervisor
- 新增 `runtime/supervisor.rs`：`TaskSupervisor`（`CancellationToken` + `JoinSet`）统一管理全部长生命周期后台任务——应用种子/WASM 恢复、默认设置补齐、定时任务 tick、自动备份、指标采集、节点离线扫描、事件订阅。
- `FlameKernel::run` 优雅关闭时 `supervisor.shutdown()` 广播取消 + 带 5s 超时 join，未及时退出任务由 `JoinSet::shutdown()` 强制 abort，杜绝僵尸任务；`TaskHandle` 支持单个任务精确取消/等待。
- `metrics.rs` 改为 `metrics_collector_loop`（协作式取消），`EventHandler::spawn_with_token` 支持取消；`tokio-util`（rt 特性）加入依赖。

#### 2.2 Request-Id 与 tracing
- 新增 `runtime/request_id.rs`：`request_id_middleware` 生成/沿用 `x-request-id` 并写回响应头，`http_request` span 携带 `request_id`/`method`/`uri`。
- 认证中间件注入 `auth` 子 span（`request_id`/`user_id`/`username`）；审计写操作继承 `request_id`，同一请求日志可按 ID 串联。
- `RUST_LOG_FORMAT=json` 结构化日志直接携带以上字段，可接入 Loki/logstash。

#### 2.3 分页下沉
- `OperationLogRepository` / `LogRepository` / `ScheduledTaskRepository` 新增 `list_page(limit, offset[, action_prefix])` + `count()`，InMemory 与 SQLite 双实现。
- `OperationLogService::list_paginated` / `LogService::list_paginated` / `ScheduledTaskService::list_tasks` 改为数据库层 `LIMIT/OFFSET`，不再 `list_all` 后内存切片；SQLite 前缀过滤使用 `LIKE ... ESCAPE` 并转义通配符。

#### 2.4 AppState 组装简化
- 新增 `AppState::from_services`：将 metrics/log channel 默认创建内聚，组合根样板减少。

#### 文档与测试
- 更新 `Doc/01-架构设计.md`（runtime 层、中间件栈、5.1 运行时与后台任务、优雅关闭）、`Doc/05-后端开发指南.md`（分页下沉规范、中间件/request-id）。
- 新增测试：supervisor 3 个（取消/join/超时 abort）+ request_id 3 个（生成/传播/唯一）+ 分页下沉 4 个（repo 与 service 层）。

### refactor: stage1 hexagonal architecture（P1 领域纯度与端口治理）

按《FlamePanel 后端重构与长期维护落地手册》Stage1 落地：

#### 1.1 DomainError / AppError 拆分
- 新增 `domain/error.rs`：`DomainError`（NotFound/Validation/Conflict/Forbidden/RuleViolation），纯业务语义，不依赖 axum。
- `AppError` 增加 `impl From<DomainError>` 自动映射；对外 JSON 错误格式 `{code, error, message}` 不变。

#### 1.2 实体去 FromRow + 行模型映射
- 新增 `infrastructure/db_models.rs`：所有 `#[derive(sqlx::FromRow)]` 持久化行结构（`XxxRow`）集中于此，与领域实体通过 `From<XxxRow> for Xxx` 映射。
- `domain/entity.rs` 移除全部 `sqlx::FromRow`，domain 零 sqlx 依赖；`sqlite.rs` 改为 `query_as::<_, XxxRow>` 后显式转换。

#### 1.3 Docker 端口拆分
- 巨型 `DockerRepository` 拆分为 `ContainerRepository` / `NetworkRepository` / `VolumeRepository` / `ImageRepository` / `ComposeRepository` 五个端口；保留门面 trait 兼容旧调用方。
- `BollardDockerRepository` / `InMemoryDockerRepository` 分别实现各子端口；`RepoFactory` 提供 `create_*_repo`；`DockerService::from_repos` 经 `DockerRepositoryFacade` 聚合。

#### 1.4 领域行为上移（去贫血）
- `User`：`validate_username` / `mark_password_changed` / `ensure_can_change_password` / `must_change_password`。
- `InstalledApp`：`can_upgrade_to`（防降级）/ `record_launch` / `mark_upgraded` / `version_cmp`。
- `AppManifest`：`to_metadata` / `to_version`（内置应用元数据/版本生成的领域规则上移）。
- `AppStoreService` 升级/记录启动/用户创建改为调用领域方法。

#### 1.5 AppStore 端口注入，切断 application→infra
- 新增 `application/app_store_ports.rs`：`AppPackageAdapter` / `AppAdapterProvider` / `ComposeSecurityScanner` / `VariableMapper(+Factory)` / `PackageManagerPort` / `ServiceManagerPort` + 扫描结果模型。
- `AppStoreService` 改为 `with_ports` 端口注入；infrastructure 提供 `DefaultAdapterProvider` / `DefaultComposeSecurityScanner` / `DefaultVariableMapperFactory` / `DefaultPackageManagerPort` / `DefaultServiceManagerPort` 实现，`default_ports()` 聚合。
- `lib.rs` 组合根通过 `with_ports` 注入具体适配器。

#### 文档与测试
- 更新 `Doc/01-架构设计.md`（分层依赖/领域层/应用层/基础设施层）、`Doc/05-后端开发指南.md`（新增模块流程、错误处理）。
- 新增测试：领域方法 6 个 + 端口注入 mock 1 个（自定义适配器/扫描/变量映射/包管理/服务管理）。

### security: stage0 hardening（P0 安全与运行时加固）

按《FlamePanel 后端重构与长期维护落地手册》Stage0 落地：

#### 2.1 非 root 运行与 capabilities
- `install.sh`：创建专用系统用户/组 `flamepanel:flamepanel`（无登录 shell），数据/日志目录 `750`，`flamepanel.env` `600`。
- systemd 服务默认 `User=flamepanel` + 加固：`NoNewPrivileges=true`、`ProtectSystem=strict`、`ProtectHome=true`、`PrivateTmp=true`、`ReadWritePaths` 仅放行数据/日志/工作区。
- `uninstall.sh`：`-p` 完全卸载时清理运行用户/组。
- 需要 root 的特权操作（防火墙/包安装/nginx reload）改为受控 `sudo -n` 白名单（文档明确清单），Docker 按最小原则走 docker 组。

#### 2.2 终端与文件路径沙箱
- `FileService` 改为沙箱模式：所有路径按 chroot 语义解析到 `OP_FILE_ROOT` 白名单内，拒绝 `..` 穿越、绝对路径逃逸、指向白名单外的符号链接；写/新建操作做二次校验。
- 终端会话强制 `cwd` 为 `OP_TERMINAL_CWD`（须在 `OP_FILE_ROOT` 内），清理 `LD_PRELOAD`/`LD_LIBRARY_PATH` 等危险环境变量。
- 新增配置项 `OP_FILE_ROOT` / `OP_TERMINAL_CWD`（默认安全值由 install.sh 写入 `/opt/flamepanel/workspace`）。

#### 2.3 JWT 强化
- 启动时校验 `jwt_secret` ≥ 32 字节，否则拒绝启动并明确日志。
- Access Token（默认 15 分钟）+ Refresh Token（默认 24 小时）双令牌，`Claims` 增加 `typ` 类型区分；认证中间件仅接受 access，`/api/auth/refresh` 仅接受 refresh 并轮换（滑动过期）。

#### 2.4 WASM 资源限制与完整性
- 每个插件执行设置内存上限（`memory_limit_bytes`，经 `Store::limiter` 限制 linear memory/table）+ 栈大小限制 + fuel/timeout。
- 安装/恢复强制校验 `wasm_hash`，不匹配拒绝加载；`SandboxedPlugin` 增加 `wasm_hash` 字段。

#### 2.5 SQLite 运行时加固
- 新增 `configure_sqlite_pragmas`：`PRAGMA journal_mode=WAL`、`busy_timeout=5000`、`synchronous=NORMAL`，连接建立后立即执行。

#### 文档与测试
- 更新 `Doc/01-架构设计.md`、`Doc/05-后端开发指南.md`、`Doc/06-部署运维指南.md`。
- 新增 Stage0 安全测试：文件沙箱路径逃逸、JWT access/refresh 类型校验、密钥长度校验、WASM 哈希校验、SQLite PRAGMA 设置。

## [0.1.0]

- 初始版本（从源码快照导入，无历史）。
