use axum::{
    extract::{Path, State},
    Json,
};
use serde_json::json;
use tokio::process::Command;

use crate::application::AppState;
use crate::core::error::AppError;
use crate::domain::{CreateDatabaseRequest, DatabaseBackup, DatabaseInstance};
use crate::middleware::auth::CurrentUser;

// ─── Image / port defaults per DB type ──────────────────────────────────────────

fn default_image(db_type: &str, version: &str) -> String {
    let v = if version.is_empty() { "latest" } else { version };
    match db_type {
        "mysql" => format!("mysql:{v}"),
        "mariadb" => format!("mariadb:{v}"),
        "postgresql" => format!("postgres:{v}"),
        "redis" => format!("redis:{v}"),
        "mongodb" => format!("mongo:{v}"),
        _ => format!("{db_type}:{v}"),
    }
}

fn default_port(db_type: &str) -> i32 {
    match db_type {
        "mysql" | "mariadb" => 3306,
        "postgresql" => 5432,
        "redis" => 6379,
        "mongodb" => 27017,
        _ => 0,
    }
}

fn env_vars(db_type: &str, password: &str) -> Vec<(String, String)> {
    match db_type {
        "mysql" | "mariadb" => vec![
            ("MYSQL_ROOT_PASSWORD".into(), password.to_string()),
        ],
        "postgresql" => vec![
            ("POSTGRES_PASSWORD".into(), password.to_string()),
        ],
        "redis" => vec![
            ("REDIS_PASSWORD".into(), password.to_string()),
        ],
        "mongodb" => vec![
            ("MONGO_INITDB_ROOT_USERNAME".into(), "root".into()),
            ("MONGO_INITDB_ROOT_PASSWORD".into(), password.to_string()),
        ],
        _ => vec![],
    }
}

fn db_username_default(db_type: &str) -> String {
    match db_type {
        "redis" => "default".into(),
        _ => "root".into(),
    }
}

// ─── GET /databases ────────────────────────────────────────────────────────────

pub async fn list(
    State(state): State<AppState>,
    _user: CurrentUser,
) -> Result<Json<Vec<DatabaseInstance>>, AppError> {
    state.db_repo.list_all().await.map(Json)
}

// ─── POST /databases ───────────────────────────────────────────────────────────

