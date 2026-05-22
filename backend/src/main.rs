use axum::Router;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::{cors::CorsLayer, trace::TraceLayer};

mod api;
mod application;
mod config;
mod core;
mod domain;
mod infrastructure;
mod middleware;
mod plugin;
mod utils;
mod websocket;

/// Terminal 会话表：session_id → 子进程句柄
/// 供 websocket/mod.rs 中的 Web 终端使用
type SessionMap = Arc<Mutex<std::collections::HashMap<String, Arc<Mutex<tokio::process::Child>>>>>;

#[tokio::main]
async fn main() {
    // ── 1. 日志初始化 ────────────────────────────────────────────────────────
    //
    // 通过环境变量控制日志级别，例如：
    //   RUST_LOG=info cargo run
    //   RUST_LOG=ops_panel_backend=debug,tower_http=trace cargo run
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    // ── 2. 加载配置 ──────────────────────────────────────────────────────────
    //
    // 优先级（低→高）：默认值 → config.toml → 环境变量（OP_ 前缀）
    // 例：OP_PORT=9090  OP_JWT_SECRET=xxx  OP_ADMIN_PASSWORD=xxx
    let config = config::Config::load().expect("配置加载失败");
    tracing::info!("配置加载成功，监听端口: {}", config.port);

    // ── 3. 确保数据目录存在 ───────────────────────────────────────────────────
    //
    // SQLite 不会自动创建父目录，data/ 必须提前存在
    std::fs::create_dir_all("data").expect("无法创建 data/ 目录");

    // ── 4. 数据库连接 & 迁移 ─────────────────────────────────────────────────
    //
    // database_url 示例：sqlite:data/ops_panel.db?mode=rwc
    // mode=rwc：文件不存在时自动创建
    let db = sqlx::SqlitePool::connect(&config.database_url)
        .await
        .expect("数据库连接失败");

    sqlx::migrate!()
        .run(&db)
        .await
        .expect("数据库迁移失败");

    tracing::info!("数据库迁移完成");

    // ── 5. 初始化管理员账号 ───────────────────────────────────────────────────
    //
    // 首次启动时将 config.admin_username / admin_password 写入 users 表
    // 若账号已存在则跳过（INSERT OR IGNORE），bcrypt hash 存储
    application::seed_admin(&db, &config)
        .await
        .expect("初始化管理员账号失败");

    // ── 6. Terminal 会话 Map ──────────────────────────────────────────────────
    //
    // Key：前端生成的 session_id（UUID）
    // Value：tokio::process::Child（pty/shell 子进程）
    let sessions: SessionMap = Arc::new(Mutex::new(std::collections::HashMap::new()));

    // ── 7. 路由组装 ───────────────────────────────────────────────────────────
    //
    // /api/**        — REST API（见 api/mod.rs，含认证中间件分层）
    // /ws/**         — WebSocket 终端（见 websocket/mod.rs）
    let app = Router::new()
        .merge(api::routes())
        .nest("/ws", websocket::routes())
        // ── 全局中间件（从下往上执行）────────────────────────────────────────
        // CORS：开发阶段 permissive，生产环境建议改为明确的 allow_origin
        .layer(CorsLayer::permissive())
        // HTTP 请求 trace 日志（受 RUST_LOG 控制）
        .layer(TraceLayer::new_for_http())
        // 将数据库连接池注入所有 Handler
        .layer(axum::Extension(db))
        // 将 Terminal 会话 Map 注入 WebSocket Handler
        .layer(axum::Extension(sessions));

    // ── 8. 启动服务器 ────────────────────────────────────────────────────────
    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    tracing::info!("🔥 Flamepanel 启动成功 → http://{}", addr);

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("端口绑定失败，请检查端口是否被占用");

    axum::serve(listener, app)
        .await
        .expect("服务器运行异常");
}