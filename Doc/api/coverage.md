<!-- 由 scripts/gen-api-docs.py 从 frontend/openapi.json 生成，请勿手改；修改 utoipa 注解后运行 just api-docs -->

# API 文档覆盖率

> 代码路由 **328** 个（方法级，不含 `/ws/*`）· 已文档化 **328** · 覆盖率 **100.0%**

补注解流程：handler 加 `#[utoipa::path(...)]` → 在 `flame-kernel/src/openapi.rs` 的 `paths(...)` 登记 → `just openapi && just api-docs` → 提交。清单由 CI 与生成物一致性检查保持最新。

| 域 | 未文档化 |
|---|---|
