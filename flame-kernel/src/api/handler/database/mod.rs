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
use utoipa::ToSchema;

/// 数据库实例（面板登记；root 口令不回显）
#[derive(Serialize, ToSchema)]
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
    pub resource_version: i64,
}

#[derive(Deserialize, ToSchema)]
pub struct InstallMysqlRequest {
    pub name: String,
    pub version: Option<String>,
    pub port: Option<i32>,
    pub root_password: Option<String>,
}

#[derive(Deserialize, ToSchema)]
pub struct InstallRedisRequest {
    pub name: String,
    pub version: Option<String>,
    pub port: Option<i32>,
    pub password: Option<String>,
}

#[derive(Deserialize, ToSchema)]
pub struct InstallPostgresRequest {
    pub name: String,
    pub version: Option<String>,
    pub port: Option<i32>,
    pub password: Option<String>,
}

/// MongoDB 安装请求：`version` 取 `8.0`（缺省）/ `9.0`；填写 `password` 时创建 `admin`
/// 超级用户并开启鉴权（≥ 8 位），留空则仅本机（127.0.0.1）免鉴权访问。
#[derive(Deserialize, ToSchema)]
pub struct InstallMongodbRequest {
    pub name: String,
    pub version: Option<String>,
    pub port: Option<i32>,
    pub password: Option<String>,
}

#[derive(Deserialize, ToSchema)]
pub struct CreateDatabaseRequest {
    pub name: String,
    pub charset: Option<String>,
}

/// 创建数据库用户（schema 名加前缀，避免与面板用户 `CreateUserRequest` 冲突）
#[derive(Deserialize, ToSchema)]
#[schema(as = DatabaseCreateUserRequest)]
pub struct CreateUserRequest {
    pub username: String,
    pub password: String,
    pub host: Option<String>,
}

/// Phase A2 扩展：数据库实例批量状态更新请求体（批量原子写事务）。
#[derive(Deserialize, ToSchema)]
pub struct BatchStatusRequest {
    /// 每项为 `[instance_id, 状态]`，状态限 `running` / `stopped` / `error`，单次 ≤ 200 项
    #[schema(value_type = Vec<Vec<serde_json::Value>>)]
    pub updates: Vec<(i64, String)>,
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
        resource_version: inst.resource_version,
    }
}

/// 数据库实例列表（分页）
#[utoipa::path(
    get,
    path = "/api/databases",
    tag = "databases",
    params(PaginationParams),
    responses(
        (status = 200, description = "实例分页", body = PaginatedResponse<DatabaseInstanceResponse>),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 database:read 权限"),
    ),
    security(("BearerAuth" = []))
)]
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

