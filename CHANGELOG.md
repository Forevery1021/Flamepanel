# Changelog

本文件记录 FlamePanel 后端（flame-kernel）与部署脚本的重大变更，遵循 [Keep a Changelog](https://keepachangelog.com/) 风格。

## [Unreleased]（1.1.0 开发中）

### feat(restore): 恢复证据同源预检与后续验收路线（R-01E-1）

- 新增只读 `preflight.py`，关联演练/专用探针、资产盘点和按需密钥报告的源快照摘要/大小、固定字段计数与 JWT 计划；混源、缺失、矛盾标记/计数或不安全输入阻断，不自动补证据。
- 最多返回 REVIEW_REQUIRED；不把报告一致性当作真实性、新鲜度、生产身份、密钥验证或启动授权。不读取资产秘密、不打开数据库、不启动进程，不修改/撤销/重签发会话。
- 新增合同与真实 BackupService/探针/盘点/密钥报告链路回归；保留 A/B/C/D 默认行为、依赖锁、云构建、镜像和门槛。
- 新增 Doc/41，将身份、业务、隔离启动、外部资产/RTO/RPO 及 R-02/R-03 拆成有前置/验收/停止条件的候选；校准 Doc/30、Doc/36、Doc/40 的当前状态，不把后续规划视作开工或发布授权。

### feat(restore): 显式授权的离线恢复主密钥匹配验证（R-01D）

- 新增独立 `keycheck.py` 和内部 `fp-restore-keycheck`；仅在明确授权后读取私有暂存主密钥，经匿名 stdin 管道复用现有 SecretCipher，不把密钥放入 argv / 环境 / 报告，不返回解密值。
- 对额外 SQLite 副本的固定加密字段做只读、有界扫描；无密文、覆盖缺口、未知版本、超预算不误报匹配。错误密钥与坏密文合并为未验证，部分成功不代表整体匹配。
- 保留 R-01C 默认不读秘密的行为；所有结果仍声明生产内核 / 身份 / 完整恢复未验证。不新增生产 API/UI、任务、依赖、构建配置或密钥生成 / 轮换策略。
- 新增标准库编排回归与真实 SecretCipher / 编译产物集成验收；同步 Doc/30、Doc/36、Doc/39 及新增 Doc/40，源码与运行证据分开记录。

### test(auth): 内核与 Agent 共用判等向量，保持独立安全边界

- 新增仅测试引用的合成 UTF-8 比较向量与自检，两端分别调用既有实现；覆盖空值、前缀、NUL、Unicode、空白、长输入及常见错误比较语义。
- 内核补原始字节逐位置回归，Agent 补合法 ASCII 逐位置及未配置令牌拒绝回归；不把空串比较相等当作鉴权成功。
- 不改比较算法、长度早退、Bearer / 裸令牌协议、依赖或公共运行时接口，不增加 Agent 对内核的依赖；功能回归不等同恒定时间证明。同步 Doc/30 与 Doc/35，实际门禁与推送状态见交付报告。

### fix(frontend): 构建配置兼容原生 ESM 加载并保持 CSP 产物

- Vite 配置显式导入 `scripts/csp.ts`，TypeScript 采用允许 `.ts` 导入的无产物检查；Vitest 配置改用 `import.meta.dirname`，不依赖打包器补齐 CommonJS 全局。
- 新增独立 Node / Vite 原生配置加载及真实临时构建回归，保持 CSP meta、内联脚本哈希、HTTP frame-ancestors 和缺占位符拒绝行为；用真实项目原生构建对比基线产物。
- 新测试进入现有 Vitest 发现范围，不屏蔽兼容警告、不改 CSP 策略、依赖锁、云原生构建或质量阈值；同步前端指南和维护路线。

### fix(dev): 云开发 SSH 连接强制核验官方主机指纹

- `cnb-workspace.sh` 的全部 SSH 入口使用官方 RSA 公钥 SHA-256 固定值预核验，再启用 strict checking 与 RSA/SHA-2；拒绝非官方目标、选项注入、异常 / 空 / 混合公钥，不回退首次自动信任。
- 信任文件独立、私有且退出后清理，不改用户 known-hosts；保留远端命令参数和退出码。指纹轮换需按官方公告评审，不提供环境变量绕过。
- 新增无网络脚本回归和公开主机公钥 fixture，覆盖成功、拒绝、全部连接入口、路径空格、临时权限 / 清理及原用户信任不变；同步 Doc/30、Doc/31、Doc/35。
- 不修改云原生构建、镜像、依赖、锁文件、生产功能或门禁阈值；实际验证结果见独立交付报告。

### feat(ops): R-01C 恢复资产与外部密钥只读盘点

- 新增严格角色 manifest、只读 SQLite 聚合与暂存资产元数据检查；报告不读取或输出密钥文件内容，不跟随 DB 中的任意路径。
- 缺失必需资产、不安全权限、符号链接、硬链接、空文件和特殊文件阻断；缺列 / 未知密文版本保守处理，不把目录存在视为完整备份。
- JWT 保留 / 重签仅为计划；结果最高为 REVIEW_REQUIRED，密钥匹配、生产身份和恢复就绪始终不作已完成声明。
- 新增 Python 安全边界回归及真实迁移 / SecretCipher 集成测试，错误密钥存在不能冒充解密成功。来源矩阵与操作方法见 Doc/39；不改生产装配、API、依赖或云原生构建。

### feat(ops): R-01B 专用恢复探针与 HTTP 拒绝路径验收

- 新增独立 `fp-restore-probe` 入口，仅装配 SQLite 迁移 / RBAC seed 与两个本机只读 HTTP 路由；不读取生产配置，不构造 Services 或启动任务 / 节点 / 通知。
- 所有迁移仅写随机私有副本；拒绝活动 schema，迁移前后检查完整性，关闭写池后才监听；短时随机 token 经 stdin 传递，绑定 loopback 随机端口，支持 TTL 与 SIGTERM 清理。
- 离线演练增加可选 `--probe-bin` 阶段，验证鉴权、写方法与生产路由拒绝、只读聚合和进程回收；默认离线模式不变，生产内核 / 生产鉴权 / RTO / RPO 不作已验证声明。
- 新增真实进程回归，保留上一片离线测试。文档见 Doc/38；不改生产启动入口、API / 权限表、依赖锁、构建和发布配置。

### feat(ops): R-01 离线 SQLite 隔离恢复演练首片

- 新增 Python 标准库工具 `scripts/restore-drill/drill.py`：本地独立快照 / 有界 HTTPS 获取、可选可信 SHA-256、完整性与最小 schema 检查、临时复制恢复、数据库重开与固定表抽样，输出 JSON / Markdown 阶段报告。
- 禁止指定生产恢复目标；拒绝活库旁文件、符号链接和非普通文件；新建私有目录，副本按 0600 写入，正常 / 可捕获失败后清理，不回显数据、URL 或异常详情。
- 新增故障注入与隔离集成测试：通过真实迁移、WAL 模式 BackupService 快照、停止连接池的恢复目标及恢复前二次备份验证；Python 合约用例由现有 Rust 集成门禁调用。
- 这是 R-01 首片而非完整服务恢复验收：不启动内核、不执行备份中的后台任务、不读取外部密钥；RPO 与生产 RTO 明确未知。详细范围见 Doc/37。
- 不改变生产 API / UI / 恢复方法、依赖锁、云原生构建或现有门禁脚本。

### fix(backup): 同值凭据更新保留密文；清理重复存储策略

- 修复备份账号编辑时，同一明文凭据可能覆盖已存密文的问题；同值更新保留原密文与正常元数据更新，行为对齐 DNS 账号。
- 新增服务级回归，检查实际仓储与服务返回值；CNB 开发环境中先在原实现复现失败，再验证修复。
- 两个服务的重复私有加解密分支收敛到 application 内部 `credential_storage`；新增 6 项边界回归，保留空值、历史明文、缺密钥、坏密文和随机 nonce 行为，不修改安全降级策略或公有接口。
- 清理两个服务中已过时的未来阶段说明；新增 Doc/36 后续路线提案，并在 Doc/30、Doc/35 与文档索引交叉引用。规划不表示已实现或已获功能开工授权。
- 本轮不修改云原生构建、镜像、依赖锁、权限、API 或数据库结构；完整验证状态见独立交付报告。

### fix(agent): CPU 拓扑启动采样缺失时回退有效核数（#187）

- `cpu_cores` 优先使用 sysinfo 的实际列表，瞬时为空时取标准库可用并发度，仍不可用时保守取 1；不改协议、采样间隔或其他指标。
- 保留原有 `cpu_cores >= 1` 断言，新增确定性单测覆盖已探测拓扑 / 空列表 / 无回退 / 无效回退；无睡眠、无测试忽略、无新增依赖。

### fix(build): 插件对话框严格类型检查与 API 生成工具链审计（#186）

- 测试改为两个具体组件的 mount factory，移除 `as never`，保留原 4 项行为断言并恢复 modelValue 的严格 props 类型。
- API 生成器定向升级至 openapi-typescript 7.13.0，移除旧 glob / braces 链；source-map-js 更新至 1.2.2。没有 force、legacy-peer-deps 或审计豁免。
- 生成器使用窄范围 `$typescript` override 保留项目 TypeScript 6；兼容性由实际 AST 生成、完整 typecheck 与单测验证。
- 在内存快照副本中按 HTTP 方法 + 完整路径生成唯一 operation ID，修复旧快照重复 handler 名在新生成器中的类型冲突；后端 API 快照不变，生成类型不手修。
- 新增 4 项生成器回归；对话框 + 生成器 8 项定向测试通过，npm audit 报告 0 漏洞。

### feat(iac): 只读 YAML 配置清单导出（#185，M-E 首片）

- 新增 `GET /api/iac/export`：下载 `flamepanel-config.yaml`（`flamepanel/v1alpha1`），无请求时间字段、区块稳定排序、响应禁止缓存。
- 入口复用 `settings:read`；网站 / 数据库 / 已安装应用分别检查对应读权限，无权限区块不查询且完全省略。
- 显式配置字段白名单；不含应用参数 / 环境变量 / 访问 URL、代理目标、数据库管理用户、密钥或原始文件。无依赖 / 迁移 / 新权限项。
- 配置清单不是完整备份或可直接重建的声明；不支持导入 / 部署 / 启停，不增加 UI，其余 M-E 架构项待独立决策。
- 新增服务单测与 4 个集成测试，覆盖 401 / 403、按资源裁剪、权限收回、稳定顺序、凭据面排除和只读性。

### fix(plugin): canonical 路径复核完整系统目录保护（#184）

- 已存在的能力根在授予时复核 canonical 路径，拒绝经符号链接间接授予系统写保护 / 凭据目录；安全的未创建目录仍可预先授予。
- 访问时读写共用完整保护策略，重新复核实际能力根，根无法解析时 fail closed；修复写访问遗漏 `/etc`、`/usr` 等写保护目录的问题。
- 恢复时丢弃指向保护目录的旧能力别名，合法根内的符号链接保持可用。
- 新增 4 项回归测试；原实现其中 3 项失败。测试仅解析路径，不写入系统文件。ABI、REST 与数据库结构不变。

### feat(plugin): WASM 插件宿主 API + 插件市场雏形（Doc/30 batch-14，M-D 收官）

- **宿主 API**（Doc/21 §5.7）：插件可从导入模块 `flame` 调用 `log`、`input_len` / `input_read`、`output`、`setting_get` / `setting_set`、`fs_read` / `fs_list` / `fs_write`、`metrics`。约定是「调用方给缓冲区，返回完整长度」，负数为稳定错误码（-1 无能力 … -6 暂无数据）。其他任何导入在实例化时失败，仍无网络和进程能力。
- **能力授予**（deny-by-default）：
  - 四种能力：`fs:read:<dir>`、`fs:write:<dir>`、`metrics:read`、`settings:write`。
  - 授予时校验：拒绝 `/`、`/proc`、`/sys`、`/dev`、`/boot`、`/root`，以及这些目录的子树和祖先。写能力另外拒绝 `/etc`、`/usr`、`/bin`、`/lib*`、`/var/lib` 等目录。凭据文件不可授予。
  - 访问时校验：canonicalize 后必须仍在能力根内，并再比对一次凭据黑名单，因此根目录内指向根外的符号链接也会被拒绝。
  - 能力随插件持久化，`reload` 不改变已授予的能力。
  - `POST /api/plugins` 新增 `capabilities` 字段；新增 `PUT /api/plugins/{id}/capabilities`（`plugin:create`）。
- **执行**：
  - `timeout_ms` 改为墙钟强制。此前它只被校验、从未生效，只靠 fuel 兜底，插件阻塞在宿主调用上时 fuel 拦不住。现由看门狗线程在超时时递增 epoch 来打断执行，执行结束后线程立即退出。
  - 执行请求新增 `input` 字段，响应新增 `logs`；插件调用了 `flame.output` 时，`output` 返回该输出。旧插件和 `args` 字段照常工作。
- **插件市场**（Doc/21 §5.8）：
  - `GET /api/plugins/market` 返回 3 个内置示例：hello / metrics-snapshot / log-viewer。它们以 WAT 内嵌、安装时编译，单测断言编译产物的哈希。
  - 可选的远程索引由设置项 `plugin_market_url` 指定，限制为：仅 http(s)，索引 ≤ 1 MiB，包体 ≤ 8 MiB 且流式计数，**SHA-256 强校验**，非法条目跳过。
  - `POST /api/plugins/market/{id}/install`：插件申请了能力时必须带 `accept_capabilities: true`（否则 400），同 ID 已安装返回 409。
- **修复插件持久化的四处不一致**：
  - 重启后插件只恢复到沙箱、不进注册表，导致 `GET /api/plugins` 为空、执行返回 404。
  - 插件设置只存在内存里。
  - 启停状态不落库。
  - 卸载不删除数据库记录，插件重启后复活。
- **修复同 ID 加载**：此前沙箱先被覆盖，注册失败才返回 400，留下不一致状态。现在先检查冲突，直接返回 **409**；安装途中任何一步失败都会回滚。插件 ID `market` 为保留字。
- **前端**（插件页）：
  - 新增「已安装 / 插件市场」两个页签。
  - 能力徽标按风险着色：写目录红、读目录黄、其余蓝。
  - 安装申请了能力的插件前先弹授权确认，列出能力的人话描述和包体哈希。
  - 「能力」对话框可修改或撤销授予。
  - 执行对话框改为「输入 → 输出 / 日志 / 耗时」。
  - 加载表单可填写能力。
  - 三语 i18n。
- **数据库**：`plugins` 表幂等新增 `capabilities`、`settings` 两列（`add_column_if_missing`），旧库可直接升级。
- **依赖**：内核把已在锁文件中的 `wat` 声明为 `wasm` feature 的可选依赖，`Cargo.lock` 没有新增 crate。
- **测试**：
  - 内核单测 23 项：
    - 能力解析与路径解析：`..`、符号链接逃逸、凭据文件。
    - 宿主 API 走真实 wasmtime：读写鉴权、越界指针、未知导入、日志上限、墙钟超时打断死循环、fuel 耗尽。
    - 市场：远程索引过滤，内置 WAT 哈希。
    - 重启恢复：能力和设置都能带回。
  - 集成测试：新增 `tests/plugin_market.rs`（4 组），覆盖 401、viewer 可浏览但安装和授予返回 403、授权同意、执行输出、409、敏感目录 400、设置与能力落库、卸载删库。
  - 前端：新增 12 例。
  - OpenAPI 共 327/327 条路由。

### feat(node): Agent Edge 出站反向通道 + 交互式远程终端（Doc/30 batch-13）

- **Agent Edge**（Doc/23 §8）：
  - Agent 新增 `AGENT_MODE=direct|edge|both`。`edge` 模式不监听任何端口，主动以 WebSocket 连接面板 `/ws/agent/edge`（`PANEL_URL` http→ws / https→wss，rustls + webpki 根证书），NAT / 防火墙后的节点零入站。
  - 首帧 `hello` 以节点 `auth_token` 常量时间校验，失败延迟 500 ms 后以 4001 关闭；同节点重连顶替旧连接并让其未完成请求立即失败；面板每 20 s Ping，90 s 无入站帧判定半开；Agent 断线指数退避 1 → 60 s 重连；删除节点即断开。开 / 断写审计 `EDGE_CONNECT` / `EDGE_DISCONNECT`。
  - 面板 `AgentClient` 重构为统一的 `send()`（断路器 + 重试），节点 Edge 在线时 `/action`、`/files/*` 经 `req` / `resp` 帧传输。**帧内不携带 token**，Agent 在进程内把请求回放到自身 `agent_routes()`，与直连模式同一套白名单、文件根目录与限流。公共方法签名不变。
  - `GET /api/nodes/{id}/status` 增加 `edge: {connected, terminal, version, connected_at}`；节点列表显示「Edge」徽标。
- **交互式远程终端**：
  - 浏览器连 `/ws/terminal?node_id=<id>`，协议与本机终端一致，另有 `exit` 帧与 `error` 帧（`TERMINAL_DISABLED` / `TOO_MANY_TERMINALS` / `EDGE_OFFLINE`）。
  - 需 `terminal:execute` + `node:execute`（缺失 4003，节点不存在 4004），开 / 关审计 `OPEN_REMOTE_TERMINAL` / `CLOSE_REMOTE_TERMINAL`。
  - Agent 侧 portable-pty（与本机终端同实现），**默认关闭**，须 `AGENT_TERMINAL=1`；单节点 ≤ 8 个，断线关闭全部 PTY；多字节 UTF-8 跨帧流式拼接。
  - 前端：节点列表「远程终端」入口（仅 Edge 在线且 Agent 开启终端时出现）→ `/nodes/:id/terminal`；TerminalView 支持节点模式，不可恢复错误停止自动重连、`EDGE_OFFLINE` 退避等待；本机与远程终端都会在连接建立 / 尺寸变化时同步 `resize`。
- **fix(agent): 面板 → Agent 鉴权一直失败**：Agent 拿整个 `Authorization` 头值与 token 比对，而面板发送的是 `Bearer <token>`，真实 Agent 上的远程命令 / 远程文件 / agent 执行模式的特权助手调用一律 401（内核单测用的是期望 `Bearer` 的 mock，未暴露）。现接受 `Bearer <token>`（scheme 大小写不敏感），兼容裸 token。
- **项目守护 unit 纳入 Agent 白名单**：`SystemdSupervisor::write_unit` 由「路径拼进脚本的 `bash -c`」改为常量脚本 `sh -c 'printf '%s' "$1" > "$2"'` + 位置参数。Agent `install_policy` 校验路径只能是 `/etc/systemd/system/flame-proj-<id>.service`，内容逐行比对 `render_unit` 的节 / 键白名单（ExecStart 限允许前缀、拒绝 systemd 执行前缀 / shell 元字符 / `..` / `LD_*` / 多余指令）。`rm -f` 同一路径与 `systemctl daemon-reload` 精确放行，此前 agent 执行模式下项目守护必然失败。
- **fix(build): clippy 1.99 `double_must_use`**：升级 `async-trait` 至 0.1.92（上游不再给 `Pin<Box<dyn Future>>` 注入 `#[must_use]`），撤销 batch-12 加的 crate 级 `allow`。
- 依赖：仅声明工作区锁文件中已有的 crate（Agent：`tower` 0.5 / `tokio-tungstenite` / `rustls` / `portable-pty` / `base64` / `futures-util`；内核 dev：`tokio-tungstenite`），`Cargo.lock` 无新增 crate。
- 测试：
  - 内核：`infrastructure::edge` 单测 9 项（含重连顶替、超时清理、PTY 上限与 opt-in、UTF-8 流式解码）。
  - 集成：新增 `tests/edge.rs`，以真实 WebSocket 起面板 + 伪 Agent + 伪浏览器，共 4 组。覆盖 hello 4001、远程命令 / 文件经 Edge（断言帧内无 token）、状态透出、终端双向流 / 跨帧 UTF-8 / exit、`EDGE_OFFLINE` / `TERMINAL_DISABLED`、401 / 4003 / 4004、删除节点断开。
  - Agent：34 项（新增 `check_auth` Bearer、`edge` 回放经真实 `/action` 鉴权、真实 PTY 回显 / resize / 上限、unit 写入 / `rm` 策略与内核源码防漂移）。
  - 前端：`TerminalView` 8 例；`NodeListPanel` / `useNodeList` / `NodesView` 的 Edge 徽标与终端入口。全量 1755 例通过。


### feat(node): 节点指标并入 MetricsHub —— 仪表盘按节点切换（Doc/30 batch-12）

- **心跳扩展为完整快照**：Agent 心跳在原 4 项之外上报 5/15 分钟负载、核数、内存 / 磁盘总量与已用、网络收发速率（按两次心跳的实际间隔计算，计数回绕记 0）。面板侧新字段全部 `#[serde(default)]`，旧 Agent 照常工作。Agent 采集改为只刷新 CPU 与内存，不再每 10 秒全量扫描进程表。
- **统一模型**：面板把心跳规整为与本机同构的 `MetricsSnapshot`：
  - 百分比钳制到 0–100，非有限值和负值记 0，核数上限 4096，已用内存不超过总量；
  - 时间戳取面板接收时刻，不信任节点时钟；
  - 结果写入 `metrics_json`，并发布到该节点独立的内存 `MetricsHub`（120 点 ≈ 20 分钟）；删除节点时释放。
- **接口**：
  - 新增 `GET /api/nodes/{id}/metrics/history`（`node:read`，节点不存在返回 404）。
  - `/ws/metrics?node_id=<id>` 推送与本机相同的 `init` / `tick` 帧。`node_id` 非法时握手返回 400；节点不存在在首帧鉴权通过后以 **4004** 关闭，未认证连接无法借此探测节点 ID。前端收到 4004 停止重连。
  - `MetricsSnapshot` 进入 OpenAPI，共 324/324 条路由。
- **前端**：
  - 仪表盘右上角新增节点切换器，只在存在节点且持有 `node:read` 时出现；当前节点写入 URL 的 `?node=`。
  - 节点列表新增「监控」入口，直达该节点的仪表盘。
  - 切换节点时关闭旧订阅并清空曲线与指标卡，两台机器的数据不会拼在一起。
  - 远程节点固定为实时范围（持久化历史只有本机），并隐藏本机进程 TOP。
  - `useSharedWebSocket` 新增 `manageLifecycle: false`，由调用方统一挂接生命周期，可在 watch 中重建端点。
- **clippy 1.99 兼容**：新版 clippy 对 `#[async_trait]` 展开代码报 `double_must_use`（329 处，均为宏生成代码）。CI 使用 rustup stable，此前 `clippy -D warnings` 会整体失败，现在 crate 级放行。（batch-13 已改为升级 async-trait 0.1.92 根治并撤销放行。）
- 测试：
  - 内核：`node_metrics` 单测 5 项。
  - 集成：`tests/node.rs` 覆盖旧协议与完整心跳、钳制、历史、401 / 403 / 404、删除后释放；`tests/ws.rs` 覆盖 `node_id` 握手放行与非法参数 400。
  - Agent：`rate_mbps` 与完整字段 2 项。
  - 前端：`NodeSwitcher` 4 例，`useMetricsStream` 节点切换 4 例，`DashboardView` 节点切换 5 例，`useSharedWebSocket`、`ws` 4004、`NodeListPanel` / `NodesView`「监控」入口。

### feat: 应用升级三方合并 + MongoDB 原生支持 + 特权助手覆盖数据库 / 运行时安装（Doc/30 batch-11）

- **应用升级不再丢失手工修改**：安装 / 升级时把面板渲染的 compose 存为基线 `.flamepanel/compose.base.yml`（0600）。升级时按 YAML 结构三方合并 base / 本地 / 新版本：只有一方改动的取改动方，两方改成不同值、序列被两方各自修改、修改与删除并存记为冲突；用户未改时原样采用新版本文本（保留注释）。默认策略 `abort` 在 `pre_upgrade` 钩子和任何文件改动之前返回 **409** 并列出冲突路径；`?conflict_strategy=local|upstream` 指定冲突处理。旧文件备份为 `.flamepanel/docker-compose.<旧版本>.yml`。早于本功能安装的应用没有基线，每处差异都交给用户决定。
- **`GET /api/app-store/installed/{id}/upgrade-preview`**（`app_store:read`）：只读预览（版本、是否有基线、冲突的本地 / 新版本值、保留的本地修改、生效的上游修改、是否重新序列化）。前端「已安装 → 升级」改为升级弹窗：先预览，有冲突必须选择「保留本地 / 采用新版本」才能升级。
- **MongoDB**：`POST /api/databases/mongodb/install`（`database:create`）。走官方仓库 8.0（默认）/ 9.0：HTTPS + 签名密钥（apt `signed-by`、yum `gpgcheck=1`）；支持 Ubuntu noble / jammy（focal 仅 8.0）、Debian bookworm / trixie、RHEL 8–10，过旧发行版明确报错。填写管理员密码（≥ 8 位）即创建 `admin` 并开启鉴权，密码经 mongosh stdin 传递、不进 argv。启停 / 状态 / 卸载（保留数据）接入实例管理。项目联链注入 `MONGODB_URI`（凭据取 `MONGODB_ADMIN_PASSWORD`），MongoDB 首次写入时自动建库，联链无需预建。前端安装弹窗新增 MongoDB（版本下拉、鉴权说明、短密码即时提示）。
- **修复**：数据库建 / 删用户对 Redis（及 MongoDB）实例会落到 MySQL 分支、去连本机 MySQL；现在只接受 MySQL / MariaDB / PostgreSQL，其余返回 400。
- **特权助手扩展到数据库 / 运行时安装（逐项白名单）**：此前 agent 模式下 PostgreSQL 初始化、配置追加、Redis 改端口、RHEL `dnf install` 等都被 Agent 拒绝，数据库安装必然失败。新增 `agent/src/install_policy.rs`：
  - `sh -c` 只放行两段内核常量脚本：逐字节比对，校验 `$0` 名称和位置参数——目标文件限数据库配置清单，内容限单行，端口限数字；
  - `sed -i` 只放行 Redis / MongoDB 端口改写模板；
  - 补齐各发行版包管理与查询、`pg_isready` / `psql --version` / initdb、MongoDB 仓库密钥与源文件（stdin 精确 argv）；
  - `include_str!` 单测对照内核源码，防止两侧脚本漂移。
- **运行时与项目构建以本机执行为主**：内核新增 `LocalUnlessRootRunner`，运行时安装（解压 / 编译）与项目构建 / 部署只把 `prefer_root` 步骤交给特权端口，其余以面板用户本机执行。此前 agent 模式下这些命令整体委托 Agent，会被白名单拒绝。
- 测试：
  - 内核单测：`compose_merge` 10 项、升级文件 / 基线、`mongodb` 7 项、`LocalUnlessRootRunner` 路由、MongoDB 连接串、权限断言。
  - 集成：`tests/database.rs` MongoDB 安装 401 / 403 / 参数 400，`tests/app_store.rs` 升级预览 401 / 403 / 404 / 200 与非法策略 400。
  - Agent：`install_policy` 5 项（含防漂移）。
  - 前端：`AppUpgradeDialog` 4 例，`DatabaseInstallModal` / `databases` 工具 +2。
  - OpenAPI 323/323。

### refactor(frontend): 中型视图拆分收尾 + 平板断点 / 无障碍巡检 + 覆盖率地板 70/65（Doc/30 batch-10）

- **FilesView 669 → 263 行**：纯函数 `utils/fileBrowser.ts`（面包屑、路径归一、图标 / 行操作、选择修剪、时间格式）；数据层 `useFileBrowser`（目录导航、按路径查询、多选，刷新后修剪已消失的选中项）与 `useFileOps`（下载、批量 / 单项删除与永久删除确认）；组件 `FileTable`（平板隐藏权限列、行菜单逐行唯一无障碍名称）、`FileSelectionBar`（region + aria-live 计数）与新建 / 重命名 / 改权限三个自包含弹窗。
- **DatabasesView 395 → 227 行**：MySQL / Redis / PostgreSQL 三份几乎相同的安装表单合并为 `DatabaseInstallModal`（按引擎重置默认名称与端口，MySQL 映射 `root_password`；名称为空禁用提交），过滤与引擎元数据移入 `utils/databases.ts`。
- **平板断点**：新增 `useResponsiveLayout`——≤767 手机抽屉 / 768–1199 常驻折叠 / ≥1200 展开。修复 768 宽 iPad 竖屏被当成手机；修复任何 resize（包括 iPad 地址栏、软键盘这类只改高度的事件）都按宽度重置折叠、覆盖用户手动选择——改为只在跨越断点时调整。
- **无障碍**：`FpInput` 无可见 label 时以显式 `aria-label`、否则 placeholder 作为可访问名称（覆盖全站工具栏搜索框）；侧栏 Logo、节点文件目录行支持键盘（role + tabindex + Enter/Space）；内核日志三枚图标按钮补 `aria-label`。新增静态守卫 `src/test/a11yStatic.spec.ts`（不引入 axe）：icon-only 按钮必须有名称、可点击非交互元素必须 role + tabindex（`role="option"` 走 aria-activedescendant 豁免）。
- **覆盖率地板**：lines/statements 60 → 70、branches 68 → 70、functions 62 → 65；实测 81.41 / 76.74 / 72.13 / 82.7（statements / branches / functions / lines）。
- 测试：前端 187 文件 1721 例（+50）：fileBrowser / databases 工具、useFileBrowser / useFileOps / useResponsiveLayout、FileTable / FileSelectionBar / 三个文件弹窗 / DatabaseInstallModal、FpInput / FpModal 无障碍、a11y 静态守卫。

### feat: 系统工具箱主机详情 + `?fields=` 响应裁剪 + `unwrap_used` 门禁（Doc/30 batch-9）

- **系统工具箱页 `/toolbox`**：常用运维工具入口（进程 / 体检 / 终端 / 防火墙 / 定时任务 / 系统日志 / 健康，按当前角色权限过滤）+ 主机详情：系统、CPU 与内存（含 Swap、负载）、时间（IANA 时区、UTC 偏移、与浏览器时钟偏差 > 30 s 时提示启用 NTP）、网卡（物理网卡在前，lo / veth / docker 等虚拟网卡默认折叠）、DNS（`/etc/resolv.conf`）、hosts（最多 200 条）、磁盘挂载、面板进程（版本 / pid / uid / 是否 root）。
- **`GET /api/toolbox/host`**（`toolbox:read`，viewer 可读）：`domain::host_info` 负责解析与整形（纯函数），`infrastructure::host_probe` 只读文件与 sysinfo、放入 `spawn_blocking`，不执行任何命令；事实缺失只让对应字段为空。
- **`?fields=` 响应裁剪**：通用中间件，任意 JSON `GET /api/...` 接口支持 `fields=a,b`——数组裁元素、分页信封（`items` / `data` / `list` / `entries` / `records`）只裁列表并保留元数据、单对象裁顶层；字段名非法或超过 32 个 → 400；鉴权之后执行，401 / 403 不受影响；已裁剪响应带 `X-Fields-Applied: 1` 并去掉 `ETag`（Doc/02 §1.6）。
- **clippy `unwrap_used` 门禁**：flame-kernel 与 Agent 的非测试代码 `deny(clippy::unwrap_used)`；存量 75 处清零——锁统一走 `lock_or_recover`（锁中毒不再 panic），ACME 证书 URL、零停机重启探测器、插件启用回读、公钥长度、cron 时间截断、调度信号量等改为显式错误或回退。
- **内核日志长列表**：行加 `content-visibility: auto` + 固有尺寸提示，跟随模式最多 2000 行时视口外行跳过布局与绘制（行可展开、高度可变，不适合固定行高虚拟滚动）。
- 测试：`host_info` 9 项 + `host_probe` 3 项 + `field_selection` 6 项单测；集成 `tests/toolbox.rs` +3（401 / 无角色 403 / viewer 可读、真实采集、静态事实整形）、新增 `tests/field_selection.rs` 5 项；前端 `hostInfo` 6 例 + `ToolboxView` 5 例；OpenAPI 321/321。

### feat(mcp): 写工具 Batch 2 + Prompts 运维技能库（Doc/30 M-AI-2）

- **写工具 Batch 2**（均需 `allow_write` 凭证 + 人工审批）：`app_native_restart`（`app_store:update`）、`backup_create`（`backup:create`）、`backup_job_run`（`backup:update`，批准后后台执行，避免长时间上传被 120 s 执行时限取消；已在执行中 409）、`scheduled_task_run`（`scheduled_task:execute`，**高危**：执行管理员预设的 shell 命令，同一任务运行中拒绝，预览中展示脱敏后的命令）。配套只读工具 `backup_job_list`、`scheduled_task_list`（命令中 `-p<密码>` / `*PASSWORD=` 等脱敏）。工具目录 18 → 24。
- **Prompts 运维技能库**：MCP 声明 `prompts` 能力，支持 `prompts/list` / `prompts/get`；7 个 Runbook（容器排查、网站 502、主机巡检、错误日志归因、安全基线、变更检查点、备份 / 定时任务复盘），统一附「先取证、变更只能经审批工具提出、不索要密钥」约束。REST `GET /api/mcp/prompts[/{name}]`（`mcp:read`）供面板使用；AI 接入中心「工具目录」列出技能，「助手」页签可一键填入（先填入输入框，确认后再发送）。
- **前端路由预取**：侧栏菜单悬停 80 ms 或键盘聚焦时预取目标视图 chunk；省流量（Save-Data）/ 2G 网络跳过，失败可重试。
- 测试：mcp 集成 +2（Batch 2 审批全流程含定时任务输出回传、Prompts REST 401/403/400/404 + 协议）；runbook / 工具单测（模板引用的工具必须存在、命令脱敏）；前端 +9 例（预取 5、技能入口 3、工具目录技能 1）。

### feat(ai): 面板内置 AI 助手 + NL2API（Doc/30 M-AI-3）

- **Provider 抽象**：`infrastructure::llm` 提供 OpenAI 兼容（`/chat/completions`，含 DeepSeek / 通义 / vLLM 等）与 Ollama（`/api/chat`）两种实现；API Key 经 `SecretCipher` 加密存储、接口只返回 `has_api_key`；`base_url` 禁止内嵌凭据；上游错误信息脱敏后以 503 返回。
- **接口**：`GET/PUT /api/assistant/config`、`POST /api/assistant/test`（列模型并检查所填模型是否存在）、`POST /api/assistant/chat`。**默认关闭**；对话需 `mcp:create`，改配置需 `settings:update`。
- **NL2API**：`api_search` / `api_get` 以 OpenAPI 为工具目录，只开放**读动作的 GET**（与 REST 权限表同源判定）+ 拒绝名单（认证、设置、文件、凭据类、下载 / 导出 / 流式、AI 自身等）；JSON 输出统一脱敏。
- **写操作不越过审批**：MCP 工具与 `/api/mcp` 共用抽出的 `mcp::gate` 闸门；助手以每用户内置凭证 `panel-assistant` 生成 M-AI-2 审批单，由人在「审批」页签批准后执行。禁止某用户使用助手应收回 `mcp:create`（内置凭证被吊销后会自动重签）。
- **限额**：每轮 ≤ `max_steps`（1–12）次模型调用、每步 ≤ 8 个工具；同一用户并发对话 429；历史 ≤ 40 条、单条 ≤ 32 KiB；服务端不保存对话。
- **前端**：AI 接入中心新增「助手」页签（对话 + 设置）：回复按纯文本渲染（不使用 v-html）、工具轨迹可折叠、审批单一键跳转；Enter 发送 / Shift+Enter 换行；三语言文案。
- **Ollama 商店化**：内置应用新增 `ollama`（端口仅绑定 `127.0.0.1`，Ollama 无鉴权）。
- 测试：`tests/assistant.rs` 5 项集成测试（模拟 OpenAI 服务：权限 401/403、Key 不经通用设置接口泄露、历史校验、端到端工具循环 + 写工具生成审批单、按角色过滤工具）+ domain / llm / nl2api 单测；前端 3 个 spec 共 +13 例；OpenAPI 318/318。

### feat(docker): `docker run` 粘贴导入与存量容器收编（Doc/30 M-D）

- **粘贴导入**：Compose 页签「从 docker run 导入」→ `POST /api/docker/compose/convert-run`。按 POSIX shell 引号 / 续行规则分词但**不执行**（管道、重定向、`;` / `&&`、命令替换直接拒绝）；覆盖端口、环境、卷 / `--mount`、网络（自定义网络声明为 external、别名 / 固定 IP）、重启、资源、ulimit、日志、健康检查、GPU 等 80+ 参数；具名卷以 `name:` 固定真实卷名，避免 Compose 加项目前缀后指向空卷；单引号内的 `$` 转写为 `$$` 防止被 Compose 插值；无法等价的参数（`--rm`、`-P` 等）写入 `warnings` 而不是悄悄丢弃；未知参数报错。
- **容器收编**：容器行「导出为 Compose」→ `GET /api/docker/containers/{id}/compose`。结合 `docker image inspect` 剔除镜像自带的环境变量 / 命令 / 标签 / 健康检查，只保留用户设置项；匿名卷、已属于 Compose 项目等情况给出提示，并附收编步骤。
- 两者都**只回填表单**，由用户审阅后点「部署」（仍需 `docker:create`）；转换接口本身无副作用。
- 新增 `domain::compose_convert`（纯函数，12 项单测含 YAML 注入与 shell 结构拒绝）、2 项集成测试（含转换结果直接部署）、前端 5 个 spec 共 +10 例；OpenAPI 314/314。

### refactor: 拆分三个大文件（Doc/30 S-01～S-03）

- `application/app_store_service.rs`（4150 行）→ `app_store_service/`：`security` / `catalog` / `sources` / `hooks` / `install` / `lifecycle` / `wasm` / `util` + 3 个测试文件，最大 464 行。
- `domain/entity.rs`（2450 行）→ `domain/entity/` 9 个域文件 + glob re-export，最大 548 行。
- `agent/src/main.rs`（1560 行）→ `config` / `heartbeat` / `server` / `executor` / `files`；Agent 单测 8 → 15（根目录穿越与符号链接逃逸、按 IP 限流、心跳协议字段等）。
- 对外路径与行为不变：拆分由脚本按行段完成，并逐行比对新旧代码（非空行多重集一致）；shell-exec 登记表随文件迁移。
- 移除误入仓库的 `scripts/tests/__pycache__`，`.gitignore` 忽略 Python 字节码。

### fix(security): Docker 引用注入与 Compose 项目目录（Doc/33 P-12、P-13）

- **P-12** 容器 / 网络 / 卷 / 镜像引用在服务层白名单校验：阻断 `%2F..%2F` 经 bollard `Url::join` 折叠后调用任意 Engine API（如仅凭 `docker:stop` 触发 `POST /images/prune`）；compose up / down 校验项目名；**compose 部署权限由 `docker:start` 改为 `docker:create`**（任意 YAML 等价 root）；日志 tail 钳制、YAML ≤ 256 KiB。
- **P-13** Compose 部署不再使用即删的临时目录：面板部署持久化到 `data/compose/<project>/`（`FLAME_COMPOSE_DIR`），应用商店以安装目录为项目目录——修复相对挂载数据落入已删除目录、已部署项目无法 `up`；`compose ls` 兼容数组输出并包含已停止项目。

### docs(api)

- docker 33 个端点补注解，**OpenAPI 覆盖率 100%**（312/312）；`gen-api-docs.py --check` 对未注解路由报错；新增 `component_schema_names_are_unique` 单测拦截同名 schema 静默覆盖（docker `RenameRequest` → `DockerRenameRequest`）。

### fix(security): 数据库 / 插件 / 应用商店过审（Doc/33 P-8～P-11）

- **P-8** `GET /api/databases/{id}`、`/{id}/databases` 补 `database:read` 校验（原规则永不命中，由 auth-only 白名单兜底，任何登录用户可读）。
- **P-9** 本地导入应用包：包内 `key` 校验为安全路径段（阻断 `../` 写出商店目录）；存在性检查前移（不再覆盖已有包）；不跟随符号链接；拒绝源与目标互相包含；失败回滚已复制文件；批量 ≤ 50 且 key 不重复。
- **P-10** 插件内存上限 1–256 MiB、超时 1–300000 ms（原可传 `usize::MAX` 绕过沙箱内存限制）；插件 ID 与设置键值约束。
- **P-11** 系统库 / 内置超级用户不可删除；batch-status 限定状态与条数；install 路径 key 与请求体一致；日志 tail 钳制 1–5000。

### docs(api)

- utoipa 补注解：databases 17、plugins 13、app-store 14，覆盖率 75.3% → 89.4%（仅余 docker 33）；数据库创建用户 schema 改名 `DatabaseCreateUserRequest`（原与面板用户同名互相覆盖）。

### fix(security): Web 服务器实例 `config_path` / `binary_path` 收紧（Doc/33 P-7）

- 创建 / 更新实例校验：引擎已知、`config_path` 位于 Web 引擎配置根（与特权助手同一份 `check_path`）、`binary_path` 为该引擎已知二进制；违者 400。此前任意路径可被 `GET …/config` 读取、被 `start` 以特权执行器作为程序运行。
- `start` / `stop` / 性能预设 / 读配置对存量行再校验；移除从未可达的裸写接口 `POST /api/web-servers/{id}/config`。
- 单测 + 集成测试 `test_web_server_instance_paths_are_restricted`。

### feat(perf): 面板站点缓存与压缩

- install.sh 生成配置与 Docker `nginx.conf`：`/assets/` `max-age=31536000, immutable`；gzip（含 API JSON，`gzip_proxied any`），`/api/auth/` 关闭压缩；Docker `/assets/` 补回被 `add_header` 继承规则吞掉的 `nosniff` / `X-Frame-Options`。真实 nginx 1.26 `nginx -t` 与 curl 头部验证。

### feat(dashboard): 图表完整跟随主题

- 主题 / 品牌预设切换时同步重写系列线色、仪表盘三色弧线与刻度、tooltip 浮层（此前只改轴与图例，品牌色切换后曲线仍是旧色，暗色下 tooltip 为白底）；补丁不含 data，历史曲线保留。

### refactor(frontend) / docs(api)

- `WebServersView` 拆出 `WebServerNativeTab`（808 → ~500 行），原生安装 / 卸载失败时展示后端错误。
- OpenAPI 补注解：firewall 11 个、web-servers 23 个端点，文档覆盖率 64.2% → 75.3%。

### feat(files): 批量复制 / 移动（Doc/33 排除项）

- `POST /api/files/copy`、`POST /api/files/move`（`file:write`）：`{sources, dest_dir, overwrite}`，返回逐项结果（`conflict` / `not_found` / `forbidden` / `invalid` / `internal`），单项失败不影响其它项；请求级 400 / 403 / 404。
- 覆盖时旧目标移入回收站；复制到源目录自动命名 `名称 (副本)`；目录不能放进自身；复制保留权限位并拒绝复制后越出文件根的相对符号链接；跨文件系统移动回退为复制 + 删除。
- 前端：选择栏与行菜单新增「复制到 / 移动到」，`FileTransferModal` 部分失败时保持打开并逐条列出原因；i18n ×3。
- 测试：6 个单测 + 集成测试（401 / viewer 403 / 冲突 / 覆盖入回收站 / 请求级错误）+ 弹窗 spec。

### feat(helper): 特权助手代办防火墙命令（Doc/30 · Doc/33 §5-1）

- 助手白名单新增防火墙：ufw / firewall-cmd / iptables 的规则 argv 先解析为规则再用面板渲染器重渲染，逐字节一致才执行（篡改端口 / 注入 rich rule / 未知子命令一律拒绝），外加固定命令（`ufw status`、`firewall-cmd --reload`、`systemctl start|stop firewalld` 等）；带环境变量或工作目录的请求拒绝。
- `FirewallManager` 经 `helper::command_access` 包装 runner：助手可用时走助手，否则保持 `sudo -n` 回退。
- `install.sh` 助手单元 `RestrictAddressFamilies` 追加 `AF_NETLINK`；**旧主机需重新运行 install.sh**（Doc/10）。

### refactor(frontend): 拆分 SettingsView（Doc/33 §5-5）

- 拆出 `SettingsAppearanceTab`、`SettingsAboutTab`、`SettingsCredentialsPanel`（改密 + 2FA + JWT 轮换），SettingsView 1217 → ~890 行；行为不变，原 spec 全部通过。

### fix(install): 发布包固定版本 + 解包前强制 SHA-256 校验（Doc/33 排除项）

- `install.sh` 新增 `--release X.Y.Z`（`FLAMEPANEL_VERSION`）；缺省把 `releases/latest` 重定向解析为具体 tag，资源与 `flamepanel-<版本>-checksums.txt` 同 tag 下载。
- 解包前比对 SHA-256：清单缺失 / 未列出 / 不符即终止，现有二进制与前端不被触碰。清单优先 GitHub 直连（可发现镜像篡改），只能从镜像取得时告警；`--insecure-skip-verify` 显式跳过。
- 离线回归 +10 项（53 → 65），经变异验证（去掉校验调用后 5 项失败）。发布签名（GPG / minisign）仍未实现，Doc/10 §2 如实标注。

### ci(security): `"-c"` 调用点登记门禁（Doc/33 §5-3）

- `scripts/check-shell-exec.py` + `scripts/shell-exec-allowlist.txt`：按文件登记 `"-c"` 次数与理由，新增、增加、减少未同步均失败（棘轮只减不增）；接入 `ci-local.sh installer` 与 CNB `installer-regression`（纯 Python，无依赖，<1 s）。
- 顺带清零一处：Redis 自定义端口的 `sh -c "sed …"` 改为逐参数 argv，并补回归单测；现存 16 处均已登记。

### feat(security): WS 会话控制（4001 / 4003 + 带内续期）与 TOTP 两步验证（Doc/33 排除项）

- **WS 会话控制**：连接期间每 60 s 复核令牌——过期 / 吊销 / 改密 / 删号 / 首帧非法发 **4001**（`session expired`），失去通道权限发 **4003**（`forbidden`）。接收循环接受 `{"type":"auth","token":…}` 带内续期（仅限同一用户），终端长会话不再因 access token 轮换被踢。前端 `utils/ws.ts`：4001 强制刷新后重连（刷新失败回登录页）、4003 停止重连并通知页面，每 60 s 检查令牌变化发续期帧。
- **TOTP 两步验证**：RFC 6238（SHA-1 / 6 位 / 30 s，±1 步）；base32 手写、无新依赖；密钥 `SecretCipher` 加密落库（新表 `user_two_factor`），10 个恢复码只存 SHA-256；TOTP 步号与恢复码均 CAS 消费（同一验证码不可重放）。
- **登录两步**：已启用 2FA 时密码通过只返回 5 分钟 `mfa` 挑战令牌（不下发 Cookie，与 access / refresh 互不通用），`POST /api/auth/login/2fa` 提交验证码或恢复码，失败计入「用户名 + IP」锁定；挑战签发后改密即失效。
- **自助 / 管理**：`/api/auth/2fa`（状态）、`setup`（需密码）、`enable`（返回恢复码一次）、`disable`（密码 + 验证码或恢复码）、`recovery-codes`；管理员重置 `PUT /api/users/{id}/2fa/reset` 需 `user:update` **且 admin 角色**（operator 种子含 `user:update`，不应能摘除他人第二因子）；离线 `fpctl reset-2fa`。删除用户同时清理 2FA 记录。
- **强制策略**：设置 `two_factor_enabled=true` 后未绑定用户除绑定 / 会话维持端点外返回 `403 TWO_FACTOR_SETUP_REQUIRED`，前端跳转独立绑定页 `/two-factor-setup`；强制期间不可自行停用。
- **前端**：登录页第二步（验证码 / 恢复码切换、挑战过期回退），设置 → 安全「两步验证」面板（独立组件，不再膨胀 SettingsView），用户页 admin 重置入口；i18n ×3（新模块 `twoFactor`）。不引入二维码库：展示 `otpauth://` 链接 + 分组密钥。
- **测试**：TOTP RFC 4226 / 6238 向量、防重放、恢复码、加密落库；`mfa` 令牌类型隔离；集成测试覆盖两步登录、401（6 个端点）/ 403（operator、viewer）、强制策略与 409；WS 复核 / 续期判定；前端 ws 关闭码与续期、TwoFactorPanel、LoginView 两步流程。

### feat(mcp): 写操作审批闸门与首批 5 个写工具（M-AI-2 · #166）

- **闸门**：AI 发起的写操作永不自动执行。凭证新增 `allow_write`（默认只读，老凭证迁移为只读）；写工具只对可写凭证可见；`tools/call` 写工具只做 dry-run 预览并生成审批单（`tool_approvals`，30 分钟过期，每凭证 ≤ 20 个待审批）。
- **写工具**：`docker_container_start` / `stop` / `restart`、`website_deploy`（write），`docker_container_remove`（destructive）；只读 `approval_get`（仅本凭证）。工具注解 `readOnlyHint` / `destructiveHint` 按风险输出，`/api/mcp/tools` 返回 `risk`。
- **审批 REST**：`GET /api/mcp/approvals[/{id}]`（mcp:read）、`POST …/approve|reject`（新权限 `mcp:update`）。批准者需具备工具本身权限，高危另需 `mcp:delete`；执行前复核凭证有效与签发者当前权限；CAS 状态机防并发重复执行；执行记入调用审计与操作日志。
- **前端**：AI 接入中心「审批」页签（待审批角标 30 s 轮询、预览 / 参数 / 风险 / 结果、批准二次确认、拒绝附理由）；签发表单「允许写操作」开关；凭证列表只读 / 可写；工具目录风险分级；`FpTabs` 支持 `badge`。
- **修复**：吊销按钮可见性与后端字段级授权一致；凭证名称占位符中的 `@` 被 vue-i18n 当作链接消息导致编译错误。

### feat(mcp): AI 接入中心与 `flamepanel mcp` stdio 桥（M-AI-1 · PR-B）

- **AI 接入中心**（系统 → AI 接入中心，`/ai-center`，`mcp:read`）：签发凭证（名称 + 有效期），明文令牌**仅弹窗展示一次**，关闭即从页面状态丢弃；按客户端生成接入片段（Cursor / Claude Code / Claude Desktop / curl 自检），URL 取当前面板地址；吊销二次确认；凭证状态区分有效 / 已吊销 / 已过期；调用记录按凭证 / 工具筛选分页；工具目录展示所需权限与当前角色是否可用。签发 / 吊销入口按 `mcp:create` 控制显示。
- **`flamepanel mcp`**：MCP stdio ↔ Streamable HTTP 桥，Claude Desktop 等只能启动本地进程的客户端不再需要 `mcp-remote`（Node.js）。令牌只从环境变量 `FLAMEPANEL_MCP_TOKEN` 读取（拒绝 `--token`，避免出现在进程列表）；维护 `Mcp-Session-Id` / `MCP-Protocol-Version`；HTTP / 网络错误按请求 id 回 JSON-RPC `-32000`，通知静默；会话 404 自动清除；EOF 时 `DELETE` 结束会话；`--insecure` 支持自签证书；45 s 超时、4 MiB 响应上限；stdout 仅输出协议消息。
- **前端类型**：`npm run openapi:generate` 重新生成 MCP 相关类型并在 `src/api/generated` 命名导出，`src/api/mcp.ts` 不手写重复定义。
- **测试**：桥接器单测（axum 模拟端点：参数解析、会话头往返、通知 202、401 / 会话 404 / 连接失败映射为 JSON-RPC 错误、非法 JSON、EOF 发送 DELETE）；前端 `mcpSnippets` 与 `AiCenterView` 组件测试（一次性令牌、空名称校验、吊销确认、viewer 无写入口）。
- **文档**：Doc/34 §3（面板签发流程、`flamepanel mcp` 参数与行为）、Doc/32 §9。

### fix(security): Doc/33 待处理项 P-1…P-6——结构化特权执行、口令不进 argv、多发行版数据库、OLS 仓库、CSP 单一来源

- **P-1 特权执行**：Embedded 带 env 的 sudo 调用改为 `sudo -n --preserve-env=<键> --`（不再静默丢 `MYSQL_PWD`）；Agent 新增结构化 `exec` 动作（program / args / env / stdin 分字段，Agent 侧不经 shell 直接执行，env 键与 stdin argv 双白名单），面板对旧版 Agent 自动回退并对无法安全回退的命令提示升级。
- **P-2 口令**：`PrivilegedCommand.stdin`；MySQL / PostgreSQL 的 SQL 经 stdin 传入，`CREATE USER … IDENTIFIED BY` 不再出现在 `ps`；`Debug` 输出脱敏 env 值与 stdin。
- **P-3 发行版**：`mysql_layout` / `postgres_layout`（RHEL `mysqld` 单元、`postgresql-server` + initdb、Alpine / SUSE / Arch MariaDB）；卸载走发行版包管理器。
- **P-4**：`MockConnectInfo` 移出生产中间件，测试用 `add_middleware_with_mock_peer`。
- **P-5 OLS**：全链路 HTTPS；修复 Debian 系密钥地址实为安装脚本导致 OLS 原生安装必失败；RHEL 系写 `.repo`（EL8/9/10 分密钥），不再安装 EL7 rpm；支持 Debian 13。
- **P-6 CSP**：`frontend/scripts/csp.ts` 单一来源，构建注入 meta 并产出 `dist/csp.conf`，`nginx.conf` / `install.sh` 改为 `include`。
- **兼容性**：Agent 与面板可不同步升级（新面板 + 旧 Agent 自动回退）；自定义 sudoers 需含 `SETENV`（或 `ALL`）才能向数据库命令传递口令。

### feat(mcp): 内置 MCP Server 后端——AI 接入凭证、12 个只读工具、调用审计（M-AI-1 · PR-A）

- **端点**：`POST/DELETE /api/mcp`（JSON-RPC 2.0 · Streamable HTTP，协议 `2025-06-18` / `2025-03-26` / `2024-11-05`）；`initialize` 下发 `Mcp-Session-Id`，通知 202，批量 ≤ 32，`GET` 405；`Origin` 与 `Host` 不一致 403（防 DNS rebinding）。手写实现，零新依赖；位于 `/api/` 下，部署无需改动。
- **凭证**（`agent_principals`）：`fpk_` + 64 hex，明文仅签发时返回一次，库内存 SHA-256；可设有效期、可吊销（同时关闭会话）。凭证绑定签发者，工具按其**当前**角色逐个鉴权；面板 JWT 不能访问 MCP 端点。
- **工具**：`system_status` / `docker_list` / `docker_logs` / `docker_inspect`（附 `oom_killed`、退出码摘要）/ `task_get` / `logs_query` / `files_read` / `website_list` / `website_config` / `database_list` / `appstore_installed` / `firewall_rules`。权限由等价 REST 路由从权限表推导；`tools/list` 只列可用工具；输出统一脱敏（口令 / 令牌键与环境变量值），`files_read` 拒绝私钥、shadow、`.env`、数据库等敏感文件；30 s 超时、100 KiB 输出上限。
- **审计**（`agent_sessions` / `tool_calls`）：每次调用记录凭证、会话、工具、脱敏参数、成败、耗时，保留最近 1 万条。
- **管理 API**：`GET /api/mcp/tools`、`GET|POST /api/mcp/principals`、`POST /api/mcp/principals/{id}/revoke`、`GET /api/mcp/sessions`、`GET /api/mcp/calls`；新增权限 `mcp:read` / `mcp:create` / `mcp:delete`（老库启动自动补授）。
- **测试**：服务 / 工具 / 协议单测，`tests/mcp.rs` 覆盖 401 / 403、完整协议流程、按角色裁剪、吊销即时失效、审计脱敏。
- **文档**：新增 Doc/34（接入指南：Cursor / Claude Code / Claude Desktop 配置、工具目录、协议细节）；Doc/30、Doc/32 §9。

### fix(security): 代码审计与优化——数据库注入、应用商店扫描绕过、归档链接、部署脚本多发行版适配（Doc/33）

- **问题**：MySQL 口令只把 `'` 替换为 `\'` 而不转义反斜杠，Redis `requirepass` 与 MySQL 配置追加经 `sh -c "echo '…'"` 拼接——均可以 root 执行任意 SQL / 命令；应用商店安全扫描为文本子串匹配，`privileged:  true`、YAML 锚点、`cap_add: [ALL]`、挂载 `/` 等均可绕过，docker.sock / 敏感目录只告警；表单值含换行可在扫描之后向 compose 注入键；应用包中的符号链接条目（如 `docker-compose.yml -> /etc/shadow`）会被解出并由内核读取回显；Apache 非反代伪静态预设把 `RewriteBase` 写在 VirtualHost 级，`configtest` 失败；发行版识别区分大小写，Rocky / Alma / openSUSE / Arch 全部按 apt 执行；`install.sh` 前端目录 750 导致全站 403、硬编码 `conf.d`、Debian 默认站点抢占 80、未处理 SELinux、后端监听 0.0.0.0。
- **数据库**（`database/*`）：新增共享 `validate_secret` / `mysql_escape_literal` / `append_line_command`（内容经 argv，不经 shell 插值）；Redis `CONFIG SET` 白名单（封堵 `dir` + `dbfilename` 写文件）；MySQL 系统库精确排除；Redis 包名 / 单元 / 配置按发行版（`os::redis_layout`），卸载走包管理器。
- **应用商店**：结构化扫描（`serde_yaml` + 合并键展开，逐服务检查特权、宿主命名空间、危险 capability、设备、seccomp/AppArmor、敏感挂载、跨文件 `extends`），无法解析即阻断，高危项未确认即阻断；`validate_fields` 限定键名与拒绝控制字符；修复内置应用模板 `{data_dir}` 从未注入导致 compose 无法解析、4 个内置应用无法部署（由结构化扫描暴露）；远程包拒绝链接条目，运行时包只放行不逃逸的相对链接；签名摘要不跟随链接、拒绝含控制字符的文件名，熵源回退不再无界读取。前端安装弹窗风险预判与后端同口径（新增「宿主资源访问」提示，i18n ×3）。
- **Web 引擎 / HTTP**：Apache htaccess 语义规则移入 `<Directory>`（全部预设真机 `apache2 -t`）；OLS apt 代号取自 os-release（白名单）；API 响应补 `nosniff` 与 `/api/*` `no-store`；刷新令牌 Cookie 在 HTTPS（`X-Forwarded-Proto`）下带 `Secure`。
- **发行版**（`infrastructure/os.rs`）：`ID` / `ID_LIKE` 标准语义识别，新增 openSUSE（zypper）/ Arch（pacman），RHEL 系优先 dnf；`dpkg-query` 精确判定安装状态（修复卸载后无法重装），分族解析版本。
- **部署脚本**：平台适配层（`detect_platform` / `pkg_install` / `nginx_conf_dir` / `disable_distro_default_site` / `configure_selinux` / `open_firewall_ports`）；目录权限 711/755/750；后端在 nginx 前只监听 127.0.0.1；2 GB 上传、API 600 s / WS 3600 s 超时、HTTPS 下 80 → 301、`index.html` 不缓存、server 级安全头、证书 SAN、IPv6；非 amd64/arm64 提前报错；`.install-state` 记录改动，`uninstall.sh` 精确回滚。
- **测试**：`scripts/tests/test_install_platform.py`（15 项，含真实 `nginx -t` 校验 HTTP / TLS 配置）接入 `check-install.sh`（共 53 项）；Rust 新增扫描绕过 17 例、注入 / 转义、链接逃逸、发行版归族、Cookie、Apache 真机等单测；前端 `composeRisk.spec.ts`。
- **文档**：新增 Doc/33（漏洞清单、待处理 P-1…P-6、发行版分级、架构建议）；README 重写；Doc/05 §9.4.1 命令与 SQL 构造规范；Doc/10 安装 / 卸载说明与 `-P` 语义更正；Doc/12 兼容矩阵分级；Doc/20 §5、Doc/21 §2.7 扫描规则。

### feat(files): 文件管理收尾——上传 / 下载 / 编辑器联调、压缩解压、回收站（M-C）

- **问题**：前端以 `multipart/form-data` 上传，后端却把整个请求体当文件内容写盘——上传的每个文件都混进了 multipart 边界与表单头；上传整体读入内存（2 GB 上限即 2 GB 内存）且同名直接覆盖；保存文件为截断重写（中途失败即丢内容），不做并发修改检测；删除不可恢复；前端 `v-permission` 使用不存在的 `file:create / update / delete`；读取二进制文件时按有损 UTF-8 返回乱码。
- **沙箱**（`file/mod.rs`）：chroot 语义统一（列表 `path` 为根内相对路径可原样回传），末级符号链接不跟随（删除 / 重命名作用于链接本身），拒绝对根目录操作；`.fp-recycle` 与 `.fp-part-*` 对文件 API 不可见。
- **编辑器**：`POST /api/files/write` 同目录临时文件 + `rename` 原子替换，保留权限与属主；`expected_modified` 与当前修改时间不符 → 409；读取非 UTF-8 → 400。前端编辑器弹窗 Ctrl+S / ⌘S 保存、未保存标记与关闭确认、409 时确认后强制覆盖。
- **上传 / 下载**：请求体为原始字节，按流写入临时文件（≤ 2 GiB，超限 / 中断删除临时文件），`overwrite` 控制同名覆盖（默认 409）；下载带 `Content-Length` 与 RFC 5987 中文文件名。前端上传弹窗支持拖拽、多文件逐个进度、已存在提示。
- **压缩 / 解压**（`file/archive.rs`、`file/zip.rs`）：压缩为 `.zip`（自研 store + deflate 写入，复用 `flate2`，不新增依赖）或 `.tar.gz`；解压支持 zip / tar / tar.gz / tgz / tar.xz / txz / gz。解压先落到目标目录内的临时目录，拒绝路径穿越、包外符号链接、硬链接，限制 16 GiB / 20 万条目，冲突时 409 不做改动或 `overwrite` 把旧条目移入回收站后替换。
- **回收站**（`file/recycle.rs`）：删除默认移入 `<OP_FILE_ROOT>/.fp-recycle/`（`permanent=true` 直接删除），保留 30 天；`GET /api/files/recycle`、`POST /api/files/recycle/{id}/restore`（原位置被占 409）、`DELETE /api/files/recycle/{id}`、`DELETE /api/files/recycle`。
- **前端**：文件页拆分为 `components/files/*`（编辑器 / 上传 / 压缩 / 解压 / 回收站抽屉），多选批量压缩与删除、行内下载、压缩包点击即解压、上级目录按钮；权限按 `file:read / write / upload` 对齐后端；i18n ×3。
- **API**：文件端点全部登记 OpenAPI；新增 6 条 PermissionRule（压缩 / 解压 / 回收站写操作 `file:write`，列表 `file:read`）。
- **测试**：`file` 模块单测（沙箱逃逸 / 原子写 / 冲突 / 回收站 / zip 往返与读取系统 `zip` 生成的压缩包 / 恶意 tar 与 zip）；`tests/files.rs` 覆盖 401、viewer 只读、上传覆盖、编辑冲突、回收站还原、zip 与 tar.gz 往返、解压冲突与替换。
- **文档**：Doc/02 §13、Doc/30、Doc/32 §9。

### feat(waf): Apache / Caddy / OpenLiteSpeed 规则下发（Doc/30 v1.1）

- **问题**：WAF 规则只编译下发到 Nginx / OpenResty，Apache / Caddy / OLS 站点完全不受 WAF 保护（下发报告也不列出）。
- **分发**（`webserver/waf_engines.rs`）：五引擎统一的 `supports` / `compile` / 路径 / 主配置钩子 / 站点 include / 命中日志解析；共享规则归一化（作用域、跳过原因）。
- **Apache**（`waf_apache.rs`）：`SetEnvIfExpr` 判定 → `<If>` `Require all denied`（403），`GlobalLog` 写 JSON 命中日志；路径按 `unescape(%{REQUEST_URI})` 匹配（防 `%HH` 绕过）；站点模板写入 Debian / RHEL 两处 `IncludeOptional`。
- **Caddy**（`waf_caddy.rs`）：命名匹配器 + `vars` → `respond 403`；`log` + `skip_log` 只记命中，规则标记写入访问日志 `user_id`；站点块 `import …/site*.caddy`。
- **OpenLiteSpeed**（`waf_ols.rs`）：分段规则库拼接进 `vhconf.conf` 的 rewrite（`RewriteCond` + 白名单取反 → `[F]`），CIDR 编译为精确正则；站点重新部署时自动补回，规则清空后恢复原样。仅下发阻断规则。
- **能力差异**：CC 在 Apache / Caddy / OLS 上跳过（无原生按客户端限频），OLS 仅告警规则跳过（无条件日志），均在 `POST /api/waf/apply` 报告 `skipped` 中附原因。
- **回灌**：同步任务扩展到 Apache / Caddy 命中日志；特权助手 `read_log` 白名单由 `waf_engines::all_log_paths()` 派生。
- **测试**：各编译器单测（含注入样例、CIDR 边界、OLS 拼接幂等 / 可逆）、日志解析（Apache `\xHH` 转义、Caddy 2.6 实测行）、manager 三引擎下发 / 回滚 / OLS 重部署补回；沙箱 Apache 2.4.68 / Caddy 2.6.2 真机冒烟。
- **文档**：Doc/20 §12.3a、Doc/02 §24c、Doc/30、Doc/32 §9；前端回灌状态文案 i18n ×3。

### feat(runtime): Python / Go / Java 运行时补齐（M-C 收尾）

- **问题**：Go 按 `1.22` 拼出 404 的发行包地址且不校验；Java 走发行版 `openjdk-N-jdk`（Alpine / RHEL 包名不同、版本受发行版限制）；Python 源码编译不装编译依赖，常得到缺 `ssl` / `sqlite3` 的解释器；三种语言的启动命令写 `gunicorn` / `java` / `go` 都会被守护器拒绝（systemd `ExecStart` 不经 shell，必须绝对路径）。
- **Go**（`runtime/go.rs`）：`go.dev/dl/?mode=json&include=all` 解析到最新补丁版，SHA-256 校验，扁平化到 `<install_path>/bin`（兼容旧 `go/bin` 布局）；`FLAMEPANEL_GO_MIRROR` / `FLAMEPANEL_GO_INDEX` / `FLAMEPANEL_GOPROXY`。项目有 `go.mod` 时部署执行 `go build -trimpath -o .fp-bin/app .`（构建缓存在 `<deploy>/.fp-go`），启动命令留空即运行产物。声明版本 `1.25 / 1.26 / 1.27`。
- **Java**（`runtime/java.rs`，新 `JavaRuntimeInstaller`）：Eclipse Temurin，经 Adoptium API 取最新 GA 并校验 SHA-256，目录式安装（旧系统 JDK 仍可用：`bin_dir` 回落 `/usr/lib/jvm`）；`FLAMEPANEL_JDK_API` / `FLAMEPANEL_JDK_MIRROR`。jar 项目带 `mvnw` / `gradlew` 时部署构建（跳过测试，仓库缓存放在部署目录），在 `target/` → `build/libs/` → 根目录中挑最大的可运行 jar（排除 `-plain` / `-sources` / `original-` 等），回填 `java -jar <jar>`；找不到产物部署返回 400 并说明。单元注入 `JAVA_HOME`、`SERVER_PORT`。声明版本 `8 / 11 / 17 / 21 / 25`。
- **Python**（`runtime/python.rs`）：目录列表解析候选补丁版（最多试 3 个，跳过还没有源码包的新版本），`FLAMEPANEL_PYTHON_MIRROR`；按发行版逐个安装编译依赖（失败不阻断）；编译后自检 `ssl / sqlite3 / zlib / ctypes / bz2 / lzma`，旧安装自检失败自动重编。启动命令留空时按 Django → gunicorn、FastAPI → uvicorn、Flask → gunicorn、`app.py|main.py` 探测；裸命令按 `.venv/bin` 实际入口补全，注入 `VIRTUAL_ENV` / `PYTHONUNBUFFERED` / `UVICORN_HOST` / `UVICORN_PORT`。声明版本 `3.10`–`3.14`。
- **共用**（`runtime/download.rs`）：镜像校验（仅 https）、版本号比较、SHA-256 校验、单目录扁平化、PATH 拼接、安装记录读写。
- **前端**：运行环境页为 Python / Go / Java 显示安装与项目约定说明，Java 显示 JDK 发行商；i18n ×3。
- **测试**：版本解析 / 资产选择 / 构建命令 / jar 探测 / 启动命令探测单测；`project_service` 覆盖三种语言的命令补全与环境注入、jar 部署回填与缺产物报错。
- **文档**：Doc/02 §16a、Doc/10 环境变量、Doc/30、Doc/32 §9。

### feat(settings): 面板设置补全 + 访问控制（Doc/32 W4-a 后端）

- **问题**：`PUT /api/settings` / `PATCH /api/settings/batch` 对任意键值照单全收（端口可写成 `abc`），且会话超时、日志级别等设置写入后没有代码读取；「轮换密钥」按钮把随机串写进 `jwt_secret` 设置项，既没轮换、又让持有 `settings:read` 的账号读到签名密钥。
- **类型化目录** `domain/settings_schema.rs`：已知键按类型校验并规范化（布尔 / 整数区间 / 枚举 / IP·CIDR 列表 / 域名列表 / 文本）；未登记键保持兼容，只限键名字符集 `[a-z0-9_.-]` ≤ 64 与值 ≤ 64 KiB。保留键（`jwt_secret*`、`waf.ingest.*`）不出现在列表、读 404、写 400；旧库残留的 `jwt_secret` 由幂等迁移清除。
- **访问控制**（对标 1Panel / 宝塔「授权 IP / 绑定域名」）：`api/access_guard.rs` 按 `allowed_ips` / `bind_domains` 放行，违规 403 `ACCESS_DENIED`；转发头只信任回环对端；本机回环且无转发头的请求始终放行（`fpctl api` 自救）。新增 `GET /api/settings/access`（当前识别到的 IP / Host 与生效策略，设置页据此提示，防止把自己锁在外面）。
- **生效**：会话超时 = refresh token 有效期（剩余不足一半时滚动续期）；登录失败锁定次数 / 时长可配；策略由设置快照派生、保存即热更新，热路径无锁读。
- **测试**：`tests/settings.rs` 新增校验 / 保留键 / 访问控制 / 401·403 用例。
- **文档**：Doc/10 §7.4.1、Doc/02。

### feat(ui): 设置页重做 + 材质 / 配色修复 + 自适应布局（Doc/32 W4-a 前端）

- **修复：预设与材质实机不生效**。旧预设三色相几乎相同（35° / 35° / 45°）且与圆角 / 密度 / 模糊捆绑，滑块只在 `custom` 下生效；OpenVue 预设未桥接品牌变量，材质只作用于部分表面。现在预设只决定颜色（新增 ocean / violet / rose / emerald / graphite），圆角 / 密度 / 模糊 / 材质（solid · glass · acrylic）为独立偏好，任意预设下即时生效；非自定义预设下拖滑块自动切到自定义；首屏脚本同步应用（CSP 哈希已更新 index.html / nginx.conf / install.sh）。
- **设置页**：左侧分区导航 + 行式 `SettingRow`（标题 / 说明在左，紧凑控件在右，窄容器自动堆叠）；分区：面板配置 / 安全 / 备份 / DNS / 主题 / 外观 / 关于；未保存标记、吸底保存栏、离开确认、批量只提交改动项；控件整体缩小（精确指针设备下输入框与按钮使用基础字号）。
- **组件**：新增 `FpMultiSelect`（`FpSelect multiple` 实际被底层单选组件忽略）；`FpInput` / `FpNumber` / `FpSelect` / `FpSwitch` / `FpTextarea` 支持 `inputId`，可被外部 `<label for>` 关联。
- **自适应**：侧栏 < 1200px 默认收起；顶栏按宽度逐级隐藏快捷应用 / 面包屑 / 状态，搜索变图标；表格窄屏横向滚动、表头与标签不折行；工具栏 ≤ 640px 纵向堆叠；弹窗 ≤ 640px 变底部抽屉；终端高度按 `dvh` 计算。已在 1440 / 1024 / 390 宽度逐页截图核对。
- **i18n**：新增文案 ×3 语言。

### feat(waf): Web 引擎命中日志回灌 + 网站变化自动重新下发（Doc/32 W3-d）

- **回灌**：后台同步（默认 10 s，`OP_WAF_SYNC_SECS`）增量读取 Nginx / OpenResty 的 `flamepanel-waf.log`，命中写入「拦截日志」并触发拦截通知；读取位置（inode + 偏移）持久化，重启不重复、轮转自动续读。WAF 页「拦截日志」显示各引擎导入状态，`GET /api/waf/status` 新增 `enforcement` / `ingest`。
- **特权助手**：新增只读操作 `read_log`，仅限两个 WAF 日志精确路径，`O_NOFOLLOW` 拒绝符号链接替换，单次 ≤ 512 KiB 且只返回完整行。
- **CC 拒绝入日志**：存在 CC 规则时命中条件包含 `$limit_req_status`，429 归因到作用域内 CC 规则。
- **自动重新下发**：网站集合（引擎 × ID × 域名）变化且存在网站级规则时自动重新下发，不再需要改域名后手动下发。
- **文档**：Doc/20 §12.2 / §12.3、Doc/10 环境变量、Doc/02 §24c、Doc/30、Doc/32 §9。

### feat(waf): WAF 规则下发到 Nginx / OpenResty（Doc/32 W3-c）

- **问题**：WAF 规则只在面板进程内评估，而站点流量并不经过面板——规则实际不保护任何站点。
- **编译器** `webserver/waf_nginx.rs`（纯函数）：IP → `geo`，UA / 路径 / SQLi / XSS → `map`，白名单屏蔽，CC（仅 `action=block`）→ `limit_req` 429，阻断 403；命中写 JSON Lines 日志 `flamepanel-waf.log`。所有用户输入以 `\xHH` 转义写入，无法注入配置。
- **下发**：`WebServerManager::deploy_waf` 与站点同一事务（引擎锁 + 回滚 + `nginx -t` + reload；非 root 经特权助手）；主配置幂等 include，Nginx / OpenResty 站点模板加入 server 级 include，旧站点下发时自动重部署（不丢 HTTPS）。
- **触发**：规则增删改后自动尽力下发；新增 `POST /api/waf/apply`（`waf:update`）返回逐引擎报告；前端 WAF 页「下发到 Web 引擎」按钮 + 结果面板（i18n ×3）。
- **修复**：CC 固定窗口因 `Instant::now().elapsed()` 恒为 0 永不重置（超阈值 IP 被永久判超限）；`inspect(website_id)` 忽略作用域，网站级规则误伤其它网站；OpenResty HTTPS 站点渲染为 "Hello from OpenResty" 占位而非真实站点；全局配置显式 `access_log`。
- **文档**：Doc/20 §12.3、Doc/02 §24c、Doc/30、Doc/32 §9。

### feat(webserver): 本机特权助手 `flamepanel helper`（Doc/32 W3-b）

- **问题**：默认安装面板以 `flamepanel` 用户运行（`NoNewPrivileges` + `ProtectSystem=strict`），W3-a 的站点配置流水线在该形态下无法写 `/etc`，只能跳过。
- **助手**：同一二进制的 root 子命令 `flamepanel helper [--socket P] [--allow-user N | --allow-uid U]`，install.sh 生成 `flamepanel-helper.service`（面板单元 `Wants=` / `After=`）。Unix socket `0660 root:<面板组>` + 每连接 `SO_PEERCRED`；只提供 `read / write / remove / exists / writable`（限 Web 引擎配置根，真实路径二次校验防符号链接逃逸）与 `run`（由引擎声明展开的精确 argv 白名单）；请求大小 / 超时 / 并发有上限；写入与命令记审计日志。
- **面板侧**：`WebServerManager` 在非 root 且 socket 存在时自动改用 `HelperEngineHost` + `HelperCommandRunner`（白名单外命令仍走原执行器），上层零改动；`OP_PRIV_HELPER=auto|on|off`、`OP_PRIV_HELPER_SOCKET`。助手不可用 → 503 + 排查提示。
- **运维**：`fpctl restart / upgrade / rollback` 同步 `try-restart` 助手；`fpctl doctor` 新增 `priv-helper` 检查（在线 / 版本一致 / 未安装）；uninstall.sh 移除助手；`scripts/helper-smoke.sh` 真机冒烟。
- **文档**：Doc/10 非 root 说明与环境变量、Doc/20 §12.2、Doc/30、Doc/32 §9。

### feat(webserver): Web 引擎站点配置流水线（Doc/32 W3-a）

- **修复：站点配置从未被引擎加载**。此前 Nginx / OpenResty 写 `sites-available/<域名>` 而全局配置只 include `sites-enabled/*`；Apache RHEL 写 `conf.d/<域名>`（无 `.conf`，`*.conf` 不匹配），Debian 写 `sites-available` 从未启用；Caddy 站点文件未被 `import`，且全局配置 `admin off` 让 `caddy reload` 必败、每次写入都回滚；OLS vhost 未注册到 `httpd_config.conf`。
- **新布局**（`webserver::layout`）：各引擎统一写入面板自有目录 `<配置根>/flamepanel/sites/<域名>.<ext>`，首次部署在主配置中幂等插入 include / `IncludeOptional` / `import`；停用 = 重命名为 `.disabled`；旧位置自动清理。OpenLiteSpeed 自动注册带标记的 `virtualHost` 块并向非 SSL listener 追加 `map`，站点文件改为 `vhosts/<域名>/vhconf.conf`；遇到同名非托管虚拟主机返回 409。生成的全局配置已包含对应钩子；Caddy 全局配置去掉 `admin off`。
- **事务**：`EngineHost` 文件端口（`LocalEngineHost`，`OP_WEBSERVER_ROOT` 可重定根；后续特权助手实现同一 trait）；同引擎「写 → 校验 → reload」进程级互斥；原子写临时文件名唯一（pid + 计数器）、沿用原文件权限、fsync；多文件改动失败逆序恢复（恢复失败记 error 日志，不再静默吞掉）；reload 失败但引擎未运行时保留已校验配置；配置校验失败返回 400 并带出引擎输出；发行版探测结果缓存。
- **完整渲染**：伪静态套用 / 站点更新 / 部署统一经 `DefaultSiteConfigWriter::render` 解析绑定证书——**不再在重写配置时静默去掉 HTTPS**；证书文件缺失时拒绝写入。Caddy 支持显式证书绑定（`tls <cert> <key>`）。
- **网站生命周期联动**：创建（失败撤销）/ 更新 / 绑定与解绑运行时 / 切换引擎（目标引擎须可部署；新引擎成功后才清理旧引擎）/ 删除（先移除 vhost）自动同步引擎配置；引擎未安装或面板无写权限时跳过并记日志。`status = "stopped"` 以停用态部署。
- **新端点** `POST /api/websites/{id}/deploy`（`website:update`，严格模式，返回 `SiteDeployment`）；前端网站列表新增「部署配置」操作（三语言），失败时展示后端原因。
- **修复：网站 `status` 更新不落库**（SQLite 与 InMemory 仓储均遗漏该列，停用站点刷新后又显示运行中）。
- **移除** 无调用方且行为错误的 `enable_site` / `disable_site`（把配置正文当路径写入 `sites-enabled`）。
- **文档**：Doc/20 §12.1、Doc/02、Doc/22、Doc/30、Doc/32 §9；**更正 Doc/10**：安装脚本从未写入 sudoers 白名单，默认非 root 安装下特权操作不可用——由后续 `flamepanel-helper`（W3-b）解决。

### feat(perf): 性能 / 稳定性压测与首份基线（Doc/32 W2-b）

- **工具**：`scripts/perf/load.py`（纯标准库 asyncio：HTTP keep-alive + 最小 WebSocket 客户端，`/proc` 采样）+ `scripts/perf/run.sh`（release 构建 → 临时目录起真实内核 → setup API → 三场景 → 报告）+ mock 内核自测；`ci-local.sh perf | perf-quick`、`just perf`。在云原生开发环境运行，不占构建额度；发版前必跑（Doc/31 §8）。
- **开发环境保活**：`cnb-workspace.sh hold` 持有 ssh 直到出现 RESULT 行（CNB 10 分钟无连接即回收）。
- **首份基线**（v1.0.0 + main `935d986` 之后，CNB 开发环境 8C / 16G，release fat-LTO，SQLite，2026-09-25）——**10 / 10 项达标**：

  | 场景 | 时长 | 并发（HTTP / WS） | 请求 | 错误 | RPS | p50 / p95 / p99 ms | RSS 峰值 | RSS 斜率 | CPU 均 / 峰 | fd 增长 |
  |---|---|---|---|---|---|---|---|---|---|---|
  | idle | 5 min | 0 / 1 | — | 0 | — | — | **22.7 MB** | 0.91 MB/10min | 0.1 / 0.4 % | 0 |
  | load | 10 min | 200 / 20 | 239 799 | **0** | 399 | 0.17 / **0.36** / 0.53 | 44.9 MB | 0.56 | 6.0 / 16.4 % | 0 |
  | soak | 30 min | 50 / 5 | 179 807 | **0** | 99.8 | 0.24 / 0.71 / 1.04 | 47.0 MB | **0.65** | 2.5 / 5.0 % | **0** |

  token 续期 4 / 4 成功（跨两次 15 min access TTL）；压测期间内核日志 error 0、warn 2（均为启动时「未提供配置文件 / JWT 密钥自动生成」）；进程线程峰值 27、fd 峰值 310。
- **观察**：① RSS 斜率在空载（0.91）与负载（0.65）下相当，说明增长由时间而非请求驱动——监控历史每分钟落库一行（保留 7 d）、`range=1h` 曲线在首小时内逐分钟变长，二者都有上界；发版前建议补一次 ≥ 2 h 浸泡确认平台期。② WS 因 token 复核关闭时直接断开 TCP、不发 Close 帧，客户端无法区分「token 过期」与「网络中断」（浸泡中 5 条 WS × 2 次过期 = 10 次重连）；已列入待办：发送 4001 Close 帧，前端据此先续期再重连。

### feat(deploy): 版本化安装布局与 fpctl upgrade / rollback（Doc/32 W1-b）

- **布局**：`/opt/flamepanel/releases/<版本>/{bin,frontend}` + `current -> releases/<版本>`（原子切换）+ `frontend -> current/frontend`；`/usr/local/bin/{flamepanel,fpctl}` 改为指向 `current/bin/` 的链接。稳定路径不变，systemd 与 nginx 配置无需改动，数据目录与版本无关。
- **新命令**：
  - `fpctl releases`：已安装版本、current / previous、布局状态（legacy / versioned / empty）。
  - `fpctl adopt`：旧布局（普通文件）收编为版本化布局，幂等；`install.sh` 部署后自动执行，失败只告警。
  - `fpctl upgrade [--version V] [--from-dir D] [--source URL] [--keep N] [--no-restart] [--force] --yes`：下载（GitHub + 与 install.sh 相同的镜像）或本地发布包 → 强制 SHA-256 校验 → 安全解包（拒绝链接、绝对路径、`..`，限制总大小）→ 执行新二进制 `--version` 核对 → `VACUUM INTO` 备份数据库到 `backups/pre-upgrade-*.db` → 原子切换 → 重启 → 60 s 健康检查与版本核对，失败自动切回旧版本并重启 → 保留最近 N 个版本（当前与上一个永不删除）。
  - `fpctl rollback [--to V] --yes`：切回上一个或指定版本；数据库不回滚，输出中提示对应备份。
- **内核**：`flamepanel --version` / `-V` 输出 `flamepanel <版本>` 并立即退出（不读配置），作为升级流程的版本探测契约。
- **安装脚本**：重跑 `install.sh` 时若检测到版本化布局，先摘掉稳定路径上的链接再写入新文件，部署后自动收编为新版本；`uninstall.sh` 可删除悬空链接。
- **测试**：`release.rs` / `artifacts.rs` 单测（版本排序、校验和、tar 安全、布局操作）+ `tests/fpctl_release.rs` 端到端（`FP_BIN_DIR` + `--no-restart`，无需 root）。

### feat(cli): fpctl 面板命令行（Doc/32 W1-a）

- **定位**：部署主机上的运维入口，对标 1pctl，面向脚本与 AI Agent。`flame-kernel/src/bin/fpctl.rs` + `src/cli/`，参数解析手写（不引入 clap），零新增依赖。
- **命令**：`status` / `version` / `info` / `start` / `stop` / `restart` / `doctor` / `logs [-f]` / `logs files` / `log-level get|set` / `user list` / `reset-password` / `port set` / `api` / `openapi` / `backup list|create` / `commands`。
- **离线优先**：状态、诊断、日志、用户、改密、改端口在面板进程停止时同样可用。日志直读文件，复用 `infrastructure::logging::reader`；用户操作复用 `SqliteUserRepository`，不跑迁移、不建库。以 root 运行后会把新建的 `-wal` / `-shm` 归还给库文件属主。
- **在线调用**：`api` 用面板同一把 JWT 密钥现签 60 s access token（新增 `JwtUtils::sign_access_with_ttl`），经本机回环访问 REST API，全程走鉴权、权限与审计。5xx 错误附请求编号，提示 `fpctl logs --grep <id>`。
- **AI 协同契约**：
  - `--json` 模式下 stdout 恰好一个信封 `{ok, command, exit, data | error, warnings?}`。
  - 稳定退出码：0 成功，1 一般错误，2 参数错误，3 面板未运行，4 权限不足，5 资源不存在。
  - 写操作支持 `--dry-run`，危险操作需 `--yes`，root 前置校验。
  - `commands --json` 输出机器可读目录。`cli/catalog.rs` 同时驱动解析、帮助与目录，单测保证与实现一致。
- **配置解析与内核同源**：
  - 安装目录：`--dir` > `$FP_HOME` > `/opt/flamepanel`。
  - 变量：进程环境 > `flamepanel.env` > `config/app.toml`。
  - JWT 密钥：`data/jwt_secret.key` > `OP_JWT_SECRET`，与 `config::loader` 一致。
- **`reset-password`**：缺省随机生成 16 位密码（`ring` CSPRNG + 拒绝采样），只显示一次。同时写 `password_changed_at`，吊销 refresh token 与 WebSocket 会话；缺省下次登录强制改密。
- **`port set`**：同时改 env 文件与 nginx `proxy_pass`（精确匹配端口），`nginx -t` 失败时整体回滚；文件原子替换并保留权限与属主。
- **部署**：
  - `install.sh` 把 `fpctl` 与面板一起部署到 `/usr/local/bin/`；旧版资源包不含 fpctl 时只告警。
  - `uninstall.sh` 同步删除。
  - release 包与 `scripts/package-release.sh` 打入 `fpctl`。
  - Docker 镜像内置 `fpctl`，并设置 `FP_HOME=/app`。
  - 安装完成提示改为 fpctl 常用命令。
- **fix(kernel)：启动致命错误写入日志文件。** 过去 `main` 返回 `Err` 时，错误只由运行时打印到 stderr（`Error: …`），发生在日志守卫刷盘之后。像 `OP_TERMINAL_CWD` 无效这类「启动即失败」，日志文件里没有死因，面板和 `fpctl logs` 都看不到。现在 `main` 先写一条 `tracing::error!` 再退出（stderr 输出保持不变）。面板未运行时，`fpctl status` 会附带最近 3 条 error 日志。
- **测试**：
  - `cli` 模块单测。
  - `tests/fpctl_cli.rs` 端到端：真实启动内核二进制，覆盖启动即失败→`status` 给出死因、离线→在线、强制改密→`reset-password`→`api`、全部退出码与信封形态。
  - 安装回归新增 3 例：包含 fpctl、旧包缺 fpctl、本地构建。
- **文档**：Doc/10 §7.4 用户手册；Doc/31 §7 同步矩阵、AGENTS 路由表新增 fpctl 行；Doc/32 §3 进度。

## [1.0.0] - 2026-09-25

首个稳定版：Doc/30 §6 原定 v1.0 范围（S-06 / S-07 / S-08 / S-10 / S-11 / S-12 / S-14，外加 S-13 / S-15 / S-16 / S-17）全部完成，并落地五级内核日志与错误反馈闭环。以下至 `[Unreleased]` 之间的条目均属于本版本。

**1.x 兼容承诺（自本版本起生效，详见 Doc/32 §1）**：`/api/*` 已发布端点 1.x 内不删字段、不改语义（新增字段只做可选，废弃先标 `deprecated` 并保留至少一个 minor）；数据库迁移只增不删，任意 1.x 可直接升级到最新 1.x；`config/app.toml` 与 `OP_*` 环境变量只增不改名（改名保留旧名别名并在启动日志告警）；`fpctl` 子命令与 `--json` 输出字段同样遵守。

### feat(log): 内核日志文件化 + 五级分级查看 + 错误请求编号闭环（Doc/32 W0）

- **日志落盘**：`infrastructure/logging`——控制台输出保持原样，另写 JSON Lines 文件 `<OP_LOG_DIR>/flamepanel.YYYY-MM-DD.log`：按天切分、单文件超 `OP_LOG_MAX_FILE_MB`（默认 64）滚动为 `.log.N`、启动与跨天时清理超过 `OP_LOG_RETENTION_DAYS`（默认 14）的文件。写入走独立线程 `fp-log-writer` + 8192 行有界队列，队列满丢行计数（概览 `dropped_lines`）而不阻塞请求；进程退出时 Guard 刷盘。panic 钩子另写 `crash-YYYYMMDD-HHMMSS.log`（位置 + 回溯）。`OP_LOG_FILE=false` 关闭文件日志。
- **五级**：`OP_LOG_LEVEL=trace|debug|info|warn|error`（缺省 info，`RUST_LOG` 仍可覆盖为完整指令），`PUT /api/kernel-logs/level` 运行时调级立即生效。
- **查询端点** `/api/kernel-logs`：概览 / 倒序分页（64 KB 反向分块读，`before` 游标，单次最多扫 32 MB）/ 五级计数（>256 MB 近似）/ 下载 / 删除历史文件（当前文件拒绝）。文件名白名单防路径穿越。权限：读与下载 `log:read`、删除 `log:delete`、调级 `settings:update`；集成测试覆盖 401 / 403 / 400 / 404 / 409。
- **错误反馈闭环**：前端 HTTP 拦截器把响应头 `X-Request-Id` 带到规范化错误上，5xx 提示附 `(ID: …)` 并延长到 8 s；日志页支持 `/system-logs?q=<id>` 深链，行详情中「只看此请求」一键筛选同一请求的全部日志。**修复**：最外层 TraceLayer 的 `http_request` span 原先不含 `request_id`（只有内层 span 有），tower_http 的请求完成事件（状态码 + 耗时）因此无法按编号检索——现预留字段并由 `request_id_middleware` 回填，客户端自带 `x-request-id` 时直接沿用（`tests/request_trace.rs`）。
- **前端**：日志中心改为「内核日志 / 实时事件」两个页签（`?tab=` 同步）。内核日志：文件切换、五级开关带计数、关键字防抖搜索并高亮、跟随最新（仅文件变大时拉取，新行短暂高亮）、加载更早、行展开看结构化字段并复制、下载 / 删除 / 调级；`/` 聚焦搜索；keepAlive 离开页面暂停轮询；尊重 `prefers-reduced-motion`。i18n × 3。
- **部署**：Docker 镜像设 `OP_LOG_DIR=/app/logs` 并声明卷，compose 新增 `flamepanel-logs` 卷；systemd 部署沿用已有 `/opt/flamepanel/logs`（已在 `ReadWritePaths`）。文档：Doc/02、Doc/07、Doc/10 环境变量表、Doc/13 排障速查。

### feat(runtime): Node.js 运行时可用化——版本解析 / 镜像 / SHA-256 校验 / pm2 + 项目启动命令展开（Doc/30 S-11）

- **修复**：① 安装 `20` / `22` 这类主版本时直接拼出 `…/v20/node-v20-linux-x64.tar.xz`（404），只有写全三段才装得上——现查镜像 `index.json` 解析到最新补丁版（按数字段比较，只取提供 `linux-<arch>` 包的版本）；② Node 项目启动命令写 `node server.js` 时守护器把程序解析成 `<deploy>/node` 并按路径前缀拒绝——Node 项目此前无法用最自然的写法启动。
- `infrastructure/runtime/node.rs`：`FLAMEPANEL_NODE_MIRROR` 镜像（仅 https，明文镜像会让校验和一起被替换，忽略并告警）；下载同版本 `SHASUMS256.txt`，发行包流式 SHA-256 校验不符即拒装；解到 `.staging` 后扁平化到 `<install_path>/bin`（`bin_dir` 兼容旧版 `node-v*/bin` 布局）；`metadata_json.pm2 = true` 时 `npm install -g pm2`；`.fp-node.json` 记录实际版本 / 架构 / 镜像 / pm2，经新增的端口默认方法 `RuntimeInstaller::describe` 合并回运行时元数据。声明版本 `20 / 22 / 24 / 26`。
- `POST /api/runtimes/install` 新增可选 `pm2`（非 node 传 true → 400）；`RuntimeService::install_runtime_with_options`。
- `domain/project_command.rs`（纯函数）：写守护单元前展开 `$RUNTIME` / `${RUNTIME}`（运行时 bin 目录）与 `$DEPLOY`，裸 `node` / `npm` / `npx` / `corepack` / `pm2` / `pm2-runtime` 补全到运行时 bin；引用 `$RUNTIME` 却未绑定运行时给出可读 400。`ProjectService::ensure_supervised_unit` 统一入口（部署 / 启动 / 重启 / 编辑 / 零停机切换），注入 `PATH`（运行时 bin 打头）与 Node 项目 `NODE_ENV=production`（联链同名变量优先）；存库命令保持用户原文。顺带修复零停机切换与编辑刷新单元时丢失联链环境变量。
- `infrastructure/runtime/node_project.rs`：部署后 `package.json` + 锁文件摘要变化才装依赖（`node_modules/.fp-deps-stamp`），有锁文件 `npm ci` 否则 `npm install`，声明 `build` 脚本则 `npm run build`；用户与 Procfile 都未给启动命令时按 `ecosystem.config.{js,cjs}` → `npm start` → `main` → `server/app/index/main.js` 探测回填。
- 前端 RuntimeView：Node 页签「同时安装 pm2」开关、`$RUNTIME` 用法提示、已安装版本显示实际版本与 pm2 标记。i18n × 3。

### feat(certificate): 证书续签历史时间线 + 自动续签仅限 ACME（Doc/30 S-10）

- `domain/cert_renewal.rs`（纯函数）：从 outbox 事件落库归并单张证书的续签时间线（签发 / 自动续签成功 / 自动续签失败带原因 / 到期预警带到期时间），5 分钟内「签发 → 自动续签成功」合并为一条，最新在前、`limit` 1–200（缺省 50）+ `truncated`。
- `GET /api/certificates/{id}/renewals?limit=`（`certificate:read`）：返回开关态、`supports_auto_renew`、到期时间、续签窗口起点（到期前 30 天）、最近签发、扫描周期与时间线；404 / 401 / 403。PermissionRule 显式登记，OpenAPI 同步。
- `Certificate::supports_auto_renew`（仅 letsencrypt / zerossl）：`PUT` 对自签 / 上传证书开启自动续签返回 400。**修复**：老库 `auto_renew` 列默认 1，上传证书进入 30 天窗口后每 6 小时徒劳续签一次并发失败事件——`needs_renewal` 现要求来源支持续签；到期预警改为覆盖窗口内所有证书（上传 / 自签 / 已关自动续签的 ACME 证书更需要人工介入）。
- 前端：证书列表非 ACME 证书的开关禁用并提示原因，新增「续签记录」入口与 `CertificateRenewalDrawer`（概况 + 时间线，切换证书丢弃迟到响应）。i18n × 3。

### test(e2e): 新手引导 / 设置 / Setup 续填三条真浏览器路径（Doc/30 S-14）

- `04-onboarding.spec.ts`：首登弹出 → Esc 跳过（按用户名写完成标记）→ 刷新不再弹 → 用户菜单「重新播放引导」重放。
- `05-settings.spec.ts`：`/settings#security` 深链；改面板名未保存时经命令面板应用内跳转被拦、「取消」留在原页；同分区改两项保存恰好一次 `PATCH /api/settings/batch` 且载荷只含变更键；保存后跳转不再拦、刷新后服务端真值一致。
- `01`：走到数据库步刷新 → 回到管理员步，用户名回填、密码为空且不落 sessionStorage，「重新开始」清草稿。`helpers/flow.ts` 新增 `login(…, { keepTour })` 与 `navigateViaPalette`；登录档限流 5 次 / 分钟与 `/api/setup/initialize` 共用，整套串行用例会触顶，`login` 命中 429 时等窗口过期后重提一次（不放宽生产限流）。

### feat(ui): 命令面板最近访问 + 切换材质（Doc/30 S-15）

- `utils/recentRoutes.ts`：`router.afterEach` 按用户名记录最近 5 条菜单路由（去重、最新在前，localStorage `paletteRecent`）；命令面板打开时分组展示「最近访问 / 全部命令」，键盘游标跨组连续，输入变化复位游标。
- `cycle-material` 动作按 `stores/theme` 的材质列表循环切换，替代遗留的「玻璃开关」。i18n × 3。

### ci: 合并 PR 与 PR 分支推送不再重复跑质量门禁（节省 CNB 额度）

- 问题：PR 合并到 `main` 会同时触发 `main.push`，把刚在 PR 上跑过的 4 条门禁（installer / backend 8 核 / frontend / e2e 8 核）原样再跑一遍；有打开 PR 的分支每次推送也会被 `$.push` 与 `pull_request` 各跑一遍 backend / frontend。
- `push` 不再直接挂门禁，改为 1 核 `gate-dispatch` 流水线 + stage `if` 判定（`.cnb/scripts/need-gates.py`，纯标准库，镜像无 git 时回退 CNB API），需要时才 `cnb:apply` 触发 `api_trigger_main_gates` / `api_trigger_branch_gates`（门禁内容与 `ifModify` 不变）。
- `main.push`：HEAD 为 `合并来自 … 的合并请求 #N` 的双亲合并提交，且 PR #N 提交状态为 `success`，且 PR 预合并提交的 tree 与合并后 tree 完全一致时跳过；直推 `main`、PR 未全绿、合并前 `main` 已前进（落地代码未被测过）时照常跑全部门禁。
- 其他分支 `push`：等待 60 s（给推送后立即建 PR 留时间）后若该分支已有打开的 PR 则跳过，由 `pull_request` 以预合并代码跑全套门禁；没有 PR 时照常跑。任何判定异常一律放行门禁。`build-npc-image` 不变。文档：Doc/31 §5.2。

### feat(firewall): 防火墙规则表单补全——前后端同口径校验 + 各后端端口段渲染（Doc/30 S-13）

- **后端校验（新增，原先完全不校验）**：`domain/firewall.rs` 纯函数 `normalize_rule`，create / update 写入前统一执行，违反即 400 `VALIDATION_ERROR` 且 `message` 一次列出全部字段错误（`field: reason; …`），不落库、不下发命令。端口支持单端口 / 端口段（`-` 或 `:`）/ 逗号分隔、1–65535、≤ 15 槽位（端口段计 2，对齐 iptables multiport），仅 TCP / UDP 可带端口，规范存储为 `80,443,8000-9000`；来源 / 目标支持 IPv4 / IPv6 地址与 CIDR，主机位归零（`192.168.1.10/24` → `192.168.1.0/24`），拒绝 `/0` 与主机名；协议 / 动作 / 方向白名单并小写，名称 1–64、描述 ≤ 255、优先级 1–999。update 传 `""` 可清空端口 / 描述。
- **安全**：firewalld 原先把 `source` / `port` 直接拼进 rich rule 字符串（`source address="…"`），可注入额外规则片段；现严格白名单 + 渲染前二次解析，脏数据到不了系统命令。
- **各后端渲染**（`infrastructure/firewall_render.rs`，纯函数逐后端断言 argv）：ufw 改用完整语法 `ACTION in|out [proto] from A to B [port 80,8000:9000]`（此前忽略方向，删除只带 `action port` 常删不掉）；firewalld 每个端口项一条 rich rule，按地址族写 `family`，deny / reject 正确映射为 `drop` / `reject`（此前 deny 被当成"移除放行端口"）；iptables 端口段 `8000:9000`、多端口 `-m multiport --dports`、出站按 `-d` 匹配、IPv6 地址走 `ip6tables`、ICMPv6 用 `ipv6-icmp`。不支持的组合明确 400：ufw ICMP、firewalld 出站。历史遗留的非法规则：删除直接删库，编辑跳过旧规则移除以便修复；firewalld 删除旧版 `--add-port` 放行规则时回落旧形态。
- **前端**：新建 / 编辑共用 `components/firewall/FirewallRuleModal`（首次提交后逐项实时提示；ICMP / 任意协议禁用并清空端口；出站把地址标注为"目标 IP"；CIDR 显示"将保存为 …"规范化预览；按当前后端提示 firewalld 出站 / ufw ICMP 限制；回车提交），表格拆为 `FirewallRuleTable`，`FirewallView` 562 → 243 行；`utils/firewallRule.ts` 与后端同口径（含 IPv6 解析与 RFC 5952 压缩、IPv4 映射地址写法与 Rust 一致）。删除确认带规则名，失败 toast 携带后端原因。i18n × 3（`firewall.hints / errors / limits`）。
- **修复（既有缺陷）**：① 编辑规则时端口 / 描述为空不发送，无法清空；② 状态判断 `'inactive'.includes('active')` 为真、firewalld `not running` 含 `running`——未启用的防火墙显示为已启用（绿色标签 + "关闭防火墙"按钮），现先判"关"。
- 测试：后端 +18（域纯函数 8（端口 / 地址解析与拒绝、规范化、跨字段、全字段错误、内置默认规则回归）、渲染 4（ufw / firewalld / iptables / 脏数据拒绝）、服务 2（校验不落库且不下发、遗留脏规则可编辑修复与删除）、集成 4（401、全字段 400 不落库、规范化落库与编辑清空端口、viewer 可读不可建 403））；前端 +15（纯函数 10 + 视图 5）。基线：后端 952 / 前端 1491（148 文件），首屏 180.3 KB gzip（不变）。文档：Doc/02 §14 校验与渲染表、Doc/06、Doc/30 S-13 ☑。

### feat(toolbox): 系统工具箱——进程列表 + 结束进程（Doc/30 S-12）

- 新增 `domain/toolbox.rs`（纯函数）：`ProcessInfo` / `ProcessQuery::normalized`（关键字小写去空白、`limit` 缺省 100 上限 500）/ `apply_query`（按 pid / 名称 / 命令行 / 用户匹配，`cpu|memory|pid|name|started` × `asc|desc` 排序）/ `protection_reason`（`init` / `kernel_thread` / `panel` / `panel_ancestor`，沿 ppid 链防环遍历）/ 命令行**脱敏**（`--password=…`、`--token x`、`KEY=…` 等敏感参数值替换为 `***`，最长 1024 字符）；`ProcessSignal` = `TERM`（缺省）/ `KILL` / `HUP` / `INT`；`ProcessSource` trait 便于测试注入。
- `MetricsHub::process_table`：复用同一个 `sysinfo::System` 采样，2 s TTL + 单飞缓存（与 `top_processes` 共享采样，不额外刷全表），过滤 Linux 线程只保留进程；锁顺序固定为 `table_cache → process_system → process_cache`。
- `application/toolbox_service.rs` + 端点：`GET /api/toolbox/processes`（`toolbox:read`；`q` / `user` / `sort` / `order` / `limit` / `include_kernel`，响应带 `total` / `matched` / `total_memory_bytes` / `panel_pid` / `sampled_at`，每行附 `protected` 原因）与 `POST /api/toolbox/processes/{pid}/kill`（`toolbox:execute`；body `{signal?, started_at?}`）。拒绝 pid ≤ 1、面板自身及其祖先、内核线程（400）；进程不存在 404；`started_at` 与当前进程不符（pid 已被复用）409；系统层权限不足（`EPERM` / `sudo` 需要密码）403 且 `message` 含 `insufficient privilege` 与配置指引（不用 500——5xx 的 message 会被统一脱敏）；经 `SharedCommandRunner` 执行 `kill -s SIG pid`（不拼 shell，5 s 超时），同属主直接发信号、非同属主且面板非 root 时 `prefer_root`（`sudo -n`）；成功后使进程表缓存失效。POST 自动审计。OpenAPI 快照与 `src/api/generated` 同步（`ProcessRow` / `ProcessListResponse` / `KillProcessRequest` / `KillProcessResponse`）。
- 前端：新路由 `/processes`「进程管理」——服务端搜索（300 ms 防抖）/ 用户筛选 / 排序 / 显示内核线程 / 条数上限，5 s 自动刷新（可关闭，页签隐藏暂停，刷新期间保留旧列表），KPI 摘要（进程总数 / 匹配数 / 总内存），截断提示；`components/toolbox/ProcessTable`（CPU 着色、内存占比条、状态标签、运行时长、脱敏命令行）+ `KillProcessModal`（信号单选，`SIGKILL` 强制结束警告），受保护进程按钮禁用并提示原因，`v-permission` `toolbox:execute`；结束请求携带 `started_at`，404 / 409 提示后刷新列表。`composables/useProcesses`、`utils/processes.ts`、i18n × 3（新模块 `processes`，25 个模块）。
- 测试：后端 +22（域纯函数 9、服务 5（保护标记与合计、同属主不提权、非同属主 `prefer_root`、保护 / 不存在 / pid 复用拒绝、命令失败映射）、handler 2（sort / order 解析、请求体缺省 TERM）、`process_table` 1、集成 5（401、管理员列出真实进程并结束子进程、保护目标与非法参数 400、静态表过滤排序、viewer 可读不可结束 403））；前端 +18（纯函数 8 + `useProcesses` 5 + 视图 5）。基线：后端 934 / 前端 1476（146 文件）/ REST 243，首屏 180.3 KB gzip（+0.9 KB，语言包）。文档：Doc/02 §19c、Doc/04 路由表 + i18n 模块清单、Doc/06、Doc/07 工具箱权限行、Doc/30 S-12 ☑。

### feat(inspection): 安装后系统体检——内核信息 + 风险 / 漏洞分析（Doc/30 S-17）

- 新增 `inspection` 域（api → application → domain ← infrastructure 严格分层）：`domain/inspection.rs` 只含纯函数——把一份 `SystemFacts` 判定为 `InspectionReport { kernel, summary, findings[] }`，评分 = 100 − Σ 未通过项权重（critical 25 / high 15 / medium 8 / low 3 / info 0，下限 0），等级 A ≥ 90 / B ≥ 75 / C ≥ 60 / D ≥ 40 / F；每条结论只带稳定 `code` + 语言无关 `evidence`，标题与建议由前端语言包维护。内核 LTS 系列与发行版 EOL 为**内置表**（`DATA_VERSION = 2026-09`），`kernel_eol` / `os_eol` 标记 `heuristic = true`，厂商内核（`el` / `amzn` / `Ubuntu` 等）自动降级为信息级，超出内置表的新系列判 `unknown` 而非失败。
- `infrastructure/system_probe.rs`：文件优先采集（`/proc`、`/etc/os-release`、`/sys/devices/system/cpu/vulnerabilities/*`、`/proc/net/tcp{,6}`、`/etc/passwd` / `/etc/shadow` 权限位、sshd_config、sysctl、`/run/reboot-required`、`update-notifier` 缓存等），命令一律经 `SharedCommandRunner`（单条 3 s 超时、整体 15 s 预算，不拼接 shell 字符串），采集失败的项进入 `unknown` 而不是误报。覆盖 8 类 28 项检查：内核 / CPU 侧信道缓解 / sysctl 加固 / 访问控制（sshd root 登录、口令登录、空口令、MaxAuthTries、默认端口，UID 0 与空口令账户，`passwd` / `shadow` / `docker.sock` 权限）/ 网络（防火墙未启用、公网高危端口暴露、公网监听总览）/ 更新（待装安全更新、无人值守升级、时间同步）/ 存储（根分区用量、无 swap）/ 面板自身（会话超时、`/metrics` 令牌、进程用户）。
- `application/inspection_service.rs`：报告 TTL 缓存（默认 10 min）+ 单飞（并发请求只探测一次），`GET /api/inspection`（`toolbox:read`，无缓存时同步探测）与 `POST /api/inspection/run`（`toolbox:execute`，强制重跑）；`Services` 新增 `command_runner`。RBAC 新增 `toolbox:read` / `toolbox:execute` 两项内置权限（86 → 88），`seed_rbac` 对老库**只补授新增权限**、不恢复管理员手动移除的旧权限（新增回归单测）。OpenAPI 快照与 `src/api/generated` 同步。
- 前端：新路由 `/inspection`「安全体检」（`InspectionScoreCard` 评分环 + `KernelInfoCard` 内核 / 发行版 / 虚拟化 / 待重启 + `FindingList` 按严重度排序、可按状态筛选、启发式标签），仪表盘 `InspectionBanner` 摘要（有 critical / high 结论时高亮并可一键跳转，`v-permission="'toolbox:read'"`）；`composables/useInspection`（`useApiQuery` + 10 min `staleTime`，重跑走 `useApiMutation` 回写缓存）；`utils/inspection.ts` 纯函数（严重度排序 / i18n key 映射 / 等级配色）。i18n × 3：`inspection.ts` + 结论目录 `inspectionFindings.ts`（`inspectionFinding.<code>.{title,advice}`，遵守语言包单文件 < 200 行门禁）。
- 测试：后端 +26（域纯函数 15：os-release / 内核版本解析、系列生命周期、厂商内核降级、EOL 最长前缀、sshd 首值优先、`/proc/net/tcp` 解析、账户与更新解析、健康 / 高危系统评分、缺失事实 → unknown、更新内核待重启、磁盘阈值、等级；探测 3：假根目录 + 命令经 runner、空根目录全 None、meminfo 解析；服务 3：TTL 内命中缓存与强制重跑、TTL 过期重采、面板事实来自设置且采集源可替换；RBAC 补授 1；集成 4：未登录 401、管理员报告含内核信息与结论、缓存命中与 `run` 强制刷新、viewer 可读不可重跑 403），前端 +24（纯函数 10 + `useInspection` 7 + 横幅 3 + 视图 4）。基线：后端 912（CNB `cargo test --all` 默认特性实测：单元 629 + 集成 279 + Agent 3 + 文档 1；此前文档口径 816 系早期 sqlite 子集累加、已漂移，本次对齐实测）/ 前端 1458（143 文件），首屏 179.4 KB gzip（+0.2 KB）。文档：Doc/02 §19b、Doc/04 路由表 + i18n 模块清单、Doc/06、Doc/07 权限矩阵（88）、Doc/30 S-17 ☑。

### feat(dashboard): 仪表盘时间范围切换——实时 / 1 h / 24 h / 7 d（Doc/30 S-16）

- 趋势图头部新增分段控件（`FpSelectButton`）：`live` 仍由 `/ws/metrics` 6 min 窗口驱动；`1h / 24h / 7d` 改为消费 S-07 `GET /api/metrics/history?range=` 的均值点（1 min 或 1 h 桶），网络 IO 图与趋势图**同源同范围**切换，指标卡与负载仪表盘保持实时。徽标随模式切换：实时显示 WS 连接状态，历史显示"1 min / 1 h 均值"或加载失败。
- 新增 `composables/useMetricsHistory(range)`：基于 `useApiQuery`（`queryKeys.metrics.history`），`live` 时禁用请求（不多出一路 REST 轮询），切换期间 `keepPrevious` 保留上一范围曲线不闪空，分钟点 60 s / 小时点 5 min 后台刷新（页签隐藏自动暂停），失败仅重试一次。`useApiQuery.enabled` 放宽为 `MaybeRefOrGetter<boolean>`。
- 范围选择持久化到 `localStorage['fp.dashboard.trendRange']`（非法值回落实时）；历史样本到达与范围切换复用三图共享的 1 s 节流重绘通道。刻度格式 / 最小间隔按范围经 `timeAxisPatch(range, compact)` 增量打补丁（实时到秒、小时级到分、7 d 带日期；网络窄图去秒并放宽间隔）。
- **修复（既有缺陷）**：① 折线图在 `type: 'time'` 轴上用 `xAxis.data` + 一维数值——时间轴忽略 `xAxis.data`，点按下标排布、刻度无时间、tooltip 显示 1970；现统一为 `[ms, 值]` 对（`toMillis` 归一 WS 的 ms 与历史桶的秒）。② `useECharts.applyOption` 以 `notMerge: true` 应用主题补丁，会把整份 option 换成只含轴配色的补丁，series 丢失后曲线永久空白并持续报 `Unknown series`——切一次主题（或任何触发 `<html>` class/style 变化的操作）仪表盘就不再画图；现改为合并模式。
- i18n：`dashboard.range.*` × 3 语言。测试：前端 +16（`useMetricsHistory.spec.ts` 5、`dashboardCharts.spec.ts` 时间轴 5、`DashboardView.spec.ts` 范围编排 6），既有断言改为 `[ms, 值]` 形态；基线前端 1434 / 139 文件，首屏 179.2 KB gzip（+1.0 KB）。文档：Doc/04 §Dashboard 图表（补丁合并模式与时间点形态规范）、Doc/30 S-16 ☑ + M-P "7 天趋势"达成。

### feat(metrics): 监控历史持久化 + `GET /api/metrics/history?range=`（Doc/30 S-07）

- 新增 `application/metrics_history_service.rs`：作为 `MetricsHub` 的 broadcast 订阅者（supervisor 任务 `metrics-history-writer`），3 s 快照在内存聚合成 **1 min 点**（均值 / 峰值 / 样本数）落库，跨整点把上一小时 1 min 点按样本数加权 rollup 成 **1 h 点**，并按保留策略裁剪（1 min 7 d / 1 h 90 d）；关停时 flush 活动桶；乱序旧样本丢弃。写放大仅每分钟一行 + 每小时一行。
- 领域新增 `MetricsResolution` / `MetricsHistoryPoint` / `MetricsHistoryRepository`；SQLite 单表 `metrics_history`（`WITHOUT ROWID`，主键 `(resolution, bucket_start)`，幂等建表迁移）与 InMemory 实现；`RepositoryFactory::create_metrics_history_repo`。
- 端点 `GET /api/metrics/history?range=1h|6h|24h|7d|30d`（`node:read`，缺省 `24h`）：≤ 24 h 返回 1 min 点（并带上当前活动桶），7 d / 30 d 返回 1 h 点；响应含 `step_secs` / `from` / `to`；非法 `range` → 400。OpenAPI 快照与 `src/api/generated` 同步（`MetricsHistoryResponse` / `MetricsHistoryPoint`）。
- 前端 `api/metrics.ts` 新增 `getMetricsHistory(range)`、`METRICS_HISTORY_RANGES` 与 `queryKeys.metrics.history(range)`；仪表盘时间范围切换列为后续切片 S-16。
- 测试：后端 7 单测（range 解析 / 桶边界 / 累加器均值峰值 / 加权 rollup / 跨分钟落库与活动桶 / 跨整点 rollup + 裁剪 / SQLite 仓储往返）+ 4 集成（401 / 400 / 形状与 step / 缺省 24h）；前端 `api/__tests__/metrics.spec.ts` 2 例。文档：Doc/02 §19、Doc/01 §7.5a、Doc/30 S-07 ☑。

### refactor(settings): 下线 `open_menu_tabs` 设置项，设置列表一次拉全（Doc/30 S-04）

- 多页签栏移除后 `open_menu_tabs` 已无消费方：从 `default_settings()` 删除，`run_migrations` 追加幂等 `DELETE FROM panel_settings WHERE key = 'open_menu_tabs'` 清理旧库残留（`ensure_defaults` 不再回写）；新增迁移回归单测（旧库残留行被清理、重复迁移安全、其余默认项完整）。
- 前端 `listSettings` 默认页大小 50 → 200（对齐后端 `PaginationParams` 上限），SettingsView 一次拉全，避免设置项增多后被分页截断。
- 基线：后端 805（+1 单测）。

### feat(ws): 推送流背压治理——lag 计数进 `/metrics`，慢消费者 `resync` 后断开（Doc/30 S-08）

- 新增 `api/ws_backpressure.rs`：`LagPolicy`（**连续** 3 次 `RecvError::Lagged` → `Resync`，任一成功送达即清零）与进程级 `WsStats` 原子计数；`run_push_session`（`/ws/metrics`、`/ws/logs` 共用骨架）接入——偶发滞后只记数继续推送，连续滞后则发控制帧 `{"type":"resync","reason":"lagged"}` 并关闭连接，不再让慢客户端长期静默丢帧。
- `GET /metrics` 新增 3 个按 `stream="metrics"|"logs"` 标签的计数器：`flamepanel_ws_lag_events_total`、`flamepanel_ws_lag_dropped_frames_total`、`flamepanel_ws_slow_consumer_disconnects_total`。
- 前端 `useSharedWebSocket` 注册表识别 `resync` 帧：跳过退避直接 `reconnectNow()`，且不把控制帧当作数据分发给订阅者；重连后的 `init` 帧携带完整历史窗口补齐曲线。
- 文档：Doc/02 §24 与 Doc/01 §7.6 记录控制帧与指标；Doc/30 S-08 标记完成。测试：后端 7 单测（策略状态机 / 计数 / Prometheus 输出）+ `/metrics` 集成断言，前端 `useSharedWebSocket.spec.ts` +2 例。

### feat(overview): 顶栏聚合徽标端点 `GET /api/overview/badges`（Doc/30 S-06）

- 后端新增 `application/overview_service.rs`：节点 / 容器 / 任务 / 证书四个来源**并发**采集（单域 2 s 超时，失败或超时只让该域为 `null`），进程内 **10 s TTL 缓存 + 单飞**——多页签并发轮询只触发一次真实聚合，Docker daemon 每 TTL 至多被查询一次。`handler/overview` 为 auth-only 端点，响应按调用者角色的 `node:read` / `docker:read` / `task:read` / `certificate:read` **逐分组裁剪**（缺角色上下文时全部不可见，fail closed）；中间件新增 `UserRole` 请求扩展供此类聚合端点复用。`alerts` 汇总离线节点、14 天内到期证书、失败任务。
- 前端 `AppHeader` 原先每 10 s 并发三个 REST（节点列表 200 条 / 容器列表 / 操作日志 6 条）并在浏览器里过滤计数，现改为 `useOverviewBadges` 单接口 **30 s** 轮询、页签隐藏不轮询、切回前台按 staleTime 补刷；分组为 `null` 时隐藏徽标而不是显示 0。新增"进行中任务"徽标（进度指示 → `/tasks`）；铃铛角标改为告警总数，通知面板先列告警明细（可点击跳转），"最近操作"改为面板打开时才拉取。修正原实现 `panelOk` 永远为 true 的缺陷。
- OpenAPI 快照与 `src/api/generated` 同步（`OverviewBadges` 等 5 个 schema）；Doc/02 §19a、Doc/30 S-06 标记完成。测试：后端 9 单测 + 6 集成（200 / 401 / 角色裁剪 / 缓存失效 / 并发单飞），前端 `useOverviewBadges.spec.ts` 5 例。
- 附带：`plugin/sandbox.rs` 在 `--no-default-features --features sqlite` 下的 clippy `needless_return`（feature 子集 clippy 现同样零告警）。

### ci: CNB 流水线接管后端 / 前端 / E2E 门禁

- `.cnb.yml` 重写：任意分支 push / PR 触发 `installer-regression`、`backend-gates`（fmt → clippy `-D warnings` → `cargo test --all` → feature 子集 `--no-default-features --features sqlite` → OpenAPI 快照漂移）、`frontend-gates`（`npm ci` → typecheck → lint → 单测 → 覆盖率地板 → build → 首屏 200 KB 预算 → i18n → `npm audit`）；PR 与 `main` push 额外跑 `e2e-playwright`（用 `.ide/Dockerfile` 镜像，真后端二进制 + Chromium）。重门禁按 `ifModify` 只在相关目录变更时执行；cargo registry / target 与 npm 缓存挂 `main` 组 copy-on-write 卷。
- 云开发容器（`vscode`）复用同一组缓存卷，开箱即有增量编译产物。
- 前端 `src/components/ui/**` 覆盖率地板（函数 90% / 分支 84%）在 `main` 上本已不达标（87.64% / 82.49%），补齐 `PageToolbar`（`/` 快捷聚焦与卸载解绑）、`LayoutContent`（说明 / KPI 摘要 / 刷新复位）、`FpStatePanel`（重试 / 空态动作位 / 骨架与转圈 / `hideEmpty` / 插槽替换）、`CopyButton`（图标复位）行为规格，门禁恢复绿色（`components/ui` 函数 96.6% / 分支 90.6%；前端单测 1390 → 1409）。
- 文档：Doc/06 §6、Doc/31 §5.2 改为以 CNB 流水线为准；Doc/30 S-05（feature 矩阵进 CI）标记完成。

### docs: 文档集精简——统一开发流程、重写路线图、删除归档与过期任务单

- 新增 `Doc/31-开发流程.md`（人类与 AI 开发者共用：交付生命周期、切片法、Issue 模板、分支 / PR 规范、门禁与执行环境含低内存机器、Review 六看、发布、AI 附加规程、黄金路径模板），取代《31 NPC 开发手册》《33 云原生开发与后端演进规划》。
- 重写 `Doc/30-路线图与里程碑.md`：基线更新至第三轮之后，已交付项（T-01…T-08、sqlite 拆分、MetricsHub、前端第二 / 三轮）入盘点；新增 **M-P 性能与可观测**、**M-UX 前端体验** 主线；§4 为可直接派发的切片 S-01…S-15（取代《32 任务单库》）。
- 删除 `Doc/archive/`（10 篇历史方案 / 评审台账，结论早已并入 30 / 31，需要论据用 git 历史查阅）；修正 Doc/02 / 04 / 05 / 06 与 CI 注释中指向归档的链接。
- `README.md`、`Doc/README.md`、`AGENTS.md` 同步：文档索引、开发流程一图、feature 子集门禁命令、`MetricsHub` / 材质 / `data-testid` 约定。

### test: 修正两处仅在 feature 子集下失败的单元测试

- `certificate_service` 到期事件去重用例改为只断言不重复发布 `CertificateExpiring`（续签结果事件 `Renewed / RenewalFailed` 每次扫描都会发，之前被误判为重复）。
- `app_store_service::installs_wasm_builtin` 加 `#[cfg(feature = "wasm")]`；`cargo test -p flame-kernel --no-default-features --features sqlite --lib` 576 例全绿，可作为 CI feature 矩阵的一项（Doc/30 S-05）。

### refactor: 指标采集/推送链路重构为 MetricsHub（单次序列化 + 广播扇出）

- `infrastructure/metrics.rs` 重写：`MetricsSampler`（独占 sysinfo `System`）→ `MetricsHub::publish` 一次序列化成 `MetricsFrame { snapshot, json: Arc<str> }` → `broadcast` 扇出；`/ws/metrics` 每连接只克隆 `Arc`，不再为每个订阅者各自 `serde_json::to_string`。历史环形缓冲改为 `std::sync::RwLock`（无 async 锁），`history()`/`latest()`/`uptime()`/`subscriber_count()` 供 REST、WS 与 `/metrics` 共用。
- 修正采集缺陷：`Disks` 从未刷新（磁盘用量长期为启动值）；网速按固定 1 s 而非真实采样间隔换算导致约 3 倍高估；进程列表接口双重 `refresh_all`（现为独立 `System` + 2 s TTL 缓存）；`/metrics` 的 uptime 改为进程启动时间差。
- `api/handler/ws` 两个推送分支合并为 `run_push_session`；`AppState` 仅持有一个 `Arc<MetricsHub>`；wire 格式 `{type:"init"|"tick",data}` 不变。
- 新增 16 个单元测试（含两个性能断言：共享序列化对比逐订阅者序列化、100 订阅者扇出单次 publish < 5 ms）。

### fix: 未启用 `docker` feature 时 `sanitize_project_name` 缺失导致编译失败

- 项目/容器命名规整从 `docker.rs` 抽到 `infrastructure/naming.rs`（无 feature 门控），`db.rs` 与 `project_builder.rs` 改为引用新位置，`docker.rs` 保留再导出；`--no-default-features --features sqlite` 恢复可编译。

### feat: 前端第三轮 · 开屏性能与首次使用体验（详见 `frontend/design.md` §17）

- 移除多页签栏（`AppTabs` / `stores/tabs`），keep-alive 白名单直接取自路由 meta；后端 `open_menu_tabs` 设置项保留未消费。
- 材质三档 `solid | glass | acrylic`（`data-material` + `--fp-surface-*` 令牌）；品牌色只发布 OKLCH 种子，色阶 / 交互态 / 自动对比前景由 CSS 相对色语法派生。
- 仪表盘：图表面板异步加载 + 骨架占位（echarts chunk 脱离首屏关键路径），统计卡在数据到达前显示骨架而非 0；指标流历史 `shallowRef` + sparkline 记忆化；首屏 gzip 178.2KB / 200KB 预算。
- 面板设置升为侧栏底部一级入口（快捷键 `,`）；`SettingsView` 六分区 hash 深链、按分区脏标记、只提交变更键的原子 `PATCH /settings/batch`、离开确认。
- 新增可跳过的新手引导（按用户持久化、版本升级重触发、锚点缺失自动跳步、非阻塞遮罩）；安装向导新增进度条 / 可回退步骤 / 密码强度计 / 刷新续填（草稿不含密码）。
- 单元测试 1331 → 1390（新增 onboarding store / OnboardingTour / SettingsView / password 等规格）。

### fix: 统一任务顺序持久化与错误确认（B02b）

- 任务创建、状态/进度更新及删除共享串行写入屏障，落库成功后才发布内存快照；请求断开后，已接纳写入继续完成。持久化错误向业务服务、取消/清理接口传播。
- SQLite / InMemory 更新缺失 ID 时拒绝 upsert；失败创建不复用本 tracker 已尝试的 ID，恢复前拒绝变更，清理失败不谎报成功数量。
- 安装锁补齐 RAII 释放，正常停机有界等待任务写入排空；新增门闩/故障注入、请求取消、文件数据库重开、HTTP 错误及鉴权回归。
- 不扩展真实任务取消或外部操作回滚/重放；响应字段、路由和数据库结构不变。

### feat: 证书到期与自动续签事件驱动及管理员邮件通知 (Ref: #T-08)

- 在领域事件 `DomainEvent` 中新增 `CertificateRenewed { id, domain }` 与 `CertificateRenewalFailed { id, domain, error }` 变体。
- 在 `CertificateService::scan_and_renew` 循环中接入事件总线发布：自动续签成功或失败均向总线投递相应事件。
- 在 `EmailChannel::render` 中补齐续签成功及续签失败的邮件模板渲染，并在失败时将错误原因通知管理员。
- 扩充 `tests/event_bus.rs` 与 `notification/mod.rs` 单元测试，断言新事件发布、订阅与邮件模板渲染行为；同步更新 `Doc/24-事件与通知系统.md`。

### fix: InMemory 仓储锁中毒容错处理 (Ref: T-02)

- 新增 `flame-kernel/src/utils/sync.rs` 工具模块，提供 `lock_or_recover` 函数，在锁中毒（PoisonError）时回收内部数据继续使用，避免单个 panic 造成级联崩溃。
- 将 `flame-kernel/src/infrastructure/db.rs` 中 172 处 `lock().unwrap()` 全部替换为 `lock_or_recover` 容错调用，零破坏现有语义和业务逻辑。
- 新增针对 `lock_or_recover` 的子线程 panic 锁中毒恢复单元测试。

### refactor: sqlite.rs 完整模块化拆分，解耦全部 29 个仓储实现 (Ref: #T-03 #T-04)

- 将原本 6056 行的单文件上帝类 `infrastructure/sqlite.rs` 彻底重构为模块化目录 `infrastructure/sqlite/`。
- 将全部 29 个 SQLite 仓储实现按职责领域完整拆分至 29 个独立子模块文件：
  - 用户与节点：`user.rs`, `node.rs`
  - Web 与站点：`website.rs`, `web_server.rs`
  - 系统配置与安全：`database.rs`, `settings.rs`, `permission.rs`, `role.rs`, `operation_log.rs`
  - 消息与监控：`outbox.rs`, `log.rs`, `firewall.rs`, `waf_rule.rs`, `waf_log.rs`
  - 应用与插件：`app_package.rs`, `app_source.rs`, `installed_app.rs`, `plugin.rs`
  - 任务与运行时：`scheduled_task.rs`, `memo.rs`, `task_store.rs`, `backup_account.rs`, `backup_job.rs`, `runtime.rs`, `project.rs`, `project_db_link.rs`, `revoked_token.rs`
  - 证书与 DNS：`certificate.rs`, `dns_provider_account.rs`
- 在 `infrastructure/sqlite/mod.rs` 中完整重新导出全部 29 个仓储结构体，实现对全局调用者的零改动兼容，`sqlite/mod.rs` 从 6056 行彻底精简至 1474 行（仅保留连接池配置、通用迁移器 `run_migrations` 与基准测试集）。

### test/ci: 安装回归补齐边界并接入双平台 CI

- 下载/部署片段测试从 16 增至 28 项，增加符号链接、空产物、复制/chmod/临时目录失败、私有权限及含空格路径；另增 5 项 CI 入口回归。
- 新增 `scripts/check-install.sh`，统一 Bash 语法检查和离线测试，错误退出码直接向上传递，不执行真实安装。
- GitHub 新增独立安装回归 job，并纳入现有 Docker 发布依赖；CNB 的 main/兜底 push 与 PR 事件复用轻量测试流水线，不改云开发/NPC 配置。
- 更新部署指南，明确分支保护与托管平台执行结果仍需单独验收。

### fix: 安装下载失败明确终止，产物检查后再替换

- 下载启用 HTTP 错误检查；镜像回退使用临时文件，拒绝空响应并清理失败半包。
- 二进制/前端下载、解包或必要文件检查失败时非零退出，不继续配置服务；失败前端包不删除既有 dist。
- 新增 16 项离线隔离回归，覆盖镜像回退、部分传输、产物缺失、正常下载和本地产物；不执行真实安装。
- 同步部署文档：明确非事务回滚，以及当前尚未自动验证 SHA256/发布签名的边界。

### fix: 前端依赖安装与云开发/CI 工具链一致性

- 修正 `.gitignore` 对前端锁文件的误忽略，纳入 npm v3 锁文件；日常安装统一 `npm ci`。
- 固定 Node/npm，并在云开发、GitHub CI、发布与 Docker 前端构建中验证版本；不变更直接业务依赖声明。
- js-yaml override 从 4.3.1 升至 4.3.2，修复 GHSA-2883-xcg3-v3hh；不批量升级其他依赖。
- 前端预热改为有界的前台安装，日志与失败可见；prepare 从仓库根安装既有 Husky 钩子。
- 补齐 CNB `docker.build.by`，避免 SSH 修复配置未进入镜像上下文而导致 Prepare 失败。

### perf: 运行时发行包下载流式落盘 + 共享 HTTP 客户端

`runtime/download.rs::fetch` 原先 `resp.bytes()` + `to_vec()` 把 Node/Go 发行包（数十 MB）
双份读入内存，512 MB 目标环境下一次安装峰值可达 ~160 MB；且每次调用 `reqwest::Client::new()`
重建连接池与 TLS 配置（`app_store/remote_source.rs` 两处同病）。

**变更**：
- 新增 `infrastructure/http.rs::shared_client()`：进程级 `OnceLock<reqwest::Client>`，
  下载器与远程源复用；AgentClient / ACME / 健康探针等需专用配置的域不收编；
- `fetch` 改为流式落盘（`chunk()` → 临时文件），返回 `DownloadedFile`（Drop 自动清理，
  成功/失败路径都不留残包）；
- `archive.rs` 新增 `untar_gz_file` / `untar_xz_file` 文件入口：gz 直接流式解包；
  xz 解码产物写临时 tar 文件（Node 包解压后 ~200 MB 不再驻留内存），复用同一
  `unpack_tar`（路径穿越防护语义不变）；字节入口保留（`untar_auto` 等调用方未动）；
- go / node / python / tomcat 四个安装器改走文件入口；行为与错误文案不变。

**测试**：新增 4 例（共享实例断言、gz 文件解包、缺文件报错、xz 文件解包 + 临时文件清理断言）；
`infrastructure::runtime` 全域 53 例、`app_store::remote_source` 7 例回归绿。

### fix: 移除非 unix 分支误用的 libc::kill（T-01）

`scheduled_task_service.rs` 的 `#[cfg(not(unix))] run_shell` 超时分支调用 `libc::kill(pid, SIGKILL)`，
但非 unix 目标上 `libc::SIGKILL` 不存在，属死代码中的编译地雷（CI 仅跑 Linux 故未暴露）。
采用最小改法：删除该分支的手动补杀，依赖既有 `kill_on_drop(true)` 在 future drop 时终止子进程；
unix 分支行为不变（仍补一次 SIGKILL 确保及时回收）。`EXEC_TIMEOUT`、返回类型、`execution.rs` 均未改动。
### ci: OpenAPI 快照漂移门禁（T-05）

`frontend/openapi.json` 是前端类型单源（85 条路径），但代码实有 219 条路由——快照漂移无人察觉。
CI `rust-test` job 新增 step：用**既有** `--bin export-openapi`（任务单原设想新增 `--dump-openapi`
子命令，实际代码已有该二进制，按实际代码为准）重新生成并与仓库文件 diff，不一致即红并提示修复命令；
`justfile` 新增 `openapi` 配方；本地已验证当前快照与生成结果一致（85 路径，无漂移）。
**未注解端点清单**（131 条，按域分组）产出到 `docs/mcp-openapi-unannotated-endpoints.md`，
供后续「分域补 utoipa 注解」拆单使用；本单不补注解、不改路由。

### chore: 版本号单源——三处版本对齐 0.9.0（T-06）

`flame-kernel/Cargo.toml`、`agent/Cargo.toml`、`frontend/package.json` 均为 0.1.0 而 README/CHANGELOG
讲 v0.9，`package-release.sh` 从 Cargo 读版本导致发行资产命名为 0.1.0。三处对齐为 **0.9.0**；
新增 `scripts/bump-version.sh <ver>`（sed 两处 Cargo + `npm version --no-git-tag-version` + 刷新
Cargo.lock，含版本号格式校验）与 `just bump VERSION` 配方；`/health` 的 version 断言从「非空」
收紧为「等于 CARGO_PKG_VERSION」（bump 后测试无需跟改）。不打 tag、不改 CI release 触发条件。

### docs: 文档体系 v2——面向 300 轮 NPC 会话重组（Doc/30–32 + AGENTS.md + Skills 重写）

**动机**：旧 24 篇文档中流程类（14/17/19/20/21/22/23/24）合计 ~7000 行且互相引用，NPC 每单为「找规矩」耗 30–60 轮；
多处 Skill 与代码现状不符（pnpm、`:id` 路由语法、不存在的 `api/app_state.rs` / `setup_app_with_admin`）。

**变更**：
- 新增 `Doc/30-路线图与里程碑`（唯一"做什么"）、`Doc/31-NPC开发手册`（300 轮预算模型、垂直切片法、任务单 v2、黄金路径模板）、
  `Doc/32-任务单库`（12 张可直接派发的 Issue：技术债 T-01~06、M-C T-07/08/11/12、Agent T-09、前端 T-10）；
- `AGENTS.md` 重写为 ≤120 行：预算协议 + 门禁 + 红线 + 按任务类型的路由表；
- 7 张 `.codebuddy/skills` 全部按当前 main 代码重写（`handler/waf` 为参照域，含可粘贴模板）；
- `Doc/06-部署运维指南` 重写为 `Doc/10`（30 秒决策树、env 全表、上线 10 项检查）；README / Doc/README 重写；
- 文档重新编号为 0x 代码参考 / 1x 运维 / 2x 生态协议 / 3x 流程规划；13/14/17/19/20/21/22/23/24 与旧 06 移入 `Doc/archive/`（只读留档）；
- `.cnb/settings.yml` NPC 角色 prompt 同步预算协议；CI 注释指向 archive。

**不变**：01–05/07 代码参考文档、09→06 测试指南、18→12 安全基线、10→13 排障、08/15/16→20/21/22、11/12→23/24 仅改编号与交叉链接，内容未动。

### refactor: i18n 按业务域拆分——三语言千行单体拆为 22 模块，key 一致性门禁进驻 CI（Doc/23 T2-5 / R-6）

`Doc/23 §3.3 T2-5` / 问题 **P1-6**（i18n 单文件巨型化）与 **R-6**（裸 key 上屏）。
**三语言包 3438 行（最大单文件 1148 行）→ 22 模块 × 3 语言（最大单文件 109 行）**，
零新增依赖（复用已装的 `typescript` 做 AST 校验），装载器对外形状不变（`typeof zhCN`）。

**为什么补 key 与建门禁必须同一个 PR**：R-6 的修法是「补齐缺失 key」，但缺 key 能
长期存活是因为**没有任何机器判据拦它**——`t('website.title')` 在 JS 里是合法字符串，
typecheck / lint / 既有单测全绿。只补 key 不建门禁，下一轮新写一个 `t()` 就会重新
长出裸 key。本 PR 先建判据、再用判据把存量清零。

**拆分口径**（按业务域，不按行数均分）：一模块 = 一处心智边界，改「网站相关文案」
只需动 `websites.ts` 一个文件 × 三语言，不与 Docker/备份冲突。22 模块：
`common` / `shell` / `auth` / `dashboard` / `memo` / `users` / `nodes` / `files` /
`databases` / `websites` / `docker` / `plugins` / `security` / `logs` / `tasks` /
`settings` / `backup` / `runtime` / `webServers` / `appStore` / `projects` / `terminal`。

**结构**：

| 层 | 文件 | 职责 |
|----|------|------|
| 装载器 | `src/locales/index.ts` | `setLanguage()` / `loadLocaleMessages()`；懒加载路径写实到 `./<lang>/index`；对外类型仍是完整消息树，调用方零改动 |
| 语言包 | `src/locales/<lang>/<module>.ts` | 22 个业务域模块 × 3 语言，各含本域命名空间 |
| 聚合 | `src/locales/<lang>/index.ts` | 显式 import + 展开；三语言模块清单与顺序一致（门禁校验） |
| 门禁 | `scripts/check-locale-keys.mjs` | 四类校验：结构一致 / key 集合一致 / 占位符一致 / 源码引用存在 |

**门禁四类校验**：① 三语言模块清单完全相同（缺模块 / 多模块 / 顺序不同分别报出）；
② 三语言扁平化 key 集合完全相等，逐条报出「哪个语言缺哪条」；③ 同一 key 的 `{param}`
集合三语言相等（同参数重复出现按去重比较）；④ 源码 `t('a.b')` 与路由 `meta.title: 'a.b'`
的静态 key 三语言必须存在，`t('a.b_' + x)` 类动态拼接按前缀校验至少一条命中。
刻意**不**扫「全部 key 形状字面量」：`app.config.errorHandler` 这类非 i18n 字面量会误伤，
把门禁做成噪音源头只会被绕过。

**门禁挂法**：CI 新增 `Frontend i18n Key Gate`（`npm run locale:check`）+ lint-staged
`"src/locales/**/*.ts": ["npm run locale:check"]`（仅语言包改动触发，卡住事故源头；
全量挂在每次提交会让「改一个注释也等全量校验」）。

**R-6 清零（门禁首轮报出 13 条引用未命中）**：10 个真缺 key（`nav.appStore` /
`tabs.closeTab` / `tabs.refresh` / `common.refresh` / `common.placeholder` /
`common.prev` / `common.next` / `website.title` / `website.switchEngine` /
`appStore.deleteSource`）× 三语言补齐（净新增 10 key，语言包 1047 → 1057）；
另 3 条（`settings.oldPasswordRequired` / `settings.newPasswordRequired` /
`settings.passwordMinLength`）是**指错了命名空间**——`login.*` 下已有语义完全相同
的三条死键，故改 `LoginView.vue` 取值指向 `login.*`，而非在 `settings` 下再抄一份。

**测试**：前端新增 **20 例**（`scripts/__tests__/locale-keys.spec.mjs`，与
`bundle-size.spec.mjs` 同列）：纯函数 3 + 源码扫描 4 + 结构一致性 2 + key 集合与
占位符 3 + 引用存在 3 + CLI 语义 2 + 真实语言包 3（含「单文件行数 < 200」的 P1-6 判据）。

**文档**：Doc/23 §7.24 交付记录（含六项回退证据与真机复验输出）、Doc/04 §11、
Doc/01 §4.1、Doc/22 §5 契约同步矩阵、Doc/09、README 基线同步。

### refactor: 节点视图拆分——824 行拆为四层，两段式状态覆盖收敛为单一写回点（Doc/23 T2-4 第七步）

`Doc/23 §3.3 T2-4` / 问题 P1-1。**NodesView 824 → 188 行（-77%）**，
零新增依赖、零契约变更（同一批 API、同一文案 key、同一轮询间隔与可见性暂停语义）。
**T2-4 巨型视图拆分队列至此七站全部交付，`src/views/**` 的可测逻辑已全部迁出覆盖率排除区。**

**为什么第七站是它，也是最后一站**：前六站（AppStore/Websites/Docker/Backup/
Dashboard/Project）交付后，NodesView（**824 行**）是队列里唯一剩下的。它的形态
与前六站都不同——**一张表 + 四个弹窗**，但数据流是「**两段式**」的：列表接口
只给「注册时填写的期望状态」与一份可能过期的 `metrics_json` 快照，真正在线与否、
实时指标如何，靠列表每次加载完成后逐节点并发查 `/status` + `/metrics` 并**写回
同一份查询缓存**。824 行里最容易写错的地方不是某个弹窗，而是「**谁在什么时候
覆盖谁**」：首屏、翻页、10s 轮询、页面切回四种时机都会触发覆盖，而写回目标必须
是「查询缓存里的**当前页**」。

**四层结构**（11 个新文件）：

| 层 | 文件 | 行数 | 职责 |
|----|------|------|------|
| 数据 | `composables/useNodeList.ts` | 124 | 列表 + 分页 + **两段式状态覆盖** + 页面切回补刷 |
| 数据 | `composables/useNodeCrud.ts` | 86 | 注册 / 编辑 / 删除（二次确认，文案带节点名） |
| 数据 | `composables/useNodeRemoteExec.ts` | 149 | 远程命令（**失败落结果区**）+ 批量命令（**全选当前页**） |
| 数据 | `composables/useNodeFiles.ts` | 134 | 远程文件：浏览 / 进入 / 上跳 / 下载 / 上传 |
| 展示 | `components/node/NodeListPanel.vue` | 149 | 八列表格（指标条 + 心跳 + 四类行操作） |
| 展示 | `components/node/NodeFormDialog.vue` | 120 | **注册与编辑合并为一个组件**（原为两个逐行相同的弹窗） |
| 展示 | `components/node/NodeExecDialog.vue` | 104 | 远程命令（结果区常驻，成败都渲染） |
| 展示 | `components/node/NodeBatchExecDialog.vue` | 136 | 批量命令（勾选 + 逐节点结果三级回落） |
| 展示 | `components/node/NodeFilesDialog.vue` | 154 | 远程文件（路径栏 + 目录列表 + 上传区） |
| 纯函数 | `utils/nodeFormat.ts` | 205 | 指标解析与摘要、取色、路径拼接、base64 编解码、表单校验与更新体组装 |
| 装配 | `views/NodesView.vue` | **188** | 列表接线 + 四个数据层的开合编排 |

**五条边界取舍**：

1. **两段式覆盖的目标是「查询缓存的当前页」而不是另存一份 ref**（本站最关键的
   一条）：视图里只有「查询缓存」这一个数据源，WS / 轮询 / 手动刷新谁先到都不会
   出现两份状态互相打架。用例用「翻页后覆盖的是第 2 页」把 key 写死的错误钉住；
2. **状态查询失败保留上一次的指标**（只把 `online` 置 false）：Agent 刚掉线时最后
   一次上报的 CPU/内存仍有诊断价值，抹成「—」反而丢失了「掉线前的样子」；
3. **`enrichNode` 的 `online` 恒为 false**：列表接口的 `status` 是注册时填的期望值，
   把它当在线会让离线机器显示绿点（富化阶段一律置 false，真实在线只由 `/status` 判定）；
4. **注册与编辑合并为一个弹窗组件**：原文件里是两个几乎逐行相同的实现（同一组四个
   字段、两份相同的校验函数、两段相同的提交前处理），合并后「加一个字段只需改一处」，
   从结构上消除「注册能填、编辑忘了」的分叉；
5. **单节点执行失败落进结果区（`exit_code: -1`），批量失败只 toast 且保留上一轮结果**
   ——两套口径**刻意不统一**（原实现即如此，逐条保留）。

**测试**：新增 **117 例**（8 个新 spec：纯函数 23 + 数据层 40 + 展示层 41 + 视图装配 13）。
覆盖率地板按阶梯第七次上调 **branches/functions 66/60 → 68/62**：本站拆出的 4 个
composable 与 1 个纯函数模块全部落回统计口径，分母由 3845 → **4149 行**；实测
73.83/70.37/63.96/75.10（statements/branches/functions/lines）。前端全量 **1311 passed**；
首屏体积 **173.4 KB gzip** 持平（新文件随 `/nodes` 路由懒加载，不进首屏）。

**范围外（未改，已记入 PR「后续建议」）**：工具栏「远程命令」入口没有选节点的入口
（进去只能被拦下提示「请至少选择一个节点」，存量行为）；批量失败保留上一轮结果与
单节点链路不一致；`useECharts` / `usePolling` 仍 0% 覆盖；`package-lock.json` 未被
Git 跟踪而 CI 跑 `npm ci`（存量）。

### refactor: 项目视图拆分——832 行拆为四层，行操作锁收敛为「行 × 动作」二维锁（Doc/23 T2-4 第六步）

`Doc/23 §3.3 T2-4` / 问题 P1-1。**ProjectView 832 → 230 行（-72%）**，
零新增依赖、零契约变更（同一批 API、同一文案 key、同一禁用条件与确认分支）。

**为什么第六站是它**：前五站（AppStore/Websites/Docker/Backup/Dashboard）交付后
ProjectView（**832 行**）是余下两个里最大的。它的形态与前五站都不同——不是
「一页多页签」、不是「两个业务域同页」、也不是「一条实时流 + 多面板」，而是
**一个列表页 + 两个抽屉 + 两个弹窗，但行操作多达十类**：部署 / 启动 / 停止 /
重启 / 健康探测 / Webhook / 日志 / 联链 / 编辑 / 删除，每一条都带自己的确认分支
或判成败口径。832 行里最容易被「顺手统一」抹平的，正是这些动作彼此不同的语义。

**四层结构**（12 个新文件）：

| 层 | 文件 | 行数 | 职责 |
|----|------|------|------|
| 数据 | `composables/useProjectList.ts` | 120 | 列表 + 客户端搜索（名称/域名）+ 增改 + 运行时候选（同 kind 去重） |
| 数据 | `composables/useProjectLifecycle.ts` | 180 | 六类生命周期动作 + 删除 + Webhook 令牌与轮换（**`actingId` 二维锁**） |
| 数据 | `composables/useProjectLogs.ts` | 61 | 日志**惰性查询**、`loading` 与 `fetching` 分开 |
| 数据 | `composables/useProjectLinks.ts` | 140 | 联链两条查询 + 关联/解除（**per-link 锁**）+ 表单纯净打开 |
| 展示 | `components/project/ProjectListPanel.vue` | 196 | 十类行操作 + 四条禁用条件 + 状态配色 |
| 展示 | `components/project/ProjectFormDialog.vue` | 166 | 创建/编辑（name 置灰、git 才有地址、rolling 才有健康路径） |
| 展示 | `components/project/WebhookDialog.vue` | 60 | 令牌一次性展示（只读 + 复制上抛） |
| 展示 | `components/project/ProjectLogsDrawer.vue` | 96 | 日志三态 + **正文上方常驻重拉入口** |
| 展示 | `components/project/ProjectLinksDrawer.vue` | 172 | 已关联列表（**unknown 染红**）+ 关联表单 |
| 纯函数 | `utils/projectFormat.ts` | 178 | 字典映射、状态配色、**载荷组装（空串归一 null）**、日志拼接、Webhook 绝对地址 |
| 装配 | `views/ProjectView.vue` | **230** | 列表接线 + 四个数据层的开合编排（两抽屉的 reset 配对） |

**四条边界取舍**：

1. **`actingId` 是「行 × 动作」的二维锁，不是行锁**（本站最关键的一条）：同一行挂着
   十个按钮，若按行加锁，用户点「部署」时整行按钮一起转圈。数据层因此用
   `<动作>-<id>` 字符串标签——「锁的粒度」被编码进契约，展示层只需逐一绑定。
   联链抽屉另有一把**独立的** `unlink-<linkId>` 锁（两族刻意不共享）；
2. **成败判定口径按动作分流，不统一**：五类动作按「请求成功即成功」，但健康探测要
   看响应体的 `healthy`（接口 200 而站点不健康是正常返回形态），Webhook 要读回
   令牌明文（仅本次响应有）。三条口径拆分前都是内联实现，拆出后各自独立成函数；
3. **失败不刷新**（统一容错壳 `withActing`）：失败也刷新会用一次额外请求掩盖首因，
   还把错误提示冲散在两次渲染里。原实现即「成功才刷新」，本次固化为契约；
4. **两个抽屉在关闭时清空目标**：日志与联链都是「打开另一个项目」的场景，残留的
   上一份数据会先渲染一屏再被替换（视觉上像「日志变了」）。原实现只在页面卸载时
   收口，本层提到 `watch`；数据层 `open()` 同时清空联链表单（否则旧 `env_name`
   会被静默复用，关联到错误的环境变量名）。

**测试（+113 例，8 个新 spec + 1 个视图 spec）**：`projectFormat` 16（空串归一、
状态配色、去重、载荷边界）、`useProjectList` 11（搜索口径、更新体不带 name、
失败不刷新）、`useProjectLifecycle` 13（失败不刷新、二维锁、healthy=false 报未通过、
Webhook 两条路径）、`useProjectLogs` 5（惰性、换项目打到新 id）、`useProjectLinks` 10
（open 清表单、per-link 锁）、`ProjectListPanel` 14（十类操作、四条禁用条件、二维锁）、
`ProjectDialogs` 14（name 置灰、条件字段、上抛分流）、`ProjectDrawers` 15
（重拉入口、unknown 染红、per-link 只锁一条）、`ProjectView` 15（接线与开合编排）。

**机器可读回归证据**（门禁实测可红）：去掉空串归一 → `projectFormat.spec` **1 例失败**；
健康探测改按请求成功判成败 → **1 例失败**；失败也刷新 → **1 例失败**；去掉抽屉
关闭时的 reset → **1 例失败**；去掉健康检查按钮禁用条件 → **1 例失败**（恢复后全绿）。

**覆盖率地板按阶梯第六次上调（branches 62 → 66 / functions 57 → 60）**：
实测 statements 72.08 / branches 68.32 / functions 62.35 / lines 73.26
（连跑三轮恒定；`lines.total` 3563 → **3845**）。`lines`/`statements` 保持终值 60。

### refactor: 仪表盘视图拆分——851 行拆为五层，三图刷新收敛到单一节流器（Doc/23 T2-4 第五步）

`Doc/23 §3.3 T2-4` / 问题 P1-1。**DashboardView 851 → 206 行（-76%）**，
零新增依赖、零契约变更（同一批接口、同一文案 key、同一排序/截断口径、同一跳转目标）。

**为什么第五站是它**：前四站交付后 DashboardView（**851 行**）是余下三个里最大的。
它是 T2-4 队列里形态最特殊的一个——不是「一页多页签」（AppStore/Docker/Backup），
也不是「两个业务域同页」（Websites），而是**一个页面上并列七块面板，其中三块
共享同一条实时数据流**：四张指标卡与三张图全部由 `/ws/metrics` 推送驱动
（REST 只提供进程表），进程表（10s 轮询）、今日待办、常用应用各自独立。
851 行里最复杂的东西不是某块面板，而是**「谁在什么时候刷新什么」**——
三张图的 1s 节流、后台标签页的补刷、主题切换的补丁式重绘，全埋在视图 setup 的
130 行 onMessage 回调里。

**五层结构**（本站比前四站多一层「编排」，15 个新文件）：

| 层 | 文件 | 行数 | 职责 |
|----|------|------|------|
| 数据 | `composables/useMetricsStream.ts` | 78 | WS 指标流：**init 覆盖 / tick 追加**、**120 点封顶**、**快照原地更新**、最后更新时间 |
| 数据 | `composables/useDashboardProcesses.ts` | 36 | 进程 TOP（10s 轮询，**`immediate: false`**） |
| 数据 | `composables/useDashboardTodos.ts` | 51 | 今日待办（**前 6 条**、勾选后重拉、失败静默） |
| 数据 | `composables/useDashboardApps.ts` | 41 | 常用应用（**`launch_count` 降序前 6**、启动尽力而为，排序在数据层） |
| 编排 | `composables/useChartThrottle.ts` | 67 | 三图**共享**的 1s 节流 + 可见性补刷 |
| 展示 | `components/dashboard/StatCardGrid.vue` | 161 | 四张指标卡（`to` 决定可点击性；跳转上抛） |
| 展示 | `components/dashboard/TrendPanel.vue` | 104 | 性能趋势（三线折线 + WS 徽标） |
| 展示 | `components/dashboard/NetworkPanel.vue` | 98 | 网络 IO（双曲线 + 两个实时读数） |
| 展示 | `components/dashboard/LoadPanel.vue` | 86 | 负载仪表盘 + 数据点数 / 最后更新两行 |
| 展示 | `components/dashboard/ProcessPanel.vue` | 56 | 进程 TOP（4 列，**不传 loading**） |
| 展示 | `components/dashboard/TodoPanel.vue` | 101 | 今日待办（viewer **不渲染**勾选框、骨架/空态互斥） |
| 展示 | `components/dashboard/CommonAppsPanel.vue` | 101 | 常用应用（次数回落 0、viewer 置灰） |
| 纯函数 | `utils/dashboardFormat.ts` | 107 | 指标卡拼装、使用率配色、MB/s 格式化 |
| 纯函数 | `utils/dashboardCharts.ts` | 202 | 三图 option 与主题补丁、调色板读取 |
| 装配 | `views/DashboardView.vue` | **206** | 面板接线 + **三图刷新编排**（共享节流器 → 一次 `refreshKey++`） |

**五条边界取舍**：

1. **三张图不各自 watch 数据，由装配层用一份共享节流器合并刷新**（本站最关键的一条）：
   若每张图各自响应数据变化，每个 WS tick 都会重绘三张图，1s 节流形同虚设
   （后端推送间隔可能 < 1s，低端机上形成长任务尖刺）。现在三图各收一个 `refreshKey`，
   它只由节流器递增——「刷新频率」这件事全站只有一个落点；
2. **option 结构（纯函数）与实例管理（面板）分工**：option 在拆分前完全测不到
   （jsdom 没有画布，ECharts 不绘图）；抽成 `utils/dashboardCharts.ts` 后可逐字段
   断言轴类型/量程/系列配色/数据精度——这些正是改错了也不报错、只会让曲线画歪的地方；
3. **快照原地 `Object.assign` 而不是整体替换**：`snap` 被四张指标卡与网络面板共同持有，
   替换成新对象会让所有已建立的引用指向旧值（表现为数值再也不更新）；
4. **主题切换走补丁不整份重建**：`applyOption` + `notMerge` 只改轴与图例配色——
   整份重建会用首屏空 option 覆盖已积累的数据点，用户切一次主题就丢历史曲线；
5. **进程表的 `immediate: false` 与「不传 loading」**：前者避免首屏双请求；
   后者避免 10s 轮询每次都把表格闪成骨架屏，保持「就地更新」的观感。

**测试（+108 例，6 个新 spec）**：`useMetricsStream` 9（init/tick 语义、封顶、引用不变）、
`useChartThrottle` 10（节流合并、可见性补刷、cancel）、`useDashboardPanels` 16
（截断 / 排序 / 重拉 / 失败静默）、`DashboardPanels` 19（卡片可点击性、列格式化、
viewer 只读、空态）、`DashboardCharts` 13（重绘时机、补丁不重建）、
`dashboardFormat` 15（阈值边界、负载封顶）、`dashboardCharts` 17（轴/系列/精度）、
`DashboardView` 10（接线与编排）。重点覆盖：**连续 tick 只递增一次 `refreshKey`**、
**1 个数据点不画折线但单值仪表盘照常**、**负载超 10 进度条封顶而数值不截断**、
**网络曲线三位小数**（两位会把轻载压成 0 线）、**主题补丁不含 `series.data`**。

**机器可读回归证据**（门禁实测可红）：删掉历史封顶 → `useMetricsStream.spec`
**1 例失败**；去掉节流（每次 schedule 立即 refresh）→ `useChartThrottle.spec` +
`DashboardView.spec` **7 例失败**；恢复后 20 例全绿。

**覆盖率地板按阶梯第五次上调（branches 58 → 62 / functions 55 → 57）**：
实测 statements 70.48 / branches 66.36 / functions 60.01 / lines 71.68
（`lines.total` 3375 → **3563**）。`lines`/`statements` 保持 §7.15 阶梯终值 60；
`functions` 到 57 留 3 点（余下 ProjectView / NodesView 的 setup 函数仍在排除区）。

**真实浏览器复验**（非静态检查）：真内核（SQLite）+ 真产物 + 真 Chromium，
走完向导 → 深链 `/dashboard` → 四张指标卡真实数值 → WS 徽标「已连接」→
数据点持续累积 → **零控制台错误**。

### refactor: 备份管理视图拆分——905 行拆为四层，覆盖率地板 branches/functions 55/50 → 58/55（Doc/23 T2-4 第四步）

`Doc/23 §3.3 T2-4` / 问题 P1-1。**BackupView 905 → 263 行（-71%）**，
零新增依赖、零契约变更（同一批 API、同一文案 key、同一确认分支与弹窗初值）。

**为什么第四站是它**：AppStoreView（1464）、WebsitesView（1181）、DockerView（1107）
交付后，BackupView（905）是余下五个里最大的。形态与前三站都不同——不是「一页多页签
各自独立」（Docker），也不是「两个业务域同页」（Websites），而是**三个数据族互相引用
却互不重叠**：任务表要拿账号缓存译名字（`target_account_id` → `oss`）、任务弹窗与
云端恢复共用同一份「启用账号」候选、云端恢复又要按账号拉远端对象。三族接口各不相同
（`backups` / `backup-accounts` / `backup-jobs`），但被这些引用串成一张网——
905 行里每一条引用的落点都得靠读代码重建。

**四层结构**（17 个新文件，3 数据 + 6 展示 + 1 纯函数）：

| 层 | 文件 | 职责 |
|----|------|------|
| 数据 | `composables/useBackupLocal.ts` | 本地备份列表、新建、下载（**不刷新**）、恢复/删除、备份域三表**统一失效** |
| 数据 | `composables/useBackupAccounts.ts` | 账号列表、保存（**编辑留空 = 不改配置**）、开关（**成败都刷新**）、连通性测试、远端对象 |
| 数据 | `composables/useBackupJobs.ts` | 任务列表、保存（**panel 范围 scope_key=null**）、开关、立即执行（**取响应体判成败**）、云端恢复（**按 key 分流确认**） |
| 展示 | `components/backup/LocalBackupPanel.vue` | 本地备份表（4 列，行操作按**文件名**上抛） |
| 展示 | `components/backup/BackupAccountPanel.vue` | 账号表（6 列，测试 loading **只锁当前行**） |
| 展示 | `components/backup/BackupJobPanel.vue` | 任务表（9 列，**跨表译名** + 账号被删回落 `#id`） |
| 展示 | `components/backup/RestoreCloudDialog.vue` | 云端恢复（**不可还原对象照样列出但按钮禁用**） |
| 展示 | `components/backup/BackupAccountDialog.vue` | 账号弹窗（**编辑态留空 = 不修改**，红字与提示互斥） |
| 展示 | `components/backup/BackupJobDialog.vue` | 任务弹窗（**app 范围才渲染「应用 Key」**，四项校验齐发） |
| 纯函数 | `utils/backupFormat.ts` | 状态映射、范围标签、可还原判定、账号候选与跨表译名 |
| 装配 | `views/BackupView.vue` | 页签骨架 + 工具栏分流 + composable ↔ 组件接线 + **跨族引用传递**（**263 行**） |

**四条边界取舍**：

1. **三族不合并成一个大 composable，引用关系显式化**——账号候选由装配层从
   `useBackupAccounts` 取出后传给另外两个消费点；合并会让「改任务逻辑碰到账号请求」
   成为常态，分开后三族互不牵连，而「谁依赖谁」在装配层一眼可读；
2. **`scope_key` 只在 app 范围渲染（不是渲染后禁用）**——面板库备份没有「应用 key」
   这个概念；数据层上报时非 app 范围**显式 `null`**（空串会被当成名为空的应用 key）；
3. **「立即执行 / 连通性测试」取响应体判成败，而非看 HTTP 状态**——接口 200 而
   `last_status = 'failed'` 是这两条链路的常见形态；按「请求成功即成功」提示会让
   用户以为已经备好了；
4. **云端恢复的「不可还原对象照样列出」**——远端桶里可能有手工上传的任意文件，
   过滤掉会让用户以为「备份丢了」；列出 + 按钮禁用才是正确的可见性，
   判据与后端受理范围同源（`.db` 与 `app/*.tar.gz` **两道**条件）。

**测试（+112 例，7 个新 spec）**：数据层 40 例（`useBackup{Local,Accounts,Jobs}`）、
展示层 41 例（三张表 + 三弹窗）、纯函数 20 例、视图装配 11 例。重点覆盖：
编辑留空上报 `null`、开关失败也刷新、接口 200 但测试失败走错误提示、切换账号清空
对象清单、`scope_key` 的 null 归一、立即执行取响应体判成败、云端恢复两条确认文案、
三张表的行操作载荷形状、loading 只锁当前行、跨表译名与账号被删回落、可还原判据
两道条件缺一不可。

**覆盖率地板按阶梯上调（branches 55 → 58 / functions 50 → 55）**：实测 statements 68.92 /
branches 64.97 / functions 58.27 / lines 70.10（`lines.total` 3032 → **3375**）。
`lines`/`statements` 保持 §7.15 阶梯终值 60；`functions` 到 55 留 3.3 点
（余下 DashboardView 的 setup 函数仍在排除区，按实测 58.3 设阈值只剩 0.3 点余量）。

**真实浏览器复验**（非静态检查）：真内核（SQLite）+ 真产物 + 真 Chromium，走完向导 →
深链 `/backups` → 三个页签逐个切换 → 打开三个弹窗 → 切任务范围为「应用目录」触发
条件字段 → 空表单点保存出现红字 → **零控制台错误**。

**范围外（未改，已记入 PR「后续建议」）**：`retention = 0`（后端约定「用默认 7」）
在弹窗中原样展示为 0（原实现如此，已固化为契约）；本地备份表无搜索/分页（原实现即无）；
`useApiQuery` 的 `retry: undefined` 绕过全局 `retry: 2`（第二步已记录）；DashboardView
（851）、ProjectView（832）、NodesView（824）未拆——T2-4 的余下工作。

---

### refactor: Docker 管理视图拆分——1107 行拆为四层，覆盖率地板 50 → 60（Doc/23 T2-4 第三步）

`Doc/23 §3.3 T2-4` / 问题 P1-1。**DockerView 1107 → 346 行（-69%）**，
零新增依赖、零契约变更（同一批 API、同一文案 key、同一确认分支与禁用条件）。

**为什么第三站是它**：AppStoreView（1464）与 WebsitesView（1181）交付后，DockerView 是
余下四个里最大的（1107 行）。形态相近（一页多页签）但**页签内部的写操作密度完全不同**——
容器表一张就有**九类行操作**，四类资源各有一条清理路径，三张「简单表」的寻址方式还各不相同
（镜像与网络按 ID、卷按**名字**）。这些差异在 1107 行里全部隐没。

**四层结构**（18 个新文件，5 数据 + 10 展示 + 1 纯函数）：

| 层 | 文件 | 职责 |
|----|------|------|
| 数据 | `composables/useDockerContainers.ts` | 列表 + 三字段搜索、九类行操作（统一容错壳）、重命名、三只只读弹窗数据、清理 |
| 数据 | `composables/useDockerImages.ts` | 列表、拉取（**后端回显优先**）、删除、清理 |
| 数据 | `composables/useDockerNetworks.ts` | 列表、创建、删除、连接、**断开（无连接时本地拦截）**、清理 |
| 数据 | `composables/useDockerVolumes.ts` | 列表、创建、删除（**按名字寻址**）、清理 |
| 数据 | `composables/useDockerCompose.ts` | 项目列表、部署（前置校验）、up/down（**不刷新**） |
| 展示 | `components/docker/ContainerListPanel.vue` | 容器表（5 列 / 10 个动作按钮，禁用条件即状态机） |
| 展示 | `components/docker/ImageListPanel.vue` | 镜像表（标签列**两级回落**：tags → repo_tags → ID） |
| 展示 | `components/docker/NetworkListPanel.vue` | 网络表（「已连接容器」列的标签 / 破折号双态） |
| 展示 | `components/docker/VolumeListPanel.vue` | 卷表（名称/驱动/挂载点 + 删除） |
| 展示 | `components/docker/ComposePanel.vue` | 部署表单 + 项目表（两条 up/down 路径互不串扰） |
| 展示 | `components/docker/ContainerLogsDialog.vue` | 日志弹窗（行数四档 + 刷新 + 空态占位） |
| 展示 | `components/docker/ContainerJsonDialog.vue` | 统计 / 详情**共用**的 JSON 预览（两处差异参数化） |
| 展示 | `components/docker/RenameDialog.vue` | 重命名（必填校验 + 回车提交） |
| 展示 | `components/docker/CreateNetworkDialog.vue` | 创建网络（名称/驱动/子网，成功才复位） |
| 展示 | `components/docker/CreateVolumeDialog.vue` | 创建卷（驱动白名单与网络**不同源**） |
| 展示 | `components/docker/ConnectContainerDialog.vue` | 连接容器（必填校验 + 无候选兜底） |
| 纯函数 | `utils/dockerFormat.ts` | 状态映射、三字段搜索、日志档位、两处驱动候选、容器候选 |
| 装配 | `views/DockerView.vue` | 页签骨架 + **惰性加载编排** + composable ↔ 组件接线（**346 行**） |

**四条边界取舍**：

1. **行操作收「行对象」而不是 ID**——投影（`$event.id`）是最容易漏改且改错不报错的东西，
   收行对象后契约只剩「哪一行」一个概念；
2. **统一容错壳 `run()`：失败不刷新**——「失败也刷新」会多打一次请求掩盖失败首因，
   且把错误提示冲散在两次渲染里；
3. **`ContainerJsonDialog` 两弹窗合一**——统计与详情原为两份逐行相同的模板，差异只有
   标题/宽度/空态/背景；合并后差异全部成为显式 prop（`variant` 命名了视觉差异）；
4. **惰性加载编排留在装配层**——「首次切到某页签才加载、切回不重复请求」同时依赖
   页签状态与五个数据层开关，放进 composable 会让数据层知道「页签」这个概念。

**测试（+137 例，7 个新 spec）**：数据层 52 例（五个 `useDocker*`）、展示层 65 例
（6 个组件 + 四弹窗一组 + 两预览弹窗一组）、纯函数 13 例、视图装配 7 例。重点覆盖：
三类状态的**禁用矩阵**、失败不刷新、暂停/恢复双向语义、无已连接容器时本地拦截、
镜像标签两级回落、Compose 两条 up/down 路径、四弹窗的本地必填校验、四页签惰性各一次。

**覆盖率地板按阶梯上调（50 → 60/60/55/50）**：实测 statements 66.2 / branches 62.11 /
functions 55.12 / lines 67.38（分母 3141 → 4271 行）。`lines`/`statements` 达到
Doc/23 §7.15 阶梯终值 60；`branches` 55；`functions` 50（余下两视图 setup 函数仍在
排除区，按实测 55.3 设阈值只剩 0.3 点余量，抖动即误红）。

**范围外（未改，已记入 PR「后续建议」）**：日志弹窗切换行数不自动重取（原语义，已固化
为契约）；创建网络/卷弹窗取消后不清空输入（`closeAndReset` 已把「成功才复位」与
「关闭」分开）；`docker:update` 仅用于重命名，权限粒度细化属产品决策。

### refactor: 网站管理视图拆分——1181 行拆为四层，覆盖率地板 45 → 50（Doc/23 T2-4 第二步）

`Doc/23 §3.3 T2-4` / 问题 P1-1。**WebsitesView 1181 → 334 行（-72%）**，
零新增依赖、零契约变更（同一批 API、同一文案 key、同一确认分支）。

**为什么第二站是它**：AppStoreView（1464 行）交付后，WebsitesView 是余下四个视图里
最大的一个（第二名 DockerView 1107 行），且形态与首站不同——首站是「一页五页签」，
本站是「两个业务域同页」（网站管理 + SSL 证书）。两个域的接口族完全不重叠
（`websites` vs `certificates`/`dnsProviders`），却各自携带全套表格与弹窗挤在同一文件里。

**四层结构**（11 个新文件）：

| 层 | 文件 | 职责 |
|----|------|------|
| 数据 | `composables/useWebsiteList.ts` | 列表查询（分页/keepPrevious）、搜索与状态过滤、增删改、引擎清单懒加载与切换 |
| 数据 | `composables/useWebsiteBinding.ts` | 绑定弹窗两个下拉的数据源、按站点类型择一上报、伪静态追加套用 |
| 数据 | `composables/useWebsiteRewrite.ts` | 伪静态状态拉取/套用/清除、候选清单（接口 supported 优先） |
| 数据 | `composables/useWebsiteCertificates.ts` | 证书列表、三条签发链路、绑定/解绑/删除、续签与 auto_renew、DNS 厂商下拉 |
| 展示 | `components/website/WebsiteListPanel.vue` | 网站表（11 列，五类行内操作全部上抛） |
| 展示 | `components/website/WebsiteDialogs.vue` | 五个网站弹窗（新建/编辑/切换引擎/绑定/伪静态），编辑态本地持有 |
| 展示 | `components/website/WebsiteFormFields.vue` | 新建与编辑共用的字段布局（避免表单漂移） |
| 展示 | `components/website/BindForm.vue` | 绑定三个下拉（运行时/项目/伪静态互斥与联动） |
| 展示 | `components/website/CertificateListPanel.vue` | 证书表（7 列，四类操作上抛） |
| 展示 | `components/website/CertificateDialogs.vue` | 证书四弹窗（ACME/自签/上传/绑定） |
| 纯函数 | `utils/websiteFormat.ts` | 域名解析/时间占位/证书三态/下拉候选四处共用的映射 |

**四条边界取舍**：

1. **两个业务域的数据层彻底分开**——两族接口无交集，分开后「改证书续签不会碰到网站绑定」
   是结构性的，不靠自觉；
2. **归一化下沉到数据层**——绑定目标的 `undefined`（FpSelect 清空）→ `number | null`
   （后端协议）由 `useWebsiteBinding.bind` 按站点类型统一归一，展示层不再理解协议差异，
   且「另一侧必须显式 null」这条后端契约第一次有了机器可读的落点；
3. **弹窗可见性留视图层，「成功才关」也留视图层**——子组件只管编辑，数据层只管请求，
   「失败时保持打开让用户修正输入」是 UI 决策；
4. **`rewriteStatus` 走快照而非 prop 下发**——父级「先 await 拉状态、再开弹窗」，
   走 prop 要等一次渲染调度才更新（时序不确定）；快照 + 写操作后回写语义确定。

**测试（+160 例，10 个新 spec）**：数据层 69 例（`useWebsite{List,Binding,Rewrite,Certificates}`）、
展示层 69 例（4 个组件）、纯函数 14 例、视图装配 8 例。重点覆盖：按站点类型择一上报、
伪静态追加套用的发生时机与顺序、HTTP-01/DNS-01 载荷互斥、两处二次确认、
续签 loading 只锁当前行、备用域名拆分过滤空项、DNS-01 厂商下拉只列启用项。

**覆盖率地板按阶梯上调（45 → 50）**：数据层重回口径后实测 lines 61.84 /
statements 60.66 / branches 58.50 / functions 48.40（分母 2097 → 3141 行）。

**真实浏览器复验**：真内核（SQLite）+ 真产物 + 真 Chromium，走完向导 → `/websites`
→ 两个页签逐个切换 → 打开「创建网站」与「ACME 签发」弹窗 → **零控制台错误**。
同时复验出 **页签标题 `website.title` 上屏裸 key**——§2.5 R-6 存量项，未顺手修改。

---

### refactor: 应用商店视图拆分——1464 行拆为四层，数据层重回覆盖率口径（Doc/23 T2-4 第一步）

`Doc/23 §3.3 T2-4` / 问题 P1-1。**首站 AppStoreView 1464 → 321 行（-78%）**，
零新增依赖、零契约变更（同一批 API、同一文案 key、同一确认分支）。

**为什么第一站是它**：T2-4 四个候选里它最大（第二名 WebsitesView 1181 行）且结构最"全能"——
5 个页签 + 5 个弹窗 + 全部请求编排挤在同一文件。它同时命中 T1-3 留下的结构性问题：
**`src/views/**` 被覆盖率口径排除**，于是该文件里最复杂的业务逻辑（安装状态机、
两处安全二次确认、白名单编辑会话）既是"测试盲区"又是"无人敢动"。拆出的数据层落在
`src/composables/**`（参与统计），114 例新单测由此真正看住这些逻辑。

**四层结构**（10 个新文件，职责边界即本次核心设计）：

| 层 | 文件 | 职责 |
|----|------|------|
| 数据 | `composables/useAppStoreCatalog.ts` | 三类列表、搜索过滤、卸装/升级、原生生命周期、日志、导入 |
| 数据 | `composables/useAppStoreInstall.ts` | 安装状态机（版本参数装配、风险提示、钩子确认、提交载荷） |
| 数据 | `composables/useAppStoreSources.ts` | 远程源 CRUD + 同步 + **两处安全二次确认** |
| 数据 | `composables/useAppStoreSecurity.ts` | 安全配置的服务端状态与读写 |
| 展示 | `components/appStore/AppStoreGrid.vue` | 卡片网格（商店 / WASM 双形态收敛为一份模板） |
| 展示 | `components/appStore/StoreSecurityPanel.vue` | 安全设置（**编辑态本地持有**） |
| 展示 | `components/appStore/AppInstallModal.vue` | 安装弹窗（状态机在 composable，`open()` 经 ref 暴露） |
| 展示 | `components/appStore/SourceDialogs.vue` | 两个源弹窗（**编辑态本地持有**） |
| 展示 | `components/appStore/{InstalledAppsPanel,SourcesPanel}.vue` | 两张表纯装配，操作全部上抛 |
| 纯函数 | `utils/appStoreFormat.ts` | 卡片/表格/下拉三处共用的格式化映射（原先各写一份） |

**三条边界取舍**：

1. **编辑态归展示层，服务端状态归数据层**——白名单是"本地暂存 → 整体提交"的编辑会话
   （后端 `registry_whitelist` 为全量替换语义）。第一版把 `reactive` 表单放 composable 再当
   prop 传下去，立即撞 `vue/no-mutating-props`；该规则指向的正是"对象跨组件共享可变状态"
   这一真实缺陷，故改为编排层收载荷（`emit('save', payload)`）。
2. **弹窗可见性留视图层**——成功才关、失败保持打开让用户修正输入，这是 UI 决策而非请求层职责。
3. **表单初值 `watch` 必须 `immediate: true`**——弹窗可能**带着可见态一起挂载**，
   不立即求值则初值恒为空串（"编辑不回显"），typecheck / lint 都拦不住，由单测抓出。

**测试（+114 例，9 个新 spec）**：数据层 51 例、展示层 55 例、纯函数 8 例。两处工程取舍：

- **虚拟滚动表格用替身**：两张表启用虚拟滚动，而它依赖真实布局尺寸——jsdom 无排版引擎
  （T1-1 已记录），**真表在 jsdom 下一行都不渲染**（实测 tbody 为空）。替身按 DataTable 公开
  契约（`rows` + 列 `body` 插槽逐行求值）实现，单元格逻辑真实覆盖且不伪造布局假设。
- **FpModal 走 Teleport 到 body**：断言落在 `document.body`（wrapper 只有注释占位）；
  另注 `defineModel('visible')` 对应 prop 名是 **`modelValue`**——用错名不报错、只是"弹窗不出现"。

**覆盖率门禁按阶梯上调（40 → 45）**：数据层重回口径后实测 lines **43.66 → 53.98**、
statements 42.66 → 53.04、branches 44.04 → 50.92、functions 31.19 → 40.61
（分母 1665 → 2097 行）。阈值随之调至 **45/45/45/35**（functions 刻意不同幅：视图内函数
仍整体在排除区，一次跳到 45 会把未拆分视图算作"欠账"）。**门禁实测可红**：`lines` 临时
抬到 60 → `ERROR: Coverage for lines (53.98%) does not meet global threshold (60%)`。

**真实浏览器复验**（非静态检查）：真内核（SQLite）+ 真产物 + 真 Chromium，登录 → `/app-store`
→ 五页签逐个切换 → **零控制台错误**。顺带复验出 `nav.appStore` 三语言缺失（页签标题上屏裸
key）——属 R-6 存量项，未顺手修改，留给 T2-5 连同 key 一致性校验脚本一并处理。

**门禁**：flame-kernel `fmt` / `clippy -D warnings` / `cargo test --all` **782 passed**（零后端改动）；
agent 三连全绿；frontend `typecheck` / `lint` 0 error / `test:unit` **564 passed**（基线 450，新增 114）/
`build` / `test:coverage`（45 档）/ `bundle:check`（173.4 KB gzip）；E2E **4 passed**。

### refactor: client.ts 拦截器单源化——两个 axios 实例共用一对工厂（Doc/23 T2-1）

`Doc/23 §3.3 T2-1` / 问题 P1-2。**消除「双份维护、改一处漏一处」的架构债**，
零新增依赖、零行为变更。

**缺口**：`src/api/client.ts` 的 `api` 与 `longApi` 各有一份请求拦截器 + 一份响应拦截器，
四段代码两两**逐行相同**（除重放目标），差别只在注释。双份维护下分化已经开始——
`longApi` 那份漏掉了 `api` 那份的「刷新失败已跳登录」注释。

**修复**：

- 抽出 `attachAuthInterceptor(instance)`——注入 `Authorization` 头；
- 抽出 `attachErrorInterceptor(instance)`——401 → refresh 单飞重放 + 后端统一错误规范化；
- `instance` 参数的**唯一**用途是「401 重放回自己」：`api` 回 `api`、`longApi` 回 `longApi`，
  长操作因此不会在重放时被 15s 默认超时截断（F-33）；
- 两实例仍各自保留（`timeout` 语义相反：15000 vs 0），消除的是**代码重复**而非**实例重复**；
- 请求拦截器读 token 仍走 `localStorage`——本模块被 store 反向依赖（store → api/auth → client），
  import store 会成环（约束同 `persistAuth`）。

**回归保护**（`src/api/__tests__/client.spec.ts`，6 例）：装配数量（防重复注册）、
两实例都注入鉴权头、**重放归属**（关键回归）、`_retried` 幂等、错误规范化行为一致、
工厂可装配到任意新实例。`src/api/client.ts` 行覆盖率 **60.5~63.0% → 93.4%**。

**观测手段的坑（值得留档）**：验证「重放回哪个实例」时，第一版用例用「两个实例装不同
adapter，看谁收到第二次请求」——**测不出来**：`config.adapter` 在 axios `mergeConfig`
中由请求 config 一方优先，重放会沿用原实例 adapter，实例归属不可见（注入回归后仍全绿 =
假绿）。改用「**只给 `api.defaults` 打独有标记头**」后，回归被稳定捕获。

**回归证据**（实测）：

```text
# 把重放目标改回 api（旧行为）——标记头泄漏
AssertionError: expected [ … ] to deeply equal [ … ]
   "auth": "Bearer new-token",
-  "marker": "",
+  "marker": "api",        ← 重放借用了 api 实例
 Test Files  1 failed (1) | Tests  1 failed | 9 passed (10)

# 双重装配请求拦截器
 × api 与 longApi 均装配了恰好一个请求拦截器与一个响应拦截器（不重复注册）
 Tests  1 failed | 9 passed (10)

# 恢复后：10 passed
```

**门禁**：flame-kernel `fmt` / `clippy -D warnings` / `cargo test --all` **782 passed**（零后端改动）；
frontend `typecheck` / `lint` 0 error / `test:unit` **450 passed**（基线 444，新增 6）/
`build` / `test:coverage` / `bundle:check`（173.3 KB gzip）全绿；E2E **4 passed**。

### feat: 覆盖率与首屏体积门禁——两道量化护栏进驻 CI（Doc/23 T1-3）

`Doc/23 §3.2 T1-3` / 问题 P2-2、P2-5。**前端首次具备"退化即红"的量化门禁**，
零新增依赖（不引 size-limit）。

**为什么值得单独一轮**：既有四层测试（单测/集成/E2E）回答的是「跑没跑过」，回答不了
「有没有在悄悄退化」——覆盖率随新代码摊薄、首屏随依赖累积变胖，而这两件事在日常 PR 里
都不可见（没人会为「首屏 +5KB」单独开 Issue）。门禁把「缓慢劣化」转成「当次就红」。

**① 覆盖率门禁**（`frontend/vitest.config.ts` → `coverage.thresholds`）：

- **口径重塑**（P2-2 原状：`include` 只列 7 个文件，等于承认"白名单之外靠人肉"）——
  改为「全量源码减五组排除」，参与行数 **545 → 1689（3.1 倍）**；
- **五组排除**：`src/views/**`（4.5k 行，真实浏览器层已由 E2E 覆盖，可测逻辑将在 T2-4
  拆分时迁入 `composables/` 自动回口径）、`src/api/generated/**`（生成物）、
  `src/locales/**`（静态文案）、`src/main.ts`/`src/App.vue`（引导与根组件）、测试自身；
- **阈值**：全局地板 **40/40/40/28**（实测 42.9~43.3 / 42.0~42.4 / 42.9~43.6 / 30.9~31.2）；另对两处高水位资产
  单独锁 **90/90/84/90**（`components/ui/**`，实测 96.0）与 **95/95/90/95**
  （`directives/**`，实测 100）——全站复用面最大 + 安全边界，不随全局地板松绑；
- **与 Doc/23 原文「lines: 60 起步」的偏差及理由**：60 是按旧白名单口径（实测 77%）拟定的
  数，分母放宽 3.1 倍后 60 会**立即变红**（那就成了"当场关掉门禁"）。故起步取 40，
  按 §7.15 阶梯上调：40 →（T2-4 后）45 → 50 → 60。

**② 首屏体积门禁**（`frontend/scripts/check-bundle-size.mjs`，136 行 + 13 例自测）：

- **口径**：`dist/index.html` 中首屏需加载的**全部资源**（入口脚本 + `modulepreload` +
  样式表）gzip 求和——含 CSS 而不仅是 JS，首屏总量才是用户真实付出的带宽；且 Doc/23 §1.2
  已明确判据是「首屏加载总量」而非单 chunk 名义大小；
- **预算**：`design.md` §11 的 200KB gzip，实测 **173.3KB（JS 156.3 + CSS 17.1）**，余量 26.7KB；
- **三道防假绿**：解析不到资源即失败（防 Vite 输出结构变化后"0 项通过"）、引用文件缺失
  即失败（防产物不完整）、单测夹具用真实临时目录 + 真实 gzip（与 `zlib.gzipSync` 逐字节对齐）。

**CI**（`.github/workflows/ci-cd.yml` 的 `frontend` job）：`Build` → **`Bundle Size Gate`**
→ `Typecheck` → `Unit Tests` → **`Coverage Gate`** → `Lint`（→ `Commitlint` → `npm audit`）。
体积门禁紧跟 Build（量测其 dist/）；覆盖率门禁与单测**刻意分列**（失败归因不同，CI 红时
一眼可辨，代价是重复跑一遍用例 ~13s）。

**顺带的基建修正**：`frontend/eslint.config.js` 此前对 `.mjs` **实际无任何规则生效**
（无匹配的 files 配置 = 漏检；同一段 `no-unused-vars` 代码在 `.ts` 报错、在 `.mjs` 静默通过，
已实测确认）；现补 Node 环境配置段并纳入 `lint-staged`。

**回归证据**（两道门禁都实际红过，不是"配置了就算"）：

```text
$ npm run test:coverage   # 全局地板临时提到 60（即 §3.2 原文数字）
ERROR: Coverage for lines (43.33%) does not meet global threshold (60%)
$ npm run test:coverage   # 组件层阈值临时提到 99
ERROR: Coverage for lines (95.95%) does not meet "src/components/ui/**" threshold (99%)
$ npm run bundle:check --budget=100
❌ 首屏体积超预算：合计 173.3 KB gzip > 预算 100.0 KB（超出 73.3 KB）
```

**手动验证**：`test:coverage` 在 Node 20（CI 版本）与 Node 24（本地）下数值一致，阈值不
随运行时漂移；`bundle:check` 连跑两轮结果稳定；门禁脚本 13 例单测全绿。

### feat: E2E 骨架（Playwright）——三条黄金路径与向导状态缓存缺陷修复（Doc/23 T1-2）

`Doc/23 §3.2 T1-2` / 问题 P2-3。**首次引入端到端测试层**，并借它抓出并修复一个用户可见缺陷。

**为什么需要这一层**：此前测试栈只有 vitest（jsdom + 桩 API）与 Rust 集成测试
（axum-test + **InMemory** 仓储）。两个已交付的 P0 缺陷恰好都落在这两层的双重盲区里——
`#84`（RBAC 种子只在 InMemory 生效，真实 SQLite 下 admin 登录后全站 403）与
`R-1`（FpColumn 包装致全站表格静默空白）都是「单测全绿、真实进程/真实浏览器全崩」。
E2E 跑**真实二进制（SQLite 后端）+ 真实构建产物 + 真实 Chromium**，补上这一层。

**架子**（`frontend/e2e/`，零侵入生产代码）：

- `helpers/backend.ts`：以 `OP_DATABASE_BACKEND=sqlite` 拉起真实内核并轮询 `/health` 就绪；
  工作目录刻意设在 `.e2e-state/`（**不是仓库根**）——内核把 `jwt_secret.key` /
  `bootstrap_token.key` / `secret_encryption.key` 写在 **CWD 相对**的 `./data` 下，
  跑在仓库根会把签名密钥丢进**未被 .gitignore 覆盖**的 `<repo>/data/`，一次 `git add -A` 即泄密；
- `global-setup` / `global-teardown`：每次运行重建全新库（向导只在空库上可走通一次），
  并前置断言 `setup status=in_progress`，避免在错实例上跑出假绿；
- `playwright.config.ts`：`webServer` 把**构建与预览串在同一条命令**（`vite preview` 只服务
  `dist/`，产物陈旧时会测旧代码）；`/api`、`/ws` 经 `VITE_PROXY_TARGET` 反代到该后端；
  串行（`workers: 1`）且**不设重试**（向导用例重试必然二次失败，只会掩盖首因）。

**三条黄金路径**（4 个用例，约 22s）：

| # | 覆盖 |
|---|------|
| ① | 空库首访落向导页 → 六步走完 → **免二次登录直达面板** → 账号落库 → 深链/刷新不被拦回向导 |
| ② | 登录交互 → 四张指标卡渲染真实数值 → **`/ws/metrics` 首帧鉴权与推送链路**（数据点累积）→ 未登录深链被守卫拦到登录页 |
| ③ | 备忘录 CRUD 闭环：新增 → 勾选 → **取消勾选**（C-3 语义）→ 刷新后仍在（证明真落库）→ 确认弹窗删除 → 刷新后消失 |

**顺带修复的缺陷（浏览器实测发现，非静态检查）**：向导提交完成后**用户卡在 `/setup`**。
`useSetupStore` 把初始化状态长期缓存（`statusLoaded`），而 `complete()` 写路径没有同步它——
提交时守卫仍读到向导期间缓存的 `in_progress`，`router.push('/dashboard')` 被判回 `/setup`；
且**刷新不能自愈**（`/setup` 路由对所有访问直接放行、不再探测状态），用户必须手工改 URL。
修复：`complete()` 收到 `completed` 时同步落状态缓存（以**后端响应**为准而非置脏重探，
避免随后复探失败时离线兜底再回落到旧值）。

**机器可读回归证据**：把 `complete()` 的状态回写移除后重跑 →
`src/stores/__tests__/setup.spec.ts` **2 例失败**、E2E 黄金路径①
「空库首访…直达仪表盘」**1 例失败**（停在 `/setup`）；恢复后 431 单测 + 4 E2E 全绿。

**CI**：新增独立 `e2e` job（Rust 工具链 + Node 双栈，构建真实二进制 → `npm ci` →
`playwright install --with-deps chromium` → `npm run build` → `npm run e2e`），
失败时上传 `playwright-report/` 与 `test-results/`（含 trace / 截图）作 artifact；
**不降级为内存后端**——本 job 的存在理由正是覆盖「单测/集成绿、真实进程崩」的盲区。

**门禁**：前端四连全绿（typecheck / lint 0 error / test:unit **431 passed**，新增 3 /
build）；后端三连全绿（fmt / clippy `-D warnings` / test **782 passed**）；零后端改动。

### fix: SQLite 模式 RBAC 种子缺失——admin 登录后全站 403（生产不可用级缺陷）

`Doc/07 §5` / `Doc/03 §3.6–3.8`。P0 缺陷：**修复前 SQLite（生产/默认）模式下面板整体不可用**。

**缺陷**：`run_migrations()` 只建表，`permissions` / `roles` / `role_permissions` 三张表在
SQLite 模式下**没有任何写入路径**——InMemory 模式靠仓储构造器种子，SQLite 模式则完全依赖这三张
表内的数据。于是生产模式下三表恒空：`RoleService::check_permission()` 对任何角色都命中空集合，
admin 登录后所有业务接口一律 `403 Forbidden: Missing permission: <resource>:<action>`，
面板除 `/api/auth/*` 外整体不可用。**向导模式与无人值守模式同样命中**（与初始化路径无关，
种子只存在于 InMemory 仓储构造器）。

**真实复现（修复前，二进制实测）**：

```text
permissions 0 / roles 0 / role_permissions 0
GET /api/users    -> 403 {"message":"Forbidden: Missing permission: user:read"}
GET /api/nodes    -> 403 {"message":"Forbidden: Missing permission: node:read"}
GET /api/websites -> 403 {"message":"Forbidden: Missing permission: website:read"}
（/api/auth/me 正常返回 role: admin —— 排障时极易误判为「前端 CSP 拦了接口」或路由守卫问题）
```

**修复**：新增 `infrastructure::sqlite::seed_rbac(pool)`，在 `RepoFactory::new_sqlite()`
迁移之后、任何服务构造之前**同步**执行（RBAC 未就绪即整体 403，不能像默认设置那样丢给后台任务）。

- `permissions`：以 `(resource, action)` 为业务主键 upsert，描述随版本演进更新；配套幂等迁移
  补建 `UNIQUE INDEX idx_permissions_resource_action`（该表此前无写入路径，存量库必然为空，
  补建索引对历史数据零风险；仍保留一次去重以覆盖手工改过库的实例）。
- `roles`：以 `UNIQUE(name)` upsert；**刻意不写显式 id**（自有 id 可能与既有行撞主键，
  而 `ON CONFLICT(name)` 只覆盖重名冲突），交由 AUTOINCREMENT 分配。
- `role_permissions`：**仅在角色尚无任何映射时**写入默认集合——已存在的映射（后续经角色管理
  定制的授权）一律不动，避免每次启动重置管理员的自定义授权。
- **不做 id 假设**：`role_permissions()` 返回的是「枚举下标 id」，落库前先翻译为
  `(resource, action)` 业务键、再回查库内真实 id——兼容 AUTOINCREMENT 取值与枚举下标错位的库。

**测试**（新增 9 例，两个层级）：

| 层级 | 文件 | 例数 | 覆盖 |
|------|------|------|------|
| 仓储级 | `infrastructure/sqlite.rs::rbac_seed_tests` | 6 | 全量落库 + 角色矩阵逐项正确；重复种子幂等；**不覆盖**自定义收紧的授权；空映射角色补齐；旧库空表升级；脏库重复权限去重（不留孤儿映射） |
| HTTP 级 | `tests/rbac_seed.rs`（新） | 3 | **真实 SQLite 仓储**下的端到端：admin 六个资源域端点 200；角色矩阵判定；重启幂等 |

**机器可读回归证据**：把 `RepoFactory::new_sqlite()` 中的 `seed_rbac()` 调用去掉后重跑 →

```text
sqlite_mode_admin_can_reach_business_endpoints ... FAILED
  /api/users 应放行——修复前三表无种子时此处为 403 Missing permission   left: 403  right: 200
sqlite_mode_role_matrix_enforced ... FAILED   （admin 应有 delete）
sqlite_mode_reseed_on_restart_is_idempotent ... FAILED   left: 0  right: 94
```

3 例失败；恢复后 9 例全绿。集成测试**刻意不复用** `common::setup_router()`——它走 InMemory
仓储、天然带种子，无法覆盖本缺陷，故自建真实 SQLite 组合根。

**手动验证（真实二进制，非静态检查）**：新库启动后 `permissions=94 / roles=3 /
role_permissions=191`（admin 94 / operator 74 / viewer 23），六个资源域端点全部 200；
重启后行数不变且自定义收紧的授权保持（93）；旧库（空 RBAC 表 + 已有用户）升级后自动补齐；
脏库（重复权限行 + 指向重复行的映射）启动不失败。

**门禁**：flame-kernel `fmt` ✅ / `clippy -D warnings` 0 告警 ✅ / `cargo test --all`
**782 passed, 0 failed**（基线 773，新增 9）✅；agent 三连 ✅；前端（零改动）四连
`typecheck` ✅ / `lint` 0 error（6 warning 存量）✅ / `test:unit` **428 passed** ✅ / `build` ✅；
零新增 Cargo / npm 依赖；OpenAPI 快照零差异。

**范围外（未改，已记入 PR「后续建议」）**：同类「InMemory 有种子 / SQLite 无」的还有
`default_firewall_rules()`（SQLite 模式 firewall_rules 表为空，面板不带预置规则，属体验差异
而非功能阻断，未顺手改)；`roles`/`permissions` 目前无 REST 管理端点，前端角色管理 UI 仍待开。

