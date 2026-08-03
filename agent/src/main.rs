use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;

use axum::{
    extract::Query,
    http::HeaderMap,
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use sysinfo::System;

// ─── Config ────────────────────────────────────────────────────────────────────

#[derive(Debug, Clone)]
struct AgentConfig {
    panel_url: String,
    node_name: String,
    node_host: String,
    agent_port: u16,
    auth_token: String,
}

impl AgentConfig {
    fn from_env() -> Self {
        Self {
            panel_url: std::env::var("PANEL_URL")
                .unwrap_or_else(|_| "http://127.0.0.1:8080".into()),
            node_name: std::env::var("NODE_NAME").unwrap_or_else(|_| hostname()),
            node_host: std::env::var("NODE_HOST").unwrap_or_else(|_| local_ip()),
            agent_port: std::env::var("AGENT_PORT")
                .ok()
                .and_then(|v| v.parse().ok())
                .unwrap_or(9527),
            auth_token: std::env::var("AUTH_TOKEN").unwrap_or_else(|_| uuid_v4()),
        }
    }
}

fn hostname() -> String {
    System::host_name().unwrap_or_else(|| "unknown".into())
}

fn local_ip() -> String {
    use std::net::UdpSocket;
    if let Ok(s) = UdpSocket::bind("0.0.0.0:0") {
        if let Ok(()) = s.connect("8.8.8.8:80") {
            if let Ok(addr) = s.local_addr() {
                return addr.ip().to_string();
            }
        }
    }
    "127.0.0.1".into()
}

fn uuid_v4() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_nanos();
    format!("node-token-{ts:x}")
}

// ─── Metrics ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
struct HeartbeatPayload {
    cpu_usage: f32,
    memory_usage_percent: f32,
    disk_usage_percent: f32,
    load_one: f32,
}

fn collect_metrics(sys: &mut System) -> HeartbeatPayload {
    sys.refresh_all();

    let cpu = sys.global_cpu_usage().clamp(0.0, 100.0);
    let mem_total = sys.total_memory();
    let mem_used = sys.used_memory();
    let mem_pct = if mem_total > 0 {
        (mem_used as f32 / mem_total as f32) * 100.0
    } else {
        0.0
    };

    let disks = sysinfo::Disks::new_with_refreshed_list();
    let (disk_total, disk_used) = disks.iter().fold((0u64, 0u64), |(t, u), d| {
        (
            t + d.total_space(),
            u + d.total_space() - d.available_space(),
        )
    });
    let disk_pct = if disk_total > 0 {
        ((disk_used as f64 / disk_total as f64) * 100.0) as f32
    } else {
        0.0
    };

    let load = System::load_average();

    HeartbeatPayload {
        cpu_usage: cpu,
        memory_usage_percent: mem_pct,
        disk_usage_percent: disk_pct,
        load_one: load.one as f32,
    }
}

// ─── Register ──────────────────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct RegisterPayload {
    name: String,
    host: String,
    agent_port: u16,
    auth_token: String,
}

async fn register(config: &AgentConfig) -> Option<i64> {
    let payload = RegisterPayload {
        name: config.node_name.clone(),
        host: config.node_host.clone(),
        agent_port: config.agent_port,
        auth_token: config.auth_token.clone(),
    };

    let client = reqwest::Client::new();
    match client
        .post(format!("{}/api/nodes/register", config.panel_url))
        .json(&payload)
        .timeout(Duration::from_secs(10))
        .send()
        .await
    {
        Ok(resp) => {
            let status = resp.status();
            if status.is_success() {
                #[derive(Deserialize)]
                struct R {
                    id: i64,
                }
                if let Ok(r) = resp.json::<R>().await {
                    eprintln!("[agent] 注册成功，节点 ID: {}", r.id);
                    return Some(r.id);
                }
            }
            eprintln!("[agent] 注册失败: HTTP {status}");
        }
        Err(e) => eprintln!("[agent] 注册请求失败: {e}"),
    }
    None
}

