use flame_kernel::config::AppConfig;
use flame_kernel::infrastructure::factory::RepoFactory;
use flame_kernel::FlameKernel;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "flame_kernel=info,tower_http=info".into()),
        )
        .init();

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
