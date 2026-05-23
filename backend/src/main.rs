use std::net::SocketAddr;
use clap::Parser;
use tower_http::{cors::CorsLayer, trace::TraceLayer};

mod api;
mod application;
mod cli;
mod config;
mod core;
mod domain;
mod infrastructure;
mod metrics;
mod middleware;
mod plugin;
mod utils;
mod websocket;

use application::AppState;
use cli::Cli;

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    if cli.command.is_some() {
        // CLI 模式：执行命令后退出
        cli::run(cli).await;
    } else {
        // 服务器模式
        start_server().await;
    }
}

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
        .await
        .expect("服务器运行异常");
}
