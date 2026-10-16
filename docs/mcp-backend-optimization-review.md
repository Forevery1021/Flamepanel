# flame-kernel 后端优化审查报告

> 审查日期：2026-09-16 · 基线：main `460c4a2`（T-01 分支 `fix/t01-libc-kill` 上执行）
> 方法：grep 定位 + 目标行段精读（遵守 AGENTS.md 大文件纪律，未整读 sqlite.rs / app_store_service.rs / entity.rs）
> 结论先行：**项目已做过一轮系统性性能加固**（代码内可见 F-01/F-14/F-17 等标记：WAL 双池、PRAGMA、sysinfo 复用），本报告聚焦**剩余**可优化点，按优先级排列。

## 已确认的良好实践（无需动）

- SQLite：WAL + busy_timeout 5s + synchronous=NORMAL；读写双池（写池 max=1 串行化、读池 max=8）——`infrastructure/factory.rs` L40–L66
- 批量写有事务包裹（`sqlite.rs` 7 处 `.begin()`，覆盖 RBAC 种子 / 批量状态更新等循环写场景）
- 无 `SELECT *`；29 个索引 / 33 张表
- metrics handler 复用 `Arc<Mutex<sysinfo::System>>`（F-17），锁及时 drop
- `AgentClient` / ACME client 复用 `reqwest::Client`；备份后端 IO 已走 `spawn_blocking`（46 处）
- WS 推送用 `interval` + `MissedTickBehavior`，有心跳与空闲超时

## P1 · 值得开单的优化点

### 1. 异步上下文中的同步文件 IO（面广，量 472 处）

`std::fs::` 在非测试代码共 472 处，而 `spawn_blocking` 仅 46 处。SQLite 层已隔离，
但 **application 层多处在 async fn 里直接同步 IO**，会阻塞 tokio worker 线程：

| 位置 | 场景 | 建议 |
|---|---|---|
| `application/certificate_service.rs` L662–L690 | 证书写盘 `create_dir_all`/`write`/`read_to_string` | 小文件、低频，改 `tokio::fs` 即可 |
| `application/project_service.rs` L616–L630 | 部署时读构建产物 `std::fs::read`（可能是大文件） | **优先**：大文件读改 `tokio::fs::read` 或 `spawn_blocking` |
| `application/app_store_service.rs`（多处） | 包解压 / 清单读写 | 解压是 CPU+IO 密集，包一层 `spawn_blocking` |
| `infrastructure/app_store/adapter/*` 三格式适配器 | 包体解析 | 同上 |

切法建议：按域拆 3–4 张 D 类单（certificate / project / app_store），每单机械替换 + 门禁绿，行为零变更。
低频路径（启动初始化、安装器一次性动作）**不必**改，避免为改而改。

### 2. `runtime/download.rs::fetch` 全量缓冲下载（数十 MB 进内存）

> ✅ **已交付**（2026-09-16，与 #3 同一提交）：`fetch` 流式落盘返回 `DownloadedFile`（Drop 清理），
> `archive.rs` 增加 `untar_gz_file`/`untar_xz_file` 文件入口，四个安装器切换；共享 Client 见 #3。

L20–L40：`resp.bytes().await` 把 Node/Go 发行包（30–80 MB）整个读进内存再 `to_vec()`（又复制一次）。
512 MB 内存目标环境下，一次安装即占用 ~160 MB 峰值。

建议：改流式 `resp.chunk().await` → `tokio::fs::File::write_all`，边下边写盘 + 边算 SHA256；
顺带把 `reqwest::Client::new()`（每次调用新建连接池）提为模块级 `OnceLock<Client>`。
规模：D · ~60 行 · 1 文件。**这是本报告性价比最高的一条。**

### 3. `app_store/remote_source.rs` 每请求新建 reqwest Client

> ✅ **已交付**（同上）：新增 `infrastructure/http.rs::shared_client()`（`OnceLock`），两处调用切换。

L260、L350 两处 `reqwest::Client::new()`。远程源同步会连续多次请求，每次重建 TLS 连接池。
建议：与 #2 同一张单，统一 `OnceLock<Client>` 或注入共享 client。

### 4. 非测试代码 unwrap 热点（可靠性优化）

Doc/30 已有 T-02（db.rs 184 处）在队列。本次扫描补充热点清单：

| 文件 | unwrap/expect 数 | 备注 |
|---|---|---|
| `infrastructure/sqlite.rs` | 257 | 多在行映射，T-03/T-04 拆分时顺路收敛 |
| `application/project_service.rs` | 158 | 含部署热路径，panic = 部署中断 |
| `application/backup_job_service.rs` | 127 | 后台任务 panic 会静默丢任务 |
| `application/app_store_service.rs` | 120 | 同上 |
| `runtime/task_state.rs` / `plugin/registry.rs` | 34+ | T-02 验收标准已点名 |

建议：等 T-02 落地 `lock_or_recover` 后，滚动开「clippy unwrap_used 门禁」单（Doc/30 已列）。

## P2 · 顺路可做（不单独开单）

- `infrastructure/runtime/tomcat.rs` L480：`std::thread::sleep(20ms)` 在异步上下文，改 `tokio::time::sleep`（1 行）
- `application/project_service.rs` L1444：`std::process::Command::new("python3")` 同步探测，改 `tokio::process` 或挪到 `spawn_blocking`（探测频度低，P2）
- `SystemResources::detect()`（app_store_service.rs L2353）：每次安装决策 new 一个 `sysinfo::System`——低频、可接受；若后续工具箱域（T-12）落地共享 System，可顺路收编
- `api/app_state.rs` L79/L87：`jwt_secret_store` 用 `std::sync::RwLock<String>`，读多写极少（仅轮换时写），可换 `arc-swap`；但当前无锁争用证据，**不建议主动动**（红线：不引新依赖需论证）

## P3 · 观察项（不动）

- `.lock().await` 38 处：tokio Mutex 跨 await 是合法用法，抽查未见长持有；无争用数据前不动
- `lib.rs` 88 处 clone：组合根一次性构建，克隆 Arc 无成本，误报
- sqlite.rs 循环内逐条 INSERT（RBAC 种子 L1552–L1608 等）：已在事务内，SQLite 单机场景足够

## 建议的开单顺序

1. **[新单] 下载器流式化 + Client 复用**（本报告 #2+#3，D · ~80 行 · 2 文件 · ~60 轮）
2. **T-02**（db.rs 锁中毒，已在任务单库）→ 解锁 unwrap_used 门禁
3. **[新单] project_service 部署路径异步 IO**（#1 优先域，D · ~90 轮）
4. T-03/T-04 拆分时顺路收敛 sqlite.rs 的 unwrap（不扩大范围，仅行映射处）

