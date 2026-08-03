use crate::api::extract::ApiJson;
use crate::api::types::{AppState, PaginatedResponse, PaginationParams};
use crate::core::error::AppError;
use crate::domain::entity::DatabaseInstance;
use axum::Router;
use axum::{
    extract::{Path, Query, State},
    Json,
};
use serde::{Deserialize, Serialize};

#[derive(Serialize)]
pub struct DatabaseInstanceResponse {
    pub id: i64,
    pub db_type: String,
    pub name: String,
    pub version: String,
    pub port: i32,
    pub status: String,
    pub install_path: String,
    pub data_dir: String,
    pub config_file: String,
    pub root_user: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Deserialize)]
pub struct InstallMysqlRequest {
    pub name: String,
    pub version: Option<String>,
    pub port: Option<i32>,
    pub root_password: Option<String>,
}

#[derive(Deserialize)]
pub struct InstallRedisRequest {
    pub name: String,
    pub version: Option<String>,
    pub port: Option<i32>,
    pub password: Option<String>,
}

#[derive(Deserialize)]
pub struct CreateDatabaseRequest {
    pub name: String,
    pub charset: Option<String>,
}

#[derive(Deserialize)]
pub struct CreateUserRequest {
    pub username: String,
    pub password: String,
    pub host: Option<String>,
}

fn to_response(inst: DatabaseInstance) -> DatabaseInstanceResponse {
    DatabaseInstanceResponse {
        id: inst.id,
        db_type: inst.db_type,
        name: inst.name,
        version: inst.version,
        port: inst.port,
        status: inst.status,
        install_path: inst.install_path,
        data_dir: inst.data_dir,
        config_file: inst.config_file,
        root_user: inst.root_user,
        created_at: inst.created_at.to_rfc3339(),
        updated_at: inst.updated_at.to_rfc3339(),
    }
}

pub async fn list(
    State(state): State<AppState>,
    Query(params): Query<PaginationParams>,
) -> Result<Json<PaginatedResponse<DatabaseInstanceResponse>>, AppError> {
    let result = state
        .database_service
        .list_instances_paginated(&params)
        .await?;
    let data = result.data.into_iter().map(to_response).collect();
    Ok(Json(PaginatedResponse::new(data, result.total, &params)))
}

pub async fn get(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<DatabaseInstanceResponse>, AppError> {
    let instance = state.database_service.get_instance(id).await?;
    Ok(Json(to_response(instance)))
}

pub async fn delete(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<&'static str>, AppError> {
    state.database_service.get_instance(id).await?;
    state.database_service.delete_instance(id).await?;
    Ok(Json("deleted"))
}

pub async fn install_mysql(
    State(state): State<AppState>,
    ApiJson(req): ApiJson<InstallMysqlRequest>,
) -> Result<Json<DatabaseInstanceResponse>, AppError> {
    let port = req.port.unwrap_or(3306);
    let pw = req.root_password.unwrap_or_default();
    let instance = state
        .database_service
        .install_mysql(req.version.as_deref(), port, &pw, &req.name)
        .await?;
    Ok(Json(to_response(instance)))
}

pub async fn install_redis(
    State(state): State<AppState>,
    ApiJson(req): ApiJson<InstallRedisRequest>,
) -> Result<Json<DatabaseInstanceResponse>, AppError> {
    let port = req.port.unwrap_or(6379);
    let instance = state
        .database_service
        .install_redis(
            req.version.as_deref(),
            port,
            req.password.as_deref(),
            &req.name,
        )
        .await?;
    Ok(Json(to_response(instance)))
}

pub async fn start(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<&'static str>, AppError> {
    state.database_service.start(id).await?;
    Ok(Json("started"))
}

pub async fn stop(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<&'static str>, AppError> {
    state.database_service.stop(id).await?;
    Ok(Json("stopped"))
}

pub async fn restart(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<&'static str>, AppError> {
    state.database_service.restart(id).await?;
    Ok(Json("restarted"))
}

pub async fn check_status(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<String>, AppError> {
    let s = state.database_service.status(id).await?;
    Ok(Json(s))
}

pub async fn create_database(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    ApiJson(req): ApiJson<CreateDatabaseRequest>,
) -> Result<Json<&'static str>, AppError> {
    state
        .database_service
        .create_database(id, &req.name, req.charset.as_deref().unwrap_or("utf8mb4"))
        .await?;
    Ok(Json("created"))
}

pub async fn drop_database(
    State(state): State<AppState>,
    Path((id, db_name)): Path<(i64, String)>,
) -> Result<Json<&'static str>, AppError> {
    state.database_service.drop_database(id, &db_name).await?;
    Ok(Json("dropped"))
}

pub async fn list_databases(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Vec<String>>, AppError> {
    let dbs = state.database_service.list_databases(id).await?;
    Ok(Json(dbs))
}

pub async fn create_user(
    State(state): State<AppState>,
    Path(id): Path<i64>,
    ApiJson(req): ApiJson<CreateUserRequest>,
) -> Result<Json<&'static str>, AppError> {
    state
        .database_service
        .create_user(
            id,
            &req.username,
            &req.password,
            req.host.as_deref().unwrap_or("localhost"),
        )
        .await?;
    Ok(Json("created"))
}

pub async fn drop_user(
    State(state): State<AppState>,
    Path((id, username)): Path<(i64, String)>,
) -> Result<Json<&'static str>, AppError> {
    state
        .database_service
        .drop_user(id, &username, "localhost")
        .await?;
    Ok(Json("dropped"))
}

pub async fn uninstall(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<&'static str>, AppError> {
    state.database_service.uninstall(id).await?;
    Ok(Json("uninstalled"))
}

/// 路由表（集中注册于 routes.rs 组合根）
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/databases", axum::routing::get(list))
        .route("/api/databases/:id", axum::routing::get(get))
        .route("/api/databases/:id", axum::routing::delete(delete))
        .route(
            "/api/databases/mysql/install",
            axum::routing::post(install_mysql),
        )
        .route(
            "/api/databases/redis/install",
            axum::routing::post(install_redis),
        )
        .route("/api/databases/:id/start", axum::routing::post(start))
        .route("/api/databases/:id/stop", axum::routing::post(stop))
        .route("/api/databases/:id/restart", axum::routing::post(restart))
        .route(
            "/api/databases/:id/status",
            axum::routing::get(check_status),
        )
        .route(
            "/api/databases/:id/databases",
            axum::routing::get(list_databases),
        )
        .route(
            "/api/databases/:id/databases",
            axum::routing::post(create_database),
        )
        .route(
            "/api/databases/:id/databases/:db_name",
            axum::routing::delete(drop_database),
        )
        .route("/api/databases/:id/users", axum::routing::post(create_user))
        .route(
            "/api/databases/:id/users/:username",
            axum::routing::delete(drop_user),
        )
        .route(
            "/api/databases/:id/uninstall",
            axum::routing::post(uninstall),
        )
}
