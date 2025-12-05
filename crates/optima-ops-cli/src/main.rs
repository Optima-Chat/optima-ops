//! Optima Ops CLI - Command-line interface for Optima operations
//!
//! This CLI provides tools for:
//! - Monitoring service health
//! - Infrastructure management
//! - Deployment operations

use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::*;

mod commands;

use commands::{env, services, version};
use optima_ops_core::AppConfig;

#[derive(Parser)]
#[command(name = "optima-ops")]
#[command(author = "Optima Team")]
#[command(version)]
#[command(about = "Optima Ops CLI - 运维工具 (带 Web Dashboard)", long_about = None)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,

    /// Override the default environment
    #[arg(short, long, global = true)]
    env: Option<String>,

    /// Output as JSON
    #[arg(long, global = true)]
    json: bool,
}

#[derive(Subcommand)]
enum Commands {
    /// Show current environment information
    Env(env::EnvCommand),

    /// Service operations (health, status, logs)
    Services(services::ServicesCommand),

    /// Show version information
    Version(version::VersionCommand),
}

#[tokio::main]
async fn main() -> Result<()> {
    // Initialize logging
    if std::env::var("RUST_LOG").is_err() {
        std::env::set_var("RUST_LOG", "warn");
    }
    tracing_subscriber::fmt::init();

    let cli = Cli::parse();

    // Load configuration
    let config = match AppConfig::load() {
        Ok(c) => c,
        Err(e) => {
            eprintln!("{} 加载配置失败: {}", "✗".red(), e);
            std::process::exit(1);
        }
    };

    // Execute command
    let result = match cli.command {
        Some(Commands::Env(cmd)) => cmd.execute(&config).await,
        Some(Commands::Services(cmd)) => cmd.execute(&config, cli.json).await,
        Some(Commands::Version(cmd)) => cmd.execute(),
        None => {
            // Show help by default
            println!("{}", "Optima Ops CLI".bold());
            println!();
            println!("使用 {} 查看帮助", "optima-ops --help".cyan());
            Ok(())
        }
    };

    if let Err(e) = result {
        optima_ops_core::handle_error(&e.into());
        std::process::exit(1);
    }

    Ok(())
}
