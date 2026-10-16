---
name: flamepanel-rest-endpoint
description: 在 FlamePanel 新增或修改 REST API 端点时使用。当任务涉及 axum 路由、handler、PermissionRule、分页、DTO、utoipa 注解、tests/ 集成测试时使用。给出与 handler/waf 一致的路由 / 权限 / 分页 / 测试模板。
---

# FlamePanel · REST 端点

> 参照：`api/handler/waf/mod.rs` + `api/permissions.rs` 的 waf 段 + `tests/waf.rs`。

## 1. 三件套（缺一即 403 / 测试红）

```rust
// ① api/handler/foo/mod.rs
use crate::api::extract::ApiJson;                      // 统一 JSON 提取（校验错误 → AppError）
use crate::api::pagination::{PaginationParams, PaginatedResponse};
use crate::api::types::AppState;
use crate::core::error::AppError;
use axum::{extract::{Path, Query, State}, routing::{get, post}, Json, Router};
use utoipa::ToSchema;

#[derive(serde::Deserialize, ToSchema)]
pub struct CreateFooRequest { pub name: String, #[serde(default = "default_true")] pub enabled: bool }
fn default_true() -> bool { true }

#[utoipa::path(get, path = "/api/foos", tag = "foo", params(PaginationParams),
    responses((status = 200, body = PaginatedResponse<Foo>)))]
pub async fn list(State(state): State<AppState>, Query(p): Query<PaginationParams>)
    -> Result<Json<PaginatedResponse<Foo>>, AppError> {
    let (data, total) = state.services.foo_service.list(p.offset(), p.page_size()).await?;
    Ok(Json(PaginatedResponse { data, page: p.page(), page_size: p.page_size(), total,
        total_pages: (total + p.page_size() - 1) / p.page_size() }))
}

#[utoipa::path(post, path = "/api/foos", tag = "foo", request_body = CreateFooRequest,
    responses((status = 200, body = Foo)))]
pub async fn create(State(state): State<AppState>, ApiJson(req): ApiJson<CreateFooRequest>)
    -> Result<Json<Foo>, AppError> {
    Ok(Json(state.services.foo_service.create(Foo { name: req.name, enabled: req.enabled, ..Default::default() }).await?))
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/foos", get(list).post(create))
        .route("/api/foos/{id}", get(get_one).put(update).delete(delete_one))   // Axum 0.8：{id}，不是 :id
}
```

```rust
// ② api/routes.rs::create_router
.merge(foo::routes())

// ③ api/permissions.rs（默认拒绝：没注册 = 所有人 403）
PermissionRule::exact("GET",    "/api/foos",  "foo", "read"),
PermissionRule::prefix("GET",   "/api/foos/", "foo", "read"),
PermissionRule::exact("POST",   "/api/foos",  "foo", "create"),
PermissionRule::prefix("PUT",   "/api/foos/", "foo", "update"),
PermissionRule::prefix("DELETE","/api/foos/", "foo", "delete"),
```

新 `resource` 还需：`domain/entity/system.rs::default_permissions()` 加 `("foo","read","查看 foo")` 等 → admin 自动获得；operator / viewer 默认矩阵在同文件角色种子处按现有粒度决定。

## 2. 约定

- 路径：资源复数、kebab-case（`/api/app-store/sources`）、动作用子路径 POST（`/api/waf/logs/clear`）；
- 分页只用 `PaginationParams`（`page/page_size`，上限 200），**禁止**手写 offset 解析；
- 入参 `ApiJson<T>`；出参 `Json<T>` 或 `Json<PaginatedResponse<T>>`；不直接返回含敏感字段的实体（token / 密码 → DTO 脱敏）；
- 404：`AppError::NotFound`；校验：`AppError::ValidationError`；唯一冲突：`AppError::Conflict`；
- 新 DTO / 实体登记到 `openapi.rs` `components(schemas(...))`，新 handler 登记到 `paths(...)`；
- handler 内 **零** `unwrap()/expect()`。

## 3. 集成测试模板（tests/foo.rs）

```rust
use axum::{body::Body, http::{Request, StatusCode}};
use serde_json::json;
use tower::ServiceExt;
mod common;
use common::{auth_header, bad_auth_header, setup_full_router};

async fn send(app: &axum::Router, method: &str, uri: &str, auth: Option<(axum::http::HeaderName, String)>, body: Option<serde_json::Value>)
    -> (StatusCode, serde_json::Value) {
    let mut req = Request::builder().method(method).uri(uri).header("content-type", "application/json");
    if let Some((k, v)) = auth { req = req.header(k, v); }
    let res = app.clone().oneshot(req.body(Body::from(body.map(|b| b.to_string()).unwrap_or_default())).unwrap()).await.unwrap();
    let status = res.status();
    let bytes = axum::body::to_bytes(res.into_body(), usize::MAX).await.unwrap();
    (status, serde_json::from_slice(&bytes).unwrap_or(json!(null)))
}

#[tokio::test]
async fn foo_crud_happy_path() {
    let app = setup_full_router().await;
    let (st, body) = send(&app, "POST", "/api/foos", Some(auth_header()), Some(json!({"name":"a"}))).await;
    assert_eq!(st, StatusCode::OK);
    let id = body["id"].as_i64().unwrap();
    let (st, body) = send(&app, "GET", &format!("/api/foos/{id}"), Some(auth_header()), None).await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(body["name"], "a");
}

#[tokio::test]
async fn foo_rejects_invalid_and_unauth() {
    let app = setup_full_router().await;
    let (st, _) = send(&app, "POST", "/api/foos", Some(auth_header()), Some(json!({"name":""}))).await;
    assert_eq!(st, StatusCode::BAD_REQUEST);                                   // ValidationError → 400
    let (st, _) = send(&app, "GET", "/api/foos", None, None).await;
    assert_eq!(st, StatusCode::UNAUTHORIZED);
    let (st, _) = send(&app, "GET", "/api/foos", Some(bad_auth_header()), None).await;
    assert!(matches!(st, StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN));
    let (st, _) = send(&app, "GET", "/api/foos/999999", Some(auth_header()), None).await;
    assert_eq!(st, StatusCode::NOT_FOUND);
}
```

> `setup_full_router` 构造完整 `Services`；新增 Service 字段后 **`tests/common/mod.rs` 两个 setup 函数都要注入**，否则整个 tests 目录编译失败。

## 4. 自查

- [ ] routes / permissions / default_permissions / openapi 四处齐
- [ ] 每端点 ≥ 正向 + 401 + 403/404 三条断言
- [ ] `cargo run -p flame-kernel -- --dump-openapi > ../frontend/openapi.json`（若已实现）或手动确认 `/api/openapi.json` 含新路径
- [ ] Doc/02 加端点行（方法 / 路径 / 权限 / 请求 / 响应）