### fix: FpCheckbox change 载荷——取消勾选不再恒传 true，恢复「取消完成」

`Doc/23` §7.13 / §3.6 **C-3**（T1-1「测试先于修复」发现的 3 项缺陷之一，属功能失效；本轮独立小 PR 清账）。

**缺陷**：FpCheckbox 把底层 `change` 事件的载荷直接 `Boolean()` 化，而底层带的是 **DOM Event 对象**
——`Boolean(event)` 恒为 `true`，**取消勾选的信号被静默吞掉**。两个消费点（`MemoList` 的
「标记完成 / 取消完成」、Dashboard 待办的 `toggleTodo`）都用 `@change="(v) => toggle(m, Boolean(v))"`
承载双向语义，于是**勾选后永远无法取消**（用户可见的功能失效，且无任何报错）。

**修复**：`onChange` 以事件源 `input.checked` 为准——浏览器在派发 `change` 之前已先行切换该状态，
读到的即是本次交互后的新值，由此 `change` 与 `update:modelValue` **两条通道同源**。
*为何不读 `model.value`*：`update:modelValue` 先于 `change` 到达，父组件同步回写 props 时
`model.value` 可能仍是旧值；读 DOM 是本次交互的确定事实，与回写时序解耦。

**测试**（新增 4 例，两个层级）：
- 组件级 `FpCheckbox.spec.ts`：原「已知缺陷·锁定现状」断言**翻转为 `change` 载荷为 `false`**；
  补「连续切换 true → false → true 逐次跟随真实状态」并断言两条通道取值一致；
