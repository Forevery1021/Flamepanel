---
name: flamepanel-quality-gates
description: 用于 FlamePanel 门禁执行与故障定位。以 ci-local.sh 为命令来源，保留真实装配、固定工具链与全部质量约束，不通过忽略测试或告警过关。
---

# FlamePanel · 门禁与 CI 修复

## 1. 执行入口

在 CNB 云原生开发环境、仓库根目录执行：

```bash
bash scripts/ci-local.sh quick
bash scripts/ci-local.sh --keep-going all
# 定点：从 ci-local.sh --help 选择 installer / backend / backend-features / frontend / e2e
```

- Rust 精确版本：`rust-toolchain.toml`；先跑 `bash scripts/check-rust-toolchain.sh`，不要浮动 stable 或用环境 override 绕开。
- 锁文件：Rust 构建 / 测试保留 `--locked`，前端使用 `npm ci`，不因修门禁升级依赖。
- 覆盖率来源：`frontend/vitest.config.ts::coverage.thresholds`；包体预算来源：`frontend/scripts/check-bundle-size.mjs`。不复制容易过期的阈值数字。
- `all` 不含性能压测、Rust cargo-audit、全部跨架构发布 / 真机验证；需要时单独运行并记录。
- GitHub 的 E2E 是独立 job；不能把 `frontend` job、`ci-local frontend` 和完整 `all` 混为一谈。
- 用户要求零云构建时，不推送 / 开 PR / 打 tag 或调用 build-start 做测试。开发环境并非普通 CI，结束前保存日志并停止。

## 2. 高频故障

| 问题                     | 正确处理                                                                         | 禁止                                     |
| ------------------------ | -------------------------------------------------------------------------------- | ---------------------------------------- |
| Rust 版本漂移            | 检查 rustup override / `RUSTUP_TOOLCHAIN`，回到固定版本                          | 追最新 stable 或弱化版本守卫             |
| unused / dead_code       | 查完整引用、宏、feature、schema、外部 API；候选先列 Doc/35                       | 直接扫删公有接口；新增 allow 吞告警      |
| Services 缺字段          | 生产 `build_services` 与测试 `setup_router` 对齐；`setup_full_router` 已委托后者 | 以旧文档假设两套装配；删测试             |
| `Symbol(pinia)` 注入告警 | 每次 mount 的 plugins 放新 `createPinia()`，按需复现生产插件，结束卸载           | 全局共享 active Pinia；静音 console.warn |
| 偶发测试失败             | 查端口 / 临时目录 / store / timer 泄漏，定点复现后验证并行全量                   | ignore / skip / 只留下串行成功结果       |
| 生成类型或快照漂移       | 按 OpenAPI 导出 / 生成流程更新并检查差异                                         | 手写替代类型、`as any`、ts-ignore        |
| 覆盖率 / 包体 / i18n     | 给实际分支补回归、按需加载、补三语 key                                           | 降门槛、删翻译调用、关闭规则             |
| 审计漏洞                 | 独立评审最小升级与锁文件；不能升级则明确阻塞项                                   | 静默忽略或把未执行说成零漏洞             |

## 3. 定点定位与收尾

```bash
cargo test --locked -p flame-kernel <关键字> -- --nocapture
cd frontend && npx vitest run <spec路径> --reporter=verbose
```

一轮只修一个根因；同一报错三轮止损并记录完整失败，不改断言迎合实现。最后检查：实际命令 / 结果、最终 revision / dirty 标记、diff 范围、锁文件、未执行检查、现存警告；完整汇总必须对应最终工作树。
