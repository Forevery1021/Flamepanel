//! R-01B dedicated probe, NOT the production kernel startup path.
//! No AppConfig, Services, command runner, scheduler, plugins or outbound client.
#[cfg(all(feature = "sqlite", unix))]
mod probe {
    use axum::{
        extract::{Request, State},
        http::StatusCode,
        middleware::{self, Next},
        response::{IntoResponse, Response},
        routing::get,
        Json, Router,
    };
    use flame_kernel::infrastructure::sqlite::{run_migrations, seed_rbac};
    use flame_kernel::utils::constant_time::constant_time_eq;
    use serde_json::json;
    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
    use std::io::{Read, Write};
    use std::os::unix::fs::OpenOptionsExt;
    use std::path::Path;
    use std::sync::Arc;
    use std::time::Duration;

    const LIMIT: u64 = 512 * 1024 * 1024;
    const MODE: &str = "flamepanel/restore-probe/v1";

    #[derive(Clone)]
    struct ProbeState {
        pool: sqlx::SqlitePool,
        token: Arc<String>,
    }

    async fn authenticate(
        State(state): State<ProbeState>,
        request: Request,
        next: Next,
    ) -> Response {
        let supplied = request
            .headers()
            .get("authorization")
            .and_then(|h| h.to_str().ok())
            .and_then(|h| h.strip_prefix("Bearer "))
            .unwrap_or_default();
        if !constant_time_eq(supplied.as_bytes(), state.token.as_bytes()) {
            return (
                StatusCode::UNAUTHORIZED,
                Json(json!({"error": "unauthorized"})),
            )
                .into_response();
        }
        next.run(request).await
    }

    async fn health() -> Json<serde_json::Value> {
        Json(json!({"mode": MODE, "status": "ready", "production_kernel": false}))
    }