- 视图级 `views/__tests__/MemoList.spec.ts`（新，2 例）：贯穿
  MemoList → FpCheckbox → 底层 input → `updateMemo`，断言取消勾选真实提交 `done: false`，
  另证未完成项勾选仍提交 `done: true`；
- 视图级 `views/__tests__/DashboardView.spec.ts`（新，1 例）：同一缺陷的第二个消费点。

**机器可读回归证据**：把 `onChange` 回退为 `Boolean(v)` 后重跑 → `FpCheckbox.spec` **2 例失败**、
`MemoList.spec` **1 例失败**、`DashboardView.spec` **1 例失败**；恢复修复后 32 例全绿。

**门禁**：前端 `typecheck` ✅ / `lint` 0 error（6 warning 存量，基线持平）✅ /
`test:unit` **428 passed**（基线 424，新增 4）✅ / `build` ✅；`components/ui/` 行覆盖率 **96.1%**；
后端（零改动）`fmt` ✅ / `clippy -D warnings` 0 告警 ✅ / `cargo test --all` **773 passed** ✅；
零新增 Cargo / npm 依赖；OpenAPI 快照零差异。

**范围外（未改，已记入 PR「后续建议」）**：`CopyButton` 失败文案（C-4，组件未被引用）仍待办；
`FpCheckbox` 非 binary 模式（数组）下 `change` 仅给出布尔态、未给出被切换项的值，
全仓暂无该模式消费点。

