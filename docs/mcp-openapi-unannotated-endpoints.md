# 未注解 utoipa 端点清单（已迁移）

> 本文件为 T-05（2026-09-16）的一次性快照，**已停止维护**。

实时清单见 [`Doc/api/coverage.md`](../Doc/api/coverage.md)：由 `scripts/gen-api-docs.py`
扫描代码路由与 `frontend/openapi.json` 对比生成，CI（installer-regression · `api docs up to date`）
保证其与代码同步。

补注解流程：handler 加 `#[utoipa::path(...)]` → 在 `flame-kernel/src/openapi.rs` 的 `paths(...)`
登记 → `just openapi && just api-docs` → 提交。
