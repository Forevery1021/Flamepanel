use axum::{http::StatusCode, response::IntoResponse, Json};
use serde::Serialize;
use thiserror::Error;
use tracing::error;

/// 稳定错误码（客户端可据此做国际化提示或分支处理）。
/// 与 HTTP 状态码一一对应，但语义更精确，跨版本保持稳定。
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    AuthUnauthorized,
    AuthForbidden,
    PasswordChangeRequired,
    NotFound,
    BadRequest,
    ValidationError,
    Conflict,
    ServiceUnavailable,
    Internal,
}

impl ErrorCode {
    pub fn as_str(&self) -> &'static str {
        match self {
            ErrorCode::AuthUnauthorized => "AUTH_UNAUTHORIZED",
            ErrorCode::AuthForbidden => "AUTH_FORBIDDEN",
            ErrorCode::PasswordChangeRequired => "PASSWORD_CHANGE_REQUIRED",
            ErrorCode::NotFound => "NOT_FOUND",
            ErrorCode::BadRequest => "BAD_REQUEST",
            ErrorCode::ValidationError => "VALIDATION_ERROR",
            ErrorCode::Conflict => "CONFLICT",
            ErrorCode::ServiceUnavailable => "SERVICE_UNAVAILABLE",
            ErrorCode::Internal => "INTERNAL_ERROR",
        }
    }

    pub fn status_code(&self) -> StatusCode {
        match self {
            ErrorCode::AuthUnauthorized => StatusCode::UNAUTHORIZED,
            ErrorCode::AuthForbidden => StatusCode::FORBIDDEN,
            ErrorCode::PasswordChangeRequired => StatusCode::FORBIDDEN,
            ErrorCode::NotFound => StatusCode::NOT_FOUND,
            ErrorCode::BadRequest | ErrorCode::ValidationError => StatusCode::BAD_REQUEST,
            ErrorCode::Conflict => StatusCode::CONFLICT,
            ErrorCode::ServiceUnavailable => StatusCode::SERVICE_UNAVAILABLE,
            ErrorCode::Internal => StatusCode::INTERNAL_SERVER_ERROR,
        }
    }
}

#[derive(Debug, Error)]
pub enum AppError {
    #[error("Unauthorized: {0}")]
    Unauthorized(String),
    #[error("Forbidden: {0}")]
    Forbidden(String),
    #[error("Password change required: {0}")]
    PasswordChangeRequired(String),
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("Bad request: {0}")]
    BadRequest(String),
    #[error("Validation error: {0}")]
    ValidationError(String),
    #[error("Conflict: {0}")]
    Conflict(String),
    #[error("Service unavailable: {0}")]
    ServiceUnavailable(String),
    #[error("Internal error: {message}")]
    Internal {
        message: String,
        #[source]
        source: Option<Box<dyn std::error::Error + Send + Sync>>,
    },
}

impl AppError {
    pub fn internal(msg: impl Into<String>) -> Self {
        Self::Internal {
            message: msg.into(),
            source: None,
        }
    }

    pub fn internal_with_source(
        msg: impl Into<String>,
        source: impl Into<Box<dyn std::error::Error + Send + Sync>>,
    ) -> Self {
        Self::Internal {
            message: msg.into(),
            source: Some(source.into()),
        }
    }

    pub fn code(&self) -> ErrorCode {
        match self {
            AppError::Unauthorized(_) => ErrorCode::AuthUnauthorized,
            AppError::Forbidden(_) => ErrorCode::AuthForbidden,
            AppError::PasswordChangeRequired(_) => ErrorCode::PasswordChangeRequired,
            AppError::NotFound(_) => ErrorCode::NotFound,
            AppError::BadRequest(_) => ErrorCode::BadRequest,
            AppError::ValidationError(_) => ErrorCode::ValidationError,
            AppError::Conflict(_) => ErrorCode::Conflict,
            AppError::ServiceUnavailable(_) => ErrorCode::ServiceUnavailable,
            AppError::Internal { .. } => ErrorCode::Internal,
        }
    }

    pub fn status_code(&self) -> StatusCode {
        self.code().status_code()
    }
}

impl From<std::io::Error> for AppError {
    fn from(e: std::io::Error) -> Self {
        AppError::internal_with_source("I/O operation failed", e)
    }
}

impl From<serde_json::Error> for AppError {
    fn from(e: serde_json::Error) -> Self {
        AppError::internal_with_source("JSON serialization failed", e)
    }
}

impl From<std::num::ParseIntError> for AppError {
    fn from(e: std::num::ParseIntError) -> Self {
        AppError::internal_with_source("Failed to parse integer", e)
    }
}

impl From<sqlx::Error> for AppError {
    fn from(e: sqlx::Error) -> Self {
        AppError::internal_with_source("Database operation failed", e)
    }
}

#[derive(Serialize)]
pub struct ErrorResponse {
    /// HTTP 状态码（兼容旧客户端）
    pub code: u16,
    /// 稳定错误码，如 `USER_NOT_FOUND`、`AUTH_UNAUTHORIZED`
    pub error: &'static str,
    pub message: String,
}

impl IntoResponse for AppError {
    fn into_response(self) -> axum::response::Response {
        let status = self.status_code();
        let error = self.code().as_str();
        let message = self.to_string();
        let body = Json(ErrorResponse {
            code: status.as_u16(),
            error,
            message,
        });

        // 服务器内部错误记录完整错误链（source 不暴露给客户端）
        if status.is_server_error() {
            if let AppError::Internal { source, .. } = &self {
                match source {
                    Some(s) => error!(error = %self, source = %s, "Internal server error"),
                    None => error!(error = %self, "Internal server error"),
                }
            } else {
                error!(error = %self, "Server error");
            }
        }

        (status, body).into_response()
    }
}
