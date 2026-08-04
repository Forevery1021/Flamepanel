use flame_kernel::config::AppConfig;
use flame_kernel::infrastructure::factory::RepoFactory;
use flame_kernel::FlameKernel;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let json_logs = std::env::var("RUST_LOG_FORMAT").map(|v| v == "json").unwrap_or(false);
    if json_logs {
        // 结构化 JSON 日志（生产可观测性：可被 logstash/loki 等直接采集）
        tracing_subscriber::fmt()
            .json()
            .with_env_filter(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| "flame_kernel=info,tower_http=info".into()),
            )
            .init();
        tracing::info!("JSON structured logging enabled (RUST_LOG_FORMAT=json)");
    } else {
        tracing_subscriber::fmt()
            .with_env_filter(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| "flame_kernel=info,tower_http=info".into()),
            )
            .init();
    }

    let config = AppConfig::load().unwrap_or_default();

    let factory = if config.database.url != "sqlite://data/app.db" {
        tracing::info!("Using SQLite backend: {}", config.database.url);
        RepoFactory::new_sqlite(&config.database.url).await?
    } else {
        tracing::info!("Using in-memory backend");
        RepoFactory::new_in_memory()
    };

    let kernel = FlameKernel::new_with_backend(config, factory);
    kernel.run().await
}
