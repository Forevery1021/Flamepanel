---
name: flamepanel-backend-module
description: 在 FlamePanel 新增后端业务域时使用（如 toolbox、certificate_renewal、mcp、runtime 新语言安装器等）。当任务涉及「新建域」「新增 XxxService / XxxRepository」「四层骨架」时使用。给出与当前代码一致的文件清单、代码骨架、组合根接线与测试基建。
---

# FlamePanel · 新增后端业务域

> 参照域：**`waf`**（最新、最小、四层齐全、有集成测试）。先 `grep -rn "Waf" flame-kernel/src --include=*.rs -l` 看它碰了哪些文件，本 Skill 与其同构。

## 1. 文件清单（新域 `foo`）

| 层 | 文件 | 说明 |
|---|---|---|
| domain | `domain/entity/<域>.rs` 追加 `pub struct Foo` + `impl Default` + 校验方法 | 实体按域分文件（`system` / `app` / `site` / `infra` / `project` / `backup` / `plugin` / `metrics` / `event`）；新域则新建 `<域>.rs`，并在 `entity/mod.rs` 加 `mod <域>;` + `pub use <域>::*;`，对外路径仍是 `crate::domain::entity::Foo` |
| domain | `domain/repository.rs` 追加 `pub trait FooRepository` | `#[async_trait]`，`Send + Sync`，返回 `Result<_, AppError>` |
| infrastructure | `infrastructure/db.rs` 追加 `InMemoryFooRepository` | 测试与 memory 后端用；锁用 `crate::utils::sync::lock_or_recover`（若尚未存在则 `lock().unwrap()` 与现有一致） |
| infrastructure | `infrastructure/sqlite.rs` 追加 `SqliteFooRepository` + `run_migrations` 内 DDL | 见 Skill sqlite-migration |
| infrastructure | `infrastructure/factory.rs` 追加 `create_foo_repo()` | 按 `create_waf_rule_repo` 同款写法（match backend） |
| application | `application/foo_service.rs` + `application/mod.rs` 声明 | Service 只编排，不写 SQL |
| api | `api/handler/foo/mod.rs` + `api/handler/mod.rs` 声明 | `routes()`、DTO（`#[derive(Deserialize, ToSchema)]`）、handler |
| api | `api/routes.rs` `.merge(foo::routes())` | 一行 |
| api | `api/permissions.rs` 追加 `PermissionRule` | 见 Skill rest-endpoint |
| api | `api/types.rs` `Services` 结构体加 `pub foo_service: Arc<FooService>` | 组合根依赖 |
| 组合根 | `lib.rs::build_services` 构造并放入 `Services` | 参考 L211 `waf_service` |
| openapi | `openapi.rs` 的 `paths(...)` / `components(schemas(...))` 登记 | 否则前端拿不到类型 |
| domain | `domain/entity/system.rs::default_permissions()` 加 `("foo","read","…")` 等 | RBAC 种子 |
| tests | `tests/foo.rs` + `tests/common/mod.rs::setup_router` 注入 InMemory 仓储 | 否则集成测试 Services 缺字段编译失败 |
| Doc | Doc/02（端点）、Doc/03（表）、Doc/07（权限） | 各加行 |

## 2. 骨架

```rust
// domain/entity/<域>.rs
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Foo {
    pub id: i64,
    pub name: String,
    pub enabled: bool,
    pub created_at: String,          // 现有实体用 String 存 ISO 时间（对齐 WafRule）
}
impl Foo {
    pub fn validate(&self) -> Result<(), AppError> {
        if self.name.trim().is_empty() { return Err(AppError::ValidationError("name 不能为空".into())); }
        Ok(())
    }
}

// domain/repository.rs
#[async_trait]
pub trait FooRepository: Send + Sync {
    async fn create(&self, foo: &Foo) -> Result<Foo, AppError>;
    async fn find_by_id(&self, id: i64) -> Result<Option<Foo>, AppError>;
    async fn list(&self, offset: i64, limit: i64) -> Result<(Vec<Foo>, i64), AppError>;
    async fn update(&self, foo: &Foo) -> Result<(), AppError>;
    async fn delete(&self, id: i64) -> Result<(), AppError>;
}

// application/foo_service.rs
pub struct FooService { repo: Arc<dyn FooRepository> }
impl FooService {
    pub fn new(repo: Arc<dyn FooRepository>) -> Self { Self { repo } }
    pub async fn create(&self, mut foo: Foo) -> Result<Foo, AppError> {
        foo.validate()?;
        self.repo.create(&foo).await
    }
    pub async fn get(&self, id: i64) -> Result<Foo, AppError> {
        self.repo.find_by_id(id).await?.ok_or_else(|| AppError::NotFound(format!("foo {id} 不存在")))
    }
}
```

`AppError` 变体：`Unauthorized / Forbidden / NotFound / BadRequest / ValidationError / Conflict / RateLimited / ServiceUnavailable` + `AppError::internal(msg)`。**不要**新造变体。

## 3. 组合根接线（lib.rs）

```rust
// build_services 内，参照 waf_service 段：
let foo_service = Arc::new(FooService::new(factory.create_foo_repo()));
// Services { ..., foo_service, ... }
```

`tests/common/mod.rs::setup_router` 与 `setup_full_router` 都构造 `Services`——**两处都要加**，否则 `cargo test` 编译失败。

## 4. 测试最低要求

- `foo_service.rs` 内 `#[cfg(test)]`：用 `InMemoryFooRepository` 测校验拒绝 + 正常创建 + NotFound；
- `tests/foo.rs`：每个端点 200 + 401 + 403（模板见 Skill rest-endpoint）。

## 5. 自查

- [ ] `cargo build` 过 = mod 链完整
- [ ] `grep -rn "foo" api/permissions.rs domain/entity/ openapi.rs tests/common/mod.rs` 四处都有
- [ ] 长任务走 `TaskSupervisor`；外部调用走 `SharedCommandRunner` / `resilience`
- [ ] 无 `unwrap()` 在非测试代码

