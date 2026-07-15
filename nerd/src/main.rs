// SPDX-License-Identifier: AGPL-3.0-only
mod config;
mod db;
mod insights;

use anyhow::Result;
use clap::{Parser, Subcommand};
use colored::Colorize;

#[derive(Parser)]
#[command(name = "nerd", version = "0.1.0", about = "Flow-first time tracking")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Start tracking time for a project
    Start {
        project: String,
        #[arg(short, long)]
        desc: Option<String>,
    },
    /// Stop the active session
    Stop,
    /// Show active session status
    Status,
    /// Sync unsynced sessions to the server
    Sync,
    /// List recent sessions
    Log {
        #[arg(short, long)]
        project: Option<String>,
        #[arg(short, long, default_value = "10")]
        limit: usize,
    },
    /// Authenticate with the nerdtime API server
    Login {
        /// API server URL (e.g. https://nerdtime.dev)
        #[arg(short, long)]
        url: Option<String>,
        /// JWT token for authentication
        token: String,
    },
    /// Show configuration
    Config,
    /// Show a heatmap of tracked time (weekday x hour grid)
    Heatmap {
        #[arg(short, long, default_value = "30")]
        days: i64,
        #[arg(short, long)]
        project: Option<String>,
    },
    /// Show productivity insights and patterns
    Insights {
        #[arg(short, long, default_value = "30")]
        days: i64,
        #[arg(short, long)]
        project: Option<String>,
    },
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    let conn = db::get_connection()?;

    match &cli.command {
        Commands::Start { project, desc } => db::start_session(&conn, project, desc.as_deref()),
        Commands::Stop => db::stop_session(&conn),
        Commands::Status => db::show_status(&conn),
        Commands::Sync => db::sync_sessions(&conn),
        Commands::Log { project, limit } => db::list_sessions(&conn, project.as_deref(), *limit),
        Commands::Login { url, token } => login(url.as_deref(), token),
        Commands::Config => show_config(),
        Commands::Heatmap { days, project } => {
            let cells = db::heatmap_data(&conn, *days, project.as_deref())?;
            print!("{}", insights::render_heatmap(&cells));
            Ok(())
        }
        Commands::Insights { days, project } => {
            let data = db::insights_data(&conn, *days, project.as_deref())?;
            print!("{}", insights::render_insights(&data));
            Ok(())
        }
    }
}

fn login(url: Option<&str>, token: &str) -> Result<()> {
    let mut cfg = config::load()?;
    if let Some(u) = url {
        cfg.api_url = u.trim_end_matches('/').to_string();
    }
    cfg.token = Some(token.to_string());
    config::save(&cfg)?;
    println!("{} Authentication saved for {}", "✓".green(), cfg.api_url);
    Ok(())
}

fn show_config() -> Result<()> {
    let cfg = config::load()?;
    println!("API URL:  {}", cfg.api_url);
    println!(
        "Token:    {}",
        if cfg.token.is_some() {
            "✓ set".green()
        } else {
            "not set".yellow()
        }
    );
    Ok(())
}