### fix: v-permission 还原边界——只撤销本指令写过的项，不抹元素自身禁用态

`Doc/23` §7.12 / §3.6 **C-2**（T1-1 与 FpCard 两轮交付时先后探测到的同一项缺陷，本轮独立小 PR 清账）。

**缺陷**：指令的 `restore()` 无条件 `removeAttribute('disabled')` / `removeAttribute('aria-disabled')`，
并把 `opacity` / `pointer-events` / `display` 内联样式**清空**。「放行」（有权限）分支因此会顺手撤销
**元素自身**的状态：`<FpButton :disabled="item.installed" v-permission="…">` 在 admin 下渲染成**可点击**。

**影响面（机器扫描，非估算）**：全仓 **20 处**「`v-permission` + `:disabled` 同元素」组合
（DockerView 6 / ProjectView 5 / AppStoreView 2 / PluginsView 2 / DatabasesView 2 /
RuntimeView 1 / FilesView 1 / BackupView 1）。RuntimeView「安装」按钮的语义最直白：
**已安装的版本仍可重复发起安装**；另有若干元素自带内联样式被一并清空。

**修复**：引入**写入账本**（`state.written`：哪些项是本指令写的）+ **元素自持基线**
（`state.baseline`：写之前元素自己是什么样）。`apply()` 改为两段式——① 撤销「本指令写过、
本次不再需要」的项（还原到基线，原本不存在则移除）；② 写入目标项（首次写入前记录基线）。
**放行**、**denied → allowed**、**manage ↔ view 模式切换**三条路径自然收敛（先还原后写入，
不叠加残留）；元素自身状态与其它指令/父组件的意图一律不碰。