pub async fn create(
    State(state): State<AppState>,
    _user: CurrentUser,
    Json(req): Json<CreateDatabaseRequest>,
) -> Result<Json<DatabaseInstance>, AppError> {
    let db_type = req.db_type.to_lowercase();
    if !["mysql", "mariadb", "postgresql", "redis", "mongodb"].contains(&db_type.as_str()) {
        return Err(AppError::BadRequest("不支持的数据库类型".into()));
    }
    let version = req.version.clone().unwrap_or_default();
    let image = default_image(&db_type, &version);
    let container_name = format!("flamepanel-{}-{}", &db_type, &req.name);
    let host_port = req.port.unwrap_or_else(|| default_port(&db_type));
    let username = db_username_default(&db_type);
    let data_dir = format!("data/db_{}", &req.name);

    // Ensure data dir
    let _ = std::fs::create_dir_all(&data_dir);

    // Pull image
    tracing::info!("Pulling image: {image}");
    let pull = Command::new("docker")
        .args(["pull", &image])
        .output()
        .await
        .map_err(|e| AppError::Internal(format!("Docker pull 失败: {e}")))?;
    if !pull.status.success() {
        let err = String::from_utf8_lossy(&pull.stderr);
        return Err(AppError::Internal(format!("拉取镜像失败: {err}")));
    }

    // Build docker run command — use owned Strings to avoid temporary lifetime issues
    let port_map = format!("{host_port}:{host_port}");
    let cwd = std::env::current_dir().unwrap_or_default();
    let cwd_display = cwd.display().to_string();
    let vol_src = format!("{cwd_display}/{data_dir}");

    let mut args: Vec<String> = vec![
        "run".into(), "-d".into(),
        "--name".into(), container_name.clone(),
        "-p".into(), port_map,
        "--restart".into(), "unless-stopped".into(),
    ];

    // DB-specific volume mount
    match db_type.as_str() {
        "mysql" | "mariadb" => {
            args.push("-v".into());
            args.push(format!("{vol_src}:/var/lib/mysql"));
        }
        "postgresql" => {
            args.push("-v".into());
            args.push(format!("{vol_src}:/var/lib/postgresql/data"));
        }
        "redis" => {
            args.push("-v".into());
            args.push(format!("{vol_src}:/data"));
        }
        "mongodb" => {
            args.push("-v".into());
            args.push(format!("{vol_src}:/data/db"));
        }
        _ => {}
    }

    // Environment variables
    for (key, val) in env_vars(&db_type, &req.password) {
        args.push("-e".into());
        args.push(format!("{key}={val}"));
    }

    // Image and optional Redis args
    args.push(image.clone());
    if db_type == "redis" {
        args.push("redis-server".into());
        args.push("--requirepass".into());
        args.push(req.password.clone());
        args.push("--port".into());
        args.push(host_port.to_string());
    }

    // Create container
    tracing::info!("Creating container: {container_name}");
    let run = Command::new("docker")
        .args(&args)
        .output()
        .await
        .map_err(|e| AppError::Internal(format!("Docker run 失败: {e}")))?;

    if !run.status.success() {
        let err = String::from_utf8_lossy(&run.stderr);
        return Err(AppError::Internal(format!("容器创建失败: {err}")));
    }

    let container_id = String::from_utf8_lossy(&run.stdout).trim().to_string();

    // Insert into DB
    let instance = state.db_repo.create(
        &req.name, &db_type, &version, host_port,
        Some(&container_id), &username, &req.password, Some(&data_dir),
    ).await?;

    // Update status to running
    let _ = state.db_repo.update_status(instance.id, "running").await;
    let updated = state.db_repo.find_by_id(instance.id).await?.unwrap();

    tracing::info!("Database instance '{}' created (container: {})", req.name, container_id);
    Ok(Json(updated))
}

// ─── POST /databases/:id/start ─────────────────────────────────────────────────

