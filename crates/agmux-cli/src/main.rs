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
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Manage review comments.
    Review {
        #[command(subcommand)]
        command: ReviewCommand,
    },
    /// Show a session review diff.
    Diff {
        session_id: String,
        #[arg(long, default_value = "session", value_parser = ["session", "uncommitted", "target", "last", "staged"])]
        scope: String,
        #[arg(long)]
        target: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Create a first-run configuration and report discovered agents.
    Init,
    /// Open the interactive terminal interface.
    Tui,
    /// Search terminal history.
    Search {
        query: String,
        #[arg(long)]
        json: bool,
        #[arg(long)]
        session_id: Option<String>,
        #[arg(long, default_value_t = 100)]
        limit: u32,
    },
    /// List active sessions.
    List {
        /// Output a JSON array.
        #[arg(long)]
        json: bool,
    },
    /// Spawn a command inside a new PTY session and attach to its output stream
    Run {
        /// Create an isolated worktree for this branch.
        #[arg(long)]
        worktree: Option<String>,
        /// Executable command name
        command: String,

        /// Additional command line arguments
        #[arg(trailing_var_arg = true, allow_hyphen_values = true)]
        args: Vec<String>,
    },
    /// Remove a session and its worktree.
    #[command(alias = "rm")]
    Remove {
        session_id: String,
        #[arg(long)]
        force: bool,
    },
    /// Freeze an active session, persisting screen to Zstd snapshot
    Freeze {
        /// Session identifier to freeze
        session_id: String,
    },
    /// Inspect or restore a persisted session snapshot
    Inspect {
        /// Session identifier to inspect
        session_id: String,
    },
}

