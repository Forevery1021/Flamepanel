# FlamePanel · Agent 工作约定

> 保持本文 ≤ 120 行。详细架构见 Doc/01，流程见 Doc/31，任务工序见 `.codebuddy/skills/`；不要在入口复制历史测试数和门禁阈值。

## 0. 一分钟定位

- 产品：Rust + Vue 的自托管 Linux 服务器面板，MIT。
- 设计方向：`api → application → domain ← infrastructure`；领域层允许 serde / chrono 等类型工具，不做 I/O；当前边界债务见 Doc/35，新增代码不得扩大具体基础设施耦合。
- 组合根：`flame-kernel/src/lib.rs::build_services`；生产后端由 `main.rs` 的 `config.database.backend` 选择，默认 SQLite。
- 路由 / 权限：`api/routes.rs`、`api/permissions.rs`；迁移：`infrastructure/sqlite/mod.rs::run_migrations`；实时数据复用 `MetricsHub`。
- 前端：Pinia + VueQuery + i18n，业务页只用 `Fp*`；页面做组装，composables / utils 承担可测试逻辑。
- Agent 已模块化：`config / heartbeat / server / executor / install_policy / files / pty / edge`，不是单文件。
- Rust 精确基线由 `rust-toolchain.toml` 维护；workspace `rust-version` 声明受支持最低基线。Node 见 `.node-version`，npm 见 `frontend/package.json::packageManager`，不要浮动 stable 或随手升级锁文件。

## 1. 范围与会话纪律

- 先明确目标、涉及文件和禁改清单；有指定 Issue 就记录计划，无编号不杜撰 Ref。
- 同一错误三轮未解决就止损，记录真实失败和已试方案；禁止以删测试或弱化断言“修复”。
- 大文件先 grep 符号再读目标范围；不整读仓储 / 权限巨文件，不猜路径、函数签名或 API。
- 本轮工程清理只修装配、工具链和文档；运行时死代码 / 重复逻辑先列 Doc/35，未经批准不删除或合并，不新增功能。

## 2. 门禁与执行环境

```bash
bash scripts/ci-local.sh quick
bash scripts/ci-local.sh --keep-going all
```

- 命令与分组以 `scripts/ci-local.sh` 为准；Rust 组首先运行 `scripts/check-rust-toolchain.sh`，保持 `--locked`、clippy `-D warnings` 与既有测试范围。
- 开发 / 测试使用 CNB 云原生开发环境，操作见 Doc/31 §5.3；不改写 `/workspace` 内他人的未提交工作，使用隔离目录并在停止前保存证据。
- “轻量流水线”仍消耗构建额度。用户要求零云构建时，不以 push / PR / tag / 普通 build-start 验证；push CI skip 不覆盖 PR 事件。
- 全量绿不等于性能、安全审计、跨架构发布或真实系统验收；明确写出未执行项目。

## 3. 代码红线

1. handler 使用 `AppError`，不泄露内部错误链或秘密；不直接创建具体基础设施。
2. REST 变更同时检查 routes、权限表、401 / 403 回归、OpenAPI 注解 / 注册与生成契约。
3. SQLite 迁移幂等，新增列考虑默认值；验证老库与 RBAC 种子。
4. 长任务走任务中心 / `TaskSupervisor`，保持取消、超时、关闭与失败日志。
5. 外部命令经 `SharedCommandRunner` 和既有执行策略，不拼接不可信 shell 参数；helper、Agent 白名单 / 路径边界不绕过。
6. 秘密使用既有哈希 / `SecretCipher` 策略，日志脱敏；不落盘或提交令牌。兼容 / 降级策略的改变必须独立评审。
7. 前端保持 `Fp*`、后端同名权限、三语 key、生成类型和 `--fp-*` 令牌；不加 `as any`、忽略类型或非语义 E2E 选择器。
8. 测试真实装配：每次 mount 新 Pinia，按需注入生产插件并卸载；不屏蔽 `console.warn` 来掩盖缺依赖。
9. 不强推共享分支，不引入未声明依赖，不删除动态 / schema / RAII / feature / 兼容代码来凑清理数量。

## 4. 导航

| 任务                | 先读                                                          |
| ------------------- | ------------------------------------------------------------- |
| 核心架构 / 安全边界 | Doc/01、Doc/35                                                |
| Issue / 交付流程    | Skill `flamepanel-issue-execution`、Doc/31 §1–§4、§9          |
| 后端 / API          | backend-module / rest-endpoint Skill、Doc/05、Doc/07、Doc/api |
| SQLite              | sqlite-migration Skill、Doc/03                                |
| 前端                | frontend-view Skill、Doc/04、`frontend/design.md`             |
| 门禁                | quality-gates Skill、Doc/06、`ci-local.sh`                    |
| 应用包              | app-package Skill、Doc/20–22                                  |
| 部署 / 兼容         | Doc/10、Doc/12                                                |
| Agent / 事件        | Doc/23、Doc/24                                                |
| 下一步              | Doc/30；历史计划查 Doc/32，不据历史“待办”自动加功能           |

## 5. 交付

- 分支与提交按 Doc/31；真实 Issue 才填写 `Ref: #<issue>`，不改历史。
- 给出变更清单、实现说明、最终源码基线、实际命令 / 结果、未解决项与后续建议。
- API / 表 / 权限 / 部署变更按 Doc/31 §7 同步对应文档；交付历史进 CHANGELOG，不重复堆进所有入口。
- 未授权外部提交或可能触发云构建时，交付工作分支 / patch 与审查报告，不谎称已经推送、合并或发布。