**测试**（新增 7 例，两个层级）：
- 指令级 `permission.spec.ts`（+6）：自身 `disabled` 放行时不抹除；`denied → allowed` 后仍在；
  原本未禁用者不残留指令写入的 `disabled`；内联 `opacity` / `aria-disabled` / `display`
  **按原值恢复**（而非清空）；
- 视图级 `RuntimeView.spec.ts`（+1）：已安装版本的「安装」按钮 **DOM `disabled` 属性态**为 `true`
  ——直接验证真实可交互性，而非仅属性存在。

**机器可读回归证据**：回退 `permission.ts` 为原实现后重跑 → `permission.spec` **5 例失败**、
`RuntimeView.spec`「安装按钮保持 disabled」**1 例失败**；恢复修复后 28 例全绿。

**门禁**：前端 `typecheck` ✅ / `lint` 0 error（6 warning 存量，基线持平）✅ /
`test:unit` **424 passed**（基线 417，新增 7）✅ / `build` ✅；
后端（零改动）`fmt` ✅ / `clippy -D warnings` 0 告警 ✅ / `cargo test --all` **773 passed** ✅；
零新增 Cargo / npm 依赖；OpenAPI 快照零差异。

**范围外（未改，已记入 PR「后续建议」）**：`FpCheckbox` change 载荷恒为 `true`（C-3）与
`CopyButton` 失败文案走错参数位（C-4）仍待办；本指令的写入账本未覆盖元素自身 `v-show` /
CSS 类级隐藏（非内联样式）。

