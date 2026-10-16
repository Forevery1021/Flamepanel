//! WAF 下发端口（Doc/32 W3 · WAF → Web 引擎前置联动）。
//!
//! `WafService`（application）只依赖本端口；实现（`infrastructure/waf_enforcement.rs`：
//! 规则编译为原生 Nginx 配置 + 原子落盘 + 校验 + reload）由组合根注入。
//! 未注入（单测 / 裁剪装配）时规则只作用于进程内引擎。

use crate::core::error::AppError;
use crate::domain::entity::WafRule;
use async_trait::async_trait;
use serde::Serialize;
use std::sync::Arc;
use utoipa::ToSchema;

/// 未下发的规则。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
pub struct WafSkippedRule {
    pub rule_id: i64,
    pub reason: String,
}

/// 单个引擎的下发结果。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
pub struct WafEngineApply {
    /// 引擎名（nginx / openresty）
    pub engine: String,
    /// 是否已写入配置（未安装 / 无写权限时为 false，见 `reason`）
    pub applied: bool,
    /// 是否已 reload（引擎未运行时为 false，下次启动生效）
    pub reloaded: bool,
    /// 已下发的规则 ID
    pub enforced: Vec<i64>,
    pub skipped: Vec<WafSkippedRule>,
    /// 为接入 WAF 而重新部署的站点（旧版本渲染的站点缺少 WAF include）
    pub redeployed_sites: Vec<String>,
    /// 未下发原因（未安装 / 无写权限）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    /// 下发失败原因（配置校验失败等；已回滚）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// 下发报告（`POST /api/waf/apply` 出参）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
pub struct WafApplyReport {
    /// 没有任何引擎下发失败（未安装的引擎不算失败）
    pub ok: bool,
    pub engines: Vec<WafEngineApply>,
}

#[async_trait]
pub trait WafEnforcementPort: Send + Sync {
    /// 把（启用）规则全集下发到本机支持的 Web 引擎。单个引擎失败记入报告而不中断其它引擎。
    async fn apply(&self, rules: &[WafRule]) -> Result<WafApplyReport, AppError>;
}

pub type SharedWafEnforcementPort = Arc<dyn WafEnforcementPort>;

/// Web 引擎命中（WAF 命中日志回灌的输入）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EngineHit {
    /// 按主机名解析出的网站（未知主机为 0）
    pub website_id: i64,
    pub time: Option<chrono::DateTime<chrono::Utc>>,
    pub client_ip: String,
    pub method: String,
    pub uri: String,
    pub user_agent: String,
    /// 请求被阻断（403）
    pub blocked: bool,
    /// 命中的规则 ID（可能含白名单规则，由服务按类型过滤）
    pub rules: Vec<i64>,
    /// CC 拒绝（429）归因的规则 ID
    pub cc_rules: Vec<i64>,
}

/// 单个引擎的命中日志回灌状态（`GET /api/waf/status`）。
#[derive(Debug, Clone, PartialEq, Eq, Serialize, ToSchema)]
pub struct WafIngestStatus {
    /// 引擎名（nginx / openresty）
    pub engine: String,
    /// 已读取到的日志偏移（字节）
    pub offset: u64,
    /// 本次进程启动以来导入的拦截日志条数
    pub imported: u64,
    /// 最近一次成功读取时间
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_run_at: Option<chrono::DateTime<chrono::Utc>>,
    /// 最近一次失败原因（成功后清空）
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_error: Option<String>,
}
