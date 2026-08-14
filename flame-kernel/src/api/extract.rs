use crate::core::error::AppError;
use axum::{
    extract::rejection::JsonRejection,
    extract::{FromRequest, Request},
    http::HeaderMap,
    Json,
};
use serde::de::DeserializeOwned;

/// 统一 JSON 请求体提取器：
/// 反序列化失败时返回统一 JSON 错误格式（而非 axum 默认纯文本 422）
pub struct ApiJson<T>(pub T);

impl<T, S> FromRequest<S> for ApiJson<T>
where
    T: DeserializeOwned,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request(req: Request, state: &S) -> Result<Self, Self::Rejection> {
        let Json(inner) = Json::<T>::from_request(req, state).await.map_err(|rej| {
            let msg = match rej {
                JsonRejection::JsonDataError(e) => format!("Invalid JSON body: {}", e),
                JsonRejection::JsonSyntaxError(e) => format!("Invalid JSON syntax: {}", e),
                JsonRejection::MissingJsonContentType(_) => {
                    "Missing JSON content-type header".to_string()
                }
                JsonRejection::BytesRejection(e) => format!("Body too large or unreadable: {}", e),
                _ => "Invalid request body".to_string(),
            };
            AppError::BadRequest(msg)
        })?;
        Ok(ApiJson(inner))
    }
}

/// 从请求头提取客户端 IP（唯一实现，供限流/审计等处复用）：
/// 优先 `X-Real-IP`（可信代理设置），回退到 `X-Forwarded-For` 链首值（最左即真实客户端）。
pub fn extract_client_ip(headers: &HeaderMap) -> String {
    if let Some(v) = headers
        .get("X-Real-IP")
        .and_then(|v| v.to_str().ok())
        .map(|s| s.trim().to_string())
        .filter(|s| !s.is_empty())
    {
        return v;
    }
    if let Some(v) = headers
        .get("X-Forwarded-For")
        .and_then(|v| v.to_str().ok())
        .map(|s| {
            s.split(',')
                .next()
                .map(|x| x.trim().to_string())
                .unwrap_or_default()
        })
        .filter(|s| !s.is_empty())
    {
        return v;
    }
    "unknown".to_string()
}
