use anyhow::Result;
use clap::Parser;
use daystrom_tui::app::App;
use daystrom_tui::config::Config;
use daystrom_tui::monitor::MonitorEngine;
use daystrom_tui::ui::run_app;
use std::path::PathBuf;
use tracing::{error, info};

#[derive(Parser)]
#[command(name = "daystrom-tui")]
#[command(about = "A powerful TUI monitoring tool for multiple hosts and services")]
struct Cli {
    /// Configuration file path
    #[arg(short, long, default_value = "config.yaml")]
    config: PathBuf,

    /// Log level
    #[arg(short, long, default_value = "info")]
    log_level: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(tracing_subscriber::EnvFilter::from_default_env())
        .init();

    info!("Starting Daystrom TUI monitoring application");

    // Load configuration
    let config = Config::load_from_file(&cli.config)?;
    info!("Loaded configuration from {}", cli.config.display());
    info!("Monitoring {} hosts with {} total services", 
          config.hosts.len(), 
          config.hosts.iter().map(|h| h.services.len()).sum::<usize>());

    // Create monitoring engine
    let engine = MonitorEngine::new(config.clone());
    
    // Start monitoring in background
    let engine_handle = engine.start().await;

    // Create and run TUI app
    let app = App::new(config, engine);
    
    if let Err(e) = run_app(app).await {
        error!("Application error: {}", e);
        std::process::exit(1);
    }

    // Stop monitoring engine
    engine_handle.abort();
    
    info!("Application shutdown complete");
    Ok(())
} 