### fix: FpCard 默认插槽内容静默丢弃——卡片主体转发至 content 命名插槽

`Doc/23` §7.10（T1-1 组件测试「测试先于修复」发现的 3 项缺陷之一，优先级最高的一项）。

**缺陷**：底层 `openvue/card` 的模板只渲染命名插槽 `content` 进 `p-card-content`，
**没有默认插槽出口**；而 FpCard 此前把 `<slot />` 直接写在 Card 标签下——它落进
Card 的默认插槽后无处渲染，**内容被静默丢弃且不抛任何错误**。影响 `RuntimeView`：
卡片的安装路径与「安装 / 设为默认 / 卸载」按钮整体不可见，用户看到的是空卡片。

**修复**：FpCard 内改为 `<template #content><slot /></template>` 显式转发，调用方
继续使用默认插槽写法（对消费方零改动）；并在模板中写明根因，防止回退。

**同类根因扫描**：对全部 32 个 Fp* 封装逐个体检「`<Comp><slot /></Comp>` + 底层有无默认
插槽出口」——FpCard 是**唯一**命中项，其余封装（Button / InlineMessage / Drawer /
Modal / Table / RadioGroup / Tabs 等）均安全。

**测试**（新增 5 例，两个层级）：
- `FpCard.spec.ts`：原「已知缺陷·锁定现状」断言**翻转为内容可见**，并补「正文与标题 /
  extra / footer 同屏不串位」「插槽缺省时 content 区域仍渲染」；
- `views/__tests__/RuntimeView.spec.ts`（新）：**视图级贯穿回归**——RuntimeView → FpCard
  → `p-card-content` → 卡片主体（安装路径 + 操作按钮）真实落到内容区，覆盖组件级断言
  够不到的「消费方渲染结果」层。

**机器可读回归证据**：回退 FpCard 模板为 `<slot />` 后重跑 → `FpCard.spec` **2 例失败**
（内容可见 / 不串位）、`RuntimeView.spec` 卡片主体与按钮断言失败；恢复后 10 例全绿。

**门禁**：前端 `typecheck` ✅ / `lint` 0 error（6 warning 存量，基线持平）✅ /
`test:unit` **417 passed**（基线 412，新增 5）✅ / `build` ✅；
后端（零改动）`fmt` ✅ / `clippy -D warnings` 0 告警 ✅ / `cargo test --all` 全绿 ✅；
零新增 Cargo / npm 依赖；OpenAPI 快照零差异。

**范围外（未改，已记入 PR「后续建议」）**：探测到 `v-permission` 指令的 `restore()`
会无条件 `removeAttribute('disabled')`——把**元素自身**的禁用态一并抹掉（如 RuntimeView
已安装版本的「安装」按钮，生产环境同样有此问题）；T1-1 记录的另两项缺陷
（`FpCheckbox` 的 change 载荷、`CopyButton` 失败文案）与覆盖率阈值门禁仍未处理。

### feat: 组件测试安全网——22 个 Fp* 组件与 hooks 的机器可读契约（Doc/23 T1-1）

`Doc/23` §3.2 的 **T1-1**（问题 P2-1，「组件级测试几近为零」）——75 个 Vue 组件此前
无任何回归保护，T2-4 巨型视图拆分因此无法安全开工。本轮把安全网建起来。

**基建（真正的拦路虎不是 @vue/test-utils，而是 jsdom 挂不起来）**

Fp* 组件的底层 OpenVue 实现硬依赖三类 jsdom 未实现的能力：`$primevue`（config 插件，
读主题/语言包/aria 文案）、`window.matchMedia`（Select 弹层方向）、`ResizeObserver` 与
`IntersectionObserver`（TabList 溢出检测、Datatable 虚拟滚动）。缺失时组件**渲染期即抛
TypeError**，测试在「挂载」这一步就失败。

- 新增 `frontend/src/test/setup.ts`：为三者提供**最小可用桩**（只保证存在且可调用，
  不模拟布局计算——jsdom 无排版引擎，强行模拟只会制造假绿；真实尺寸断言归 T1-2 E2E）；
- 新增 `frontend/src/components/ui/__tests__/helpers.ts`：`mountFp()` 统一挂载入口，
  默认注入**与 `main.ts` 同源**的 OpenVue config（含 flamePreset 品牌主题）+ Confirmation /
  Toast 服务 + 测试 i18n；`createHost()` 为 Teleport 浮层提供宿主。

**测试交付（22 spec / 180 例，覆盖 22 个组件 + 2 个 hooks）**

任务单原定 10 个核心组件，实际一次做全：表格（`FpTable` 12 例，含 R-1 列收集同一性护栏与
受控分页）、浮层（`FpModal`/`FpDrawer` 13 例，Teleport 宿主 + 插槽装配 + 关闭事件）、
表单字段族（10 个组件 86 例，字段态三件套：视觉非法态 + `aria-invalid` +
`aria-describedby` 指向真实节点；label 与控件 id 关联；空值归一）、反馈族（4 个 34 例，
消息「新顶旧」单例契约、确认弹窗危险色默认、role 语义）、结构族（10 个 45 例，severity 逐档
映射、插槽装配、图标源唯一）、页面骨架（3 个 20 例，四插槽分工、刷新防重复点击、剪贴板双路径）。

**覆盖率**：`vitest.config.ts` 的 `coverage.include` 扩入 `src/components/ui/**`，
实测行覆盖率 **95.4%**（188/197 行，验收线 70%）；`__tests__/` 经 `coverage.exclude` 排除。

**「测试先于修复」发现 3 项真实缺陷**（均未越范围修改，以「已知缺陷·本轮记录，未修」注释 +
断言锁定现状，修复时需同步翻转断言）：

1. **`FpCard` 默认插槽内容被静默丢弃**——底层 `openvue/card` 只渲染命名插槽 `content`，
   而 `<slot />` 落进默认插槽 → 无处渲染。已确认影响 `RuntimeView`：卡片主体（安装路径 +
   安装/设为默认/卸载按钮）整体不可见且无任何报错；
2. **`FpCheckbox` 的 `change` 载荷恒为 `true`**——底层事件带 DOM Event 对象，
   `Boolean(event)` 恒真，取消勾选信号丢失；`MemoList` 的「取消完成」因此失效；
3. **`CopyButton` 失败文案不精确**——把文案传给了 `toast.error(err, fallback)` 的 err 参数位，
   回落默认「操作失败」（有反馈、非静默，仅文案不准）。

**门禁**：前端 `typecheck` ✅ / `lint` 0 error（6 warning 存量，基线持平）✅ /
`test:unit` **412 passed**（基线 232，新增 180）✅ / `build` ✅；
后端（零改动）`fmt` ✅ / `clippy -D warnings` 0 告警 ✅ / `cargo test --all` **773 passed** ✅；
agent 三连 ✅；零新增 Cargo / npm 依赖。

**范围外**：3 项缺陷修复（建议各自独立小 PR，优先于 T2-4）、`CopyButton` 未登记进
`components/ui/README.md`、覆盖率尚无阈值门禁（属 T1-3）。

### feat: 内容安全策略基线——三处同源下发与首屏脚本哈希白名单（Doc/23 T0-5）

`Doc/23` §3.1 的 **T0-5**（问题 P0-1、P0-5）——方案 A 中的 **CSP 部分**。此前全站没有
任何 CSP（`index.html` 无 CSP meta、`nginx.conf` 未下发 `Content-Security-Policy`），
而 access token 存于 `localStorage`（方案 A 决定保留），二者叠加即「一次 XSS = 令牌被读走」。
`script-src` 去掉 `unsafe-inline` 是切断该链路的前端唯一手段。

**三处同源下发（改一处必须三处同步）**

| 位置 | 作用 |
|------|------|
| `frontend/index.html` | `<meta http-equiv="Content-Security-Policy">`——开发 / 直出 dist 场景 |
| `nginx.conf` | 响应头——Docker 镜像内 nginx（生产主路径） |
| `install.sh` | `CSP_POLICY` 变量 + 各 server/location 引用——systemd 裸机安装 |

**策略**：`default-src 'self'`；`script-src 'self'` + 首屏防闪烁脚本 sha256（**禁
`unsafe-inline`/`unsafe-eval`**）；`style-src 'self' 'unsafe-inline'`（Vue `:style` 与 UI 库
运行时注入需要）；`img-src` 含 `data:`·`blob:`（主题背景 / 导出预览）；`font-src` 含 `data:`；
`connect-src 'self'`（同源 WS 亦被覆盖，已实测）；`object-src 'none'` / `base-uri 'self'` /
`form-action 'self'` / `frame-ancestors 'none'`（**仅 HTTP 头**）。

**四个实测踩坑点**

1. CSP 哈希源**必须单引号包裹**——无引号时构建/类型检查/lint 全绿，只有浏览器静默拦掉脚本；
2. HTML 解析器**丢弃 `<script>` 后的首个换行**，按文件原文算哈希与浏览器可能不同；
3. `frame-ancestors` 写在 meta 中**会被忽略**（Chrome 明确告警），只写 nginx；
4. nginx `add_header` **继承规则**：下级 `location` 一旦自带 `add_header`，上层全部头不再继承
   ——`location = /index.html` 与 `location /assets/` 必须各自重复声明 CSP，否则静默消失。

**验证（真实链路，非静态检查）**：真实 nginx + 真实后端 + 真实 Chromium——curl 三路径均返回
CSP 头；跑通 Setup 向导与登录页 **0 违规**；CDP 驱动真实登录后遍历 8 个重页面（dashboard 出
3 个 ECharts canvas、docker 5 张表、terminal 出 xterm）**全程 0 违规**；负面控制注入无 hash
内联脚本被浏览器拦截；`install.sh` 渲染后的配置 `nginx -t` 通过且 80/443 均实测返头。

**未改项（有据，非省略）**：方案 A 的「缩短 access token 寿命」未执行——实测
`DEFAULT_ACCESS_TTL_MINUTES` 已是 **15 分钟**（refresh 24h 滑动过期），继续缩短会显著抬高
刷新频率与 401 重放压力，边际收益低于改造成本。

**回归保护**：新增 `src/config/__tests__/csp.spec.ts`（17 例，5 个断言分组）。六项独立回退
实测全部被捕获（改脚本不改哈希 2 例、去引号 3 例、加回 `unsafe-inline` 2 例、漏 `/assets/`
声明 2 例、install 与 nginx 分叉 1 例、meta 写 `frame-ancestors` 2 例），还原后全绿。
前端门禁：typecheck ✅ / lint 0 error（6 warning 存量，基线持平）/ test:unit **232 passed**
（基线 215）/ build ✅。

### feat: 应用商店描述国际化——双语字段按语言取值与双语搜索（Doc/23 T0-4）

`Doc/23` §3.1 的 **T0-4**（问题 P0-6）：商店卡片把后端双语字段的中文项直接绑进模板
（`{{ pkg.short_desc_zh }}`，3 处），搜索也只匹配中文字段——en/ja 界面下应用描述一律
中文，i18n 在该模块实质失效；英文关键词搜不到应用。

**实现（取值与搜索收敛到单一工具 + 响应式 composable）**

- **`src/utils/localeField.ts`**（新，纯函数）：`toLangTag`（locale 归一）、
  `pickLocalizedText`（取值链）、`matchesLocalizedQuery`（双语搜索）；不依赖 vue/i18n，
  可单测；
- **`src/composables/useLocalizedDesc.ts`**（新）：响应式包装——函数内读
  `useI18n().locale`，模板 / `computed` 调用即自动订阅语言变更；
- **`src/views/AppStoreView.vue`**：3 处直绑改 `localizedDesc(pkg)`，搜索改
  `matchesQuery(p, search.value)`。

**取值链（任务单约定：缺失回退 zh）**

| 界面语言 | 主字段 | 兜底 |
|----------|--------|------|
| zh-CN / zh-TW | `short_desc_zh` | `short_desc_en`（历史/第三方源反向兜底） |
| en-US / en | `short_desc_en` | `short_desc_zh` |
| ja-JP 等（后端无专属字段） | `short_desc_zh` | `short_desc_en` |

空串/纯空白一律视为缺失——后端 `short_desc_en: Option<String>` 与 1Panel / 宝塔适配器
（`short_desc_en: None`）都会产出该形态；若不判空，en 界面卡片描述会渲染为空白。

**响应式（本任务的隐性要点）**：取值函数读取 `locale.value`，因此**切换语言后已渲染的
卡片描述同步更新**，无需重新挂载或刷新——与 T0-2「指令响应式化」属同一类问题
（历史写法把「取值」与「渲染时机」解耦失败）。

**为什么不做成全站通用 i18n 方案**：`short_desc_*` 是**后端随数据下发的双语字段**，
不是前端词条，不适用 `locales/*`（那是静态文案）。二者边界清晰：静态文案走 i18n key，
数据侧双语走本工具。

**回归保护**：新增 3 spec / 33 例——`utils/__tests__/localeField.spec.ts`（22，含
**源码护栏**「视图不得再直绑 `*_zh`」）、`composables/__tests__/useLocalizedDesc.spec.ts`
（7，含切换语言后已渲染文本更新）、`views/__tests__/AppStoreView.spec.ts`（4，视图级
zh/en/ja 与切语言）。
**修复前实测**（回退模板直绑）→ `AppStoreView.spec` **2 例失败**、`localeField.spec`
护栏 **2 例失败**，恢复后 33 例全绿。
前端门禁：typecheck ✅ / lint 0 error（6 warning 存量，基线持平）/ test:unit
**215 passed**（基线 182）/ build ✅；后端 773 passed、agent 438 passed（两端零改动）。

### fix: 图标源本地化——移除 presetIcons 的 esm.sh CDN 外联（Doc/23 T0-3）

`Doc/23` §3.1 的 **T0-3**（问题 P0-4）：`uno.config.ts` 启用了
`presetIcons({ cdn: 'https://esm.sh/' })`。审计原文将其记为「无外网环境下**运行时**
图标加载失败」，**实测更严重**：只要源码出现 `i-*` 图标类，构建期就会去 esm.sh 抓取
图标集 JSON——**断外网环境下 `npm run build` 直接失败**
（`FetchError: [GET] https://esm.sh/@iconify-json/mdi/icons.json`，
`ECONNREFUSED`）。这不只是可用性问题，而是把第三方 CDN 变成了**构建期硬依赖**。

**为什么直接整体移除（而非改挂本地 `@iconify-json/*`）**

1. 全站图标唯一来源是 `@openvue/openicons` 字体图标（`oi-*`，随包本地分发），
   源码中 `i-*` 图标类使用量为 **0**（对 `src/` 全量扫描确认）；
2. presetIcons 在本仓无任何实际产出，启用它只会平白引入一条远程抓取路径。

故删除预设并**零新增依赖**；注释中写明「将来若确需图标集，装 `@iconify-json/*`
本地包后再启用，禁止重新挂任何远程 CDN」。

**断网三组对照实测**

| 组 | 条件 | 结果 |
|----|------|------|
| EXP-A | 修复前 + esm.sh 可达 + `i-*` 探针 | 构建成功（图标内联进产物 CSS） |
| EXP-B | 修复前 + esm.sh 不可达 + 同一探针 | ❌ **构建失败**（`FetchError`，`ECONNREFUSED`） |
| EXP-C | 修复后 + esm.sh 不可达 + 同一探针 | ✅ **构建成功（exit=0）**，产物无 `esm.sh` 痕迹，字体资产本地打包 |

**回归保护**：`src/config/__tests__/iconSource.spec.ts`（4 例）——presets 不含
`@unocss/preset-icons`、`i-*` 不再产出图标规则、源码无 `i-*` 混用、源码与配置无远程 CDN 引用。
**修复前实测**（回退配置）→ **3 例失败**，恢复后 4 例全绿。
前端门禁：typecheck ✅ / lint 0 error（6 warning 存量，基线持平）/ test:unit
**182 passed**（基线 178）/ build ✅；后端与 agent 零改动。

### feat: 权限指令响应式化与角色单一来源——存储变更同文档广播（Doc/23 T0-2）

`Doc/23` §3.1 的 **T0-2**（问题 P0-3、P1-5）：`v-permission` 指令自行
`localStorage.getItem(STORAGE_KEYS.role)` 取角色，且只在 `mounted`/`updated` 判定。

**两条缺口**

1. **读取双轨**——绕过 auth store（P1-5 存储双轨之一）：store 侧经 `/auth/me` 同步的
   角色、登出清空等状态变化对指令不可见；本地存储被篡改即可解锁按钮（观感层被绕过）；
2. **非响应式**——`updated` 的触发条件是「父组件重渲染」，与角色变更无关；切换账号
   或角色下发后，**已挂载的按钮**仍按旧角色渲染，只有重新挂载才生效。

**实现**

- **角色单一来源**：指令判定一律取自 `useAuthStore().role`；指令钩子在组件树外运行
  （`inject` 不可用），经 `getActivePinia()` 取应用级 Pinia；无 Pinia → 空角色
  （**fail-closed**，宁可少给按钮不可越权放行）；
- **响应式重算**：每个绑定元素持 `effectScope` + `watchEffect`，effect 内同时读
  「绑定值 + store 角色」两个依赖，任一变化即幂等重算；`unmounted` 停止作用域；
- **绑定值变化**：`updated` 只把新绑定值写回 `shallowRef`，统一由 effect 重算，
  避免出现两条应用路径；
- **存储单源桥接**：新增 `utils/storage.ts::writeStorage(key, value)`——写盘 + 广播
  同文档变更；`api/client.ts` 的 401 刷新回写与凭证清空改经它执行。

**为什么需要 `writeStorage`（关键取舍）**：`useStorage` 只监听原生 `storage` 事件，
而该事件**仅在其它文档**触发——同文档直写不通知任何订阅者。而 `api/client.ts` 被
store 反向依赖（store → api/auth → client），不能 import store（成环），只能直写存储。
不桥接则「刷新拿到新角色 → 存储已是新值、store 仍是旧值」，指令永远看不到新角色——
**统一来源只做一半等于没做**。

**回归保护**：`directives/__tests__/permission.spec.ts`（18 例）、
`utils/__tests__/storage.spec.ts`（6 例）、`api/__tests__/client.spec.ts`（4 例）。
**修复前实测**（三处独立回退）：指令回退为裸读 localStorage + 仅钩子判定 → **6 例失败**；
`client.ts` 回退为直写 → **2 例失败**；移除 `updated` 写回 → **1 例失败**。
前端门禁：typecheck ✅ / lint 0 error（6 warning 存量，基线持平）/ test:unit
**178 passed**（基线 160）/ build ✅；后端与 agent 零改动。

### feat: 路由级 RBAC 守卫——页面级权限判定与菜单/⌘K 同源（Doc/23 T0-1）

`Doc/23` §3.1 的 **T0-1**（问题 P0-2）：`beforeEach` 只查登录态，`isMenuHiddenForRole`
又只作用于侧栏——viewer 直接在地址栏输入 `/users`、`/terminal` 即可进入页面（API 虽
403，但页面框架照常渲染，构成观感越权）；⌘K 命令面板同样列出无权限入口。

**实现**

- **页面级权限点**：`menuRoutes` meta 新增 `perm`（19 条路由，与后端
  `api/permissions.rs` 的 `resource:action` 字面对齐）；
- **判据同源**：新增 `hasRoutePermission`（守卫）与 `isRouteVisibleInMenu`
  （侧栏 / ⌘K）——后者在前者之上叠加**既有**角色级菜单规则 `isMenuHiddenForRole`。
  菜单是「主动推荐入口」，比「URL 可直达」更保守；反向绝不允许（否则形成死路）；
- **守卫**：`routerGuard` 拆为具名导出 + 新增 `checkPagePermission`（无权限重定向
  `/dashboard`，已在目标页时放行防自环）；
- **提示通道**：守卫在组件树外运行（`useFpToast` 的 `inject` 会失败），且刷新时先于
  挂载执行——新增 `utils/notice.ts`（pub/sub + 队列），无订阅者时暂存、订阅后补投；
  消息只携带 i18n key，由 `App.vue` 经 `useSafeText` 取词（语言包未就绪不上屏裸 key）；
- **菜单一致性**：viewer 新增隐藏 `/terminal`（写能力面，否则「菜单可见 → 点入被弹回」）；
- 三语言补 `common.noPermission`；⌘K `buildNavigationCommands` 按角色过滤。

**刻意保留的语义边界**（收紧会造成可用性事故，已在代码注释登记）

1. 无 `meta.perm` → 放行：`/dashboard` 是守卫兜底重定向目标（自判死循环）、`/settings`
   含仅需登录的自助改密、`/operation-logs` 后端对三种内置角色都放行 read；
2. 未知/自定义角色与空角色 → 放行：前端矩阵只能表达 admin/operator/viewer，误锁
   （整站不可用）代价远高于观感收益。

**前端守卫不是安全边界**——真正的拦截在每个 API 的服务端 RBAC（Doc/07 §3，默认拒绝）。

**回归保护**：新增 3 spec / 25 用例——`router/__tests__/rbacGuard.spec.ts`（9，真实 router
驱动导航）、`directives/__tests__/routePermission.spec.ts`（12，含「菜单可见 ⇒ 守卫放行」
遍历断言与 perm 覆盖度校验）、`utils/__tests__/notice.spec.ts`（4）。**修复前实测**：
临时回退守卫判定后 rbacGuard **3 例失败**（viewer `/users`、viewer `/terminal`、
operator `/users` 均未被拦）。前端门禁：typecheck ✅ / lint 0 error / test:unit
**160 passed**（基线 135）/ build ✅；后端 773 passed（零后端改动）。

### fix: P0 兜底链修复——错误边界下沉与首错可见（Doc/23 R-2/R-3）

`Doc/23` §2.5 记录的两个 P0 运行期缺陷：错误边界**粒度**错误（页面一崩就丢整个
Layout），以及致命错误兜底自身**渲染空白**；`monitor` 则把首次错误静默吞掉。

**R-3：`monitor.ts` 首错静默**

- **现象**：`reportError` 把「登记去重表」与「输出」写进同一条提前 `return` 分支——
  每个 `source+message` 的**第一次**错误永不输出，只有第二次起才可见；而首错恰恰是
  排障最需要的一条（后续重复只是它的回声）。
- **修复**：首错立即登记**并输出**，之后相同错误才在 30s 窗口内节流（累计计数在窗口外
  重放时透出）。
- **回归保护**：`src/utils/__tests__/monitor.spec.ts`（6 例）。**修复前 6 例全败**。

**R-2：错误边界卸载整个 Layout + FpStatePanel 渲染空白**

- **现象**：`App.vue` 的 `onErrorCaptured` 挂在顶层并整棵替换 `router-view`——任一页面
  组件崩溃即丢失侧栏/顶栏/页签，用户失去全部自救入口；且兜底面板传 `title`/`description`
  而**不传 `error`**，而 `FpStatePanel` 当时只认 `error`，三态全不命中 → 渲染**空白**。
  再叠加语言包未加载时 `t('common.failed')` 上屏裸 key，兜底彻底失效。
- **根因（两条独立缺陷）**：① 边界**位置**错了——错误边界应贴近被保护对象（页面），
  而非整棵树；② `FpStatePanel` 的错误态判定依赖单一 prop，且文案直接取 i18n，
  没有「i18n 不可用」的降级路径。
- **修复**：
  - 新增 `components/ui/FpErrorBoundary.vue`——可复用边界（`resetKey` 复位、`report`
    开关、`return false` 阻断重复上报）；**零第三方依赖**（面板用原生 `<button>` + 令牌
    自绘：兜底层必须比它保护的对象更稳，若崩溃源是 beta 依赖则依赖它的面板会同归于尽）；
  - **边界下沉**：`App.vue` 顶层仅兜 Layout/路由层致命错误；`Layout.vue` 内容区包
    `router-view`，**页面崩溃保留导航骨架**（用户可切菜单自救）；
  - `FpStatePanel` 错误态判定改为 `error || title`（兼容「只传标题」的历史用法），
    文案改经 `useSafeText` 取词；
  - 新增 `composables/useSafeText.ts` + `utils/fallbackText.ts`——三语言硬编码兜底文案
    （语言优先级：已保存语言 → 浏览器语言 → 英文），i18n 未就绪时绝不透出裸 key。
- **回归保护**：`ui/__tests__/FpErrorBoundary.spec.ts`（7）、`ui/__tests__/FpStatePanel.spec.ts`
  （7）、`utils/__tests__/fallbackText.spec.ts`（5）、`components/__tests__/Layout.spec.ts`
  （3，锁定「页面崩溃保留骨架」这一验收点）。**修复前 `FpStatePanel` 4 例、`Layout` 2 例失败**。
- **文案新增**：`locales/*` 三语言补 `common.renderError.title` / `common.renderError.reload`。
- **文档同步**：`Doc/23` §2.5 状态置「已交付」、§3.5 任务勾选、新增 §7.4 交付记录；
  `Doc/04` §10.3「渲染错误边界」三条硬约束；`components/ui/README.md` 与
  `components/ui/docs/FpErrorBoundary.md` 登记；`Doc/09`、`Doc/22`、`Doc/README`、`Doc/13`
  测试基线 107 → **135**。

**门禁结果**

- 前端：`typecheck` ✅ / `lint` 0 error（6 warning 存量，**新增文件 0 warning**）✅ /
  `test:unit` **135 passed**（基线 107，新增 28）✅ / `build` ✅；
- 首屏字节核对：index.html 入口 + `modulepreload` 合计 **173KB → 171KB gzip**（无回归；
  本轮 `auth` chunk 并入 index 属 Rolldown 分块重排，首屏总量反降）；
- 后端：`cargo fmt --all -- --check` ✅ / `cargo clippy --all-targets -- -D warnings` 0 告警 ✅ /
  `cargo test --all` **773 passed** ✅（本轮零后端改动）；