#[derive(Subcommand, Debug)]
enum ReviewCommand {
    Resolve {
        comment_id: String,
        #[arg(long)]
        session: Option<String>,
    },
    Comment {
        session_id: String,
        #[arg(long)]
        file: String,
        #[arg(long)]
        line: u64,
        #[arg(long)]
        content: String,
    },
    List {
        session_id: String,
        #[arg(long, value_parser = ["draft", "sent", "resolved"])]
        status: Option<String>,
    },
    Dispatch {
        session_id: String,
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
    if matches!(cli.command, Some(Commands::Init)) {
        let agents = agmux_core::agent_discovery::discover_installed_agents();
        let path =
            agmux_core::config_manager::ensure_config_exists(std::path::Path::new("agmux.toml"))?;
        println!("Configuration: {}", path.display());
        for agent in agents {
            println!(
                "{} ({}): {}",
                agent.name,
                agent.binary_name,
                agent
                    .path
                    .map_or_else(|| "not found".into(), |p| p.display().to_string())
            );
        }
        return Ok(());
    }
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
        Some(Commands::Review { command }) => {
            use agmux_proto::*;
            let mut resolved_id = None;
            let request = match command {
                ReviewCommand::Resolve {
                    comment_id,
                    session,
                } => {
                    resolved_id = Some(comment_id.clone());
                    IpcMessage::ResolveReviewCommentRequest(ResolveReviewCommentRequest {
                        session_id: session,
                        comment_id,
                    })
                }
                ReviewCommand::Comment {
                    session_id,
                    file,
                    line,
                    content,
                } => IpcMessage::AddReviewCommentRequest(AddReviewCommentRequest {
                    session_id,
                    file_path: file,
                    line_number: line,
                    content,
                }),
                ReviewCommand::List { session_id, status } => {
                    IpcMessage::ListReviewCommentsRequest(ListReviewCommentsRequest {
                        session_id,
                        status,
                    })
                }
                ReviewCommand::Dispatch { session_id } => {
                    IpcMessage::DispatchReviewRoundRequest(DispatchReviewRoundRequest {
                        session_id,
                    })
                }
            };
            match agmux_cli::ipc_client::review_request(&socket_path, &token, request).await? {
                IpcMessage::ResolveReviewCommentResponse(r) if r.success => {
                    println!("Resolved comment {}", resolved_id.unwrap_or_default())
                }
                IpcMessage::ResolveReviewCommentResponse(r) => {
                    anyhow::bail!("{}", r.error.unwrap_or_default())
                }
                IpcMessage::AddReviewCommentResponse(r) if r.success => {
                    println!("{}", r.comment_id.unwrap_or_default())
                }
                IpcMessage::ListReviewCommentsResponse(r) if r.success => {
                    for c in r.comments {
                        println!(
                            "{} [{}] {}:{}: {}",
                            c.id, c.status, c.file_path, c.line_number, c.content
                        );
                    }
                }
                IpcMessage::DispatchReviewRoundResponse(r) if r.success => print!("{}", r.prompt),
                IpcMessage::AddReviewCommentResponse(r) => {
                    anyhow::bail!("{}", r.error.unwrap_or_default())
                }
                IpcMessage::ListReviewCommentsResponse(r) => {
                    anyhow::bail!("{}", r.error.unwrap_or_default())
                }
                IpcMessage::DispatchReviewRoundResponse(r) => {
                    anyhow::bail!("{}", r.error.unwrap_or_default())
                }
                other => anyhow::bail!("Unexpected review response: {other:?}"),
            }
        }
        Some(Commands::Init) => unreachable!("init handled before daemon startup"),
        None | Some(Commands::Tui) => agmux_tui::run_tui(&socket_path, &token).await?,
        Some(Commands::Diff {
            session_id,
            scope,
            target,
            json,
        }) => {
            use agmux_proto::{DiffLineType, DiffScope};
            let scope = match scope.as_str() {
                "session" => DiffScope::SessionDiff,
                "uncommitted" => DiffScope::FullUncommitted,
                "target" => DiffScope::VersusTarget,
                "last" => DiffScope::LastCommit,
                "staged" => DiffScope::Staged,
                _ => unreachable!(),
            };
            let files = agmux_cli::ipc_client::get_diff(
                &socket_path,
                &token,
                agmux_proto::GetDiffRequest {
                    session_id,
                    scope,
                    target,
                },
            )
            .await?;
            if json {
                println!("{}", serde_json::to_string(&files)?);
            } else {
                for file in files {
                    println!(
                        "--- {}\n+++ {}",
                        file.old_path.as_deref().unwrap_or("/dev/null"),
                        file.new_path.as_deref().unwrap_or("/dev/null")
                    );
                    for hunk in file.hunks {
                        println!("{}", hunk.header);
                        for line in hunk.lines {
                            let prefix = match line.kind {
                                DiffLineType::Context => ' ',
                                DiffLineType::Added => '+',
                                DiffLineType::Removed => '-',
                                DiffLineType::NoNewline => '\\',
                            };
                            println!("{prefix}{}", line.content);
                        }
                    }
                }
            }
        }
        Some(Commands::Search {
            query,
            json,
            session_id,
            limit,
        }) => {
            let matches = agmux_cli::ipc_client::search_logs(
                &socket_path,
                &token,
                query.clone(),
                session_id,
                limit,
            )
            .await?;
            if json {
                println!("{}", serde_json::to_string(&matches)?);
            } else {
                for item in matches {
                    let snippet = item
                        .snippet
                        .replace(&query, &format!("\x1b[1;31m{query}\x1b[0m"));
                    println!("{}:{}: {}", item.session_id, item.line_no, snippet);
                }
            }
        }
        Some(Commands::List { json }) => {
            let sessions = agmux_cli::ipc_client::list_sessions(&socket_path, &token).await?;
            if json {
                println!("{}", serde_json::to_string(&sessions)?);
            } else {
                println!(
                    "{:<24} {:<8} {:<24} {:<24} {:<16} STARTED",
                    "SESSION ID", "PID", "COMMAND", "CWD", "STATE"
                );
                for session in sessions {
                    println!(
                        "{:<24} {:<8} {:<24} {:<24} {} {}",
                        session.session_id,
                        session.pid,
                        session.command,
                        session.cwd,
                        state_badge(&session.state),
                        session.created_at
                    );
                }
            }
        }
        Some(Commands::Run {
            command,
            args,
            worktree,
        }) => {
            let exit_code = run_command(&socket_path, &token, command, args, worktree).await?;
            std::process::exit(exit_code);
        }
        Some(Commands::Remove { session_id, force }) => {
            let response =
                agmux_cli::ipc_client::remove_session(&socket_path, &token, session_id, force)
                    .await?;
            if !response.success {
                anyhow::bail!(
                    "{}",
                    response.error.unwrap_or_else(|| "Removal failed".into())
                );
            }
            if let Some(stash) = response.stash_created {
                println!("Backup stash: {stash}");
            }
        }
        Some(Commands::Freeze { session_id }) => {
            let resp = agmux_cli::freeze_session(&socket_path, &token, session_id.clone()).await?;
            if resp.success {
                println!(
                    "Frozen session {} ({} bytes compressed)",
                    session_id, resp.compressed_size
                );
            } else {
                eprintln!("Failed to freeze session {}", session_id);
                std::process::exit(1);
            }
        }
        Some(Commands::Inspect { session_id }) => {
            let resp = agmux_cli::resume_session(&socket_path, &token, session_id.clone()).await?;
            if resp.success {
                let text = String::from_utf8_lossy(&resp.screen_data);
                print!("{text}");
            } else {
                eprintln!("Failed to inspect snapshot for session {}", session_id);
                std::process::exit(1);
            }
        }
    }

    Ok(())
}

fn state_badge(state: &str) -> String {
    let color = match state {
        "RUNNING" => 32,
        "WAITING_INPUT" => 33,
        "STUCK" => 35,
        "COMPLETED" => 36,
        "FAILED" => 31,
        _ => 0,
    };
    format!("\x1b[{color}m[{state}]\x1b[0m")
}
