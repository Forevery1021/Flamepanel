//! M-AI-2 写操作审批（Doc/34 §审批）。
//!
//! | 端点 | 权限 | 说明 |
//! |---|---|---|
//! | `GET /api/mcp/approvals` | `mcp:read` | 审批单分页（最新在前），可按状态 / 凭证过滤 |
//! | `GET /api/mcp/approvals/{id}` | `mcp:read` | 审批单详情（含预览与执行结果） |
//! | `POST /api/mcp/approvals/{id}/approve` | `mcp:update` | 批准并立即执行 |
//! | `POST /api/mcp/approvals/{id}/reject` | `mcp:update` | 拒绝（可附理由，AI 经 `approval_get` 可见） |
//!
//! **批准的附加校验**（字段级授权，均在执行前完成）：
//! 1. 批准者本人须具备该工具等价 REST 权限（如 `docker:stop`）；高危工具另需 `mcp:delete`。
//! 2. 发起凭证仍有效且开启 `allow_write`；签发者**当前**角色仍具备该工具权限。
//! 3. 状态机 `pending → executing` 为比较并设置，并发批准只有一个会执行。
use super::tools::{self, Caller, RiskLevel};
use crate::api::app_state::{UserId, UserRole, Username};
use crate::api::types::{AppState, PaginatedResponse, PaginationParams};
use crate::core::error::AppError;
use crate::domain::mcp::{ApprovalFilter, ApprovalStatus, ToolApproval, ToolCallRecord};
use axum::{
    extract::{Path, Query, State},
    Extension, Json,
};
use serde::Deserialize;
use serde_json::Value;
use utoipa::{IntoParams, ToSchema};

/// 批准后执行的超时（人工触发，允许比协议调用更长，如站点下发 + reload）。
const APPROVED_EXEC_TIMEOUT_SECS: u64 = 120;

#[derive(Debug, Deserialize, IntoParams)]
pub struct ApprovalQuery {
    /// 按状态过滤：pending / executing / succeeded / failed / rejected / expired
    pub status: Option<String>,
    pub principal_id: Option<i64>,
    pub page: Option<i64>,
    pub page_size: Option<i64>,
}

#[derive(Debug, Default, Deserialize, ToSchema)]
pub struct RejectRequest {
    /// 拒绝理由（≤ 500 字节，回传给 AI）
    #[serde(default)]
    pub reason: Option<String>,
}

