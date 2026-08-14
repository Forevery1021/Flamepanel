//! OpenAPI 文档（Stage3.3）。
//!
//! 聚合关键路由的 `#[utoipa::path]` 注解与核心 DTO（`ToSchema`），编译期生成
//! OpenAPI 3.x 文档。组合根挂载 `/api/openapi.json` 与（可选 feature）`/api/swagger-ui`。

use utoipa::openapi::security::{HttpAuthScheme, HttpBuilder, SecurityScheme};
use utoipa::{Modify, OpenApi};

/// OpenAPI 文档（核心路由 + DTO）。
#[derive(OpenApi)]
#[openapi(
    info(
        title = "FlamePanel API",
        description = "FlamePanel 面板后端 REST API（OpenAPI 3.x，由 utoipa 编译期生成）",
        version = crate::VERSION
    ),
    servers(
        (url = "/", description = "默认")
    ),
    paths(
        // auth
        crate::api::handler::auth::login,
        crate::api::handler::auth::refresh,
        // health
        crate::api::handler::health::detail,
        // users
        crate::api::handler::user::list,
        crate::api::handler::user::create,
        crate::api::handler::user::update,
        crate::api::handler::user::delete,
        // nodes
        crate::api::handler::node::list,
        crate::api::handler::node::create,
        crate::api::handler::node::register_agent,
        crate::api::handler::node::heartbeat,
        crate::api::handler::node::status,
        crate::api::handler::node::remote_execute,
        crate::api::handler::node::remote_action,
        crate::api::handler::node::batch_execute,
        crate::api::handler::node::remote_list_files,
        crate::api::handler::node::remote_download_file,
        crate::api::handler::node::remote_upload_file,
        // websites
        crate::api::handler::website::list,
        crate::api::handler::website::create,
        crate::api::handler::website::get,
        crate::api::handler::website::update,
        crate::api::handler::website::delete,
        // settings
        crate::api::handler::settings::list_settings,
        crate::api::handler::settings::update_setting,
        // backups
        crate::api::handler::backup::list_backups,
        crate::api::handler::backup::create_backup,
        crate::api::handler::backup::delete_backup,
        // scheduled tasks
        crate::api::handler::scheduled_task::list_tasks,
        crate::api::handler::scheduled_task::create_task,
        crate::api::handler::scheduled_task::update_task,
        crate::api::handler::scheduled_task::delete_task,
        crate::api::handler::scheduled_task::run_task,
        // app-store
        crate::api::handler::app_store::list_packages,
        crate::api::handler::app_store::list_installed,
        crate::api::handler::app_store::batch_import_packages,
        // tasks（统一 Task 进度查询/取消，Phase B1）
        crate::api::handler::task::list_tasks,
        crate::api::handler::task::get_task,
        crate::api::handler::task::cancel_task,
        crate::api::handler::task::prune_tasks,
        // metrics
        crate::api::handler::metrics::processes,
        // operation logs
        crate::api::handler::operation_log::export,
        // outbox（事件落库）
        crate::api::handler::outbox::list,
    ),
    components(schemas(
        crate::domain::entity::User,
        crate::domain::entity::ServerNode,
        crate::domain::entity::Website,
        crate::domain::entity::ScheduledTask,
        crate::domain::entity::AppMetadata,
        crate::domain::entity::InstalledApp,
        crate::domain::entity::AppFormat,
        crate::domain::entity::InstallMode,
        crate::api::types::CreateUserRequest,
        crate::api::types::UpdateUserRequest,
        crate::api::types::CreateNodeRequest,
        crate::api::types::CreateWebsiteRequest,
        crate::api::types::PaginationParams,
        crate::api::types::PaginatedResponse<crate::domain::entity::User>,
        crate::api::types::PaginatedResponse<crate::domain::entity::ServerNode>,
        crate::api::types::PaginatedResponse<crate::domain::entity::Website>,
        crate::api::types::PaginatedResponse<crate::domain::entity::OutboxEvent>,
        crate::domain::entity::OutboxEvent,
        crate::api::handler::outbox::OutboxListQuery,
        crate::api::handler::auth::LoginRequest,
        crate::api::handler::auth::LoginResponse,
        crate::api::handler::node::HeartbeatRequest,
        crate::api::handler::node::RemoteExecRequest,
        crate::api::handler::node::RemoteBatchExecRequest,
        crate::api::handler::node::RemoteActionRequest,
        crate::api::handler::node::RemoteListQuery,
        crate::api::handler::node::RemoteUploadRequest,
        crate::api::handler::health::HealthDetail,
        crate::api::handler::health::HealthChecks,
        crate::api::handler::health::HealthCheckItem,
        crate::api::handler::settings::SettingEntry,
        crate::api::handler::settings::UpdateSettingRequest,
        crate::api::handler::backup::BackupEntryDto,
        crate::api::handler::scheduled_task::CreateTaskRequest,
        crate::api::handler::scheduled_task::UpdateTaskRequest,
        crate::api::handler::app_store::AppStoreListResponse,
        crate::api::handler::app_store::InstalledAppResponse,
        crate::api::handler::app_store::BatchImportPackagesRequest,
        crate::api::handler::app_store::BatchImportPackagesResponse,
        crate::api::handler::task::TaskListResponse,
        crate::runtime::task_state::TaskRecord,
        crate::runtime::task_state::TaskState,
        crate::runtime::task_state::TaskKind,
        crate::api::handler::metrics::ProcessEntry,
    ))
)]
pub struct ApiDoc;

/// JWT Bearer 认证（OpenAPI security scheme）。
pub struct SecurityAddon;

impl Modify for SecurityAddon {
    fn modify(&self, openapi: &mut utoipa::openapi::OpenApi) {
        let components = openapi.components.as_mut().unwrap();
        components.add_security_scheme(
            "BearerAuth",
            SecurityScheme::Http(
                HttpBuilder::new()
                    .scheme(HttpAuthScheme::Bearer)
                    .bearer_format("JWT")
                    .build(),
            ),
        );
    }
}

/// 生成 OpenAPI 文档 JSON（进程级缓存）。
pub fn openapi_json() -> serde_json::Value {
    static CACHE: std::sync::OnceLock<serde_json::Value> = std::sync::OnceLock::new();
    CACHE
        .get_or_init(|| {
            let mut doc = ApiDoc::openapi();
            SecurityAddon.modify(&mut doc);
            serde_json::to_value(&doc).unwrap_or_default()
        })
        .clone()
}

/// OpenAPI 路由：`GET /api/openapi.json`（始终可用）。
pub fn openapi_router() -> axum::Router<crate::api::types::AppState> {
    use axum::routing::get;
    axum::Router::new().route(
        "/api/openapi.json",
        get(|| async { axum::Json(openapi_json()) }),
    )
}

/// 无 Swagger UI 时返回空 Router（保持组合根签名一致）。
/// 说明：为避免额外依赖与 CDN 路径，交互式 UI 由前端调用 /api/openapi.json 自行渲染。
pub fn swagger_ui_router() -> axum::Router<crate::api::types::AppState> {
    axum::Router::new()
}
