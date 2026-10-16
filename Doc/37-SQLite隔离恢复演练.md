# 37 · SQLite 隔离恢复演练（R-01 首片）

## 1. 范围与完成含义

本片提供离线演练工具和自动化回归，接受 FlamePanel 已导出的独立 SQLite 快照。**PASS 只表示快照获取、完整性 / 最小 schema 验证、隔离复制、SQLite 重开和固定表抽样通过，不表示整个面板已恢复可用。**

不启动生产内核、不执行迁移、不调度任务、不连接 Agent、不发邮件、不连接外部数据库，不导入 UI / REST / MCP，也不改变现有生产恢复方法。需要 Python 3.11+ 和 Linux；仅使用标准库，无新增依赖或构建配置变更。

工具不是备份来源真实性鉴定器；请使用自己的可信备份。对远端来源建议先通过既有授权下载流程获取文件，再在受限开发环境运行。SQLite 文件本身含密码哈希和凭据密文，应按敏感数据保管。

## 2. 用法

在仓库根目录，先准备一个已存在、权限受控的父目录；`--out` 指定的子目录必须不存在：

```bash
python3 scripts/restore-drill/drill.py \
  --backup /secure/export/flamepanel-snapshot.db \
  --out /secure/reports/drill-unique-run
```

可选参数：

- `--sha256 <64位十六进制>`：从可信渠道获得的预期校验和；不符则恢复前失败。不提供时报告仅记录实算 SHA-256，`checksum_verified=false`，不能将其视为来源认证。
- `--backup-time <ISO-8601含时区时间>`：备份完成时间，不能在未来；只计算操作员声明的备份年龄，不推断真实 RPO。
- `--max-bytes`：默认 512 MiB，允许 1 字节至 8 GiB。选择与环境内存、磁盘相适配的上限。
- `--timeout`：默认 120 秒，上限 3600 秒。检查单调时钟期限；SQLite progress handler 中断长查询。HTTPS 单次网络等待不超过 5 秒，整体仍受系统 DNS / 文件系统调用行为影响，不是硬实时隔离保证。
- `--url https://…`：替代 `--backup`，只允许 HTTPS、验证服务器证书、拒绝重定向和 URL 中的用户名 / 密码；不使用环境代理。不支持 S3 签名、SFTP、备份账号凭据或自动重试。鉴权下载优先在现有授权工具中完成，避免令牌进入命令行 / shell 历史。

退出码：`0` = 离线演练 PASS；`1` = 演练失败（检查报告）；`2` = 参数错误或报告目录无法安全建立 / 写入。不要覆盖旧报告后重复运行，请使用新的目录名。

## 3. 安全与隔离规则

- 不接收恢复目标路径；恢复目标只能在工具自行创建的私有随机子目录内。
- 输出目录以 0700 创建且必须不存在；报告与快照副本 0600。已有目录、文件或符号链接不能被报告覆盖。
- 本地源用只读、no-follow、non-blocking 打开，要求普通文件；拒绝符号链接、FIFO 与存在 WAL / SHM / rollback journal 旁文件的活库。
- 只支持独立备份，不支持直接复制运行中的 `app.db`。源文件大小 / 修改时间 / 变化时间发生变化时拒绝继续；这不是在线一致性快照算法。
- 获取时检查空间与大小，恢复前按副本大小再次检查；预留 8 MiB 余量。磁盘可用量可能并发变化，因此真实写入 ENOSPC 仍会失败并清理。
- 不把原文件交给 SQLite；只读打开隔离副本，关闭 trusted schema，启用 query-only；不开启扩展或执行备份中的任务定义。
- 正常结束、已捕获异常、SIGTERM / Ctrl-C 会关闭连接并删除副本，只留报告。SIGKILL、主机宕机、权限突变可能留下 `.isolated-*`，工具不承诺这些情况下安全擦除；在私有输出目录检查并按保密策略清理。
- 报告不包含输入路径、完整 URL、查询串、用户名、密码哈希、设置值或原始异常。只包含固定错误码、摘要、大小、阶段耗时与抽样行数。报告摘要和行数仍属于运维元数据，应受权限保护。

## 4. 报告阶段

`report.json` schema：`flamepanel/restore-drill/v1`；同时生成可读的 `report.md`。