- 零新增 npm / Cargo 依赖。


### docs: 开发文档体系整合——标准流程/前端迭代路线/NPC 协作流程入库与 FpColumn 全站表格修复 (Doc/22–24)

Issue #29 附件（标准开发指导流程、前端代码评审与迭代路线图、前端编译验证与运维可读性
评审报告、FpColumn 修复补丁）此前仅以 Issue 附件形式存在，仓库内无对应正文；本轮完成
「文档入库 + 去重合并 + 索引接线 + 交付项落地」的整合。

**文档整合（3 份新正文 + 2 处悬空链接修复）**

- `Doc/22-标准开发流程.md`：端到端流水线（Issue→分支→分层开发→契约同步→门禁→PR→发布）
  总纲，含后端六步 / 前端六步、契约同步矩阵、文档同步矩阵、红线清单、常见坑 TOP5；
  与 `Doc/14`（协作与发布细则）、`Doc/05`/`Doc/04`（分层细节）保持「总纲 + 细则」引用关系，
  不重复其正文；
- `Doc/23-前端评审与迭代路线图.md`：合并《前端代码评审与迭代路线图》与《前端编译验证与
  运维可读性评审报告》两份评审（去重后单一台账）——P0/P1/P2/P3 问题清单（含证据行号）、
  T0–T3 可执行任务、R-1–R-10 运行期缺陷（含 R-1 交付记录与封装层设计约束）、中长期方向与
  现代化面板对标；
- `Doc/24-NPC协作开发流程.md`：NPC（AI 员工）协作机制——@npc 触发/工作模式/重触发与评论上限、
  模型画像与轮次预算、AGENTS.md + 7 个 Skills 双层记忆结构、Issue 任务单模板与设计逻辑、
  阶段拆解范例、人工 Review 要点与 Review 反馈格式、轮次管理与风险对策；
- 修复 `Doc/20` 两处指向**未入库文件**的悬空链接（原文「姊妹篇 / 执行手册」指向
  `FlamePanel-AI原生与现代化运维开发指南.md`、`FlamePanel-NPC开发流程指导.md`）→ 改指
  `Doc/21`、`Doc/24`；
- `Doc/README.md`：正文计数 21→24、导航表补 22/23/24 三行、文档地图新增「流程」子图；
  `Doc/13` 文档计数同步；根 `README.md` 文档中心与「参与贡献」链路接入三份新文档；
  `AGENTS.md` 新增「流程索引」表（流程问题去哪查）并挂接三份新文档。

**代码交付（R-1，来自附件补丁）**

- `frontend/src/components/ui/FpColumn.vue`：改为对 `openvue/column` 的**透明再导出**。
  根因：DataTable 通过 `child.type.name === 'Column'`（HelperSet）收集列、渲染期通过
  `vnode.type.extends.props[prop]`（getVNodeProp）读列元数据——原 script setup 包装组件
  导致列收集**静默失败**（全站表格行高 0、单元格无文本、无任何报错）；对齐 name 后又会因
  缺少 `extends` 直接 TypeError。透明再导出保持 VNode type 即 Column 本体，并写明机制约束；
- `frontend/src/components/ui/__tests__/FpTable.spec.ts`（新增）：组件级回归测试——
  `FpColumn === Column` 同一性断言 + 裸 Column / FpColumn 两条渲染对比（修复前 1 败 1 过、
  修复后全过）；
- 文档同步：`components/ui/README.md`（组件清单标注透明再导出约束 + 规范新增例外条文）、
  `components/ui/docs/FpButtonLink-FpFileUpload-FpColumn.md`（机制约束与回归保护位置）、
  `Doc/04` §15.9（Fp* 唯一入口补「透明再导出例外」）。

**文档层面的基线同步**

- `Doc/22` 测试金字塔与基线更新为实测值：后端 **773（单元 554 + 集成 215 + Agent 3 + 文档测试 1，33 个测试可执行体）**、
  前端 **107**（原文档 615/80+ 已过期）；补记门禁环境依赖（`python3.11-venv` 与
  `deploy_creates_real_venv_with_system_python` 的关系）；
- `Doc/23` 评审基线区分「评审当时（18 spec / 104 用例）」与「R-1 交付后（19 spec / 107 用例）」。

**门禁结果**

- 后端：`cargo fmt --all -- --check` ✅ / `cargo test --all` **773 passed, 0 failed**（33 个测试可执行体）✅；
  沙箱缺 `python3.11-venv` 时 `deploy_creates_real_venv_with_system_python` 失败，安装后全绿
  （环境依赖，与本次改动无关）；
- 前端：`typecheck` ✅ / `lint` 0 error（存量 warning，新增文件 0 warning）✅ /
  `test:unit` **107 passed**（基线 104，新增 3）✅ / `build` ✅；
- 后端代码零改动（本次仅前端组件 + 文档）。

### feat: 伪静态规则持久化写穿——套用即生效与五引擎语法注入（Doc/20 §5.5 收口）

历史 PR #47「后续建议」记录项：`POST /api/websites/:id/rewrite` 只返回生成的规则文本，
**从未写入 vhost**——「一键套用伪静态」是个只读演示接口，站点行为毫无变化。

- **domain**：`Website` 新增 `rewrite_preset: Option<String>`（已套用模板名；serde 默认，
  现有响应仅增不改）；
- **迁移**：`websites` 幂等加列 `rewrite_preset TEXT`（`add_column_if_missing`，旧库升级安全；
  DDL 同步），CRUD 列清单与 SQLite/InMemory 双仓储同步；
- **规则库引擎适配（核心）**：`RewritePreset` 补齐 Caddy / OpenLiteSpeed 语法，
  并新增 `nginx/apache/caddy/ols_site_rules` 与 `preset_rules_for_engine` 适配入口；
- **五引擎配置生成器注入**（`webserver/config.rs`）：
  - Nginx / OpenResty：规则块整块替换默认 `location /`（HTTP 与 HTTPS 两个 server 块均注入）；
  - Apache：`<IfModule mod_rewrite.c>` 指令追加进 `VirtualHost`（SpringBoot 模板走 `ProxyPass`）；
  - OpenLiteSpeed：`rewrite { enable 1; rules <<<… }` 块注入 `virtualHost`（SpringBoot 走 `context` proxy）；
  - Caddy：站点块内替换 `file_server browse`（静态类模板保留 `file_server`，SpringBoot 换 `reverse_proxy`）；
  - 脏数据（未知模板名 / 空串）静默降级为「未套用」——配置生成热路径不因脏数据失败；
- **套用即生效（写穿链路）**：新增 `SiteConfigPort` 端口（application）+
  `DefaultSiteConfigWriter` 实现（infrastructure，复用 `write_config_file_atomic`
  原子写 → `config test` → reload → 失败回滚原文件）；`WebsiteService::apply_rewrite_preset`
  顺序为「落库 → 写盘」，**写盘失败回滚库中模板名**（重新读取当前行规避 OCC 版本冲突，
  避免「面板显示已套用、宿主未生效」的状态分叉）；未注入写盘端口（单测/裁剪装配）时仅落库；
- **API**：新增 `GET /api/websites/:id/rewrite`（`website:read`，当前模板 + 按引擎语法规则文本 +
  模板清单，模板库单一事实来源在后端）与 `DELETE /api/websites/:id/rewrite`（`website:update`，清除回落默认路由）；
  `POST` 保持返回纯规则文本的既有契约；
- **前端**：网站列表新增「伪静态」列（套用状态 Tag）、行操作新增套用/清除弹窗
  （模板下拉取接口 `supported`、规则文本预览、清除按钮仅已套用时可见），i18n 三语言齐备；
- **契约**：新增 2 路径 + 1 schema（`SiteRewriteStatus`），`Website` 仅新增可选字段；
  OpenAPI 快照重生成无删除行；零新增依赖；
- **测试**：后端新增 18 个（规则库 5：多引擎适配 / SpringBoot 反代形态 / Caddy 路由语义 /
  脏数据降级 / 清单往返；配置生成器 7：五引擎注入与默认行为保持 / HTTPS 双块 / 脏数据降级；
  写盘端口 1：未知引擎前置拒绝；SQLite 1：迁移幂等 + 回读 + NULL 语义；
  服务层 2：写穿 + 失败回滚 + 按引擎语法出参；集成 1：POST→GET→DELETE 端到端
  （套用、状态、400 未知模板、404、三方法 401）；权限映射 1）；
  `cargo test --all` **773 passed, 0 failed**（基线 755），fmt / clippy -D warnings 全绿；
  前端新增 API 用例 2（GET/DELETE 形态），typecheck / lint（0 error）/
  test:unit（104 通过，基线 102）/ build 四连全绿。

### feat: 商店安全配置可视化与未签名源门禁——验签策略/镜像白名单真实生效（Doc/20 §4.2 收口 · Step 4-C）

历史 PR #34「后续建议」记录项：验签策略（`app_signature_policy`）与镜像仓库白名单
（`appstore_registry_whitelist`）两项 settings 只能手改数据库——**且组合根从未调用
`AppStoreService::with_settings_service`，两项配置全程空转（死配置）**；
未签名第三方源也没有前端二次确认，与 §4.2「默认弹风险确认」的要求不符。

- **配置真实生效（修掉死配置）**：组合根 `lib.rs` 为 `AppStoreService` 注入
  `SettingsService`（`settings_service` 提前构造，消除重复实例），验收方式为
  设置写入后 `signature_policy()` / `trusted_registries()` 读取路径可观测变化；
- **新增 `GET/PUT /api/app-store/security`**（`app_store:read` / `app_store:update`）：
  返回 `StoreSecurityConfig`——验签策略 / 内置白名单（只读）/ 追加项 / **生效清单**
  （内置 + 追加去重，与安装期安全扫描实际使用者一致）/ **未签名源数量**；
  PUT 支持两项独立或同时更新，`set_many` 原子写入后回读；未装配设置服务返回
  503（配置写入不允许静默丢弃）；空更新与非法值拒绝；
- **白名单条目形态校验**：仅接受 `host[:port]`（拒绝 scheme / 路径 / 非数字端口 /
  非法字符），上限 50 项，归一（trim + 小写）后与内置白名单一并去重，避免粘贴整份
  仓库列表导致扫描失焦；
- **未签名源二次确认（前后端双门禁）**：`POST /api/app-store/sources` 与
  `PUT /api/app-store/sources/:id` 新增 `confirm_unsigned`——创建未配置公钥的源须显式
  确认；**签名源清空公钥**（信任锚卸载）同样受门禁约束（防绕过创建期校验降级）；
  策略 `required` 下未签名源一律拒绝（避免落一个必然不可用的源）；前端新增风险确认弹窗；
- **前端「应用商店 · 安全设置」页**：策略切换（含语义说明与 required 影响提示）、
  未签名源计数告警、镜像仓库白名单 chip 增删（重复/非法即时提示）与内置清单只读展示；
  源列表新增「公钥」编辑弹窗（清空触发二次确认）；i18n 三语言齐备；
- **契约**：新增 2 端点 + 2 schema，既有 2 个请求体仅新增可选字段（默认 false，兼容旧客户端）；
  OpenAPI 快照重生成仅含新增项，前端生成类型同步；零新增依赖；
- **测试**：新增 7 个后端用例（服务层 6：配置默认值与读写回读 / 非法输入矩阵与超量 /
  归一去重 / 未签名源门禁与 required 策略 / 清空公钥降级门禁 / 未装配设置服务写入失败；
  集成 1：两端点端到端 + 门禁 400 分支）与 4 个前端 API 用例；
  `cargo test --all` **755 passed, 0 failed**（flame-kernel 752 + agent 3），
  fmt / clippy -D warnings 全绿；前端 typecheck / lint（0 error）/ test:unit（102 通过）/ build 四连全绿。

### feat: WAF 拦截通知——阻断命中事件化与按规则节流聚合（Doc/20 §6.2 · Step 4-A）

历史 PR #61「后续建议」记录项：WAF 阻断命中只落 `waf_logs` 表，管理员不主动看
面板就**发现不了正在发生的攻击**——「拦截日志 + 事件通知」中的通知一环缺失。

- **domain**：`DomainEvent` 新增 `WafBlocked` 变体（website_id / rule_id /
  rule_name / rule_type / client_ip / request_summary / hit_count）；
- **application**：`WafService` 注入事件总线（`with_event_bus`，组合根 wire-up）——
  阻断命中发布 `WafBlocked`；新增**按规则节流聚合** `BlockThrottle`（规则维度
  进程内窗口，默认 300s）：首次命中立即通知、窗口内重复命中只累计不重复通知、
  窗口过期后首次命中把累计次数经 `hit_count` 一并透出——兼顾「首个异常请求可达」
  与「持续攻击不淹没收件箱」；`log`（仅告警）动作不进通知；未注入总线（裁剪装配）
  静默跳过、发布失败仅告警不影响请求处理；
- **notification**：`EmailChannel::render` 新增映射——主题 `WAF Blocked: {rule_name}`，
  正文含规则类型、来源 IP、请求行摘要与窗口累计拦截次数；
- **契约**：不改动任何 REST 请求/响应结构（事件经既有 Outbox 通道
  `GET /api/outbox-events?type=WafBlocked` 可查），OpenAPI 快照零差异；
- **测试**：新增 9 个（服务层 5：事件字段完整性 / 窗口内节流不重复通知且日志逐条
  登记 / 窗口过期累计透出与规则维度独立 / 未注入总线静默不 panic / log 动作不通知；
  通知渠道 3：`WafBlocked` 邮件渲染主题与正文 / 静默事件不产生空邮件 / SMTP 未配置
  静默跳过；集成 1：完整装配下共享事件总线收到 `WafBlocked`，锁定组合根接线）；
  `cargo test --all` **748 passed, 0 failed**（flame-kernel 745 + agent 3），
  fmt / clippy -D warnings 全绿。

### feat: 备份云端还原补全应用侧——scope=app 数据一键还原回安装目录（Doc/20 §6.5 · Step 4-B）

历史 PR #41/#21「后续建议」记录项：`restore_from_cloud` 仅收 `.db`（面板库），
应用备份上传到云端后**没有还原路径**——「备份应用 → 云端 → 异机恢复」链路断在最后一环。

- **application**：`BackupJobService::restore_from_cloud` 改为按远端 key 前缀分流的统一入口
  （返回 `{scope, filename, message}`）；新增 `restore_app_from_cloud` 与内部
  `swap_in_app_data`：
  - key 解析 `app/<key>/<file>.tar.gz`（穿越 / 反斜杠 / 绝对路径 / 多余层级一律拒绝，
    应用 key 复用安装编排同一字符白名单，防路径拼接注入）；
  - 目标应用须已安装（否则 404 并提示先安装），安装目录经**布局守卫**
    （`<apps>/<key>` 或 `<apps>/<key>/<container>`，即容器模式与目录式安装两种形态，
    其余路径拒绝——防脏数据/越权路径把数据写到面板或系统目录）；
  - 备份包拉取到临时目录（超时 600s）→ 解包到**目标同级暂存目录**（同一文件系统，
    rename 原子）→ 旧目录改名保留 → 暂存目录 rename 就位 → 清理旧目录（尽力而为）；
    解包或落位失败**自动回滚旧目录**，应用数据不出现半包状态；
  - 解包**整包拒绝**路径穿越 / 符号链接 / 硬链接 / 特殊文件（设备、fifo）条目
    （`tar` crate 对 `..` 默认静默跳过、链接条目会真实建立——还原场景不可接受），
    并受解包体积上限 1 GiB 约束（防解压炸弹）；空包同样拒绝（防用空目录覆盖数据）；
  - 临时包与暂存目录无论成败均清理；打包/还原临时文件名补随机后缀（并发同应用不互踩）；
- **api**：`POST /api/backup-jobs/restore-cloud` 响应新增 `scope` 字段（panel | app），
  权限与路由不变，OpenAPI 快照与前端生成类型同步；
- **frontend**：备份中心「备份账号」页新增「云端恢复」弹窗——选账号加载远端对象列表、
  按 key 前缀标注范围（面板库 / 应用数据）、非可还原对象按钮置灰、二次确认文案区分两路语义，
  i18n 三语言齐备；
- **测试**：新增 7 个后端用例（面板库端到端 + 非法 key 矩阵 1、应用还原端到端 roundtrip 1、
  容器模式安装目录布局 1、恶意归档（符号链接 / 路径穿越 / 空包 / 非 gzip）与 key 形态校验 1、
  外部布局拒绝 1、REST 集成端到端 + 400/404 分支 1；集成测试组合根对齐组合根 wire-up：
  备份账号服务与已安装应用仓储改为共享单例）与 1 个前端用例；
- **文档**：Doc/02 §16.2（分流语义 + 安全边界）、Doc/20 §6.5（交付记录）同步。

### feat: Tomcat WAR 产物落位——部署后探测构建产物并幂等部署到 webapps（Doc/20 §5.3 · M-C 收尾）

上一轮 Stage B-2 只落地了「WAR 约定校验与日志」，构建产物从未实际复制到
Tomcat `webapps/`——WAR 项目部署完仍是空壳，无法访问（历史 PR「后续建议」记录项）。

- **infrastructure**：`infrastructure/runtime/tomcat.rs` 新增 WAR 产物探测与落位——
  `find_war_artifact`（扫描候选目录 **项目根 / `target` / `build/libs`**，
  跨越目录统一裁决：同名 `<context>.war` 优先 → 修改时间最新 → 路径升序确定性兜底，
  避免「根目录留有过期包」被误选；候选目录与产物文件均要求真实目录/普通文件，
  `DirEntry::file_type` 不跟随符号链接，指向部署根之外的链接产物一律排除）与
  `deploy_war_artifact`（`<install_path>/webapps/<context>.war`；既有目标
  SHA-256 内容摘要一致则**跳过**，不刷新 mtime 触发 Tomcat 无谓重部署；
  内容变化先写同目录临时文件再 `rename` **原子替换**，不产生半包状态）；
- **application**：`ProjectService::resolve_war_convention` 在约定校验后调用新增
  `place_war_artifact` 完成实际落位——落位成功透出产物路径与目标路径日志，
  内容未变化透明跳过；**未探测到产物不阻断部署**（构建可能由项目侧流水线另行
  完成），落告警附候选位置提示（`mvn package` / `gradle war`）；
- **契约**：不改动任何 REST 请求/响应结构，`POST /api/projects/:id/deploy` 行为
  增强（部署即完成 WAR 落位），OpenAPI 快照零差异；
- **测试**：新增 10 个（产物探测 5：同名跨目录优先 / 同档取最新 / 无候选返回
  None 且非法 context 拒绝 / 符号链接产物排除 / 符号链接候选目录跳过；
  落位 4：摘要一致幂等跳过 / 内容变化原子替换且无临时残留 / 非普通文件与非法
  context 拒绝 / 候选位置提示文案；服务层集成 1：部署落位 → 变化重落 → 无产物
  不阻断）；全量 **733 passed**，fmt / clippy -D warnings 全绿。

### fix: 容器模式状态与日志收口——process_status/logs 按执行面分流

容器模式（`build_type=dockerfile`）项目此前状态与日志无差别走 systemd 守护器：
`docker run` 拉起的容器在 systemd 视角下无对应 unit → 状态恒为 `stopped`、
日志恒为空（历史 PR「后续建议」记录项）。本轮按 `build_type` 分流：

- **infrastructure**：`project_builder.rs` 新增 `ContainerBuilder::status_detail`
  （`docker inspect` 单次模板探测 `Running|ExitCode|RestartCount` → 同构
  `ProcessStatus`：运行中不暴露退出码、已退出且退出码非 0 落 `failed`，
  对齐 `--restart on-failure` 重试耗尽终态）与 `ContainerBuilder::logs`
  （`docker logs --tail <n>`，tail 钳制 1–2000，stdout/stderr 合并流逐行映射
  `LogLine`）；容器不存在时状态回落 `stopped`、日志返回空表（合法态不报错）；
  容器名派生抽 `container_name()` 复用（`status()` 保留为粗粒度薄封装）；
- **application**：`ProjectService::process_status` / `logs` 按 `build_type` 分流——
  容器模式走 `ContainerBuilder`，原生模式（runtime/buildpacks）维持守护器不变；
- **契约**：`GET /:id/status` / `GET /:id/logs` 响应结构不变（两路同构），
  OpenAPI 快照仅描述文案更新；
- **测试**：新增 6 个（状态解析含 failed/stopped/不存在三分支 1、日志解析合并与
  空流 1、docker inspect 命令形状 1、docker logs 命令形状与 tail 钳制 1、
  容器不存在日志空表 1、服务层按 build_type 分流 1）。

### fix: 缺陷清账批——测试目录隔离、WAF 日志保留清理接线、腾讯云 DNS 分页与 CHANGELOG 归并

本轮清账历史 PR「后续建议」记录的残留缺陷（#51/#52/#58/#59/#60/#61），
五项均为工程债/功能缺口修复，无 API 与数据库结构变更：

- **测试隔离**：`runtime_service`（10 例）与 `project_service`（9 例）共用固定
  `/tmp/fp-rt-test`、`/tmp/fp-proj-*` 目录，并发用例互相踩踏（连续多轮遗留）——
  改为 per-test 唯一目录（`uuid` v4 后缀，零新增依赖），用例尾部统一清理；
- **WAF 日志保留清理接线**：新增 `WafService::cleanup_logs_older_than`（委托
  `WafLogRepository::delete_older_than`）并挂入组合根每日 `data-retention-cleanup`
  后台任务（保留 30 天，与系统日志同级）——此前双仓储实现齐备但无人调用，
  `waf_logs` 表随运行无限膨胀；
- **腾讯云 DNS 记录分页**：`DescribeRecordList` 缺省 `Limit=20`，记录数超出时
  单页遍历漏匹配 → 改为 `Offset/Limit` 显式分页逐页翻遍（页大小 100、上限 200 页
  防御异常响应），页内匹配与翻页判定抽为纯函数并配单测；
- **CHANGELOG 归并**：两个重复 `## [Unreleased]` 段合并为单一 Unreleased 段，
  构建器自动协商条目由「伪标题」改为 `### feat:` 同级条目并归位到时间线正确位置。
- **测试**：新增 2 个（WAF 保留清理 1、腾讯云分页匹配/翻页判定 1）；全量 504 通过，
  fmt / clippy -D warnings 全绿。

### feat: 多运行时扩展 Stage B-2——Node tar.xz 目录式安装、Python pyenv + venv、Java Tomcat WAR 约定（Doc/20 §5.3 · M-C）

- **Node 目录式安装**：官方 `node-v<ver>-linux-<arch>.tar.xz`（nodejs.org/dist）下载 +
  进程内 xz 解压到 `/opt/flamepanel/runtimes/node/<ver>`，18/20/22 多版本真正并存
  （不再共用同一 `nodejs` 系统包）；架构 `uname -m` 归一（x86_64→x64 / aarch64→arm64）；
  新增依赖 `lzma-rs`（纯 Rust xz 解码，零 C 依赖，仅 3 个小型传递依赖）；
- **Python pyenv 式安装**：官方源码包 `Python-<ver>.tar.xz` 解压 →
  `./configure --prefix=<install_path> --with-ensurepip=install` + `make -j && make install`
  落到 `/opt/flamepanel/runtimes/python/<ver>`，免污染系统 Python、3.9–3.13 并存，
  编译中间目录用后清理；
- **Python 项目 venv 自动创建**：`kind=python` 项目部署后按约定创建
  `<deploy_path>/.venv`（绑定 Runtime 解释器，无绑定回落系统 `python3`）；项目根存在
  `requirements.txt` 且内容摘要变化时执行 `pip install -r`（`.venv/.fp-deps-stamp`
  摘要缓存，部署幂等、不重复联网装包）；
- **Java Tomcat WAR 约定**：新增 `tomcat` kind（与 JDK 分离，多版本并存）——官方
  `apache-tomcat-<ver>.tar.gz`（dlcdn.apache.org）解压 + **一层目录提升**
  （`promote_release_dir`，Tomcat 发行包容器带同名顶层目录）；`kind=war` 项目要求
  绑定 `tomcat` 运行时（绑定 JDK / 未绑定明确报错），构建产物落
  `<install_path>/webapps/<context>.war`（context 由项目名归一，路径穿越与 `ROOT`
  保留字拒绝）；启动命令约定 `<install_path>/bin/catalina.sh run`（前台运行，
  日志走守护器管道，避免 `startup.sh` fork 后台导致的误判退出）；
- **共用基建**：新增 `infrastructure/runtime/download.rs`（下载 + 架构归一 +
  SHA-256 内容摘要）、`archive::{untar_xz, untar_auto}`（魔数自协商 gz/xz，
  共用同一份路径穿越整体拒绝逻辑）、`PrivilegedCommand.cwd`（源码编译类安装的
  工作目录，经统一特权执行端口，`embedded|agent` 自适应）；
- **修复**：`deploy_locked` 中构建器协商后的部署准备步骤（构建器协商 / 语言约定 /
  Procfile 探测）失败时未回写 `error` 状态的问题（原先 `?` 直接逃逸出函数），
  现统一落 `error` 供排查；
- **测试**：新增 31 个（archive xz/自协商/格式拒绝 4、download 架构/摘要/目录 4、
  node 安装器 3、python 安装器 + venv 约定 6、tomcat 安装器 + WAR 约定 7、
  project_service 语言约定 4、注册表/tomcat kind 3）；全量 715 通过，
  fmt / clippy -D warnings 全绿。

### feat: 多运行时扩展 Stage B-1——Java/Node/Python/Go Runtime 安装器全量接入（Doc/20 §5.3 · M-C）

- **组合根**：`default_runtime_registry` 由 php 单语言扩展为五语言全量注册
  （php/java/node/python/go），`GET /api/runtimes/available?kind=` 各语言版本清单即刻可用；
- **Java**：发行版 `openjdk-<ver>-jdk`（8/11/17/21），fallback `openjdk-<ver>-jdk`；
- **Python**：发行版 `python3.x` 解释器包（3.9–3.13）；
- **Node.js**：发行版 `nodejs`（18/20/22）；官方 tar.xz 目录式安装需新增 lzma 依赖，
  按「不引入 Cargo.toml 之外依赖」纪律顺延后续 Stage；
- **Go**：官方 tar.gz 目录式安装（`https://go.dev/dl/go<ver>.linux-<arch>.tar.gz`）——
  `uname -m` 架构探测（x86_64→amd64 / aarch64→arm64），进程内流式解压到
  `/opt/flamepanel/runtimes/go/<ver>`，零新增依赖；`archive::untar_gz` 工具收敛
  remote_source 的 untar 语义，路径穿越条目整体拒绝（恶意包防护）；
- **加固**：`LanguageRuntimeInstaller::kinds()` 由 match unreachable 改为白名单
  映射，消除新增 kind 时的 panic 路径；
- **测试**：新增 9 个（各语言 spec 4 + kind_slice 白名单 1 + 注册表全量 1 +
  archive 解包/穿越拒绝 2 + Go URL/架构映射 1）；全量 684 通过。

### feat: WAF 基础防护首期——规则引擎、数据层与前端视图（Doc/20 §6.2）

- **SQLite 迁移**：新增 `waf_rules`（规则表）与 `waf_logs`（拦截日志表）幂等建表，
  配套 `idx_waf_rules_website/enabled`、`idx_waf_logs_website/created` 索引，老库升级安全。
- **domain**：`WafRule`（rule_type/action 白名单 + cc 参数解析）与 `WafLog` 实体
  （请求行摘要/UA 超长截断，敏感头与 body 不落库）+ `WafRuleRepository` /
  `WafLogRepository` trait；Sqlite/InMemory 双仓储 + RepoFactory 注册。
- **评估引擎**（`webserver/waf.rs`）：引擎与 Web 服务器适配分离的纯逻辑评估——
  CC 固定窗口限频（对齐 rate_limiter 思想）、SQL 注入/XSS 内置特征（OWASP CRS
  精选子集）、IP/UA 黑白名单（CIDR 支持）、路径封禁；白名单优先级最高；默认
  动作 `log` 仅告警（观察期防误拦截），`block` 阻断。
- **application**：新增 `waf_service.rs`——规则 CRUD 校验 + 引擎变更即时重建
  （RwLock<Arc<WafEngine>>）+ `inspect` 评估与日志登记（日志写失败不阻断评估结论）
  + 日志分页/清理/保留清理；启动时后台加载启用规则。
- **REST**：新增 `GET/POST /api/waf/rules`、`GET/PUT/DELETE /api/waf/rules/{id}`、
  `GET /api/waf/logs`、`POST /api/waf/logs/clear`、`GET /api/waf/status`，新
  `waf` 权限资源组（operator 无 delete、viewer 只读）。
- **前端**：新增「WAF 防护」视图（`/waf` 路由，ops 组侧栏）——规则 CRUD 弹窗
  （类型白名单/站点下拉/CC 参数/动作切换）+ 拦截日志分页（站点过滤 + 清理）
  + 引擎规则数诊断 Tag；`api/waf.ts` + queryKeys + OpenAPI 快照重生成 + 冒烟测试
  7 例；i18n 三语言齐备。
- **测试**：后端新增 20 个（实体 3 + 引擎 8 + 服务层 5 + SQLite 仓储 4 + REST
  集成 3）；前端新增 7 个 API 冒烟。
- **文档**：Doc/02 §24c、Doc/03 §3.15、Doc/07 权限表、Doc/20 §6.2 进度同步。

### feat: SSL 证书自动化 Step 3——腾讯云/华为云 DNS Provider 与前端厂商管理（Doc/20 §6.1 · Plan Step 3）

- **腾讯云 Provider**：`TencentDnsProvider`——TC3-HMAC-SHA256 签名（DNSPod API 3.0），CreateRecord / DescribeRecordList / DeleteRecord；TXT 幂等写入/同值复用/同值清理；`_acme-challenge.<主域>` SubDomain 拆分（根域归一为 `@`）；DescribeDomainList 零副作用探测。
- **华为云 Provider**：`HuaweiDnsProvider`——IAM project token 认证（X-Subject-Token，缓存 1 小时）+ DNS v2 recordsets；recordset 幂等写入/清理；token 签发即零副作用探测。
- **全覆盖路由**：`build_provider` 支持 cloudflare | aliyun | tencent | huawei 四厂商（未知类型明确拒绝）；连通性测试四厂商均走真实 API（`probe_real_api` 注入可离线）。
- **前端**：设置页新增「DNS 厂商」Tab（`SettingsDnsTab`：账号 CRUD + 启停 + 测试连接 + 各厂商凭据占位符）；网站 ACME 签发表单 DNS-01 模式新增厂商账号下拉（透传 `dns_provider_account_id`）；i18n 三语言同步。
- **测试**：后端新增 4 个单元测试（凭据校验/域名拆分/TC3 签名确定性/hex sha256 已知向量）；前端新增 2 个 API 冒烟（tencent/huawei 凭据透传）。
- **文档**：Doc/02 §24b、Doc/03 §3.14z、Doc/20 §6.1 进度同步。

### feat: SSL 证书自动化 Stage 5——真实 DNS 厂商 API 与泛域名无人值守续签（Doc/20 §6.1 · Plan Step 2）

- **真实 Provider**：新增 `infrastructure/acme/dns_real.rs`——CloudflareDnsProvider（API Token Bearer，v4 REST，TXT 幂等写入/同值复用/同值清理）与 AliyunDnsProvider（RPC 签名 HMAC-SHA1，AddDomainRecord/DescribeDomainRecords/DeleteDomainRecord）；泛域名 `*.` 归一化 `_acme-challenge.<主域>`，主域逐级匹配；hmac/sha1 版本对齐依赖树既有传递版本，零新增依赖树节点。
- **连通性测试升级**：cloudflare 走 `/user/tokens/verify`、aliyun 走 DescribeDomains（零副作用探测）；tencent/huawei 保持凭据结构校验（真实 API 属 Plan Step 3）；探测模式可注入（离线测试 Structure / 生产 Real）。
- **签发/续签路由**：`CertificateService` 增加按证书 `dns_provider` 声明在启用账号中解析（`open_credentials` 透明解密 → build_provider 构造），签发与续签统一路由；声明厂商无可用账号返回 400 明确报错；泛域名无人值守续签闭环打通。
- **REST**：`POST /api/certificates/issue/acme` 新增 `dns_provider_account_id` 请求字段；OpenAPI 快照与前端生成类型同步。
- **测试**：后端新增 7 个（Provider 单元 6 + 集成 1）；前端新增 1 个 API 冒烟（账号 ID 透传）。
- **文档**：Doc/02 §24a/24b、Doc/03 §3.14z、Doc/20 §6.1 进度同步。

### feat: SSL 证书自动化 Stage 4——DNS 厂商账号数据层与凭据托管（Doc/20 §6.1）

- **SQLite 迁移**：新增 `dns_provider_accounts` 表幂等建表（`UNIQUE(name)` 防重复命名，`idx_dns_provider_accounts_enabled` 索引），老库升级安全。
- **domain**：`DnsProviderAccount` 实体（provider_type 白名单与 `Certificate::is_valid_dns_provider` 对齐）+ `DnsProviderAccountRepository` trait；Sqlite/InMemory 双仓储 + RepoFactory 注册。
- **application**：新增 `dns_provider_service.rs`——CRUD + 连通性测试登记；凭据静态加密复用 `SecretCipher`（AES-256-GCM，空 `{}` 不加密、同明文保留既有密文防 nonce 噪音，历史明文行静默兼容）；`open_credentials` 供 Stage 5 真实 Provider 构造（返回值不落日志）。
- **REST**：新增 `GET/POST /api/dns-providers`、`PUT/DELETE /api/dns-providers/{id}`、`POST /api/dns-providers/{id}/test`，新 `dns_provider` 资源组权限（operator 无 delete、viewer 只读）；响应不回显 credentials_json。
- **测试**：后端新增 26 个（实体 2 + SQLite 仓储 5 + 服务层 11 + 权限 1 + REST 集成 2 + 路由覆盖等）。
- **文档**：Doc/03 §3.14z、Doc/20 §6.1 进度同步。

### feat: SSL 证书自动化 Stage 3——续签后台任务与到期预警（Doc/20 §6.1）

- **续签编排**：`CertificateService` 新增 `renew_certificate`（ACME 来源复用既有签发链路幂等回写；HTTP-01 优先用绑定站点 webroot，泛域名走 DNS-01；自签/上传返回 400）与 `scan_and_renew`（逐张续签，单张失败不阻断，返回成功/失败计数）。
- **后台任务**：`lib.rs` 组合根新增 `cert-renew-tick` 循环（每 6 小时，响应取消），扫描 `auto_renew` 且进入 30 天续签窗口的证书自动续签。
- **到期预警**：`CertificateExpiring` 事件按 30 天级（>7 天）/7 天级（≤7 天）两级发布，同一自然日同级别去重（内存态），经事件总线 → Outbox → 邮件通知（链路既有）。
- **REST**：新增 `POST /api/certificates/{id}/renew`（权限 certificate:update）；OpenAPI 快照与前端生成类型同步。
- **前端**：`WebsitesView` 证书 Tab——`auto_renew` 开关（FpSwitch，写回 update 端点）、ACME 证书行内「续签」按钮；`api/certificates.ts` 补 `renewCertificate`；i18n 三语言齐备。
- **测试**：后端新增 7 个（服务层 4：窗口外跳过/两级事件/来源拒绝/边界语义 + 集成 3：自签续签 400/匿名 401/缺失 404）；前端新增 1 个 API 冒烟。
- **文档**：Doc/02 §24a、Doc/20 §6.1 进度同步。

### feat: SSL 证书自动化 Stage 2——ACME 签发链路与站点绑定（Doc/20 §6.1）

- **ACME v2 客户端**：新增 `infrastructure/acme/client.rs`——基于仓库既有 `reqwest` + `ring`（rcgen 传递依赖，声明后直接使用）自研最小协议子集：目录发现、JWS ES256 请求签名（RFC 7515/7638）、newAccount（201/200 existing 统一处理）、newOrder、authz/challenge、finalize（CSR）、订单轮询、证书链下载；Replay-Nonce 缓存与刷新防重放。默认 Let's Encrypt 生产目录，staging 目录常量供联调。
- **验证质询**：`infrastructure/acme/challenge.rs`——HTTP-01 验证文件落 webroot `.well-known/acme-challenge/`（token 路径穿越二次把关）；DNS-01 TXT 值 = SHA-256(key_authorization) base64url。`infrastructure/acme/dns.rs`——`DnsProvider` 端口（六边形）+ 内存实现；真实厂商 API（Cloudflare/阿里云/腾讯云/华为云）接入属后续增量。
- **签发编排**：新增 `application/certificate_service.rs`——ACME 签发（泛域名强制 DNS-01、无厂商前置 400 拒绝、验证痕迹成功失败都清理）、自签多 SAN（rcgen，内网场景）、第三方证书上传（轻量 DER 解析 notAfter，零新增解析依赖）；同域名集合幂等（已存在记录走续签回写）；`CertificateIssued` 事件入总线（Outbox 存档 + 邮件渠道映射）。
- **站点绑定**：签发/上传后绑定网站——SAN 覆盖站点域名校验、经 `WebServerManager.write_config_file_atomic` 原子写 443 配置（写 → 校验 → reload → 失败回滚）、`ssl_enabled` 回写；解绑置 false。
- **REST**：新增 `GET/POST /api/certificates`、`GET/PUT/DELETE /api/certificates/:id`、`POST /api/certificates/issue/acme|self-signed`、`POST /api/certificates/:id/bind`，权限新 `certificate` 资源组（read/update/delete，operator 含 update 无 delete、viewer 只读）；OpenAPI 快照 + 前端生成类型同步。
- **前端**：`WebsitesView` 升级双 Tab（网站/SSL 证书）——证书列表（SAN/来源/到期状态徽标/自动续签/关联网站）、ACME 签发表单（DNS-01 切换 + 提示）、自签、上传、绑定/解绑/删除；`api/certificates.ts` 七函数；i18n 三语言齐备。
- **测试**：后端新增 14 个（协议单元 5 + 证书服务 3 + 集成 8：自签落盘/上传解析/前置校验/权限拒绝）；前端新增 7 个 API 冒烟。
- **文档**：Doc/02 §证书端点、Doc/03 §3.14y、Doc/20 §6.1 进度同步。

