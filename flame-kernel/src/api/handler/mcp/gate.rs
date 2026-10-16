//! 工具调用闸门：MCP 协议端点与面板内置助手（M-AI-3）共用的**唯一**执行路径。
//!
//! 一次调用依次经过：
//! 1. 凭证是否允许写（[`write_enabled`]）；
//! 2. 调用者角色是否具备工具等价 REST 路由的权限（[`allowed`]）；
//! 3. 只读工具 → 执行（30 s 超时）；写 / 高危工具 → dry-run 预览 + 生成审批单（M-AI-2），**绝不直接执行**；
//! 4. 写审计（`tool_calls`，参数已脱敏）。
//!
//! 两个入口共用本模块，保证「AI 从哪里进来都受同一套规则约束」。
use super::tools::{self, Caller, ToolDef};
use crate::api::types::AppState;
use crate::domain::mcp::{AgentPrincipal, ToolCallRecord};
use serde_json::Value;

/// 单次工具执行超时。
pub const TOOL_TIMEOUT_SECS: u64 = 30;

/// 调用上下文。
pub struct GateCtx<'a> {
    pub state: &'a AppState,
    /// 发起调用的凭证（助手使用每用户一个的内置凭证）
    pub principal: &'a AgentPrincipal,
    pub username: &'a str,
    pub role: &'a str,
    pub session_id: Option<String>,
}

/// 调用者角色是否具备工具所需权限。
pub async fn allowed(ctx: &GateCtx<'_>, tool: &ToolDef) -> Result<(), String> {
    let (res, act) = tool
        .permission()
        .ok_or_else(|| format!("工具 {} 未声明权限", tool.name))?;
    match ctx
        .state
        .role_service
        .check_permission(ctx.role, res, act)
        .await
    {
        Ok(true) => Ok(()),
        Ok(false) => Err(format!("权限不足：缺少 {res}:{act}")),
        Err(e) => Err(e.to_string()),
    }
}

/// 写工具需要凭证开启 `allow_write`（M-AI-2）。
pub fn write_enabled(ctx: &GateCtx<'_>, tool: &ToolDef) -> Result<(), String> {
    if tool.is_read_only() || ctx.principal.allow_write {
        Ok(())
    } else {
        Err(format!(
            "此凭证未开启写操作，不能调用 {}（在面板「AI 接入中心」签发凭证时勾选「允许写操作」）",
            tool.name
        ))
    }
}

/// 当前上下文可见（可调用）的工具。
pub async fn visible(ctx: &GateCtx<'_>) -> Vec<&'static ToolDef> {
    let mut out = Vec::new();
    for t in tools::TOOLS {
        if write_enabled(ctx, t).is_ok() && allowed(ctx, t).await.is_ok() {
            out.push(t);
        }
    }
    out
}

/// 调用结果（附审批单 ID，供前端展示「去审批」入口）。
pub struct CallOutcome {
    pub result: tools::ToolResult,
    pub approval_id: Option<i64>,
}

/// 闸门 + 执行 / 审批 + 审计。
pub async fn call(ctx: &GateCtx<'_>, tool: &'static ToolDef, args: &Value) -> CallOutcome {
    let started = std::time::Instant::now();
    let mut audit_args = args.clone();
    tools::redact(&mut audit_args);
    let gate = match write_enabled(ctx, tool) {
        Ok(()) => allowed(ctx, tool).await,
        Err(e) => Err(e),
    };
    let mut approval_id = None;
    let result = match gate {
        Err(e) => Err(e),
        Ok(()) if tool.is_read_only() => {
            let caller = Caller {
                username: ctx.username,
                principal_id: ctx.principal.id,
            };
            match tokio::time::timeout(
                std::time::Duration::from_secs(TOOL_TIMEOUT_SECS),
                tools::execute(ctx.state, &caller, tool, args),
            )
            .await
            {
                Ok(r) => r,
                Err(_) => Err(format!("工具执行超时（{TOOL_TIMEOUT_SECS}s）")),
            }
        }
        // M-AI-2 变更闸门：写工具只做预览并生成审批单，绝不直接执行
        Ok(()) => request_approval(ctx, tool, args, &audit_args)
            .await
            .map(|(id, text)| {
                approval_id = Some(id);
                text
            }),
    };
    ctx.state
        .mcp_service
        .record_call(ToolCallRecord {
            id: 0,
            principal_id: ctx.principal.id,
            session_id: ctx.session_id.clone(),
            tool: tool.name.to_string(),
            arguments: if audit_args.is_null() {
                "{}".into()
            } else {
                audit_args.to_string()
            },
            ok: result.is_ok(),
            error: result.as_ref().err().cloned().unwrap_or_default(),
            duration_ms: started.elapsed().as_millis() as i64,
            created_at: chrono::Utc::now(),
        })
        .await;
    CallOutcome {
        result,
        approval_id,
    }
}

/// 写工具：dry-run 预览 → 审批单 → 返回给模型的说明文本。
async fn request_approval(
    ctx: &GateCtx<'_>,
    tool: &ToolDef,
    args: &Value,
    audit_args: &Value,
) -> Result<(i64, String), String> {
    let preview = match tokio::time::timeout(
        std::time::Duration::from_secs(TOOL_TIMEOUT_SECS),
        tools::preview(ctx.state, tool, args),
    )
    .await
    {
        Ok(r) => r?,
        Err(_) => return Err(format!("预览超时（{TOOL_TIMEOUT_SECS}s）")),
    };
    let raw = if args.is_null() {
        "{}".to_string()
    } else {
        args.to_string()
    };
    let approval = ctx
        .state
        .mcp_service
        .request_approval(
            ctx.principal.id,
            ctx.session_id.clone(),
            tool.name,
            tool.risk,
            &raw,
            &audit_args.to_string(),
            &preview,
        )
        .await
        .map_err(|e| e.to_string())?;
    let who = if tool.risk == tools::RiskLevel::Destructive {
        "具备高危操作权限（mcp:delete）的面板用户"
    } else {
        "面板用户"
    };
    Ok((
        approval.id,
        format!(
            "已提交审批单 #{id}，尚未执行。\n\n预览：{preview}\n\n需要{who}在「AI 接入中心 → 审批」中批准后才会执行；\
             审批单 {ttl} 分钟内未处理将过期。请告知用户前往面板处理，之后用 approval_get {{\"id\": {id}}} 查询结果。",
            id = approval.id,
            ttl = crate::application::mcp_service::APPROVAL_TTL_MINUTES,
        ),
    ))
}