/// `GET /api/mcp/approvals` — 审批单分页
#[utoipa::path(
    get,
    path = "/api/mcp/approvals",
    tag = "mcp",
    params(ApprovalQuery),
    responses(
        (status = 200, description = "审批单分页（最新在前）", body = PaginatedResponse<ToolApproval>),
        (status = 400, description = "status 不合法"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 mcp:read 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn list_approvals(
    State(state): State<AppState>,
    Query(q): Query<ApprovalQuery>,
) -> Result<Json<PaginatedResponse<ToolApproval>>, AppError> {
    let status = match q.status.as_deref().filter(|s| !s.is_empty()) {
        None => None,
        Some(s) => Some(
            ApprovalStatus::parse(s)
                .ok_or_else(|| AppError::BadRequest(format!("未知审批状态: {s}")))?,
        ),
    };
    let filter = ApprovalFilter {
        principal_id: q.principal_id,
        status,
    };
    let pagination = PaginationParams {
        page: q.page,
        page_size: q.page_size,
    };
    let (items, total) = state
        .mcp_service
        .list_approvals(&filter, pagination.page(), pagination.page_size())
        .await?;
    Ok(Json(PaginatedResponse::new(items, total, &pagination)))
}

/// `GET /api/mcp/approvals/{id}` — 审批单详情
#[utoipa::path(
    get,
    path = "/api/mcp/approvals/{id}",
    tag = "mcp",
    params(("id" = i64, Path, description = "审批单 ID")),
    responses(
        (status = 200, description = "审批单", body = ToolApproval),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 mcp:read 权限"),
        (status = 404, description = "不存在"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn get_approval(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<ToolApproval>, AppError> {
    Ok(Json(state.mcp_service.get_approval(id).await?))
}

/// 批准前的字段级授权：返回工具定义。
async fn authorize_approver(
    state: &AppState,
    role: &str,
    a: &ToolApproval,
) -> Result<&'static tools::ToolDef, AppError> {
    let tool = tools::find(&a.tool)
        .filter(|t| !t.is_read_only())
        .ok_or_else(|| AppError::Conflict(format!("工具 {} 已不可用", a.tool)))?;
    let (res, act) = tool
        .permission()
        .ok_or_else(|| AppError::internal(format!("工具 {} 无权限映射", tool.name)))?;
    let rs = &state.role_service;
    if !rs.check_permission(role, res, act).await? {
        return Err(AppError::Forbidden(format!(
            "Missing permission: {res}:{act} (approving {})",
            tool.name
        )));
    }
    if tool.risk == RiskLevel::Destructive && !rs.check_permission(role, "mcp", "delete").await? {
        return Err(AppError::Forbidden(
            "Missing permission: mcp:delete (approving a destructive tool)".into(),
        ));
    }
    let p = state.mcp_service.get_principal(a.principal_id).await?;
    if !p.is_active(chrono::Utc::now()) || !p.allow_write {
        return Err(AppError::Conflict(
            "发起该请求的凭证已吊销、过期或已关闭写操作".into(),
        ));
    }
    let owner = state
        .user_service
        .get_user(p.user_id)
        .await
        .map_err(|_| AppError::Conflict("凭证签发者已不存在".into()))?;
    if !rs.check_permission(&owner.role, res, act).await? {
        return Err(AppError::Conflict(format!(
            "凭证签发者 {} 当前角色已无 {res}:{act} 权限",
            owner.username
        )));
    }
    Ok(tool)
}

/// `POST /api/mcp/approvals/{id}/approve` — 批准并执行
#[utoipa::path(
    post,
    path = "/api/mcp/approvals/{id}/approve",
    tag = "mcp",
    params(("id" = i64, Path, description = "审批单 ID")),
    responses(
        (status = 200, description = "已执行；status 为 succeeded 或 failed，result 为输出或错误", body = ToolApproval),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 mcp:update，或缺少工具本身权限 / 高危工具的 mcp:delete"),
        (status = 404, description = "不存在"),
        (status = 409, description = "非待审批状态、已过期、凭证失效或签发者权限已变更"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn approve(
    State(state): State<AppState>,
    Extension(UserId(user_id)): Extension<UserId>,
    Extension(Username(username)): Extension<Username>,
    Extension(UserRole(role)): Extension<UserRole>,
    Path(id): Path<i64>,
) -> Result<Json<ToolApproval>, AppError> {
    let current = state.mcp_service.get_approval(id).await?;
    let tool = authorize_approver(&state, &role, &current).await?;
    let a = state.mcp_service.begin_approval(id, user_id).await?;
    let args: Value = serde_json::from_str(&a.raw_arguments).unwrap_or(Value::Null);
    let caller = Caller {
        username: &username,
        principal_id: a.principal_id,
    };
    let started = std::time::Instant::now();
    let outcome = tokio::time::timeout(
        std::time::Duration::from_secs(APPROVED_EXEC_TIMEOUT_SECS),
        tools::execute_approved(&state, &caller, tool, &args),
    )
    .await
    .unwrap_or_else(|_| Err(format!("执行超时（{APPROVED_EXEC_TIMEOUT_SECS}s）")));

    // 审计：工具调用记录（与协议端点同表）+ 面板操作日志（记在批准者名下）
    state
        .mcp_service
        .record_call(ToolCallRecord {
            id: 0,
            principal_id: a.principal_id,
            session_id: a.session_id.clone(),
            tool: tool.name.to_string(),
            arguments: a.arguments.clone(),
            ok: outcome.is_ok(),
            error: outcome.as_ref().err().cloned().unwrap_or_default(),
            duration_ms: started.elapsed().as_millis() as i64,
            created_at: chrono::Utc::now(),
        })
        .await;
    let _ = state
        .operation_log_service
        .log(
            &username,
            "mcp_approve",
            Some(&format!("#{id} {} {}", tool.name, a.arguments)),
            None,
        )
        .await;
    Ok(Json(state.mcp_service.finish_approval(id, &outcome).await?))
}

/// `POST /api/mcp/approvals/{id}/reject` — 拒绝
#[utoipa::path(
    post,
    path = "/api/mcp/approvals/{id}/reject",
    tag = "mcp",
    params(("id" = i64, Path, description = "审批单 ID")),
    request_body = RejectRequest,
    responses(
        (status = 200, description = "已拒绝", body = ToolApproval),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 mcp:update 权限"),
        (status = 404, description = "不存在"),
        (status = 409, description = "非待审批状态"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn reject(
    State(state): State<AppState>,
    Extension(UserId(user_id)): Extension<UserId>,
    Extension(Username(username)): Extension<Username>,
    Path(id): Path<i64>,
    body: axum::body::Bytes,
) -> Result<Json<ToolApproval>, AppError> {
    // 请求体可省略（无理由拒绝）
    let req: RejectRequest = if body.iter().all(u8::is_ascii_whitespace) {
        RejectRequest::default()
    } else {
        serde_json::from_slice(&body)
            .map_err(|e| AppError::BadRequest(format!("请求体不合法: {e}")))?
    };
    let reason = req.reason.unwrap_or_default();
    let a = state
        .mcp_service
        .reject_approval(id, user_id, &reason)
        .await?;
    let _ = state
        .operation_log_service
        .log(
            &username,
            "mcp_reject",
            Some(&format!("#{id} {}", a.tool)),
            None,
        )
        .await;
    Ok(Json(a))
}
