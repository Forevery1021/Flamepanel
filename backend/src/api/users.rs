use axum::{
    extract::{State, Query},
    routing::{get, put, delete},
    Json, Router,
};
use serde::{Deserialize, Serialize};

use crate::application::AppState;
use crate::core::error::AppError;
use crate::middleware::auth::RequireAdmin;

#[derive(Debug, Deserialize)]
pub struct UpdateUserRequest {
    pub role: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ResetPasswordRequest {
    pub new_password: String,
}

#[derive(Debug, Serialize)]
pub struct UserResponse {
    pub id: i64,
    pub username: String,
    pub role: String,
    pub created_at: String,
    pub last_login: Option<String>,
}

pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/list", get(list_users))
        .route("/update-role", put(update_user_role))
        .route("/reset-password", put(reset_user_password))
        .route("/delete", delete(delete_user))
}

async fn list_users(
    State(state): State<AppState>,
    _admin: RequireAdmin,
) -> Result<Json<Vec<UserResponse>>, AppError> {
    let users = state.user_repo.list().await?;
    Ok(Json(
        users
            .into_iter()
            .map(|u| UserResponse {
                id: u.id,
                username: u.username,
                role: u.role,
                created_at: u.created_at.format("%Y-%m-%d %H:%M:%S").to_string(),
                last_login: u.last_login.map(|t| t.format("%Y-%m-%d %H:%M:%S").to_string()),
            })
            .collect(),
    ))
}

async fn update_user_role(
    State(state): State<AppState>,
    _admin: RequireAdmin,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<serde_json::Value>, AppError> {
    let id: i64 = params
        .get("id")
        .ok_or(AppError::BadRequest("缺少 id 参数".into()))?
        .parse()
        .map_err(|_| AppError::BadRequest("id 格式无效".into()))?;

    let role = params
        .get("role")
        .ok_or(AppError::BadRequest("缺少 role 参数".into()))?;

    if role != "admin" && role != "user" {
        return Err(AppError::BadRequest("role 必须为 admin 或 user".into()));
    }

    state.user_repo.update_role(id, role).await?;

    tracing::info!("管理员 '{}' 更新了用户 {} 的角色为 {}", _admin.0.sub, id, role);

    Ok(Json(serde_json::json!({"success": true, "message": "角色更新成功"})))
}

async fn reset_user_password(
    State(state): State<AppState>,
    _admin: RequireAdmin,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<serde_json::Value>, AppError> {
    let id: i64 = params
        .get("id")
        .ok_or(AppError::BadRequest("缺少 id 参数".into()))?
        .parse()
        .map_err(|_| AppError::BadRequest("id 格式无效".into()))?;

    let new_password = params
        .get("password")
        .ok_or(AppError::BadRequest("缺少 password 参数".into()))?;

    if new_password.len() < 6 {
        return Err(AppError::BadRequest("密码至少6位".into()));
    }

    let password_hash = tokio::task::spawn_blocking({
        let pwd = new_password.clone();
        move || bcrypt::hash(pwd, bcrypt::DEFAULT_COST)
    })
    .await
    .map_err(|e| AppError::Internal(format!("线程错误: {e}")))?
    .map_err(|e| AppError::Internal(format!("密码哈希失败: {e}")))?;

    state.user_repo.update_password(id, &password_hash).await?;

    tracing::info!("管理员 '{}' 重置了用户 {} 的密码", _admin.0.sub, id);

    Ok(Json(serde_json::json!({"success": true, "message": "密码已重置"})))
}

async fn delete_user(
    State(state): State<AppState>,
    _admin: RequireAdmin,
    Query(params): Query<std::collections::HashMap<String, String>>,
) -> Result<Json<serde_json::Value>, AppError> {
    let id: i64 = params
        .get("id")
        .ok_or(AppError::BadRequest("缺少 id 参数".into()))?
        .parse()
        .map_err(|_| AppError::BadRequest("id 格式无效".into()))?;

    if _admin.0.sub == state
        .user_repo
        .find_by_id(id)
        .await?
        .map(|u| u.username)
        .unwrap_or_default()
    {
        return Err(AppError::BadRequest("不能删除自己".into()));
    }

    state.user_repo.delete(id).await?;

    tracing::info!("管理员 '{}' 删除了用户 {}", _admin.0.sub, id);

    Ok(Json(serde_json::json!({"success": true, "message": "用户已删除"})))
}