// ─── Heartbeat ─────────────────────────────────────────────────────────────────

async fn send_heartbeat(panel_url: &str, node_id: i64, metrics: &HeartbeatPayload) {
    let client = reqwest::Client::new();
    match client
        .post(format!("{panel_url}/api/nodes/heartbeat/{node_id}"))
        .json(metrics)
        .timeout(Duration::from_secs(10))
        .send()
        .await
    {
        Ok(resp) => {
            if !resp.status().is_success() {
                eprintln!("[agent] 心跳失败: HTTP {}", resp.status());
            }
        }
        Err(e) => eprintln!("[agent] 心跳请求失败: {e}"),
    }
}

// ─── Agent HTTP Server ─────────────────────────────────────────────────────────

fn check_auth(token: &str) -> bool {
    let expected = std::env::var("AUTH_TOKEN").unwrap_or_else(|_| String::new());
    token == expected
}

fn auth_error() -> (axum::http::StatusCode, Json<serde_json::Value>) {
    (
        axum::http::StatusCode::UNAUTHORIZED,
        Json(serde_json::json!({"error": "Unauthorized"})),
    )
}

#[derive(Debug, Deserialize)]
struct ExecRequest {
    command: String,
    timeout_secs: Option<u64>,
}

#[derive(Debug, Deserialize)]
struct ListQuery {
    path: Option<String>,
}

#[derive(Debug, Deserialize)]
struct DownloadQuery {
    path: String,
}

#[derive(Debug, Deserialize)]
struct UploadQuery {
    path: String,
}

#[derive(Debug, Serialize)]
struct FileEntry {
    name: String,
    is_dir: bool,
    size: u64,
    modified: String,
}

#[derive(Debug, Serialize)]
struct ExecResponse {
    output: String,
    exit_code: i32,
    duration_ms: u64,
}

async fn exec_endpoint(
    headers: HeaderMap,
    Json(body): Json<ExecRequest>,
) -> Result<Json<ExecResponse>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    let token = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if !check_auth(token) {
        return Err(auth_error());
    }

    let timeout = body.timeout_secs.unwrap_or(30);
    let start = std::time::Instant::now();

    let result = tokio::time::timeout(
        Duration::from_secs(timeout),
        tokio::process::Command::new(if cfg!(windows) { "cmd" } else { "sh" })
            .arg(if cfg!(windows) { "/C" } else { "-c" })
            .arg(&body.command)
            .output(),
    )
    .await;

    let response = match result {
        Ok(Ok(output)) => ExecResponse {
            output: String::from_utf8_lossy(&output.stdout).to_string()
                + String::from_utf8_lossy(&output.stderr).as_ref(),
            exit_code: output.status.code().unwrap_or(-1),
            duration_ms: start.elapsed().as_millis() as u64,
        },
        Ok(Err(e)) => ExecResponse {
            output: format!("Command execution error: {e}"),
            exit_code: -1,
            duration_ms: start.elapsed().as_millis() as u64,
        },
        Err(_) => ExecResponse {
            output: "Command timed out".into(),
            exit_code: -1,
            duration_ms: start.elapsed().as_millis() as u64,
        },
    };

    Ok(Json(response))
}

async fn list_files_endpoint(
    headers: HeaderMap,
    Query(q): Query<ListQuery>,
) -> Result<Json<Vec<FileEntry>>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    let token = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if !check_auth(token) {
        return Err(auth_error());
    }

    let base = q.path.as_deref().unwrap_or(".");
    let dir = PathBuf::from(base);

    let mut entries = Vec::new();
    if let Ok(mut rd) = tokio::fs::read_dir(&dir).await {
        while let Ok(Some(entry)) = rd.next_entry().await {
            let metadata = entry.metadata().await;
            let name = entry.file_name().to_string_lossy().to_string();
            let is_dir = metadata.as_ref().map(|m| m.is_dir()).unwrap_or(false);
            let size = metadata.as_ref().map(|m| m.len()).unwrap_or(0);
            let modified = metadata
                .ok()
                .and_then(|m| m.modified().ok())
                .map(|t| {
                    let secs = t
                        .duration_since(std::time::UNIX_EPOCH)
                        .unwrap_or_default()
                        .as_secs();
                    secs.to_string()
                })
                .unwrap_or_default();
            entries.push(FileEntry {
                name,
                is_dir,
                size,
                modified,
            });
        }
    }
    entries.sort_by(|a, b| b.is_dir.cmp(&a.is_dir).then(a.name.cmp(&b.name)));
    Ok(Json(entries))
}

