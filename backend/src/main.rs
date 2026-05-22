use axum::Router;
use std::net::SocketAddr;
use std::sync::Arc;
use tokio::sync::Mutex;
use tower_http::{cors::CorsLayer, trace::TraceLayer};

mod api;
mod config;
mod websocket;
mod plugin;
mod core;
mod middleware;
mod domain;
mod infrastructure;
mod application;
mod utils;

type SessionMap = Arc<Mutex<std::collections::HashMap<String, Arc<Mutex<tokio::process::Child>>>>>;

#[tokio::main]
async fn main() {
    // 日志
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    let config = config::Config::load().expect("配置加载失败");

    // 数据库
    let db = sqlx::SqlitePool::connect(&config.database_url)
        .await
        .expect("数据库连接失败");

    sqlx::migrate!().run(&db).await.expect("数据库迁移失败");

    // Session Map 用于 Terminal
    let sessions: SessionMap = Arc::new(Mutex::new(std::collections::HashMap::new()));

    let app = Router::new()
        .merge(api::routes())
        .nest("/ws", websocket::routes())
        .layer(CorsLayer::permissive())
        .layer(TraceLayer::new_for_http())
        .layer(axum::Extension(db))
        .layer(axum::Extension(sessions));

    let addr = SocketAddr::from(([0, 0, 0, 0], config.port));
    tracing::info!("🚀 Ops Panel 启动成功 → http://{}", addr);

    let listener = tokio::net::TcpListener::bind(&addr)
        .await
        .unwrap();

    axum::serve(listener, app)
        .await
        .unwrap();
}