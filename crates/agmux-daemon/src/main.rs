//! `agmuxd` background daemon entry point.

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Parser;

use agmux_daemon::{DaemonServer, default_socket_path};

#[derive(Parser, Debug)]
#[command(
    name = "agmuxd",
    version,
    about = "Agmux background multiplexer daemon"
)]
struct Args {
    /// Path to IPC Unix domain socket.
    #[arg(short, long)]
    socket: Option<PathBuf>,

    /// Path to authentication secret file.
    #[arg(short, long)]
    token_path: Option<PathBuf>,
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info")),
        )
        .init();

    let args = Args::parse();
    let socket_path = args.socket.unwrap_or_else(default_socket_path);
    let token_path = args
        .token_path
        .unwrap_or_else(agmux_core::default_token_path);

    let auth_token = agmux_core::get_or_create_token(&token_path).with_context(|| {
        format!(
            "Failed loading or creating auth token at {}",
            token_path.display()
        )
    })?;

    let server = DaemonServer::new(socket_path, auth_token);
    server.run().await
}