pub async fn start(
    State(state): State<AppState>,
    _user: CurrentUser,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, AppError> {
    let inst = state.db_repo.find_by_id(id).await?
        .ok_or(AppError::NotFound("数据库实例不存在".into()))?;
    if let Some(ref cid) = inst.container_id {
        let out = Command::new("docker").args(["start", cid]).output().await
            .map_err(|e| AppError::Internal(format!("启动容器失败: {e}")))?;
        if !out.status.success() {
            let err = String::from_utf8_lossy(&out.stderr);
            return Err(AppError::Internal(format!("启动失败: {err}")));
        }
    }
    state.db_repo.update_status(id, "running").await?;
    Ok(Json(json!({"message": "已启动"})))
}

// ─── POST /databases/:id/stop ──────────────────────────────────────────────────

pub async fn stop(
    State(state): State<AppState>,
    _user: CurrentUser,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, AppError> {
    let inst = state.db_repo.find_by_id(id).await?
        .ok_or(AppError::NotFound("数据库实例不存在".into()))?;
    if let Some(ref cid) = inst.container_id {
        let out = Command::new("docker").args(["stop", cid]).output().await
            .map_err(|e| AppError::Internal(format!("停止容器失败: {e}")))?;
        if !out.status.success() {
            let err = String::from_utf8_lossy(&out.stderr);
            return Err(AppError::Internal(format!("停止失败: {err}")));
        }
    }
    state.db_repo.update_status(id, "stopped").await?;
    Ok(Json(json!({"message": "已停止"})))
}

// ─── DELETE /databases/:id ─────────────────────────────────────────────────────

pub async fn delete(
    State(state): State<AppState>,
    _user: CurrentUser,
    Path(id): Path<i64>,
) -> Result<Json<serde_json::Value>, AppError> {
    let inst = state.db_repo.find_by_id(id).await?
        .ok_or(AppError::NotFound("数据库实例不存在".into()))?;

    // Stop and remove container
    if let Some(ref cid) = inst.container_id {
        let _ = Command::new("docker").args(["stop", cid]).output().await;
        let _ = Command::new("docker").args(["rm", cid]).output().await;
    }
    // Remove data dir
    if let Some(ref dir) = inst.data_dir {
        let _ = std::fs::remove_dir_all(dir);
    }

    state.db_repo.delete(id).await?;
    Ok(Json(json!({"message": "已删除"})))
}

// ─── POST /databases/:id/backup ────────────────────────────────────────────────

pub async fn backup(
    State(state): State<AppState>,
    _user: CurrentUser,
    Path(id): Path<i64>,
) -> Result<Json<DatabaseBackup>, AppError> {
    let inst = state.db_repo.find_by_id(id).await?
        .ok_or(AppError::NotFound("数据库实例不存在".into()))?;

    if inst.status != "running" {
        return Err(AppError::BadRequest("数据库未运行".into()));
    }

    let cid = inst.container_id.as_ref()
        .ok_or(AppError::BadRequest("容器未找到".into()))?;

    let backup_dir = format!("data/db_{}_backups", &inst.name);
    let _ = std::fs::create_dir_all(&backup_dir);
    let ts = chrono::Utc::now().format("%Y%m%d_%H%M%S");
    let filename = format!("{}_{}_{}.sql", inst.name, inst.db_type, ts);
    let host_path = format!("{}/{}", std::env::current_dir().unwrap_or_default().display(), &backup_dir);
    let container_path = format!("/tmp/{}", filename);

    let (status, _output) = match inst.db_type.as_str() {
        "mysql" | "mariadb" => {
            let out = Command::new("docker")
                .args(["exec", cid, "mysqldump", "-uroot", &format!("-p{}", inst.password), "--all-databases", "--result-file", &container_path])
                .output().await;
            (out.is_ok(), out)
        }
        "postgresql" => {
            let out = Command::new("docker")
                .args(["exec", "-e", &format!("PGPASSWORD={}", inst.password), cid, "pg_dumpall", "-U", "root", "-f", &container_path])
                .output().await;
            (out.is_ok(), out)
        }
        "redis" => {
            let out = Command::new("docker")
                .args(["exec", cid, "redis-cli", "-a", &inst.password, "--rdb", &container_path, "SAVE"])
                .output().await;
            (out.is_ok(), out)
        }
        "mongodb" => {
            let out = Command::new("docker")
                .args(["exec", cid, "mongodump", "--username", "root", "--password", &inst.password, "--archive", &container_path])
                .output().await;
            (out.is_ok(), out)
        }
        _ => return Err(AppError::BadRequest("不支持的备份类型".into())),
    };

    if !status {
        let err = _output.map_or("未知错误".into(), |o| String::from_utf8_lossy(&o.stderr).to_string());
        return Err(AppError::Internal(format!("备份失败: {err}")));
    }

    // Copy file from container to host
    let _ = Command::new("docker")
        .args(["cp", &format!("{cid}:{container_path}"), &host_path])
        .output().await;

    // Determine size
    let local_file = std::path::Path::new(&host_path).join(&filename);
    let size = std::fs::metadata(&local_file).map(|m| m.len() as i64).unwrap_or(0);

    let backup = state.db_backup_repo.create(id, &filename, size).await?;
    Ok(Json(backup))
}

// ─── GET /databases/:id/backups ────────────────────────────────────────────────

pub async fn list_backups(
    State(state): State<AppState>,
    _user: CurrentUser,
    Path(id): Path<i64>,
) -> Result<Json<Vec<DatabaseBackup>>, AppError> {
    state.db_backup_repo.list_by_instance(id).await.map(Json)
}

// ─── Routes ───────────────────────────────────────────────────────────────────

pub fn routes() -> axum::Router<AppState> {
    axum::Router::new()
        .route("/", axum::routing::get(list).post(create))
        .route("/{id}/start", axum::routing::post(start))
        .route("/{id}/stop", axum::routing::post(stop))
        .route("/{id}", axum::routing::delete(delete))
        .route("/{id}/backup", axum::routing::post(backup))
        .route("/{id}/backups", axum::routing::get(list_backups))
}
