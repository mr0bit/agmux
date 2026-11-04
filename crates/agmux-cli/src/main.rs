//! `agmux` CLI entry point.

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::{CommandFactory, Parser, Subcommand};
use futures_util::SinkExt;

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

    /// Serve MCP over stdio.
    #[arg(long)]
    enable_mcp: bool,

    /// Update the installed binaries.
    #[arg(long)]
    update: bool,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Check for and install a release.
    Update {
        #[arg(long)]
        check: bool,
        #[arg(long)]
        force: bool,
        #[arg(long)]
        version: Option<String>,
    },
    /// Register a workspace panel.
    Panels {
        #[command(subcommand)]
        command: PanelCommand,
    },
    /// Register the desktop URI scheme handler.
    RegisterScheme,
    /// Resolve a session or panel deep link.
    Open { uri: String },
    /// Serve the local companion dashboard or print its state.
    Gui {
        #[arg(long)]
        snapshot: bool,
        #[arg(long)]
        json: bool,
    },
    /// Manage numbered session scratchpad pages.
    Note {
        #[command(subcommand)]
        command: NoteCommand,
    },
    /// Serve the MCP protocol over stdio.
    Mcp,
    /// Schedule archive on turn completion.
    ArchiveSelf { session_id: Option<String> },
    /// Schedule termination on turn completion.
    KillSelf { session_id: Option<String> },
    /// Cancel a pending teardown.
    CancelTeardown { session_id: Option<String> },
    /// Discover commands and their usage.
    AgentContext {
        #[arg(long)]
        command: Option<String>,
    },
    /// Manage shared Kanban tasks.
    Task {
        #[command(subcommand)]
        command: TaskCommand,
    },
    /// Manage structured worker completion reports.
    Report {
        #[command(subcommand)]
        command: ReportCommand,
    },
    /// Manage named resource locks.
    Lock {
        #[command(subcommand)]
        command: LockCommand,
    },
    /// Reserve files or directories to prevent collision across worktrees.
    Reserve {
        paths: Vec<String>,
        #[arg(long)]
        session: Option<String>,
    },
    /// Release reserved files or directories.
    Release {
        paths: Vec<String>,
        #[arg(long)]
        session: Option<String>,
    },
    /// List active file reservations.
    Reservations {
        #[arg(long)]
        json: bool,
    },
    /// Watch journaled workspace events.
    Watch {
        /// Optional named cursor to remember position across reconnections.
        #[arg(long = "as")]
        cursor: Option<String>,
        /// Replay events since this sequence number.
        #[arg(long)]
        since: Option<u64>,
        /// Keep stream open following live events.
        #[arg(long, short)]
        follow: bool,
        /// Format output as JSON.
        #[arg(long)]
        json: bool,
    },
    /// Send a message to another session.
    Send { target_id: String, message: String },
    /// Send raw text input to an active session PTY.
    Input { session_id: String, text: String },
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
        /// Register the Agmux MCP server for a supported agent (claude or codex).
        #[arg(long)]
        enable_mcp: bool,
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
    /// Emit a Tier 1 deterministic lifecycle hook event (`working`, `permission_request`, `stop`).
    InternalHook {
        /// Session identifier emitting the hook.
        #[arg(long)]
        session: Option<String>,
        /// Hook event type (`working`, `permission_request`, `stop`).
        #[arg(long)]
        event: String,
    },
    /// Connect to a remote host over SSH or Tailscale (`agmux connect ssh://user@box`).
    Connect {
        /// Remote target URI (e.g. `ssh://user@box`).
        target: String,
        /// Optional forwarded bridge socket path.
        #[arg(long)]
        remote_socket: Option<String>,
        /// Optional remote daemon authentication token.
        #[arg(long)]
        remote_token: Option<String>,
    },
    /// Manage federated remote hosts (`agmux remote connect` / `agmux remote list`).
    Remote {
        #[command(subcommand)]
        command: RemoteCommand,
    },
    /// Run environment diagnostics (`REQ-REMOTE-DOCTOR-REPAIR-004`).
    Doctor {
        /// Output diagnostic report as JSON.
        #[arg(long)]
        json: bool,
    },
    /// Manage or repair the local daemon environment (`agmux daemon repair`).
    Daemon {
        #[command(subcommand)]
        command: DaemonCommand,
    },
}