### feat: SSL 证书自动化 Stage 1——certificates 数据层与 ACME 账户密钥持久化（Doc/20 §6.1）

- **SQLite 迁移**：新增 `certificates` 表幂等建表（`UNIQUE(domains)` 防同域名集合重复，`idx_certificates_expires`/`idx_certificates_website` 索引），老库升级安全。
- **domain**：`Certificate` 实体（source/dns_provider 白名单校验、`validate_domains` 域名正则白名单、`needs_renewal` 30 天续签窗口判定、`domain_list` 防御性解析）+ `CertificateRepository` trait（含 `find_by_domains`/`find_by_website`）。
- **infrastructure**：Sqlite/InMemory 双仓储实现（F-14 读写双池）+ RepoFactory 注册 `create_certificate_repo`；新增 `infrastructure/acme/account.rs`——`AcmeAccountKeyStore` 账户密钥持久化（ECDSA P-256，落 `data/acme/account.key` 0600、目录 0700，幂等复用，空文件自愈重生成），Stage 1 零新增协议依赖（签发链路 instant-acme 属 Stage 2）。
- **测试**：新增 12 个单元测试（实体规则 4 + SQLite 仓储 4 + InMemory 2 + 密钥存储 2）；迁移双跑幂等验证内建。
- **文档**：Doc/03 §3.14y 同步。

### feat: 备份账号凭据静态加密——AES-256-GCM 落库加密与历史明文兼容（Doc/20 §6.5 收尾）

- **加密原语**：新增 `utils/secret_box.rs`——`SecretCipher`（AES-256-GCM 认证加密，随机 nonce，AAD 绑定版本前缀，密文自描述 `enc:v1:BASE64(nonce|ct|tag)`）；主密钥经 HKDF-SHA256 域分离派生（不与 JWT 签名共用密钥材料）。
- **密钥管理**：主密钥持久化 `data/secret_encryption.key`（首次启动生成，0600，重启复用）；密钥不可用时启动不中断，降级明文并告警日志。
- **服务层接入**：`BackupAccountService` 写路径（create/update）统一加密落库（空 `{}` 不加密、同明文不重复加密），使用前（构造 BackupBackend）透明解密；历史明文行静默兼容（首次编辑写回即迁移）。
- **安全语义**：篡改检测（GCM tag 校验失败报错）、错误密钥不可解、密钥丢失需重建备份账号（Doc/03 §3.14a 已注明）。
- **测试**：新增 10 个单元测试（加密原语 7 + 服务层 3：落库密文断言/历史明文探测/更新重加密）。

### feat: 服务联链——项目数据库关联与连接串注入（M-B 主线二 Stage A-10，Doc/20 §5.8 ⑤）

- **SQLite 迁移**：新增 `project_db_links` 表幂等建表（`UNIQUE(project_id, env_name)` + `UNIQUE(project_id, db_instance_id)`，`idx_project_db_links_project` 索引），老库升级安全。
- **domain**：`ProjectDbLink` 实体（env_name 白名单校验：大写字母/数字/下划线）+ `ProjectDbLinkRepository` trait + Sqlite/InMemory 双仓储 + RepoFactory 注册。
- **infrastructure**：新增 `project_link.rs`——`NativeDbLinkProvider` 按实例类型生成标准连接串（mysql/mariadb/postgres/redis），URL 特殊字符 percent-encode，主机恒为回环地址；`DatabaseServiceOps` 适配器复用幂等建库链路；`SystemdSupervisor` 新增 `ensure_unit_with_env`（联链连接串经 unit `Environment=` 注入）；`ContainerBuilder.start_with_env` 容器模式 `docker run -e` 注入（禁参数注入/空白字符）。
- **application**：`ProjectService` 新增 `link_database`/`unlink_database`/`list_links` 编排——实例存在且运行中校验、同项目 env/实例唯一、redis 跳过建库、其余类型幂等创建业务库（默认库名 = 项目名）；start/restart/deploy 编排注入 `resolve_link_env` 解析的连接串；项目删除级联清理联链（幂等）。
- **安全**：凭据经运行时环境变量（MYSQL_ROOT_PASSWORD / POSTGRES_SUPER_PASSWORD / REDIS_PASSWORD）参与拼接，仅内存与进程环境流转，不落日志、不落库；联链实例删除/停止时注入空连接串并告警（应用启动失败可见，不静默缺失）。
- **api**：新增 `GET/POST /api/projects/:id/links`、`GET .../links/available`（可关联实例清单）、`DELETE .../links/:linkId`，权限挂 project:read/update；OpenAPI 快照重生成 + 前端生成类型同步。
- **前端**：`ProjectView` 新增「数据库联链」抽屉——已关联列表（类型徽标/实例名/环境变量名 + 解除）、关联表单（实例下拉 + 自定义变量名）、注入语义说明；i18n 三语言齐备；`api/projects.ts` 补四个联链函数，queryKeys 增 `projects.links`。
- **测试**：后端新增 15 个（连接串生成 7、服务编排 4、SQLite 仓储 3、REST 集成 1）+ 前端冒烟 5 个。

## [Unreleased]

### feat: 健康检查零停机切换——rolling 切换策略与失败回滚告警（M-B 主线二 Stage A-9，Doc/20 §5.8 ④）

- **SQLite 迁移**：`projects` 幂等加列 `health_check_path`（默认空 = 未启用）/`deploy_strategy`（默认 restart），建表 DDL 同步更新，老库升级安全。
- **domain**：`Project` 实体补 `health_check_path`/`deploy_strategy`（serde 默认值，现有 API 响应仅增不改）+ `is_valid_deploy_strategy` 白名单 + `health_check_url()` 回环 URL 归一化；`DomainEvent` 新增 `ProjectZeroDowntimeFailed`（失败告警事件，邮件渠道同步映射）。
- **infrastructure**：新增 `health_probe.rs`——`HttpHealthProbe` 轮询探测器（`http://127.0.0.1:{port}{path}`，2xx/3xx 通过，单次超时/轮询窗口/间隔可调），零新增依赖（reqwest 已有）。
- **application**：`ProjectService` restart 编排接入零停机路径——`deploy_strategy=rolling` 且已配置健康检查端点时「基线探测 → 重启 → 轮询新实例健康 → 通过即切换 / 失败回滚旧实例（error 落库）+ `ProjectZeroDowntimeFailed` 事件告警」，对齐 Dokku app.json checks 语义；restart 策略维持直接重启既有行为；健康检查路径校验（禁空白字符防头注入，长度 ≤512）。
- **api**：新增 `POST /api/projects/:id/health-check`（手动探测，权限 `project:update`）；创建/更新请求与 `ProjectDto` 增 `health_check_path`/`deploy_strategy`；OpenAPI 快照重生成 + 前端生成类型同步。
- **前端**：`ProjectView` 增「切换策略」下拉与「健康检查路径」输入（rolling 时显示）、「健康检查」操作按钮（未配置路径禁用）；`api/projects.ts` 补 `checkProjectHealth`；i18n 三语言同步。
- **测试**：后端新增 8 个（探测器 3、服务编排 4、迁移回读 1）+ REST 集成 2。

### feat: 构建器自动协商——Dockerfile 容器模式与双执行面（M-B 主线二 Stage A-8，Doc/20 §5.8 ②）

- **SQLite 迁移**：`projects` 幂等加列 `build_type`（默认 runtime），老库升级安全。
- **domain**：`Project` 实体补 `build_type`（serde 默认值，现有 API 响应仅增不改）+ `is_valid_build_type` 白名单。
- **infrastructure**：新增 `project_builder.rs`——构建器协商（Dockerfile → pack-cli → Runtime 探测顺序）与容器执行面（`ContainerBuilder`：docker build/run/rm/inspect，端口映射回环绑定，构建超时 30min 与 compose 对齐）；镜像 tag `flame-proj/<name>:<id>`（项目名白名单校验防注入）。
- **application**：`ProjectService` 部署编排接入构建器协商——含 Dockerfile 项目落容器模式（docker run 拉起，启停/重启/删除走 Docker 执行面，跳过 systemd unit 与 Procfile 协商）；无 Dockerfile 环境回退 runtime（Procfile 协商不变）；新增 `builder_decision` 查询。
- **api**：新增 `GET /api/projects/:id/builder`（权限 `project:read`）；`ProjectDto` 增 `build_type`；OpenAPI 快照重生成 + 前端生成类型同步；ProjectView 增「构建器」列，i18n 三语言齐备。
- **测试**：project_builder 单测 4 个 + ProjectService 协商编排 2 个 + SQLite 列回读 1 个 + REST 集成 1 个。

### feat: Procfile 进程模型——部署后进程声明探测与构建器协商（M-B 主线二 Stage A-8，Doc/20 §5.8 ③）

- **SQLite 迁移**：`projects` 幂等加列 `procfile_json`（Procfile 进程类型声明 JSON，默认 NULL），建表 DDL 同步更新，老库升级安全。
- **domain**：`Project` 实体补 `procfile_json`（serde 默认值，现有 API 响应仅增不改）+ `procfile_processes()` 声明解析辅助。
- **infrastructure**：新增 `project_procfile.rs`——无依赖 Procfile 解析器（`TYPE: COMMAND` 行语法，`#` 注释/空行忽略），进程类型白名单（web/worker/clock）、类型名与命令注入双重防护（shell 元字符拒绝，与 systemd 单元 `parse_exec` 同一威胁模型）；`load_from_dir` 文件不存在返回 Ok(None)。
- **application**：`ProjectService` 部署编排接入 Procfile 探测——git/本地部署成功后解析项目根 `Procfile`：声明入库 `procfile_json`、`web` 命令在用户未显式配置 `run_command` 时自动回填（Dokku「用户只管推代码」的构建器协商体验）；用户显式命令优先不覆盖；Procfile 被删除时清空历史声明；解析非法降级为告警不阻断部署（启动时 systemd 单元校验兜底拒绝）；新增 `procfile_processes` 查询方法。
- **api**：新增 `GET /api/projects/:id/procfile`（进程声明查看，ps:scale 伸缩面板数据源），权限挂 `project:read`；`ProjectDto` 增 `has_procfile`；OpenAPI 快照重生成 + 前端生成类型同步。
- **测试**：后端新增 12 个（解析器 9、服务编排 2、迁移回读 1）+ REST 集成 1 + 权限表覆盖扩展；前端新增 1 个（API 请求形态）。

### feat: git push 即部署——Webhook 部署通道（M-B 主线二 Stage A-7，Doc/20 §5.8）

- **SQLite 迁移**：`projects` 幂等加列 `deploy_type`（manual | webhook）/`auto_deploy`/`webhook_token`，建表 DDL 同步更新，老库升级安全。
- **domain**：`Project` 实体补部署三字段（serde 默认值，现有 API 响应仅增不改）+ `is_valid_deploy_type` 白名单。
- **application**：`ProjectService` 新增 webhook 编排——`enable_webhook`（开启/轮换一次性令牌，uuid v4 双段熵源）/`disable_webhook`（回落 manual 清空令牌）/`verify_webhook_token`（常量时间比较防时序侧信道）/`deploy_by_webhook`（同项目互斥复用部署锁）；部署主体抽出 `deploy_locked(restart_if_running)`——webhook push 部署时项目此前在运行则拉取新代码后自动重启（Dokku「push 即部署」体验），手动部署保持「部署 ≠ 启动，成功回落 stopped」既有语义。
- **api/middleware**：`POST /api/projects/:id/webhook` 免 JWT 但集中守卫——部署令牌经 URL query `?token=`（优先）或 `X-Webhook-Token` 头校验（与节点心跳同一管理模式）；令牌错误 401，未启用 404。
- **api**：新增 `POST /api/projects/:id/webhook`（push 触发部署，兼容 GitHub/GitLab/Gitee/Gitea/通用 JSON，命中即视为一次 push）、`/webhook/enable`（返回一次性令牌与 webhook_url）、`/webhook/disable`；enable/disable 挂 `project:update` 权限；`ProjectDto` 增 `deploy_type`/`auto_deploy`/`has_webhook_token`（令牌明文不回显）；OpenAPI 快照重生成。
- **前端**：`ProjectView` 增「部署方式」列与 Webhook 按钮（开启展示一次性令牌弹窗、已开启可轮换令牌、一键复制 webhook 地址）；`api/projects.ts` 补 `enableProjectWebhook`/`disableProjectWebhook`；i18n 三语言同步。
- **测试**：后端新增 5 个（服务编排 3、REST 集成 1、迁移回读 1）+ 权限表扩展；前端新增 2 个（API 请求形态）。

### feat: 网站⇄运行时⇄项目绑定——站点类型与配置生成分支（M-B 主线二 Stage A-6，Doc/20 §5.5）

- **SQLite 迁移**：`websites` 幂等加列 `site_type`（默认 static）/`runtime_id`/`project_id` + `idx_websites_site_type` 索引，建表 DDL 同步更新，老库升级安全。
- **domain**：`Website` 实体补 `site_type`/`runtime_id`/`project_id`（serde 默认值，现有 API 响应仅增不改）+ `is_valid_site_type` 白名单（runtime_php/reverse_proxy/static/container_app）。
- **webserver**：新增 `site_binding.rs`——php-fpm pool socket 路径（`/run/php/php<ver>-fpm-<site>.sock`，fastcgi 配置片段含 SCRIPT_FILENAME 与 .ht 拒绝）、项目站点反代片段（`proxy_pass http://127.0.0.1:<port>` + WebSocket upgrade 头，nginx/OLS/Apache/Caddy 四引擎适配）、伪静态规则库（WordPress/Discuz/Typecho/Laravel/ThinkPHP/SpringBoot，nginx 与 htaccess 双语法）。
- **application**：`WebsiteService` 新增绑定编排 `bind_runtime`/`unbind_runtime`（site_type 白名单、php 运行时须已安装、项目须配置端口、runtime/project 互斥）与 `list_sites_by_runtime`/`list_sites_by_project`/`apply_rewrite_preset`；`with_binding_repos` 注入校验仓储（旧构造兼容）。
- **api**：新增 `POST /api/websites/:id/runtime`（绑定/解绑）、`POST /api/websites/:id/rewrite`（伪静态套用）、`GET /api/runtimes/:id/sites`；权限注册 website:update / website:read，路由覆盖测试同步扩展；OpenAPI 快照重生成。
- **前端**：`WebsitesView` 增站点类型列/选择与「绑定」弹窗（PHP 运行时/项目下拉 + 伪静态模板一键套用），`api/websites.ts` 补 `bindWebsiteRuntime`/`applyWebsiteRewrite` 与 SITE_TYPES/REWRITE_PRESETS 常量；i18n 三语言同步。
- **测试**：后端新增 9 个（site_binding 单测 5、绑定编排集成 2、权限表扩展、服务测试字面量补字段）；前端新增 3 个（API 请求形态 + 常量对齐）。

### feat: 项目服务与进程守护——ProjectService + ProcessSupervisor（M-B 主线二 Stage A-3，Doc/20 §5.4/5.6）

- **application 端口**：新增 `project_ports.rs`——`ProcessSupervisor` trait（ensure_unit/remove_unit/start/stop/restart/status/logs），服务层零 infrastructure 依赖。
- **application 服务**：新增 `project_service.rs`——项目 CRUD（名称/端口/git url/部署路径白名单校验、运行时绑定存在性校验）、部署（local_path 目录在部署根内 + canonicalize 防符号链接逃逸 / git clone --depth 1）、守护编排（start/stop/restart/status/logs，同项目互斥 F-10 同款语义）；运行中禁止改启动命令（409）；删除自动停进程且保留部署目录（零锁定纪律）。
- **infrastructure**：新增 `supervisor/systemd.rs`——`SystemdSupervisor` systemd 模板单元守护（`flame-proj-<id>.service`），unit 生成含 WorkingDirectory/PORT/Restart=on-failure；启动命令禁 shell 元字符、可执行文件路径前缀白名单（部署目录 + /usr/bin + /usr/local/bin）；全部特权操作经 `PrivilegedCommandRunner`；无 systemd 环境优雅降级（503/unknown）。
- **api**：新增 `/api/projects` 十一端点与 `/api/runtimes` 七端点（安装/卸载/设默认/available 版本清单），权限挂 `project:*`/`runtime:*` 共 8 项；OpenAPI schema 同步并重生成前端类型。
- **组合根接线**：`Services`/`AppState` 新增 `project_service` 字段。
- **测试**：新增 22 个测试——端口 mock 1、systemd 实现 5（命令切分注入拒绝/路径前缀校验/unit 渲染/base64 向量/命名）、服务编排 6、REST 集成 5、运行时服务 1（无安装器 available 降级）+ 权限表覆盖测试扩展。

### feat: RuntimeService 安装编排——运行时安装/卸载/设默认（M-B 主线二 Stage A-2，Doc/20 §5.1/5.3）

- **application 端口**：新增 `runtime_ports.rs`——`RuntimeInstaller` trait（install/uninstall/is_package_installed）与 `RuntimeInstallerRegistry` 按 kind 路由注册表（对齐 `NativeInstaller` 模式），`RuntimeService` 零 infrastructure 依赖。
- **application 服务**：新增 `runtime_service.rs`——安装编排（同 (kind,version) 幂等登记、失败落 error 保留记录、error/not_installed 可重试、已安装 Conflict 拒绝）、卸载（尽力清理 + 删记录）、设默认（要求已安装）、`available()` 可安装版本清单（声明版本 × 系统包探测）、kind/版本号白名单校验（防包名与路径拼接注入）。
- **infrastructure**：新增 `runtime/` 模块——`LanguageRuntimeInstaller` 通用基类（PackageManager 发行版感知路由 + 安装目录自建）+ PHP 安装规格（php8.x-fpm 声明式包名映射），`default_runtime_registry()` 组合根注册。
- **组合根接线**：`Services`/`AppState` 新增 `runtime_service` 字段（base_dir 默认 /opt/flamepanel/runtimes，测试可覆盖）。
- **测试**：新增 12 个单元测试（注册表路由 2、安装器包名映射/目录幂等 3、服务编排 7——成功流转/失败落 error/重复拒绝/错误重试/卸载清理/设默认门槛/校验拒绝）。REST 端点与前端见 Stage A-4/A-5。

### feat: 运行时与项目数据层——runtimes/projects 表与仓储实现（M-B 主线二 Stage A-1，Doc/20 §5.2）

- **SQLite 迁移**：新增 `runtimes` 表（kind/version 多版本共存，UNIQUE(kind, version)，`idx_runtimes_kind` 索引）与 `projects` 表（UNIQUE(name)，`idx_projects_status`/`idx_projects_runtime_id` 索引），幂等建表老库安全。
- **domain**：新增 `Runtime` 实体（kind/status 白名单、`is_installed` 判定）与 `Project` 实体（kind/source/status 白名单）+ `RuntimeRepository`/`ProjectRepository` trait（Runtime 含事务化 `set_default` 同 kind 唯一默认）。
- **infrastructure**：`SqliteRuntimeRepository`/`SqliteProjectRepository` 与 `InMemoryRuntimeRepository`/`InMemoryProjectRepository` 双实现 + RepoFactory 注册，对齐 F-14 读写双池模式。
- **测试**：新增 8 个单元测试（双表 CRUD/UNIQUE 约束/set_default 同 kind 作用域/更新不存在 id 报错）。

### feat: 备份中心前端接入——备份账号与计划任务管理视图（M-A 收尾，Doc/20 §6.5）

- **API 层**：新增 `api/backupAccounts.ts`（列表/创建/编辑/删除/连通性测试/云端对象列表与上传/删除 8 方法）与 `api/backupJobs.ts`（任务 CRUD/启停/立即执行/云端恢复 7 方法），类型取自 OpenAPI 生成模块（`AccountDto`/`JobDto` 等），禁止手写重复定义。
- **视图**：`BackupView.vue` 升级为三页签（本地备份 | 备份账号 | 计划任务）——账号页支持 s3/sftp/local 类型创建与编辑（编辑时 config_json 留空表示不修改，敏感配置不回显）、FpSwitch 启停、真实探测测试；任务页支持 panel/app 范围、cron 表达式、目标账号选择、保留份数、启停与立即执行。全部 vue-query 数据层 + Fp* 组件 + `v-permission` 控权。
- **OpenAPI 同步**：重新导出 `frontend/openapi.json`（含 backup-accounts/backup-jobs 全部端点）并重生成 `src/api/generated/openapi.ts`；`generated/index.ts` 具名导出备份域 Schema 类型。
- **i18n**：`backup.*` 新增 45 个 key，三语言（zh-CN/en-US/ja-JP）同步齐备。
- **测试**：新增 `backupAccounts.spec.ts` 15 个冒烟用例（请求路径/方法/长超时通道断言），vitest 55 全绿。

### feat: 备份任务体系 Stage 3——计划备份、调度执行与云端恢复（M-A 收尾，Doc/20 §6.5）

- **SQLite 迁移**：新增 `backup_jobs` 表（scope: panel|app、cron schedule、target_account_id、retention、UNIQUE(name)，`idx_backup_jobs_enabled` 索引），幂等建表老库安全。
- **domain**：新增 `BackupJob` 实体（scope 白名单 `is_valid_scope`）与 `BackupJobRepository` trait（CRUD + `record_run`）；`DomainEvent` 新增 `BackupSucceeded`/`BackupFailed`（通知渠道渲染同步）。
- **infrastructure**：`SqliteBackupJobRepository` / `InMemoryBackupJobRepository` 双实现 + RepoFactory 注册。
- **application**：`BackupJobService`——任务 CRUD（编辑保留执行历史、目标账号存在性校验）+ 调度 tick（同任务防重叠，F-10 同款语义）+ 执行编排（panel 复用 VACUUM INTO 一致性快照；app 目录 tar.gz 打包，安装目录名与 key 一致性校验防穿越，上传后清理本地临时包）+ 远端保留策略（同前缀按 key 字典序保留最近 N 份）+ 云端恢复（远端 key 校验 → 拉取到本地备份目录 → 复用恢复前二次备份保护流程）。
- **api**：新增 `/api/backup-jobs` 七端点（GET/POST 列表与创建、PUT/DELETE `/:id`、PUT `/:id/enabled`、POST `/:id/run`、POST `/restore-cloud`），权限挂 backup 组；OpenAPI schema 同步。
- **调度接线**：后台 Supervisor 新增 `backup-job-tick`（30s 周期，响应取消）。
- **测试**：新增 11 个测试——服务层 8（CRUD 校验、panel 执行落库、app 打包端到端、停用/缺失账号失败落库、保留策略清理、tick 到期触发与防重叠、云端恢复与非法 key 拒绝）+ SQLite 仓储 3（CRUD/UNIQUE/record_run）。
- **修复**：`tests/backup.rs` 并行时序脆弱断言加固（stage2 种子文件泄漏到共享备份目录导致 api_flow list 断言偶发失败），现断言「创建的备份存在」而非精确数量。
### feat: PostgreSQL 原生数据库支持（M-A 收尾 · Doc/20 §6.4）

- **infrastructure/database**：新增 `PostgresManager`——发行版包安装（apt/dnf/apk 经 `PackageManager` 自动路由）、RHEL 系 `initdb` 探测、`pg_isready` 就绪等待、非默认端口配置；`psql`/`ALTER SYSTEM` 配置管理（标识符白名单校验 + SQL 字面量转义双重防护）；库/用户 CRUD 与授权；密码经 `PGPASSWORD` 环境变量传递（不落 argv/`ps`）。
- **domain**：`DatabaseType` 新增 `Postgres` 变体（as_str/from_name/default_port/description/service_name/package_name 元数据齐备，别名 postgresql/pgsql）。
- **application**：`DatabaseService` 新增 `install_postgres` 与 postgres 生命周期/库用户分派；`DatabaseNativeInstaller` 注册 `postgres` key（应用商店原生安装同链路）。
- **api**：新增 `POST /api/databases/postgres/install`（权限沿用 `database:create`，经 suffix 规则自动映射）；实例生命周期端点零改动（postgres 类型自动走新分派）。
- **frontend**：数据库页新增 PostgreSQL 安装弹窗（名称/版本/端口/密码）与类型筛选；`installPostgres` API；i18n 三语言齐备（顺带补齐视图既有引用缺失的 `password/optional/rootPassword/versionPlaceholder` 四 key）。
- **测试**：新增 5 个测试——PostgresManager 经统一端口路由与 SQL 注入拒绝 3、`DatabaseType::Postgres` 元数据 1、服务层 postgres 生命周期分派 2（start/stop/status 流转，不依赖真实 PostgreSQL）。

### feat: 备份账号体系 Stage 2——BackupBackend 三实现与云端备份路由（M-A 收尾，Doc/20 §6.5）

- **infrastructure/backup_backend**：新增 `BackupBackend` trait（upload/list/download/delete/test_connection）与三实现——`LocalBackend`（本地目录/挂载盘，递归列表、key 穿越防护）、`SftpBackend`（ssh2，密码或私钥认证，远端 mkdir_p，全部 IO 移入 spawn_blocking）、`S3Backend`（object_store aws feature，S3 兼容一网打尽：AWS S3 / MinIO / OSS / COS / R2，path-style 自建端点自适应）。
- **application**：`BackupAccountService` 新增 `test_connection`（真实探测：连接 + 认证 + 写探针，结果落库，失败带原因）与备份编排按账号路由四方法（upload/download/list/delete 远端对象，停用账号与非法文件名拒绝）。
- **api**：`POST /:id/test` 升级为真实探测（失败返回 200 + `last_test_status=failed` + `test_message`）；新增 `GET /:id/objects`（backup:read）、`POST /:id/objects/upload`（backup:create，本地备份文件名穿越防护）、`DELETE /:id/objects?key=`（backup:delete，幂等）；OpenAPI schema 同步。
- **依赖**：新增 `object_store`（aws feature）与 `ssh2`（vendored-openssl）。
- **测试**：新增 10 个后端测试（local 三实现往返/穿越拒绝 3、sftp 配置与拒绝连接 4、s3 配置解析与穿越 3）+ 5 个服务层测试（真实探测成功/失败/坏配置、编排往返、停用账号拒绝）+ 2 个 REST 集成用例（失败路径、云端对象全链路）。

### feat: 备份账号体系 Stage 1——数据层与服务骨架（M-A 收尾，Doc/20 §6.5）

- **SQLite 迁移**：新增 `backup_accounts` 表（s3 | sftp | local，UNIQUE(name) 防重复命名，`idx_backup_accounts_enabled` 索引），幂等建表老库安全。
- **domain**：新增 `BackupAccount` 实体（类型白名单校验 `is_valid_account_type`、连通性测试状态）与 `BackupAccountRepository` trait（CRUD + `record_test`）。
- **infrastructure**：`SqliteBackupAccountRepository` / `InMemoryBackupAccountRepository` 双实现 + RepoFactory 注册。
- **application**：`BackupAccountService`——CRUD + 连通性测试结果登记（编辑不重置测试历史；config_json 必须合法 JSON）。
- **api**：`/api/backup-accounts` 系列五端点（GET/POST 列表与创建、PUT/DELETE `/:id`、POST `/:id/test`），权限沿用 backup 组（新增 `backup:update` 权限元数据），config_json 不回显防敏感泄漏；OpenAPI schema 注册。
- **测试**：新增 4 个 SQLite 仓储测试（CRUD/UNIQUE 防重/record_test/删除幂等）+ 3 个服务层测试 + 1 个 REST 集成测试 + 1 组权限映射测试。

### feat: x-flame 扩展字段格式与零锁定纪律（M-M1，Doc/20 §4.6/§7.1，Ref: #29）

- **infrastructure/adapter**：新增 `xflame.rs`——解析 Flame 包 compose 顶层 `x-flame` 扩展字段（Compose Specification extension 规范，CasaOS `x-casaos` 同构）：app 级 `category`/`arch`/`screenshots` 补充元数据（app.json 显式声明优先，不覆盖）、service 级 env/port/volume 人类可读说明转补充表单字段（「表单即元数据」，商店页面零额外维护）；env 键名白名单校验（大写字母/数字/下划线，≤128 字符），路径穿越/非法键静默忽略；`deny_unknown_fields` 拼写错误不静默吞掉。
- **application**：安装编排注入 CasaOS 同构系统变量 `$PUID`/`$PGID`（默认 1000）/`$TZ`（宿主机时区）/`$APP_ID`——用户表单值优先级更高，仅缺省时注入；数据统一落盘 `data/apps/<key>/`，规避容器 root 运行。
- **infrastructure/security_scanner**：质量规约进商店准入——镜像显式 `:latest` 标签 → Low 告警（不可回滚不可复现）；未显式声明 tag → Info 提示。
- **部署**：`uninstall.sh` 新增 `--keep-apps`（`-k`）选项——卸载面板保留 `data/apps` 全部应用数据与落盘 compose（「面板可弃，应用照常」零锁定纪律）；与 `--purge` 互斥校验。
- **测试**：新增后端 11 个单元测试（x-flame 解析 4 / Flame 适配器合并 4 / 系统变量注入 2 / latest 告警 3，含恶意输入用例）。

### feat: Compose 生命周期钩子（Doc/20 §4.3，Ref: #29）

- **domain**：新增 `AppHooks` 实体（pre_install/post_install/pre_upgrade/post_upgrade/pre_uninstall，相对包根路径）；`AppVersionInfo` 增加 `hooks` 字段（serde 默认空，旧包/旧响应兼容）。
- **infrastructure/adapter**：OnePanelAdapter 解析 `scripts/` 自动填充 hooks（init.sh → pre_install + post_install，1Panel 语义超集；upgrade.sh → pre_upgrade；uninstall.sh → pre_uninstall）；FlameAdapter 支持 `app.json` 的 `hooks` 显式声明优先、`scripts/` 目录标准命名探测兜底。
- **infrastructure/security_scanner**：新增 `scan_script` 脚本静态扫描——`rm -rf /`（含 `/*`/`~`/`$HOME` 变体）、`curl|sh` 管道执行、写 `/etc/passwd|/etc/shadow|crontab`、反弹 shell（`/dev/tcp/`）、fork 炸弹 → Block；`mkfs`/`dd of=/dev/*` → High 告警；`ComposeSecurityScanner` 端口新增 `scan_script` 默认方法。
- **application**：`AppStoreService` 新增钩子执行器——包根相对路径安全解析（字符白名单 + 规范化路径不得逃逸包根 + 脚本禁止符号链接等效 `O_NOFOLLOW`）→ 静态扫描 → 统一特权命令端口执行（超时 300s，输出落任务日志）；接入三链路：安装（pre 失败中止 / post 失败回滚 compose down）、升级（pre 失败中止 / post 失败仅告警）、卸载（pre 失败仅告警不阻断）。
- **api/openapi**：`AppHooks` schema 注册；`frontend/openapi.json` 快照与生成类型同步。
- **frontend**：安装弹窗新增钩子脚本确认提示（列出脚本清单 + 显式勾选确认，未确认拒绝提交）；`AppVersionInfo`/安装请求类型补 `hooks`/`acknowledge_scripts`；i18n 三语言齐备。
- **测试**：新增后端 17 个单元测试（扫描器 8 / 适配器 4 / 执行器与路径安全 5）。

### feat: 远程包安装链路补全与前端源管理（M-A Issue 1.4，Ref: #29）

- **application**：`AppStoreService` 新增 `ensure_remote_package_fetched`——http 源包（同步只落索引）安装时经 `fetch_package` 按需下载包体到 `data/apps/remote/<key>/` 缓存并回写 `source_path`（Circuit Breaker 包裹外部下载；源不存在/停用拒绝安装），接入 `install_inner` 编排。
- **domain/infrastructure**：`AppPackageRepository` trait 新增 `update_source_path`（InMemory/SQLite 双实现）；修复 `HttpRemoteSource::fetch_package` 缓存命中时未下钻单层包裹目录的缺陷（`drill_down_single_dir` 幂等化）。
- **api**：`/api/app-store/sources` 系列端点 OpenAPI schema 补全注册；新增 `export-openapi` bin 导出快照；`frontend/openapi.json` 快照与生成类型同步。
- **frontend**：应用商店新增「远程源」tab——源列表（类型/地址/验签公钥/最近同步）、添加源（git/http + 可选公钥）、手动同步、启停、删除；i18n 三语言补齐；新增远程源 API 冒烟测试 6 例。
- **测试**：新增后端 3 个单元测试（懒加载拉取回写/缺失源拒绝/update_source_path 持久化，含本地 HTTP 服务端到端 manifest+包体下载）。

### feat: 应用包签名验签与可信来源（M-A Issue 1.3，Ref: #29）

- **infrastructure**：新增 `signature.rs`——Ed25519 包签名与验签：包内容摘要（文件清单 SHA-256，`signature.sig` 自身剔除）、`signature.sig` 验签（远程包）、app.json `signature` 字段验签（本地导入，规范化 JSON + 其余文件清单为签名原文）、密钥对生成、公钥格式校验。
- **application**：`AppStoreService` 接入验签三链路——本地导入（app.json 带 signature 即强制校验）、远程源同步（git 源包体就地验签，未配公钥按策略放行/拒绝）、安装编排（远程包按来源 url 关联源强制验签）；新增 `trusted_registries`（settings `appstore_registry_whitelist` 追加镜像白名单）与 `signature_policy`（settings `app_signature_policy`：optional/required）。
- **安全扫描**：`ComposeSecurityScanner` 端口新增 `scan_compose_with_registries`（默认方法），安装链路传入可配置镜像白名单。
- **settings**：新增默认项 `app_signature_policy`（optional）、`appstore_registry_whitelist`（空）。
- **依赖**：新增 `ed25519-dalek = "2"`。
- **测试**：新增 14 个单元测试（摘要确定性/内容敏感、签名往返、篡改拒绝、公钥校验、策略解析、注册源验签、缺签名拒绝、白名单合并等）。

### feat: 远程应用源同步器与 REST API（M-A Issue 1.2，Ref: #29）

- **infrastructure**：新增 `remote_source.rs`——`RemoteAppSource` 端口 + `GitRemoteSource`（`git clone --depth 1` 懒加载，遍历一级子目录经适配器解析索引）/ `HttpRemoteSource`（JSON manifest，同步只落索引，安装时按需下载包体并安全解包，拒绝路径穿越条目）。
- **application**：`AppStoreService` 新增远程源管理（create/update/delete/list）与 `sync_source` 编排——经 Circuit Breaker 拉取索引 → `create_many` 幂等落库 → 触达 `last_synced_at`，同步任务复用统一任务中心，同源并发同步互斥。
- **api**：`/api/app-store/sources` CRUD + `/:id/sync` + `/:id/packages` 六端点，权限元数据 `permissions.rs` 同步注册（app_store:read/create/update/delete）。
- **依赖**：新增 `flate2`、`tar`（HTTP 包体 tar.gz 解包）。
- **测试**：新增 8 个单元/集成测试（url/key 校验、manifest 解析、路径穿越拒绝、CRUD/幂等同步/停用拒绝/任务中心跟踪）。

### feat: 远程应用源数据层（M-A Issue 1.1，Ref: #29）

- **SQLite 迁移**：新增 `app_sources` 表（多远程源，UNIQUE(url)，`idx_app_sources_enabled` 索引）；`app_packages` 幂等加列 `source`（默认 local）/`source_url`/`synced_at`，新增 `idx_app_packages_source` 索引。老库启动自动迁移。
- **domain**：新增 `AppSource` 实体与 `AppSourceRepository` trait；`AppPackage` 实体补充来源三字段（serde 默认值向后兼容）。
- **infrastructure**：`SqliteAppSourceRepository` / `InMemoryAppSourceRepository` 双实现 + RepoFactory 注册；AppPackage 仓储 SQL 同步三列。
- **测试**：新增 5 个单元测试（CRUD / UNIQUE 防重 / 触碰同步时间 / 迁移幂等 / 新列回读）。

### docs: 更新优化开发文档（Doc/ 系列）

- **版本数据同步**：路由规模 179→181、测试基线 295→330（153 单元 + 153 集成 + 13 setup + 7 stage5 + 4 agent）、handler 模块 22→23（新增 `setup`）。
- **WS 首帧鉴权**：Doc/01/02/04/05/13 从 `?token=` 查询参数改为「首帧鉴权」（R2）——连接建立后首帧发送 `{"type":"auth","token":"<access_token>"}` + RBAC 校验。
- **httpOnly Cookie**：Doc/02/07 同步 refresh token 从响应体迁移到 `Set-Cookie: flame_refresh=<jwt>; HttpOnly; SameSite=Lax` 的变更。
- **错误码补充**：Doc/02 错误码表补 `RATE_LIMITED`(429)；Doc/01 错误码计数 8→10。
- **前端视图数**：Doc/04 与 frontend/design.md 视图计数同步为 23 视图 + Setup 向导、导航组数 5→6。

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
- **Bug 修复（向导模式失效）**：`config/loader.rs` 不再在 `admin_password` 缺失时自动填充随机密码——原逻辑导致「未配置 `OP_ADMIN_PASSWORD`」永远被判定为无人值守模式、Setup 向导永不出现；现空密码保持为空 → 向导模式，显式配置才进入无人值守。新增回归测试 `admin_password_stays_empty_without_env`。
- **SMTP 未配置噪音**：`EmailNotifier::is_configured()` 判定默认 `localhost:25` 无凭据视为未配置，事件通知静默跳过（debug 日志），不再对每次 `UserCreated` 等事件报 `Connection refused` ERROR。
- 测试基线：330 个测试全部通过（153 单元 + 153 集成 + 13 setup + 7 stage5 + 4 agent）。

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
