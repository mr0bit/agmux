//! `agmux` CLI entry point.

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{Parser, Subcommand};

use agmux_cli::{ensure_daemon_running, run_command};
use agmux_core::default_socket_path;

#[derive(Parser, Debug)]
#[command(
    name = "agmux",
    version,
    about = "Agmux agentic terminal multiplexer CLI"
)]
struct Cli {
    /// Path to IPC Unix domain socket.
    #[arg(short, long)]
    socket: Option<PathBuf>,

    /// Path to authentication secret file.
    #[arg(short, long)]
    token_path: Option<PathBuf>,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// List active sessions.
    List {
        /// Output a JSON array.
        #[arg(long)]
        json: bool,
    },
    /// Spawn a command inside a new PTY session and attach to its output stream
    Run {
        /// Executable command name
        command: String,

        /// Additional command line arguments
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
        )
        .init();

    let cli = Cli::parse();
    let socket_path = cli.socket.unwrap_or_else(default_socket_path);
    let token_path = cli
        .token_path
        .unwrap_or_else(agmux_core::default_token_path);

    // 1. Ensure daemon is running (auto-spawn if not alive)
    ensure_daemon_running(&socket_path, &token_path).await?;

    // 2. Load auth token
    let token = agmux_core::get_or_create_token(&token_path)
        .with_context(|| format!("Failed to read auth token from {}", token_path.display()))?;

    // 3. Dispatch subcommand
    match cli.command {
        Commands::List { json } => {
            let sessions = agmux_cli::ipc_client::list_sessions(&socket_path, &token).await?;
            if json {
                println!("{}", serde_json::to_string(&sessions)?);
            } else {
                println!(
                    "{:<24} {:<8} {:<24} {:<24} STARTED",
                    "SESSION ID", "PID", "COMMAND", "CWD"
                );
                for session in sessions {
                    println!(
                        "{:<24} {:<8} {:<24} {:<24} {}",
                        session.session_id,
                        session.pid,
                        session.command,
                        session.cwd,
                        session.created_at
                    );
                }
            }
        }
        Commands::Run { command, args } => {
            let exit_code = run_command(&socket_path, &token, command, args).await?;
            std::process::exit(exit_code);
        }
    }

    Ok(())
}