#[derive(Subcommand, Debug)]
enum PanelCommand {
    Open {
        target: String,
        #[arg(long)]
        session: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
enum NoteCommand {
    /// Replace a page's text.
    Set {
        session_id: String,
        page: u32,
        content: String,
    },
    /// Print a page.
    Get { session_id: String, page: u32 },
    /// Print all pages in order.
    List { session_id: String },
}

#[derive(Subcommand, Debug)]
enum RemoteCommand {
    /// Connect to a remote host and mount its sessions.
    Connect {
        target: String,
        #[arg(long)]
        socket: Option<String>,
        #[arg(long)]
        token: Option<String>,
    },
    /// List connected remote host labels.
    List {
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand, Debug)]
enum DaemonCommand {
    /// Repair broken `daemon.token` permissions (`0600`) and remove stale lockfiles.
    Repair,
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

#[derive(Subcommand, Debug)]
enum LockCommand {
    /// Acquire a named lock.
    Acquire {
        /// Name of the resource lock.
        name: String,
        /// Session ID holding the lock.
        #[arg(long)]
        session: Option<String>,
        /// Wait until the lock is available.
        #[arg(long, short)]
        wait: bool,
    },
    /// Release a named lock.
    Release {
        /// Name of the resource lock.
        name: String,
        /// Session ID holding the lock.
        #[arg(long)]
        session: Option<String>,
    },
    /// List active named locks.
    List {
        /// Output as JSON.
        #[arg(long)]
        json: bool,
    },
}

#[derive(Subcommand, Debug)]
enum TaskCommand {
    /// List Kanban tasks.
    List {
        /// Filter by status (todo, in_progress, done).
        #[arg(long)]
        status: Option<String>,
        /// Output as JSON.
        #[arg(long)]
        json: bool,
    },
    /// Create a new Kanban task.
    Create {
        /// Task identifier (e.g. T-101).
        task_id: String,
        /// Task title.
        title: String,
        /// Optional task description.
        #[arg(long, default_value = "")]
        description: String,
    },
    /// Claim a Kanban task.
    Claim {
        /// Task identifier.
        task_id: String,
        /// Session ID claiming the task.
        #[arg(long)]
        session: Option<String>,
    },
    /// Mark a Kanban task as finished.
    Finish {
        /// Task identifier.
        task_id: String,
        /// Session ID finishing the task.
        #[arg(long)]
        session: Option<String>,
    },
    /// Release a claimed Kanban task back to todo.
    Release {
        /// Task identifier.
        task_id: String,
        /// Session ID releasing the task.
        #[arg(long)]
        session: Option<String>,
    },
}

#[derive(Subcommand, Debug)]
enum ReportCommand {
    /// Post a structured worker completion report.
    Post {
        /// Task identifier.
        task_id: String,
        /// Structured summary of work completed.
        #[arg(long)]
        summary: String,
        /// Optional pull request URL.
        #[arg(long, alias = "pr")]
        pr_url: Option<String>,
        /// Optional head commit SHA.
        #[arg(long, alias = "sha")]
        commit_sha: Option<String>,
        /// Session ID posting the report.
        #[arg(long)]
        session: Option<String>,
    },
    /// View structured worker reports for a task.
    View {
        /// Task identifier.
        task_id: String,
        /// Output as JSON.
        #[arg(long)]
        json: bool,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt()
        .with_writer(std::io::stderr)
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("warn")),
        )
        .init();

    let cli = Cli::parse();
    if cli.update && cli.command.is_some() {
        anyhow::bail!("--update cannot be combined with a subcommand");
    }
    if cli.update || matches!(cli.command, Some(Commands::Update { .. })) {
        let (check, force, version) = match cli.command {
            Some(Commands::Update {
                check,
                force,
                version,
            }) => (check, force, version),
            _ => (false, false, None),
        };
        return tokio::task::spawn_blocking(move || {
            agmux_cli::self_updater::update(check, force, version.as_deref())
        })
        .await?;
    }
    if cli.enable_mcp && cli.command.is_some() {
        anyhow::bail!(
            "--enable-mcp in server mode cannot be combined with a subcommand; use run --enable-mcp for an agent"
        );
    }
    if let Some(Commands::AgentContext { ref command }) = cli.command {
        if let Some(name) = command {
            let mut root = Cli::command();
            let subcommand = root
                .find_subcommand_mut(name)
                .ok_or_else(|| anyhow::anyhow!("Unknown command: {name}"))?;
            subcommand.print_long_help()?;
            for nested in subcommand.get_subcommands_mut() {
                println!("\n");
                nested.print_long_help()?;
            }
            println!("\n\n{}", agmux_cli::agent_context::examples(name));
        } else {
            println!("{}", agmux_cli::agent_context::OVERVIEW);
        }
        return Ok(());
    }
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
    if matches!(cli.command, Some(Commands::RegisterScheme)) {
        println!(
            "{}",
            agmux_cli::companion_gui::register_desktop_protocol(None)?.display()
        );
        return Ok(());
    }
    let socket_path = cli.socket.unwrap_or_else(default_socket_path);
    let token_path = cli
        .token_path
        .unwrap_or_else(agmux_core::default_token_path);

    if let Some(Commands::Doctor { json }) = cli.command {
        let db_path = if std::env::var_os("AGMUX_DB_PATH").is_some()
            || socket_path == agmux_core::default_socket_path()
        {
            agmux_core::storage_engine::default_database_path()
        } else {
            socket_path.with_extension("db")
        };
        let active_remotes = if let Ok(tok) = std::fs::read_to_string(&token_path) {
            agmux_cli::ipc_client::list_remotes(&socket_path, tok.trim())
                .await
                .map(|r| r.remotes)
                .unwrap_or_default()
        } else {
            Vec::new()
        };
        let report = agmux_cli::doctor::run_doctor(&token_path, &db_path, active_remotes);
        if json {
            println!("{}", serde_json::to_string(&report)?);
        } else {
            println!(
                "Doctor status: {}",
                if report.healthy {
                    "HEALTHY"
                } else {
                    "ISSUES_FOUND"
                }
            );
            println!("PTY available: {}", report.pty_available);
            println!(
                "Token permissions OK: {} ({:04o})",
                report.token_permissions_ok, report.token_mode
            );
            println!("DB accessible: {}", report.db_accessible);
            for issue in &report.issues {
                println!("ISSUE: {issue}");
            }
        }
        if !report.healthy {
            std::process::exit(1);
        }
        return Ok(());
    }

    if let Some(Commands::Daemon {
        command: DaemonCommand::Repair,
    }) = cli.command
    {
        let actions = agmux_cli::doctor::repair_daemon_env(&token_path, &socket_path)?;
        for action in actions {
            println!("{action}");
        }
        return Ok(());
    }

    // 1. Ensure daemon is running (auto-spawn if not alive)
    ensure_daemon_running(&socket_path, &token_path).await?;

    // 2. Load auth token
    let token = agmux_core::get_or_create_token(&token_path)
        .with_context(|| format!("Failed to read auth token from {}", token_path.display()))?;

    // 3. Dispatch subcommand
    if cli.enable_mcp {
        return agmux_cli::mcp_server::serve(&socket_path, &token).await;
    }
    match cli.command {
        Some(Commands::Panels {
            command: PanelCommand::Open { target, session },
        }) => {
            let session_id = session
                .or_else(|| std::env::var("AGMUX_SESSION_ID").ok())
                .context("Specify --session or AGMUX_SESSION_ID")?;
            let result = agmux_cli::companion_gui::request(
                &socket_path,
                &token,
                agmux_proto::CompanionRequest::Open { session_id, target },
            )
            .await?;
            println!("{}", serde_json::to_string(&result)?);
        }
        Some(Commands::Open { uri }) => {
            agmux_cli::companion_gui::open_link(&socket_path, &token, &uri).await?
        }
        Some(Commands::Gui { snapshot, json: _ }) => {
            if snapshot {
                println!(
                    "{}",
                    serde_json::to_string_pretty(
                        &agmux_cli::companion_gui::snapshot(&socket_path, &token).await?
                    )?
                );
            } else {
                agmux_cli::companion_gui::serve(socket_path.clone(), token.clone()).await?;
            }
        }
        Some(Commands::Note { command }) => {
            let request = match command {
                NoteCommand::Set {
                    session_id,
                    page,
                    content,
                } => agmux_proto::SearchNotesRequest::Set {
                    session_id,
                    page,
                    content,
                },
                NoteCommand::Get { session_id, page } => {
                    agmux_proto::SearchNotesRequest::Get { session_id, page }
                }
                NoteCommand::List { session_id } => {
                    agmux_proto::SearchNotesRequest::List { session_id }
                }
            };
            let list = matches!(request, agmux_proto::SearchNotesRequest::List { .. });
            match agmux_cli::ipc_client::review_request(
                &socket_path,
                &token,
                agmux_proto::IpcMessage::SearchNotesRequest(request),
            )
            .await?
            {
                agmux_proto::IpcMessage::SearchNotesResponse(response) => {
                    if let Some(error) = response.error {
                        anyhow::bail!("{error}");
                    }
                    for (page, content) in response.pages {
                        if list {
                            println!("[{page}]\n{content}");
                        } else {
                            println!("{content}");
                        }
                    }
                }
                other => anyhow::bail!("Unexpected note response: {other:?}"),
            }
        }
        Some(Commands::Mcp) => agmux_cli::mcp_server::serve(&socket_path, &token).await?,
        Some(
            cmd @ (Commands::ArchiveSelf { .. }
            | Commands::KillSelf { .. }
            | Commands::CancelTeardown { .. }),
        ) => {
            let (action, session_id) = match cmd {
                Commands::ArchiveSelf { session_id } => ("archive", session_id),
                Commands::KillSelf { session_id } => ("kill", session_id),
                Commands::CancelTeardown { session_id } => ("cancel", session_id),
                _ => unreachable!(),
            };
            let id = session_id
                .or_else(|| std::env::var("AGMUX_SESSION_ID").ok())
                .ok_or_else(|| anyhow::anyhow!("Session ID is required"))?;
            let response = agmux_cli::ipc_client::review_request(
                &socket_path,
                &token,
                agmux_proto::IpcMessage::TeardownRequest(agmux_proto::TeardownRequest {
                    session_id: id,
                    action: action.into(),
                }),
            )
            .await?;
            match response {
                agmux_proto::IpcMessage::TeardownResponse(r) if r.success => {
                    println!("Teardown {action} accepted")
                }
                agmux_proto::IpcMessage::TeardownResponse(r) => {
                    anyhow::bail!("{}", r.error.unwrap_or_default())
                }
                _ => anyhow::bail!("Unexpected teardown response"),
            }
        }
        Some(Commands::Task { command }) => match command {
            TaskCommand::List { status, json } => {
                let resp = agmux_cli::ipc_client::list_tasks(&socket_path, &token, status).await?;
                if let Some(err) = resp.error {
                    anyhow::bail!("{err}");
                }
                if json {
                    println!("{}", serde_json::to_string(&resp.tasks)?);
                } else if resp.tasks.is_empty() {
                    println!("No tasks found");
                } else {
                    println!(
                        "{:<16} {:<16} {:<20} TITLE",
                        "TASK ID", "STATUS", "CLAIMED BY"
                    );
                    for t in resp.tasks {
                        println!(
                            "{:<16} {:<16} {:<20} {}",
                            t.id,
                            t.status,
                            t.claimed_by.as_deref().unwrap_or("-"),
                            t.title
                        );
                    }
                }
            }
            TaskCommand::Create {
                task_id,
                title,
                description,
            } => {
                let resp = agmux_cli::ipc_client::create_task(
                    &socket_path,
                    &token,
                    task_id.clone(),
                    title,
                    description,
                )
                .await?;
                if resp.success {
                    println!("Created task '{task_id}'");
                } else {
                    anyhow::bail!(
                        "{}",
                        resp.error.unwrap_or_else(|| "Failed to create task".into())
                    );
                }
            }
            TaskCommand::Claim { task_id, session } => {
                let sid = session
                    .or_else(|| std::env::var("AGMUX_SESSION_ID").ok())
                    .unwrap_or_else(|| format!("cli-{}", std::process::id()));
                let resp = agmux_cli::ipc_client::claim_task(
                    &socket_path,
                    &token,
                    task_id.clone(),
                    sid.clone(),
                )
                .await?;
                if resp.success {
                    println!("Task '{task_id}' claimed by session '{sid}'");
                } else {
                    anyhow::bail!(
                        "{}",
                        resp.error.unwrap_or_else(|| "Failed to claim task".into())
                    );
                }
            }
            TaskCommand::Finish { task_id, session } => {
                let sid = session
                    .or_else(|| std::env::var("AGMUX_SESSION_ID").ok())
                    .unwrap_or_else(|| format!("cli-{}", std::process::id()));
                let resp = agmux_cli::ipc_client::finish_task(
                    &socket_path,
                    &token,
                    task_id.clone(),
                    sid.clone(),
                )
                .await?;
                if resp.success {
                    println!("Task '{task_id}' finished by session '{sid}'");
                } else {
                    anyhow::bail!(
                        "{}",
                        resp.error.unwrap_or_else(|| "Failed to finish task".into())
                    );
                }
            }
            TaskCommand::Release { task_id, session } => {
                let sid = session
                    .or_else(|| std::env::var("AGMUX_SESSION_ID").ok())
                    .unwrap_or_else(|| format!("cli-{}", std::process::id()));
                let resp = agmux_cli::ipc_client::release_task(
                    &socket_path,
                    &token,
                    task_id.clone(),
                    sid.clone(),
                )
                .await?;
                if resp.success {
                    println!("Task '{task_id}' released by session '{sid}'");
                } else {
                    anyhow::bail!(
                        "{}",
                        resp.error
                            .unwrap_or_else(|| "Failed to release task".into())
                    );
                }
            }
        },
        Some(Commands::Report { command }) => match command {
            ReportCommand::Post {
                task_id,
                summary,
                pr_url,
                commit_sha,
                session,
            } => {
                let sid = session
                    .or_else(|| std::env::var("AGMUX_SESSION_ID").ok())
                    .unwrap_or_else(|| format!("cli-{}", std::process::id()));
                let resp = agmux_cli::ipc_client::post_report(
                    &socket_path,
                    &token,
                    task_id.clone(),
                    sid.clone(),
                    summary,
                    pr_url,
                    commit_sha,
                )
                .await?;
                if resp.success {
                    println!("Report posted for task '{task_id}' by session '{sid}'");
                } else {
                    anyhow::bail!(
                        "{}",
                        resp.error.unwrap_or_else(|| "Failed to post report".into())
                    );
                }
            }
            ReportCommand::View { task_id, json } => {
                let resp =
                    agmux_cli::ipc_client::view_reports(&socket_path, &token, task_id.clone())
                        .await?;
                if let Some(err) = resp.error {
                    anyhow::bail!("{err}");
                }
                if json {
                    println!("{}", serde_json::to_string(&resp.reports)?);
                } else if resp.reports.is_empty() {
                    println!("No reports found for task '{task_id}'");
                } else {
                    for r in resp.reports {
                        println!(
                            "Task: {}\nSession: {}\nSummary: {}\nPR URL: {}\nCommit SHA: {}\nCreated At: {}\n---",
                            r.task_id,
                            r.session_id,
                            r.summary,
                            r.pr_url.as_deref().unwrap_or("-"),
                            r.commit_sha.as_deref().unwrap_or("-"),
                            r.created_at
                        );
                    }
                }
            }
        },
        Some(Commands::Send { target_id, message }) => {
            let request =
                agmux_proto::IpcMessage::SendMessageRequest(agmux_proto::SendMessageRequest {
                    target_session_id: target_id.clone(),
                    message,
                });
            match agmux_cli::ipc_client::review_request(&socket_path, &token, request).await? {
                agmux_proto::IpcMessage::SendMessageResponse(r) if r.success && r.queued => {
                    println!(
                        "[QUEUED ✉] Target session is WAITING; message held in queue until prompt resolution"
                    )
                }
                agmux_proto::IpcMessage::SendMessageResponse(r) if r.success => {
                    println!("Delivered message to {target_id}")
                }
                agmux_proto::IpcMessage::SendMessageResponse(r) => {
                    anyhow::bail!("{}", r.error.unwrap_or_default())
                }
                other => anyhow::bail!("Unexpected send response: {other:?}"),
            }
        }
        Some(Commands::Input { session_id, text }) => {
            let stream = agmux_proto::connect_ipc(&socket_path)
                .await
                .context("Failed to connect to daemon")?;
            let mut framed = agmux_proto::new_framed(stream);
            framed
                .send(agmux_proto::IpcMessage::HandshakeRequest(
                    agmux_proto::HandshakeRequest {
                        client_version: env!("CARGO_PKG_VERSION").into(),
                        token: token.clone(),
                    },
                ))
                .await?;
            use futures_util::StreamExt;
            match framed.next().await.context("Handshake failed")?? {
                agmux_proto::IpcMessage::HandshakeResponse(r) if r.success => {}
                other => anyhow::bail!("Handshake failed: {other:?}"),
            }
            framed
                .send(agmux_proto::IpcMessage::TerminalInputFrame(
                    agmux_proto::TerminalInputFrame {
                        session_id,
                        data: format!("{text}\n").into_bytes(),
                    },
                ))
                .await?;
        }

        Some(Commands::Lock { command }) => match command {
            LockCommand::Acquire {
                name,
                session,
                wait,
            } => {
                let sid = session
                    .or_else(|| std::env::var("AGMUX_SESSION_ID").ok())
                    .unwrap_or_else(|| format!("cli-{}", std::process::id()));
                let resp = agmux_cli::ipc_client::acquire_lock(
                    &socket_path,
                    &token,
                    name.clone(),
                    sid.clone(),
                    wait,
                )
                .await?;
                if resp.success && resp.acquired {
                    println!("Lock '{name}' acquired by session '{sid}'");
                } else if resp.success && !resp.acquired {
                    let holder = resp.holder_session_id.unwrap_or_else(|| "unknown".into());
                    eprintln!("Lock '{name}' is currently held by session '{holder}'");
                    std::process::exit(1);
                } else {
                    anyhow::bail!(
                        "{}",
                        resp.error
                            .unwrap_or_else(|| "Failed to acquire lock".into())
                    );
                }
            }
            LockCommand::Release { name, session } => {
                let sid = session
                    .or_else(|| std::env::var("AGMUX_SESSION_ID").ok())
                    .unwrap_or_else(|| format!("cli-{}", std::process::id()));
                let resp = agmux_cli::ipc_client::release_lock(
                    &socket_path,
                    &token,
                    name.clone(),
                    sid.clone(),
                )
                .await?;
                if resp.success {
                    println!("Lock '{name}' released by session '{sid}'");
                } else {
                    anyhow::bail!(
                        "{}",
                        resp.error
                            .unwrap_or_else(|| "Failed to release lock".into())
                    );
                }
            }
            LockCommand::List { json } => {
                let resp = agmux_cli::ipc_client::list_locks(&socket_path, &token).await?;
                if let Some(err) = resp.error {
                    anyhow::bail!("{err}");
                }
                if json {
                    println!("{}", serde_json::to_string(&resp.locks)?);
                } else if resp.locks.is_empty() {
                    println!("No active locks");
                } else {
                    println!("{:<24} {:<24} ACQUIRED_AT", "LOCK NAME", "HOLDER SESSION");
                    for lock in resp.locks {
                        println!(
                            "{:<24} {:<24} {}",
                            lock.name, lock.holder_session_id, lock.acquired_at
                        );
                    }
                }
            }
        },

        Some(Commands::Reserve { paths, session }) => {
            let cwd = std::env::current_dir()?;
            let canonical_paths: Vec<String> = paths
                .iter()
                .map(|p| agmux_cli::ipc_client::canonicalize_repo_path(p, &cwd))
                .collect();
            let sid = session
                .or_else(|| std::env::var("AGMUX_SESSION_ID").ok())
                .unwrap_or_else(|| format!("cli-{}", std::process::id()));
            let resp = agmux_cli::ipc_client::reserve_files(
                &socket_path,
                &token,
                sid.clone(),
                canonical_paths.clone(),
            )
            .await?;
            if resp.success {
                println!(
                    "Reserved {} path(s) for session '{}'",
                    canonical_paths.len(),
                    sid
                );
            } else {
                anyhow::bail!(
                    "{}",
                    resp.error
                        .unwrap_or_else(|| "Active reservation conflict".into())
                );
            }
        }
        Some(Commands::Release { paths, session }) => {
            let cwd = std::env::current_dir()?;
            let canonical_paths: Vec<String> = paths
                .iter()
                .map(|p| agmux_cli::ipc_client::canonicalize_repo_path(p, &cwd))
                .collect();
            let sid = session
                .or_else(|| std::env::var("AGMUX_SESSION_ID").ok())
                .unwrap_or_else(|| format!("cli-{}", std::process::id()));
            let resp = agmux_cli::ipc_client::release_files(
                &socket_path,
                &token,
                sid.clone(),
                canonical_paths,
            )
            .await?;
            if resp.success {
                println!(
                    "Released {} reservation(s) for session '{}'",
                    resp.released_count, sid
                );
            } else {
                anyhow::bail!(
                    "{}",
                    resp.error
                        .unwrap_or_else(|| "Failed to release reservations".into())
                );
            }
        }
        Some(Commands::Reservations { json }) => {
            let resp = agmux_cli::ipc_client::list_reservations(&socket_path, &token).await?;
            if let Some(err) = resp.error {
                anyhow::bail!("{err}");
            }
            if json {
                println!("{}", serde_json::to_string(&resp.reservations)?);
            } else if resp.reservations.is_empty() {
                println!("No active file reservations");
            } else {
                println!(
                    "{:<40} {:<24} ACQUIRED_AT",
                    "RESERVED PATH", "HOLDER SESSION"
                );
                for r in resp.reservations {
                    println!(
                        "{:<40} {:<24} {}",
                        r.path, r.holder_session_id, r.acquired_at
                    );
                }
            }
        }
        Some(Commands::Watch {
            cursor,
            since,
            follow,
            json,
        }) => {
            agmux_cli::ipc_client::watch_events(
                &socket_path,
                &token,
                cursor,
                since,
                follow,
                |frame| {
                    if let Some(ev) = &frame.event {
                        if json {
                            println!("{}", serde_json::to_string(&frame)?);
                        } else {
                            let tag = if ev.replay {
                                "\x1b[33m[REPLAY]\x1b[0m"
                            } else {
                                "\x1b[32m[LIVE]\x1b[0m"
                            };
                            let sid = ev.session_id.as_deref().unwrap_or("-");
                            println!(
                                "{:<18} #{:<4} {:<24} {:<16} {}",
                                tag, ev.seq, ev.event_type, sid, ev.payload
                            );
                        }
                    }
                    Ok(true)
                },
            )
            .await?;
        }

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
        Some(Commands::AgentContext { .. }) => {
            unreachable!("agent-context handled before daemon startup")
        }
        Some(Commands::Update { .. }) => unreachable!("update handled before daemon startup"),
        Some(Commands::Init) => unreachable!("init handled before daemon startup"),
        Some(Commands::RegisterScheme) => {
            unreachable!("registration handled before daemon startup")
        }
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
            enable_mcp,
        }) => {
            let args = if enable_mcp {
                agmux_cli::mcp_server::agent_arguments(&command, args, &socket_path, &token_path)?
            } else {
                args
            };
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
        Some(Commands::InternalHook { session, event }) => {
            let sid = session
                .or_else(|| std::env::var("AGMUX_SESSION_ID").ok())
                .unwrap_or_else(|| format!("cli-{}", std::process::id()));
            let resp = agmux_cli::ipc_client::send_internal_hook(
                &socket_path,
                &token,
                sid.clone(),
                event.clone(),
            )
            .await?;
            if resp.success {
                println!(
                    "Hook '{}' applied to session '{}' -> {}",
                    event, sid, resp.new_state
                );
            } else {
                anyhow::bail!(
                    "{}",
                    resp.error
                        .unwrap_or_else(|| "Failed to apply internal hook".into())
                );
            }
        }
        Some(Commands::Connect {
            target,
            remote_socket,
            remote_token,
        }) => {
            let resp = agmux_cli::ipc_client::connect_remote(
                &socket_path,
                &token,
                target,
                remote_socket,
                remote_token,
            )
            .await?;
            if resp.success {
                println!(
                    "Connected to remote '{}' ({} mounted session(s))",
                    resp.host_label,
                    resp.mounted_sessions.len()
                );
            } else {
                anyhow::bail!(
                    "{}",
                    resp.error
                        .unwrap_or_else(|| "Failed to connect remote".into())
                );
            }
        }
        Some(Commands::Remote { command }) => match command {
            RemoteCommand::Connect {
                target,
                socket,
                token: rem_tok,
            } => {
                let resp = agmux_cli::ipc_client::connect_remote(
                    &socket_path,
                    &token,
                    target,
                    socket,
                    rem_tok,
                )
                .await?;
                if resp.success {
                    println!(
                        "Connected to remote '{}' ({} mounted session(s))",
                        resp.host_label,
                        resp.mounted_sessions.len()
                    );
                } else {
                    anyhow::bail!(
                        "{}",
                        resp.error
                            .unwrap_or_else(|| "Failed to connect remote".into())
                    );
                }
            }
            RemoteCommand::List { json } => {
                let resp = agmux_cli::ipc_client::list_remotes(&socket_path, &token).await?;
                if json {
                    println!("{}", serde_json::to_string(&resp.remotes)?);
                } else if resp.remotes.is_empty() {
                    println!("No connected remotes");
                } else {
                    for r in resp.remotes {
                        println!("{r}");
                    }
                }
            }
        },
        Some(Commands::Doctor { .. }) | Some(Commands::Daemon { .. }) => {
            unreachable!("handled before daemon startup")
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