    async fn summary(
        State(state): State<ProbeState>,
    ) -> Result<Json<serde_json::Value>, StatusCode> {
        let query = async {
            // Static allowlist; no SQL, paths, IDs or settings supplied by the caller.
            let (users,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM users")
                .fetch_one(&state.pool)
                .await?;
            let (websites,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM websites")
                .fetch_one(&state.pool)
                .await?;
            let (scheduled_tasks,): (i64,) = sqlx::query_as("SELECT COUNT(*) FROM scheduled_tasks")
                .fetch_one(&state.pool)
                .await?;
            Ok::<_, sqlx::Error>(json!({"mode": MODE, "counts": {
                "users": users, "websites": websites, "scheduled_tasks": scheduled_tasks
            }}))
        };
        let result = tokio::time::timeout(Duration::from_secs(2), query)
            .await
            .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?
            .map_err(|_| StatusCode::SERVICE_UNAVAILABLE)?;
        Ok(Json(result))
    }

    fn copy_snapshot(source: &Path, target: &Path) -> Result<(), &'static str> {
        for suffix in ["-wal", "-shm", "-journal"] {
            let mut sidecar = source.as_os_str().to_os_string();
            sidecar.push(suffix);
            if Path::new(&sidecar).exists() {
                return Err("LIVE_SOURCE_REFUSED");
            }
        }
        let mut input = std::fs::OpenOptions::new()
            .read(true)
            .custom_flags(libc::O_NOFOLLOW | libc::O_NONBLOCK)
            .open(source)
            .map_err(|_| "SOURCE_UNAVAILABLE")?;
        let before = input.metadata().map_err(|_| "SOURCE_UNAVAILABLE")?;
        if !before.is_file() || before.len() == 0 || before.len() > LIMIT {
            return Err("SOURCE_LIMIT");
        }
        let mut output = std::fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .mode(0o600)
            .open(target)
            .map_err(|_| "COPY_FAILED")?;
        let copied = std::io::copy(&mut (&mut input).take(LIMIT + 1), &mut output)
            .map_err(|_| "COPY_FAILED")?;
        output.sync_all().map_err(|_| "COPY_FAILED")?;
        let after = input.metadata().map_err(|_| "SOURCE_UNAVAILABLE")?;
        if copied != before.len()
            || after.len() != before.len()
            || before.modified().ok() != after.modified().ok()
        {
            return Err("SOURCE_CHANGED");
        }
        Ok(())
    }

    async fn migrated_pool(path: &Path) -> Result<sqlx::SqlitePool, &'static str> {
        let options = SqliteConnectOptions::new()
            .filename(path)
            .create_if_missing(false)
            .foreign_keys(true)
            .pragma("trusted_schema", "OFF")
            .busy_timeout(Duration::from_secs(2));
        let pool = SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options.clone())
            .await
            .map_err(|_| "DATABASE_OPEN_FAILED")?;
        let checked = async {
            let (integrity,): (String,) = sqlx::query_as("PRAGMA integrity_check")
                .fetch_one(&pool).await.map_err(|_| "INTEGRITY_FAILED")?;
            if integrity != "ok" { return Err("INTEGRITY_FAILED"); }
            // Fail closed on executable/unexpected schema before applying production SQL migrations.
            let (unsafe_schema,): (i64,) = sqlx::query_as(
                "SELECT (SELECT COUNT(*) FROM sqlite_schema WHERE type IN ('trigger', 'view')) + (SELECT COUNT(*) FROM pragma_table_list WHERE type IN ('virtual', 'shadow'))"
            ).fetch_one(&pool).await.map_err(|_| "SCHEMA_CHECK_FAILED")?;
            if unsafe_schema != 0 { return Err("ACTIVE_SCHEMA_REFUSED"); }
            let (core,): (i64,) = sqlx::query_as(
                "SELECT COUNT(*) FROM sqlite_schema WHERE type = 'table' AND name IN ('users','permissions','panel_settings')"
            ).fetch_one(&pool).await.map_err(|_| "SCHEMA_CHECK_FAILED")?;
            if core != 3 { return Err("CORE_SCHEMA_REQUIRED"); }
            sqlx::query("SELECT id, username, password_hash, role FROM users LIMIT 0")
                .execute(&pool).await.map_err(|_| "CORE_COLUMNS_REQUIRED")?;
            sqlx::query("SELECT id, resource, action FROM permissions LIMIT 0")
                .execute(&pool).await.map_err(|_| "CORE_COLUMNS_REQUIRED")?;
            sqlx::query("SELECT key, value FROM panel_settings LIMIT 0")
                .execute(&pool).await.map_err(|_| "CORE_COLUMNS_REQUIRED")?;
            run_migrations(&pool).await.map_err(|_| "MIGRATION_FAILED")?;
            seed_rbac(&pool).await.map_err(|_| "RBAC_SEED_FAILED")?;
            let (integrity,): (String,) = sqlx::query_as("PRAGMA integrity_check")
                .fetch_one(&pool).await.map_err(|_| "INTEGRITY_FAILED")?;
            if integrity != "ok" { return Err("INTEGRITY_FAILED"); }
            Ok::<_, &'static str>(())
        }.await;
        pool.close().await;
        checked?;
        // Close write pool before HTTP; every subsequent connection is OS/SQLite read-only.
        SqlitePoolOptions::new()
            .max_connections(1)
            .connect_with(options.read_only(true))
            .await
            .map_err(|_| "READ_ONLY_OPEN_FAILED")
    }

    pub async fn run() -> Result<(), &'static str> {
        let args: Vec<_> = std::env::args_os().skip(1).collect();
        if args.len() != 4 || args[0] != "--snapshot" || args[2] != "--ttl" {
            return Err("USAGE_SNAPSHOT_TTL_REQUIRED");
        }
        let ttl: u64 = args[3]
            .to_str()
            .and_then(|v| v.parse().ok())
            .filter(|v| (1..=300).contains(v))
            .ok_or("INVALID_TTL")?;
        let mut token = String::new();
        std::io::stdin()
            .take(66)
            .read_to_string(&mut token)
            .map_err(|_| "TOKEN_REQUIRED")?;
        let token = token.trim_end_matches('\n');
        if token.len() != 64 || !token.bytes().all(|b| b.is_ascii_hexdigit()) {
            return Err("TOKEN_REQUIRED");
        }
        let temp = tempfile::Builder::new()
            .prefix(".probe-")
            .tempdir_in(std::env::current_dir().map_err(|_| "WORKDIR_REQUIRED")?)
            .map_err(|_| "WORKDIR_REQUIRED")?;
        let copy = temp.path().join("probe.db");
        copy_snapshot(Path::new(&args[1]), &copy)?;
        let deadline = tokio::time::Instant::now() + Duration::from_secs(ttl);
        let pool = tokio::time::timeout_at(deadline, migrated_pool(&copy))
            .await
            .map_err(|_| "MIGRATION_TIMEOUT")??;
        let state = ProbeState {
            pool: pool.clone(),
            token: Arc::new(token.to_owned()),
        };
        let app = Router::new()
            .route("/health", get(health))
            .route("/summary", get(summary))
            .layer(middleware::from_fn_with_state(state.clone(), authenticate))
            .with_state(state);
        let listener = tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
            .await
            .map_err(|_| "BIND_FAILED")?;
        let port = listener.local_addr().map_err(|_| "BIND_FAILED")?.port();
        // Register shutdown before announcing readiness. Token and database content never printed.
        let mut terminate =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .map_err(|_| "SIGNAL_FAILED")?;
        println!("{}", json!({"mode": MODE, "port": port, "migrated": true}));
        std::io::stdout().flush().map_err(|_| "READY_FAILED")?;
        let served = axum::serve(listener, app)
            .with_graceful_shutdown(async move {
                tokio::select! {
                    _ = tokio::time::sleep_until(deadline) => {},
                    _ = terminate.recv() => {},
                    _ = tokio::signal::ctrl_c() => {},
                }
            })
            .await;
        pool.close().await;
        served.map_err(|_| "SERVE_FAILED")?;
        temp.close().map_err(|_| "CLEANUP_FAILED")?;
        Ok(())
    }
}

#[cfg(all(feature = "sqlite", unix))]
#[tokio::main]
async fn main() -> std::process::ExitCode {
    match probe::run().await {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(code) => {
            eprintln!("RESTORE_PROBE: {code}");
            std::process::ExitCode::FAILURE
        }
    }
}

#[cfg(not(all(feature = "sqlite", unix)))]
fn main() -> std::process::ExitCode {
    eprintln!("RESTORE_PROBE: UNIX_SQLITE_REQUIRED");
    std::process::ExitCode::FAILURE
}
