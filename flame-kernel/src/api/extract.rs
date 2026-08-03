use crate::core::error::AppError;
use axum::{extract::rejection::JsonRejection, extract::FromRequest, http::Request, Json};
use serde::de::DeserializeOwned;

/// 统一 JSON 请求体提取器：
/// 反序列化失败时返回统一 JSON 错误格式（而非 axum 默认纯文本 422）
pub struct ApiJson<T>(pub T);

#[async_trait::async_trait]
impl<T, S, B> FromRequest<S, B> for ApiJson<T>
where
    T: DeserializeOwned,
    B: axum::body::HttpBody + Send + 'static,
    B::Data: Send,
    B::Error: std::error::Error + Send + Sync,
    S: Send + Sync,
{
    type Rejection = AppError;

    async fn from_request(req: Request<B>, state: &S) -> Result<Self, Self::Rejection> {
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
