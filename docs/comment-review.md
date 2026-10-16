# Comment language review

[简体中文](#中文说明)

## Scope

Reviewed and translated/shortened comments in:

- `flame-kernel/src/main.rs`: early version dispatch, helper/MCP protocol output, log flushing, SQLite default.
- `flame-kernel/src/lib.rs`: composition root, security/credential paths, service injection, task ownership, background effects, initialization, and shutdown.
- `agent/src/main.rs`, `config.rs`, `heartbeat.rs`: module boundaries, structured arguments, connection modes, terminal opt-in, legacy heartbeat compatibility, CPU fallback, and rate semantics.
- `frontend/src/App.vue`: layered error boundaries, out-of-tree notices, readable locale fallback.
- Root Cargo release profile and GitHub workflows: concise build/lifecycle rationale.

No executable Rust/TypeScript/Vue logic, locale values, protocol keys, shell-policy constants, Cargo dependency versions, or lockfiles were intentionally changed. CI/release configuration and the new offline release-asset helper are separate functional changes and have their own validation requirements.

One inaccurate Agent file-root comment was corrected to match `from_env`: the default is the current directory. This is a documentation correction, not a new denial policy. The outbox subscriber comment now explicitly avoids claiming atomic business-transaction/outbox persistence.

## Keep rationale, remove narrative duplication

Preserve why a restriction exists: stdout reserved for MCP, a guard retained for flushing, key loss invalidating old ciphertext, independent Agent boundaries, environment clearing, lifecycle drain, and compatibility. Historical phase labels and descriptions that only repeat code can be removed or shortened. Do not delete `SAFETY`, lint/type directives, license notices, schema documentation, doctests, or tool-consumed markers without separate inspection.

## Remaining inventory

[`comment-audit.csv`](./comment-audit.csv) is a heuristic list of line-leading Chinese comment candidates across Rust, frontend source, and scripts. Reproduce it with:

```bash
python3 scripts/audit-comment-language.py > docs/comment-audit.csv
```

It is **not** a complete parser, proof of dead code, or deletion authorization. It can count comment-like lines inside multiline strings and miss trailing comments. Do not interpret counts as exact unique comments or a translation-completion percentage. Thousands of candidates remain; whole-repository comment translation/review is **not complete**.

Continue module by module: identify actual comments (not user-facing strings or test fixtures), review security/compatibility intent, translate concisely, compare executable code, and run targeted plus final-tree CNB regressions. Prioritize authentication, command execution, filesystem boundaries, migrations, and task lifecycle. Do not let a bulk replacement change raw shell strings, generated contracts, locale content, or assertions.

## 中文说明

本批次只处理内核启动与装配、Agent 入口/配置/心跳、前端 App 入口、根构建配置及 GitHub 工作流中的已审查注释，不宣称全仓完成。

保留安全边界、兼容约束、生命周期与资源守卫的原因说明；精简重复描述和历史阶段标号。不得批量改写字符串、协议键、语言包、测试夹具、工具指令、许可证或 SAFETY 注释。

CSV 是静态候选清单，不是语法分析或可删除列表。后续应逐模块审查并做“不改变可执行代码”的对比与 CNB 回归。完整英文翻译和全部注释清理仍有后续工作，不用候选行数冒充完成率。