async fn download_file_endpoint(
    headers: HeaderMap,
    Query(q): Query<DownloadQuery>,
) -> Result<Vec<u8>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    let token = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if !check_auth(token) {
        return Err(auth_error());
    }

    let path = PathBuf::from(&q.path);
    if !path.is_file() {
        return Err((
            axum::http::StatusCode::NOT_FOUND,
            Json(serde_json::json!({"error": "File not found"})),
        ));
    }

    tokio::fs::read(&path).await.map_err(|e| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("Read error: {e}")})),
        )
    })
}

async fn upload_file_endpoint(
    headers: HeaderMap,
    Query(q): Query<UploadQuery>,
    body: axum::body::Bytes,
) -> Result<Json<serde_json::Value>, (axum::http::StatusCode, Json<serde_json::Value>)> {
    let token = headers
        .get("Authorization")
        .and_then(|v| v.to_str().ok())
        .unwrap_or("");
    if !check_auth(token) {
        return Err(auth_error());
    }

    let target = PathBuf::from(&q.path);
    if let Some(parent) = target.parent() {
        let _ = std::fs::create_dir_all(parent);
    }

    tokio::fs::write(&target, &body).await.map_err(|e| {
        (
            axum::http::StatusCode::INTERNAL_SERVER_ERROR,
            Json(serde_json::json!({"error": format!("Write error: {e}")})),
        )
    })?;

    Ok(Json(
        serde_json::json!({"message": "ok", "size": body.len()}),
    ))
}

fn agent_routes() -> Router {
    Router::new()
        .route("/exec", post(exec_endpoint))
        .route("/files/list", get(list_files_endpoint))
        .route("/files/download", get(download_file_endpoint))
        .route("/files/upload", post(upload_file_endpoint))
}

// ─── Main ──────────────────────────────────────────────────────────────────────

#[tokio::main]
async fn main() {
    let config = AgentConfig::from_env();

    eprintln!("[agent] Flamepanel Agent 启动");
    eprintln!("[agent] 名称: {}", config.node_name);
    eprintln!("[agent] 主机: {}:{}", config.node_host, config.agent_port);
    eprintln!("[agent] 面板: {}", config.panel_url);

    // Register with the panel
    let node_id = loop {
        if let Some(id) = register(&config).await {
            break id;
        }
        eprintln!("[agent] 5 秒后重试...");
        tokio::time::sleep(Duration::from_secs(5)).await;
    };

    let config_clone = config.clone();

    // Spawn heartbeat task
    let panel_url = config.panel_url.clone();
    tokio::spawn(async move {
        let mut sys = System::new_all();
        tokio::time::sleep(Duration::from_secs(3)).await;
        let mut interval = tokio::time::interval(Duration::from_secs(10));
        loop {
            interval.tick().await;
            let metrics = collect_metrics(&mut sys);
            send_heartbeat(&panel_url, node_id, &metrics).await;
        }
    });

    // Start HTTP server for agent commands
    let addr = SocketAddr::from(([0, 0, 0, 0], config_clone.agent_port));
    eprintln!("[agent] HTTP 服务监听: {addr}");

    let app = agent_routes();
    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("Agent HTTP 端口绑定失败");

    axum::serve(listener, app.into_make_service())
        .await
        .expect("Agent HTTP 服务运行异常");
}
