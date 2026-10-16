use flame_kernel::config::AppConfig;
use flame_kernel::infrastructure::factory::RepoFactory;
use flame_kernel::infrastructure::logging::{self, LoggingSettings};
use flame_kernel::FlameKernel;

#[tokio::main]
async fn main() -> std::process::ExitCode {
    // Report the binary version for fpctl adopt/upgrade without logs or configuration.
    if std::env::args()
        .skip(1)
        .any(|a| a == "--version" || a == "-V")
    {
        println!("flamepanel {}", env!("CARGO_PKG_VERSION"));
        return std::process::ExitCode::SUCCESS;
    }
    // Run the root helper for allowlisted web-engine operations via its systemd unit.
    // Log only to stderr/journald, not the panel log directory.
    let args: Vec<String> = std::env::args().skip(1).collect();
    if args.first().map(String::as_str) == Some("helper") {
        let _ = tracing_subscriber::fmt()
            .with_env_filter(
                tracing_subscriber::EnvFilter::try_from_default_env()
                    .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
            )
            .with_writer(std::io::stderr)
            .with_ansi(std::io::IsTerminal::is_terminal(&std::io::stderr()))
            .try_init();
        return flame_kernel::webserver::helper::run_cli(&args[1..]).await;
    }
    // Bridge MCP stdio to Streamable HTTP. Reserve stdout for protocol messages;
    // write diagnostics only to stderr and do not initialize normal logging.
    if args.first().map(String::as_str) == Some("mcp") {
        return flame_kernel::cli::mcp_stdio::run_cli(&args[1..]).await;
    }
    // Keep the console/rotating JSONL logging guard alive until exit so Drop flushes its queue.
    let (log_settings, log_warnings) = LoggingSettings::from_env();
    let _log_guard = logging::init(log_settings);
    for w in log_warnings {
        tracing::warn!("{w}");
    }
    tracing::info!(
        version = env!("CARGO_PKG_VERSION"),
        "FlamePanel kernel starting"
    );

    // Log fatal startup failures before exiting; the guard flushes on return from main.
    match run().await {
        Ok(()) => std::process::ExitCode::SUCCESS,
        Err(e) => {
            tracing::error!(error = %e, "kernel exited with a fatal error: {e}");
            eprintln!("Error: {e}");
            std::process::ExitCode::FAILURE
        }
    }
}

async fn run() -> Result<(), Box<dyn std::error::Error>> {
    let config = AppConfig::load().unwrap_or_default();

    // Default to SQLite; use memory only when explicitly configured.
    #[cfg(feature = "sqlite")]
    let factory = if config.database.backend == "memory" {
        tracing::warn!(
            "Using in-memory backend (OP_DATABASE_BACKEND=memory). Data will be lost on restart!"
        );
        RepoFactory::new_in_memory()
    } else {
        tracing::info!("Using SQLite backend: {}", config.database.url);
        RepoFactory::new_sqlite(&config.database.url).await?
    };
    #[cfg(not(feature = "sqlite"))]
    let factory = {
        tracing::info!("SQLite backend disabled (feature `sqlite` off); using in-memory backend");
        RepoFactory::new_in_memory()
    };

    let kernel = FlameKernel::new_with_backend(config, factory);
    kernel.run().await
}