| 阶段 | 行为 | 典型失败 |
| --- | --- | --- |
| acquire | 有界获取到私有 staging，计算摘要 | SOURCE_UNAVAILABLE、LIVE_DATABASE_REFUSED、SIZE_LIMIT、REMOTE_TRUNCATED |
| validate | 可选可信摘要比对、SQLite header / integrity_check、核心表最小列集合 | CHECKSUM_MISMATCH、NOT_SQLITE、SQLITE_CHECK_FAILED、SCHEMA_MISMATCH |
| capacity | 再次检查恢复副本空间 | INSUFFICIENT_SPACE |
| restore | 私有目录复制到临时文件，fsync，摘要比对后 rename | INSUFFICIENT_SPACE、COPY_MISMATCH、IO_FAILED |
| reopen | 关闭前序连接后只读重开恢复库，重新检查 | SQLITE_CHECK_FAILED、SCHEMA_MISMATCH |
| sample | 对 users / permissions / panel_settings 固定列按主键读取最多 20 行，与 staging 比较 | SAMPLE_MISMATCH、TIMEOUT |

阶段失败后，后续阶段为 `NOT_RUN`，总结果 FAIL；无跳过后继续报成功的逻辑。最小 schema 不是完整版本兼容检查；没有调用当前迁移，也不证明所有历史版本都可升级。

- `elapsed_seconds`：包含获取与前检的本轮耗时。
- `storage_rto_seconds`：恢复复制开始到重开 / 抽样通过的耗时；失败时为空。**不是生产业务 RTO。**
- `production_rto_seconds`：本片始终为空。
- `rpo_seconds`：本片始终为空。无业务写入时间、水位、备份完成时间等证据，不能仅凭文件 mtime 或完整性检查得出 RPO。
- `backup_age_seconds`：可选，仅根据操作员提供时间计算；不等于数据丢失窗口。

外部 `data/secret_encryption.key` 等关键文件不在 SQLite 快照内；本片不会读取或上传这些密钥，不验证凭据可解密。文件存储、网站目录、容器卷和远端数据库不在本片范围。

## 5. 自动化验收

必须在 CNB 云原生开发的独立目录运行，不修改 `/workspace`，不触发普通云构建：

```bash
python3 -m unittest discover -s scripts/restore-drill -v
cargo test --locked -p flame-kernel --no-default-features --features sqlite --test restore_drill
bash scripts/ci-local.sh --keep-going all
```

`flame-kernel/tests/restore_drill.rs` 随现有后端测试自动执行，无需改 CI：

1. 执行 Python 合约用例，包括正常、损坏、预检空间不足 / 写入 ENOSPC、远端失败，以及路径、大小、截止时间、中断、摘要、报告脱敏等护栏。
2. 使用真实 `run_migrations` 建立合成数据库，以真实 `BackupService::create_backup` 在 WAL 写池仍打开时生成快照。
3. 对真实快照调用离线工具，断言报告和原文件未修改。
4. 在另一个已停止连接池的临时数据库调用真实 `restore_backup_from_path`，重新读取快照值，并确认 `pre-restore-*` 包含恢复前数据；检查源库和快照字节不变。

空间不足通过 `disk_usage` 与 ENOSPC 故障注入测试，不填满 CNB 宿主机磁盘。远端失败单测用网络异常注入；额外真实 HTTPS 连接拒绝可作为本轮证据，但不等于 S3 / SFTP 验收。

可选 `FP_RESTORE_DRILL_ARTIFACTS=/root/new-unique-directory` 导出 Rust 用例中的**合成**快照及报告；目录必须不存在。完整门禁时不要沿用该环境变量，否则重复运行会因拒绝覆盖而失败。

## 6. 下一片验收前置

本片不声称 R-01 全部完成。全内核启动与 HTTP 验证应在下一片设计中解决：恢复库可能含可执行定时任务、自动续签、节点及通知配置，不能拿生产备份直接启动普通内核来“试一下”。

下一片先明确安全的启动模式或容器 / 网络 / 特权隔离方案、后台任务禁止清单、外部密钥处理、认证探针、升级迁移副本、完整恢复资产清单和真实 RTO / RPO 的测量定义。必须评审后再实施，不在本片隐式启用生产副作用。

## 7. R-01B 可选迁移与 HTTP 探针

用户继续授权后新增 [Doc/38 专用恢复探针](./38-恢复探针启动隔离.md)。默认模式仍离线；显式 `--probe-bin` 才在额外副本上执行迁移与只读 HTTP 检查。不启动生产内核，不验证生产鉴权或外部密钥，不把探针耗时当作生产 RTO。
