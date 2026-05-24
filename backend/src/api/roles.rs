use axum::{
    extract::{Path, State},
    routing::{get, put},
    Json, Router,
};

use crate::application::{AppState, RoleService};
use crate::core::error::AppError;
use crate::domain::{CreateRoleRequest, UpdateRoleRequest, AssignRoleRequest};
use crate::middleware::auth::{CurrentUser, RequireAdmin};

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/roles", get(list_roles).post(create_role))
        .route("/roles/{id}", get(get_role).put(update_role).delete(delete_role))
        .route("/permissions", get(list_permissions))
        .route("/assign-role", put(assign_role))
        .route("/my-permissions", get(my_permissions))
}

fn role_service(state: &AppState) -> RoleService {
    RoleService::new(
        state.role_repo.clone(),
        state.permission_repo.clone(),
        state.user_repo.clone(),
    )
}

async fn list_roles(
    State(state): State<AppState>,
    _admin: RequireAdmin,
) -> Result<Json<serde_json::Value>, AppError> {
    let svc = role_service(&state);
    let roles = svc.list_roles().await?;
    Ok(Json(serde_json::to_value(roles).unwrap()))
}

async fn get_role(
    State(state): State<AppState>,
    _admin: RequireAdmin,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, AppError> {
    let svc = role_service(&state);
    let role = svc.get_role(id).await?;
    Ok(Json(serde_json::to_value(role).unwrap()))
}

async fn create_role(
    State(state): State<AppState>,
    _admin: RequireAdmin,
    Json(req): Json<CreateRoleRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let svc = role_service(&state);
    let role = svc.create_role(&req).await?;
    Ok(Json(serde_json::to_value(role).unwrap()))
}

async fn update_role(
    State(state): State<AppState>,
    _admin: RequireAdmin,
    Path(id): Path<i64>,
    Json(req): Json<UpdateRoleRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let svc = role_service(&state);
    svc.update_role(id, &req).await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

async fn delete_role(
    State(state): State<AppState>,
    _admin: RequireAdmin,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, AppError> {
    let svc = role_service(&state);
    svc.delete_role(id).await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

async fn list_permissions(
    State(state): State<AppState>,
    _admin: RequireAdmin,
) -> Result<Json<serde_json::Value>, AppError> {
    let svc = role_service(&state);
    let perms = svc.list_permissions().await?;
    Ok(Json(serde_json::to_value(perms).unwrap()))
}

async fn assign_role(
    State(state): State<AppState>,
    _admin: RequireAdmin,
    Json(req): Json<AssignRoleRequest>,
) -> Result<Json<serde_json::Value>, AppError> {
    let svc = role_service(&state);
    svc.assign_role(&req).await?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

async fn my_permissions(
    State(state): State<AppState>,
    user: CurrentUser,
) -> Result<Json<serde_json::Value>, AppError> {
    let svc = role_service(&state);
    let perms = svc.get_user_permissions(&user.0.role).await?;
    Ok(Json(serde_json::json!({
        "role": user.0.role,
        "username": user.0.sub,
        "permissions": perms,
    })))
}