/// 单个数据库实例
#[utoipa::path(
    get,
    path = "/api/databases/{id}",
    tag = "databases",
    params(("id" = i64, Path, description = "实例 ID")),
    responses(
        (status = 200, description = "实例", body = DatabaseInstanceResponse),
        (status = 404, description = "实例不存在"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 database:read 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn get(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<DatabaseInstanceResponse>, AppError> {
    let instance = state.database_service.get_instance(id).await?;
    Ok(Json(to_response(instance)))
}

/// 删除实例登记（不卸载软件；卸载见 uninstall）
#[utoipa::path(
    delete,
    path = "/api/databases/{id}",
    tag = "databases",
    params(("id" = i64, Path, description = "实例 ID")),
    responses(
        (status = 200, description = "`\"deleted\"`", body = String),
        (status = 404, description = "实例不存在"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 database:delete 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn delete(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<&'static str>, AppError> {
    state.database_service.get_instance(id).await?;
    state.database_service.delete_instance(id).await?;
    Ok(Json("deleted"))
}

/// 经系统包管理器安装 mysql 并登记实例（端口缺省 3306；口令经 stdin / 环境变量传递，不进 argv）
#[utoipa::path(
    post,
    path = "/api/databases/mysql/install",
    tag = "databases",
    request_body = InstallMysqlRequest,
    responses(
        (status = 200, description = "已安装并登记", body = DatabaseInstanceResponse),
        (status = 400, description = "口令含控制字符等校验失败"),
        (status = 500, description = "安装失败（非 root 且无 sudo 时不可用，见 Doc/10）"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 database:create 权限"),
    ),
    security(("BearerAuth" = []))
)]
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

/// 经系统包管理器安装 redis 并登记实例（端口缺省 6379；口令经 stdin / 环境变量传递，不进 argv）
#[utoipa::path(
    post,
    path = "/api/databases/redis/install",
    tag = "databases",
    request_body = InstallRedisRequest,
    responses(
        (status = 200, description = "已安装并登记", body = DatabaseInstanceResponse),
        (status = 400, description = "口令含控制字符等校验失败"),
        (status = 500, description = "安装失败（非 root 且无 sudo 时不可用，见 Doc/10）"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 database:create 权限"),
    ),
    security(("BearerAuth" = []))
)]
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

/// 经系统包管理器安装 postgres 并登记实例（端口缺省 5432；口令经 stdin / 环境变量传递，不进 argv）
#[utoipa::path(
    post,
    path = "/api/databases/postgres/install",
    tag = "databases",
    request_body = InstallPostgresRequest,
    responses(
        (status = 200, description = "已安装并登记", body = DatabaseInstanceResponse),
        (status = 400, description = "口令含控制字符等校验失败"),
        (status = 500, description = "安装失败（非 root 且无 sudo 时不可用，见 Doc/10）"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 database:create 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn install_postgres(
    State(state): State<AppState>,
    ApiJson(req): ApiJson<InstallPostgresRequest>,
) -> Result<Json<DatabaseInstanceResponse>, AppError> {
    let port = req.port.unwrap_or(5432);
    let instance = state
        .database_service
        .install_postgres(
            req.version.as_deref(),
            port,
            req.password.as_deref(),
            &req.name,
        )
        .await?;
    Ok(Json(to_response(instance)))
}

/// 经 MongoDB 官方仓库（HTTPS + 签名校验）安装 mongod 并登记实例（端口缺省 27017；
/// 管理员密码经 mongosh stdin 传递，不进 argv）
#[utoipa::path(
    post,
    path = "/api/databases/mongodb/install",
    tag = "databases",
    request_body = InstallMongodbRequest,
    responses(
        (status = 200, description = "已安装并登记", body = DatabaseInstanceResponse),
        (status = 400, description = "不支持的版本 / 发行版、密码过短或含控制字符、已安装"),
        (status = 500, description = "安装失败（非 root 且无 sudo 时不可用，见 Doc/10）"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 database:create 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn install_mongodb(
    State(state): State<AppState>,
    ApiJson(req): ApiJson<InstallMongodbRequest>,
) -> Result<Json<DatabaseInstanceResponse>, AppError> {
    let port = req
        .port
        .unwrap_or(crate::database::mongodb::MONGODB_DEFAULT_PORT);
    let instance = state
        .database_service
        .install_mongodb(
            req.version.as_deref(),
            port,
            req.password.as_deref(),
            &req.name,
        )
        .await?;
    Ok(Json(to_response(instance)))
}

/// 启动数据库服务
#[utoipa::path(
    post,
    path = "/api/databases/{id}/start",
    tag = "databases",
    params(("id" = i64, Path, description = "实例 ID")),
    responses(
        (status = 200, description = "`\"started\"`", body = String),
        (status = 404, description = "实例不存在"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 database:start 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn start(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<&'static str>, AppError> {
    state.database_service.start(id).await?;
    Ok(Json("started"))
}

/// 停止数据库服务
#[utoipa::path(
    post,
    path = "/api/databases/{id}/stop",
    tag = "databases",
    params(("id" = i64, Path, description = "实例 ID")),
    responses(
        (status = 200, description = "`\"stopped\"`", body = String),
        (status = 404, description = "实例不存在"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 database:stop 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn stop(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<&'static str>, AppError> {
    state.database_service.stop(id).await?;
    Ok(Json("stopped"))
}

/// 重启数据库服务
#[utoipa::path(
    post,
    path = "/api/databases/{id}/restart",
    tag = "databases",
    params(("id" = i64, Path, description = "实例 ID")),
    responses(
        (status = 200, description = "`\"restarted\"`", body = String),
        (status = 404, description = "实例不存在"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 database:start 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn restart(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<&'static str>, AppError> {
    state.database_service.restart(id).await?;
    Ok(Json("restarted"))
}

/// 服务运行状态（`running` / `stopped`）
#[utoipa::path(
    get,
    path = "/api/databases/{id}/status",
    tag = "databases",
    params(("id" = i64, Path, description = "实例 ID")),
    responses(
        (status = 200, description = "状态", body = String),
        (status = 404, description = "实例不存在"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 database:read 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn check_status(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<String>, AppError> {
    let s = state.database_service.status(id).await?;
    Ok(Json(s))
}

/// 创建库（MySQL / MariaDB 可指定字符集，缺省 utf8mb4；PostgreSQL 忽略字符集）
#[utoipa::path(
    post,
    path = "/api/databases/{id}/databases",
    tag = "databases",
    params(("id" = i64, Path, description = "实例 ID")),
    request_body = CreateDatabaseRequest,
    responses(
        (status = 200, description = "`\"created\"`", body = String),
        (status = 400, description = "库名需为 1–64 位字母数字 / `_` / `$`；字符集不在白名单；Redis 不支持"),
        (status = 404, description = "实例不存在"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 database:create 权限"),
    ),
    security(("BearerAuth" = []))
)]
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

/// 删除库（系统库受保护）
#[utoipa::path(
    delete,
    path = "/api/databases/{id}/databases/{db_name}",
    tag = "databases",
    params(("id" = i64, Path, description = "实例 ID"),
        ("db_name" = String, Path, description = "库名")),
    responses(
        (status = 200, description = "`\"dropped\"`", body = String),
        (status = 400, description = "库名非法 / 系统库（mysql、information_schema、performance_schema、sys、postgres、template0/1）不可删除"),
        (status = 404, description = "实例不存在"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 database:delete 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn drop_database(
    State(state): State<AppState>,
    Path((id, db_name)): Path<(i64, String)>,
) -> Result<Json<&'static str>, AppError> {
    state.database_service.drop_database(id, &db_name).await?;
    Ok(Json("dropped"))
}

/// 列出用户库（系统库已排除）
#[utoipa::path(
    get,
    path = "/api/databases/{id}/databases",
    tag = "databases",
    params(("id" = i64, Path, description = "实例 ID")),
    responses(
        (status = 200, description = "库名列表", body = [String]),
        (status = 400, description = "Redis 不支持"),
        (status = 404, description = "实例不存在"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 database:read 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn list_databases(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<Vec<String>>, AppError> {
    let dbs = state.database_service.list_databases(id).await?;
    Ok(Json(dbs))
}

/// 创建数据库用户（口令不进 argv）
#[utoipa::path(
    post,
    path = "/api/databases/{id}/users",
    tag = "databases",
    params(("id" = i64, Path, description = "实例 ID")),
    request_body = CreateUserRequest,
    responses(
        (status = 200, description = "`\"created\"`", body = String),
        (status = 400, description = "用户名 / host / 口令校验失败"),
        (status = 404, description = "实例不存在"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 database:update 权限"),
    ),
    security(("BearerAuth" = []))
)]
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

/// 删除数据库用户（host 固定 localhost；内置超级用户受保护）
#[utoipa::path(
    delete,
    path = "/api/databases/{id}/users/{username}",
    tag = "databases",
    params(("id" = i64, Path, description = "实例 ID"),
        ("username" = String, Path, description = "用户名")),
    responses(
        (status = 200, description = "`\"dropped\"`", body = String),
        (status = 400, description = "用户名非法 / root、postgres 等内置超级用户不可删除"),
        (status = 404, description = "实例不存在"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 database:update 权限"),
    ),
    security(("BearerAuth" = []))
)]
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

/// 卸载数据库软件并删除实例登记
#[utoipa::path(
    post,
    path = "/api/databases/{id}/uninstall",
    tag = "databases",
    params(("id" = i64, Path, description = "实例 ID")),
    responses(
        (status = 200, description = "`\"uninstalled\"`", body = String),
        (status = 404, description = "实例不存在"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 database:delete 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn uninstall(
    State(state): State<AppState>,
    Path(id): Path<i64>,
) -> Result<Json<&'static str>, AppError> {
    state.database_service.uninstall(id).await?;
    Ok(Json("uninstalled"))
}

/// Phase A2 扩展：数据库实例批量状态原子更新（批量写事务，PATCH /api/databases/batch-status）。
/// 批量更新实例状态（单事务）
#[utoipa::path(
    patch,
    path = "/api/databases/batch-status",
    tag = "databases",
    request_body = BatchStatusRequest,
    responses(
        (status = 200, description = "`\"updated\"`", body = String),
        (status = 400, description = "为空 / 超过 200 项 / 状态不在 running、stopped、error 之内"),
        (status = 401, description = "未认证"),
        (status = 403, description = "缺少 database:update 权限"),
    ),
    security(("BearerAuth" = []))
)]
pub async fn update_batch_status(
    State(state): State<AppState>,
    ApiJson(req): ApiJson<BatchStatusRequest>,
) -> Result<Json<&'static str>, AppError> {
    if req.updates.is_empty() {
        return Err(AppError::BadRequest("updates 不能为空".into()));
    }
    if req.updates.len() > 200 {
        return Err(AppError::BadRequest("updates 单次最多 200 项".into()));
    }
    if let Some((id, s)) = req
        .updates
        .iter()
        .find(|(_, s)| !matches!(s.as_str(), "running" | "stopped" | "error"))
    {
        return Err(AppError::ValidationError(format!(
            "updates: instance {id} has unknown status {s:?} (running / stopped / error)"
        )));
    }
    state
        .database_service
        .update_instances_status_batch(&req.updates)
        .await?;
    Ok(Json("updated"))
}

/// 路由表（集中注册于 routes.rs 组合根）
pub fn routes() -> Router<AppState> {
    Router::new()
        .route("/api/databases", axum::routing::get(list))
        .route("/api/databases/{id}", axum::routing::get(get))
        .route("/api/databases/{id}", axum::routing::delete(delete))
        .route(
            "/api/databases/mysql/install",
            axum::routing::post(install_mysql),
        )
        .route(
            "/api/databases/redis/install",
            axum::routing::post(install_redis),
        )
        .route(
            "/api/databases/postgres/install",
            axum::routing::post(install_postgres),
        )
        .route(
            "/api/databases/mongodb/install",
            axum::routing::post(install_mongodb),
        )
        .route("/api/databases/{id}/start", axum::routing::post(start))
        .route("/api/databases/{id}/stop", axum::routing::post(stop))
        .route("/api/databases/{id}/restart", axum::routing::post(restart))
        .route(
            "/api/databases/{id}/status",
            axum::routing::get(check_status),
        )
        .route(
            "/api/databases/{id}/databases",
            axum::routing::get(list_databases),
        )
        .route(
            "/api/databases/{id}/databases",
            axum::routing::post(create_database),
        )
        .route(
            "/api/databases/{id}/databases/{db_name}",
            axum::routing::delete(drop_database),
        )
        .route(
            "/api/databases/{id}/users",
            axum::routing::post(create_user),
        )
        .route(
            "/api/databases/{id}/users/{username}",
            axum::routing::delete(drop_user),
        )
        .route(
            "/api/databases/{id}/uninstall",
            axum::routing::post(uninstall),
        )
        .route(
            "/api/databases/batch-status",
            axum::routing::patch(update_batch_status),
        )
}
