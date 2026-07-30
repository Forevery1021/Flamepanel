use axum::{
    Router, extract::State, http::{Request, StatusCode, header},
    response::{Response, IntoResponse},
    middleware::{self, Next},
};
use tower_http::trace::TraceLayer;
use tracing::info;
use crate::utils::jwt::JwtUtils;
use crate::api::rate_limiter;
use crate::api::types::{AppState, UserId, route_permission};

pub fn add_middleware(router: Router, state: AppState) -> Router {
    rate_limiter::init_global_limiter(120, 60);
    router
        .layer(middleware::from_fn(rate_limiter::rate_limit_middleware))
        .layer(middleware::from_fn_with_state(state, auth_middleware))
        .layer(TraceLayer::new_for_http())
}

pub fn log_request(method: &str, uri: &str, status: u16) {
    info!(method = %method, uri = %uri, status = status, "HTTP request");
}

async fn auth_middleware<B>(
    State(state): State<AppState>,
    mut req: Request<B>,
    next: Next<B>,
) -> Result<Response, StatusCode> {
    let path = req.uri().path();
    if path == "/health" || path.starts_with("/ws/") || path == "/api/auth/login" {
        return Ok(next.run(req).await);
    }

    let auth_header = req.headers()
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let token = auth_header
        .strip_prefix("Bearer ")
        .ok_or(StatusCode::UNAUTHORIZED)?;

    let jwt = JwtUtils::new(&state.jwt_secret, 24);
    let claims = jwt.verify(token).map_err(|_| StatusCode::UNAUTHORIZED)?;

    let user_id: i64 = claims.sub.parse().map_err(|_| StatusCode::UNAUTHORIZED)?;
    req.extensions_mut().insert(UserId(user_id));

    Ok(next.run(req).await)
}

pub async fn rbac_middleware<B>(
    State(state): State<AppState>,
    req: Request<B>,
    next: Next<B>,
) -> Result<Response, StatusCode> {
    let path = req.uri().path();
    if path == "/health" || path.starts_with("/ws/") || path == "/api/auth/login" {
        return Ok(next.run(req).await);
    }

    let user_id = req.extensions()
        .get::<UserId>()
        .map(|u| u.0)
        .unwrap_or(0);

    let user = state.user_service.user_repo.find_by_id(user_id).await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .ok_or(StatusCode::UNAUTHORIZED)?;

    if let Some((resource, action)) = route_permission(req.method(), path) {
        let allowed = state.role_service.check_permission(&user.role, resource, action).await
            .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;
        if !allowed {
            return Ok(StatusCode::FORBIDDEN.into_response());
        }
    }

    Ok(next.run(req).await)
}