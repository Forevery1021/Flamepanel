use axum::{
    extract::{FromRequestParts, State},
    http::header::AUTHORIZATION,
    http::request::Parts,
    middleware::Next,
    response::Response,
    body::Body,
    http::Request,
};
use jsonwebtoken::{decode, encode, DecodingKey, EncodingKey, Header, Validation};
use serde::{Deserialize, Serialize};
use std::{future::Future, pin::Pin, sync::Arc};

use crate::application::AppState;
use crate::core::error::AppError;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Claims {
    pub sub: String,
    pub role: String,
    pub exp: usize,
    pub iat: usize,
}

lazy_static::lazy_static! {
    static ref JWT_SECRET: Vec<u8> = std::env::var("OP_JWT_SECRET")
        .unwrap_or_else(|_| "your-super-secret-jwt-key-change-in-production".to_string())
        .into_bytes();
}

pub fn create_jwt(username: &str, role: &str, expires_in_secs: u64) -> Result<String, AppError> {
    let now = chrono::Utc::now();
    let claims = Claims {
        sub: username.to_string(),
        role: role.to_string(),
        iat: now.timestamp() as usize,
        exp: (now + chrono::Duration::seconds(expires_in_secs as i64)).timestamp() as usize,
    };

    encode(
        &Header::default(),
        &claims,
        &EncodingKey::from_secret(&JWT_SECRET),
    )
    .map_err(|_| AppError::Internal("JWT 创建失败".into()))
}

/// Axum 中间件：验证请求头中的 Bearer Token
pub async fn auth_middleware(
    mut req: Request<Body>,
    next: Next,
) -> Result<Response, AppError> {
    let token = req
        .headers()
        .get(AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .ok_or(AppError::Unauthorized)?;

    let claims = decode::<Claims>(
        token,
        &DecodingKey::from_secret(&JWT_SECRET),
        &Validation::default(),
    )
    .map_err(|_| AppError::Unauthorized)?
    .claims;

    req.extensions_mut().insert(Arc::new(claims));
    Ok(next.run(req).await)
}

/// 提取器：从请求扩展中获取当前用户 Claims
pub struct CurrentUser(pub Arc<Claims>);

impl<S> FromRequestParts<S> for CurrentUser
where
    S: Send + Sync,
{
    type Rejection = AppError;

    fn from_request_parts(
        parts: &mut Parts,
        _state: &S,
    ) -> impl Future<Output = Result<Self, Self::Rejection>> + Send {
        async move {
            parts
                .extensions
                .get::<Arc<Claims>>()
                .cloned()
                .map(CurrentUser)
                .ok_or(AppError::Unauthorized)
        }
    }
}

/// 提取器：要求当前用户为 admin 角色
pub struct RequireAdmin(pub Arc<Claims>);

impl<S> FromRequestParts<S> for RequireAdmin
where
    S: Send + Sync,
{
    type Rejection = AppError;

    fn from_request_parts(
        parts: &mut Parts,
        _state: &S,
    ) -> impl Future<Output = Result<Self, Self::Rejection>> + Send {
        async move {
            let claims = parts
                .extensions
                .get::<Arc<Claims>>()
                .cloned()
                .ok_or(AppError::Unauthorized)?;

            if claims.role != "admin" {
                return Err(AppError::Forbidden);
            }

            Ok(RequireAdmin(claims))
        }
    }
}

/// 提取器：要求当前用户拥有指定权限（admin 角色自动通过）
///
/// 在 handler 中使用：先提取 CurrentUser，再调用 check_perm 验证权限
pub async fn check_perm(claims: &Claims, state: &AppState, perm: &str) -> Result<(), AppError> {
    if claims.role == "admin" {
        return Ok(());
    }
    let perms = state.role_repo.get_user_permissions(&claims.role).await?;
    if perms.contains(&perm.to_string()) {
        Ok(())
    } else {
        Err(AppError::Forbidden)
    }
}

/// Permission guard middleware — checks specific permission via Axum State
pub fn require_perm(
    perm: &'static str,
) -> impl Fn(State<AppState>, Request<Body>, Next) -> Pin<Box<dyn Future<Output = Result<Response, AppError>> + Send>> + Clone {
    move |State(state): State<AppState>, req: Request<Body>, next: Next| {
        let perm = perm;
        Box::pin(async move {
            let claims = req
                .extensions()
                .get::<Arc<Claims>>()
                .cloned()
                .ok_or(AppError::Unauthorized)?;

            if claims.role != "admin" {
                let perms = state.role_repo.get_user_permissions(&claims.role).await?;
                if !perms.contains(&perm.to_string()) {
                    return Err(AppError::Forbidden);
                }
            }

            Ok(next.run(req).await)
        })
    }
}
