mod commands;
mod config;
mod models;
mod storage;
mod api;
mod ui;

use anyhow::Result;
use clap::Parser;
use tracing_subscriber::EnvFilter;

use crate::commands::Cli;

fn main() -> Result<()> {
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| EnvFilter::new("info"))
        )
        .init();

    // Parse command line arguments
    let cli = Cli::parse();

    // Initialize tokio runtime
    tokio::runtime::Builder::new_multi_thread()
        .enable_all()
        .build()?
        .block_on(async {
            cli.execute().await
        })
}
