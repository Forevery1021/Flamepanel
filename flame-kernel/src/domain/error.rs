use std::fmt;

/// 领域层纯业务错误。
///
/// 只描述业务语义（未找到、校验失败、冲突、越权等），**不**依赖
/// axum / sqlx / bollard 等外部框架；HTTP 状态码映射由应用/接口层
/// 通过 `crate::core::error::AppError` 完成（`AppError: From<DomainError>`）。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DomainError {
    /// 资源不存在
    NotFound(String),
    /// 参数/输入校验失败
    Validation(String),
    /// 与现有状态冲突（如重复创建、非法状态迁移）
    Conflict(String),
    /// 越权 / 业务规则不允许
    Forbidden(String),
    /// 其他业务规则约束
    RuleViolation(String),
}

impl DomainError {
    pub fn not_found(msg: impl Into<String>) -> Self {
        DomainError::NotFound(msg.into())
    }

    pub fn validation(msg: impl Into<String>) -> Self {
        DomainError::Validation(msg.into())
    }

    pub fn conflict(msg: impl Into<String>) -> Self {
        DomainError::Conflict(msg.into())
    }

    pub fn forbidden(msg: impl Into<String>) -> Self {
        DomainError::Forbidden(msg.into())
    }

    pub fn rule_violation(msg: impl Into<String>) -> Self {
        DomainError::RuleViolation(msg.into())
    }
}

impl fmt::Display for DomainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            DomainError::NotFound(m) => write!(f, "{}", m),
            DomainError::Validation(m) => write!(f, "{}", m),
            DomainError::Conflict(m) => write!(f, "{}", m),
            DomainError::Forbidden(m) => write!(f, "{}", m),
            DomainError::RuleViolation(m) => write!(f, "{}", m),
        }
    }
}

impl std::error::Error for DomainError {}
