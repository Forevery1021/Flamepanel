use axum::{
    extract::FromRequestParts,
    middleware::Next,
    response::Response,
    body::Body,
    http::Request,
};
use jsonwebtoken::{encode, EncodingKey, Header};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::future::Future;
use crate::core::error::AppError;
use chrono;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String,
    pub exp: usize,
    pub iat: usize,
}

const JWT_SECRET: &[u8] = b"your-super-secret-jwt-key-change-in-production";

pub fn create_jwt(username: &str) -> Result<String, AppError> {
    let now = chrono::Utc::now();
    let claims = Claims {
        sub: username.to_string(),
        iat: now.timestamp() as usize,
        exp: (now + chrono::Duration::days(7)).timestamp() as usize,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(JWT_SECRET),
    )
    .map_err(|_| AppError::Internal("JWT 创建失败".into()))
}

pub async fn auth_middleware(
    req: Request<Body>,
    next: Next,
) -> Result<Response, AppError> {
    // 开发阶段临时跳过 Token 验证（生产环境再开启）
    Ok(next.run(req).await)
}

pub struct CurrentUser(pub Arc<Claims>);

impl<S> FromRequestParts<S> for CurrentUser
where
    S: Send + Sync,
{
    type Rejection = AppError;

    fn from_request_parts(
        _parts: &mut axum::http::request::Parts,
        _state: &S,
    ) -> impl Future<Output = Result<Self, Self::Rejection>> + Send {
        // 开发阶段临时返回默认用户，绕过认证
        async {
            let claims = Claims {
                sub: "admin".to_string(),
                iat: 0,
                exp: usize::MAX,
            };
            Ok(CurrentUser(Arc::new(claims)))
        }
    }
}