use std::net::SocketAddr;
use tower_http::{cors::CorsLayer, trace::TraceLayer};

pub mod api;
pub mod application;
pub mod cli;
pub mod config;
pub mod core;
pub mod domain;
pub mod infrastructure;
pub mod metrics;
pub mod middleware;
pub mod plugin;
pub mod utils;
pub mod websocket;

use application::AppState;

pub async fn start_server() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let cfg = config::Config::load().expect("配置加载失败");
    tracing::info!("配置加载成功，监听端口: {}", cfg.port);

    std::fs::create_dir_all("data").expect("无法创建 data/ 目录");

    let db = sqlx::SqlitePool::connect(&cfg.database_url)
        .await
        .expect("数据库连接失败");

    sqlx::migrate!()
        .run(&db)
        .await
        .expect("数据库迁移失败");

    tracing::info!("数据库迁移完成");

    application::seed_admin(&db, &cfg)
        .await
        .expect("初始化管理员账号失败");

    let (metrics_tx, _rx) = tokio::sync::broadcast::channel::<metrics::MetricsSnapshot>(16);
    let metrics_history = std::sync::Arc::new(tokio::sync::Mutex::new(
        metrics::MetricsHistory::new(60),
    ));
    metrics::spawn_metrics_collector(metrics_history.clone(), metrics_tx.clone());

    let state = AppState::new(db, metrics_tx, metrics_history);

    let app = axum::Router::new()
        .merge(api::routes())
        .nest("/ws", websocket::routes())
        .with_state(state)
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http());

    let addr = SocketAddr::from(([0, 0, 0, 0], cfg.port));
    tracing::info!("Flamepanel 启动成功 -> http://{}", addr);

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .expect("端口绑定失败");

    axum::serve(listener, app.into_make_service())
        .with_graceful_shutdown(shutdown_signal())
        .await
        .expect("服务器运行异常");
}

async fn shutdown_signal() {
    tokio::signal::ctrl_c()
        .await
        .expect("failed to install Ctrl+C handler");
    tracing::info!("收到关闭信号，正在优雅退出...");
}
