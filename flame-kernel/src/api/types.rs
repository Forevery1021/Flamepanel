//! T9：`api/types.rs` 上帝文件拆分（原 1867 行）为「每域一文件」。
//!
//! 本文件保留为**兼容再导出层**，统一 `pub use` 各拆分模块的类型/函数，
//! 确保既有 `use crate::api::types::*` 及精确引用零改动继续可用。

// ── AppState / 服务聚合 / 请求上下文 ─────────────────────
pub use crate::api::app_state::{AppState, Services, UserId, Username};

// ── 请求 / 响应 DTO ──────────────────────────────────────
pub use crate::api::dto::{
    CreateNodeRequest, CreateUserRequest, CreateWebServerInstanceRequest, CreateWebsiteRequest,
    PluginMetricsResponse, PluginReloadRequest, PluginSettingRequest, UpdateUserRequest,
    WebServerResponse,
};

// ── 权限映射声明式表（Stage3.4）──────────────────────────
pub use crate::api::permissions::{route_permission, PermissionRule, ROUTE_PERMISSIONS};

// ── Pagination ────────────────────────────────────────────
pub use crate::api::pagination::{PaginatedResponse, PaginationParams};
