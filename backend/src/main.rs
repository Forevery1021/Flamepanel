use clap::Parser;

use ops_panel_backend::{cli, cli::Cli};

#[tokio::main]
async fn main() {
    let cli = Cli::parse();

    if cli.command.is_some() {
        cli::run(cli).await;
    } else {
        ops_panel_backend::start_server().await;
    }
}
