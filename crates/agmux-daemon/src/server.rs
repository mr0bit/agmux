//! IPC server listening on local domain sockets and routing client requests.

use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use futures_util::{SinkExt, StreamExt};

use agmux_core::pty_manager::PtyManager;
use agmux_proto::{
    IpcListener, IpcMessage, IpcServerStream, ListSessionsResponse, SearchLogsResponse,
    SessionExitEvent, SessionInfo, SessionStateChangedEvent, SpawnSessionResponse,
    TerminalOutputFrame, connect_ipc, new_framed,
};

pub use agmux_core::default_socket_path;

/// The core daemon server instance.
pub struct DaemonServer {
    socket_path: PathBuf,
    auth_token: String,
    pty_manager: PtyManager,
}

impl DaemonServer {
    /// Create a new server instance with the given socket path and auth token.
    pub fn new(socket_path: PathBuf, auth_token: String) -> Self {
        Self {
            socket_path,
            auth_token,
            pty_manager: PtyManager::new(),
        }
    }

    /// Run the IPC listener loop.
    pub async fn run(self) -> Result<()> {
        #[cfg(unix)]
        if let Some(parent) = self.socket_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create socket dir {}", parent.display()))?;
        }

        // Acquire single-instance lockfile BEFORE touching or removing any socket file!
        let lock_path = crate::instance_lock::InstanceLock::lock_path_for_socket(&self.socket_path);
        let _instance_lock =
            crate::instance_lock::InstanceLock::acquire(&lock_path).with_context(|| {
                format!(
                    "Failed to acquire single-instance lock at {}",
                    lock_path.display()
                )
            })?;

        #[cfg(unix)]
        if self.socket_path.exists() {
            // Check if another daemon is already responding
            if connect_ipc(&self.socket_path).await.is_ok() {
                anyhow::bail!(
                    "Active daemon already running on socket {}",
                    self.socket_path.display()
                );
            }
            // Remove stale socket
            let _ = fs::remove_file(&self.socket_path);
        }

        #[cfg(windows)]
        {
            if connect_ipc(&self.socket_path).await.is_ok() {
                anyhow::bail!(
                    "Active daemon already running on pipe {}",
                    self.socket_path.display()
                );
            }
        }

        let db_path = if std::env::var_os("AGMUX_DB_PATH").is_some()
            || self.socket_path == agmux_core::default_socket_path()
            || crate::instance_lock::is_named_pipe_path(&self.socket_path)
        {
            agmux_core::storage_engine::default_database_path()
        } else {
            self.socket_path.with_extension("db")
        };

        let storage_engine = agmux_core::storage_engine::StorageEngine::open(&db_path)
            .context("Failed to open session database")?;
        if let Ok(recovered) =
            crate::recovery_manager::RecoveryManager::recover_orphaned_sessions(&storage_engine)
            && !recovered.is_empty()
        {
            tracing::info!("Recovered {} orphaned session(s) as DEAD", recovered.len());
        }
        let storage = Arc::new(std::sync::Mutex::new(storage_engine));

        let listener = IpcListener::bind(&self.socket_path)
            .with_context(|| format!("Failed to bind socket {}", self.socket_path.display()))?;

        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(&self.socket_path, fs::Permissions::from_mode(0o600));
        }

        tracing::info!(
            "agmuxd listening on {} (version {})",
            self.socket_path.display(),
            env!("CARGO_PKG_VERSION")
        );

        let pty_manager = self.pty_manager;
        let auth_token = Arc::new(self.auth_token);
        let (events, _) = tokio::sync::broadcast::channel::<SessionStateChangedEvent>(256);

        let lock_manager = crate::lock_manager::LockManager::new(storage.clone());
        let file_reservations =
            crate::file_reservation_table::FileReservationTable::new(storage.clone());
        let journal = crate::journal_watcher::JournalWatcher::new(storage.clone());
        let kanban_manager =
            crate::kanban_manager::KanbanManager::with_journal(storage.clone(), journal.clone());
        let shutdown_coordinator =
            crate::shutdown_coordinator::ShutdownCoordinator::new(pty_manager.clone());
        let config_watcher = crate::config_watcher::ConfigWatcher::new(
            crate::config_watcher::ConfigWatcher::default_config_path(),
        );
        let _config_watch_task = config_watcher.spawn_watch_loop(journal.clone());
        let subscription_manager =
            crate::subscription_manager::SubscriptionManager::new(pty_manager.clone());
        let event_broadcaster = crate::event_broadcaster::EventBroadcaster::new(
            storage.clone(),
            config_watcher.clone(),
        );
        let pty_resizer = agmux_core::pty_resizer::PtyResizer::new(pty_manager.clone());
        let subtab_spawner = agmux_core::subtab_spawner::SubtabSpawner::new(pty_manager.clone());
        let hook_listener = crate::hook_listener::HookListener::new(
            storage.clone(),
            events.clone(),
            journal.clone(),
            event_broadcaster.clone(),
            pty_manager.clone(),
        );
        let teardown_scheduler = crate::teardown_scheduler::TeardownScheduler::new(
            storage.clone(),
            pty_manager.clone(),
            event_broadcaster.clone(),
            lock_manager.clone(),
            file_reservations.clone(),
            kanban_manager.clone(),
        );
        let ssh_federation = crate::ssh_federation::SshFederationManager::new();
        let network_auth = crate::network_auth::NetworkAuthValidator::new(auth_token.as_str());
        let (shutdown_tx, mut shutdown_rx) = tokio::sync::watch::channel(false);

        let ctx = DaemonContext {
            pty_manager,
            expected_token: auth_token,
            storage,
            events,
            lock_manager,
            file_reservations,
            journal,
            kanban_manager,
            shutdown_coordinator: shutdown_coordinator.clone(),
            config_watcher,
            subscription_manager,
            event_broadcaster,
            pty_resizer,
            subtab_spawner,
            hook_listener,
            teardown_scheduler,
            ssh_federation,
            network_auth,
            shutdown_tx,
        };

        #[cfg(unix)]
        let mut sigterm =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())?;
        #[cfg(unix)]
        let mut sighup = tokio::signal::unix::signal(tokio::signal::unix::SignalKind::hangup())?;

        loop {
            #[cfg(unix)]
            tokio::select! {
                accept_res = listener.accept_stream() => {
                    match accept_res {
                        Ok(stream) => {
                            let ctx = ctx.clone();
                            tokio::spawn(async move {
                                if let Err(e) = handle_client(stream, ctx).await {
                                    tracing::debug!("Client session ended: {e:#}");
                                }
                            });
                        }
                        Err(e) => {
                            tracing::error!("Error accepting IPC client connection: {e}");
                        }
                    }
                }
                _ = shutdown_rx.changed() => {
                    if *shutdown_rx.borrow() {
                        break;
                    }
                }
                _ = sigterm.recv() => {
                    let _ = shutdown_coordinator.shutdown_all().await;
                    break;
                }
                _ = sighup.recv() => {
                    let _ = shutdown_coordinator.shutdown_all().await;
                    break;
                }
                _ = tokio::signal::ctrl_c() => {
                    let _ = shutdown_coordinator.shutdown_all().await;
                    break;
                }
            }

            #[cfg(not(unix))]
            tokio::select! {
                accept_res = listener.accept_stream() => {
                    match accept_res {
                        Ok(stream) => {
                            let ctx = ctx.clone();
                            tokio::spawn(async move {
                                if let Err(e) = handle_client(stream, ctx).await {
                                    tracing::debug!("Client session ended: {e:#}");
                                }
                            });
                        }
                        Err(e) => {
                            tracing::error!("Error accepting IPC client connection: {e}");
                        }
                    }
                }
                _ = shutdown_rx.changed() => {
                    if *shutdown_rx.borrow() {
                        break;
                    }
                }
                _ = tokio::signal::ctrl_c() => {
                    let _ = shutdown_coordinator.shutdown_all().await;
                    break;
                }
            }
        }

        Ok(())
    }
}

/// Shared daemon service contexts passed to client session handlers.
#[derive(Clone)]
pub struct DaemonContext {
    pub pty_manager: PtyManager,
    pub expected_token: Arc<String>,
    pub storage: Arc<std::sync::Mutex<agmux_core::storage_engine::StorageEngine>>,
    pub events: tokio::sync::broadcast::Sender<SessionStateChangedEvent>,
    pub lock_manager: crate::lock_manager::LockManager,
    pub file_reservations: crate::file_reservation_table::FileReservationTable,
    pub journal: crate::journal_watcher::JournalWatcher,
    pub kanban_manager: crate::kanban_manager::KanbanManager,
    pub shutdown_coordinator: crate::shutdown_coordinator::ShutdownCoordinator,
    pub config_watcher: crate::config_watcher::ConfigWatcher,
    pub subscription_manager: crate::subscription_manager::SubscriptionManager,
    pub event_broadcaster: crate::event_broadcaster::EventBroadcaster,
    pub pty_resizer: agmux_core::pty_resizer::PtyResizer,
    pub subtab_spawner: agmux_core::subtab_spawner::SubtabSpawner,
    pub hook_listener: crate::hook_listener::HookListener,
    pub teardown_scheduler: crate::teardown_scheduler::TeardownScheduler,
    pub ssh_federation: crate::ssh_federation::SshFederationManager,
    pub network_auth: crate::network_auth::NetworkAuthValidator,
    pub shutdown_tx: tokio::sync::watch::Sender<bool>,
}

/// Process an individual client connection through handshake and session streaming.
async fn handle_client(stream: IpcServerStream, ctx: DaemonContext) -> Result<()> {
    let DaemonContext {
        pty_manager,
        expected_token: _,
        storage,
        events,
        lock_manager,
        file_reservations,
        journal,
        kanban_manager,
        shutdown_coordinator,
        config_watcher,
        subscription_manager,
        event_broadcaster,
        pty_resizer,
        subtab_spawner,
        hook_listener,
        teardown_scheduler,
        ssh_federation,
        network_auth,
        shutdown_tx,
    } = ctx;

    let framed = new_framed(stream);
    let (mut sink, mut reader) = framed.split();

    // 1. Enforce Authentication Handshake as first message
    let first_msg = reader
        .next()
        .await
        .context("Connection closed before handshake")?
        .context("Framing error reading handshake")?;

    match first_msg {
        IpcMessage::HandshakeRequest(req) => {
            let handshake_resp = network_auth.verify_handshake(&req);
            if !handshake_resp.success {
                let _ = sink
                    .send(IpcMessage::HandshakeResponse(handshake_resp))
                    .await;
                anyhow::bail!("Client authentication failed: invalid token");
            }

            sink.send(IpcMessage::HandshakeResponse(handshake_resp))
                .await
                .context("Failed to send handshake response")?;
        }
        _ => {
            anyhow::bail!("Protocol violation: expected HandshakeRequest as first frame");
        }
    }

    // 2. Establish bidirectional message routing
    let (out_tx, mut out_rx) = tokio::sync::mpsc::channel::<IpcMessage>(256);

    let writer_task = tokio::spawn(async move {
        while let Some(msg) = out_rx.recv().await {
            if sink.send(msg).await.is_err() {
                break;
            }
        }
    });

    let mut event_rx = events.subscribe();
    let mut bcast_rx = event_broadcaster.subscribe();
    let event_out = out_tx.clone();
    let event_task = tokio::spawn(async move {
        loop {
            tokio::select! {
                res = event_rx.recv() => {
                    match res {
                        Ok(event) => {
                            if event_out
                                .send(IpcMessage::SessionStateChangedEvent(event))
                                .await
                                .is_err()
                            {
                                break;
                            }
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                        Err(_) => break,
                    }
                }
                res = bcast_rx.recv() => {
                    match res {
                        Ok(msg) => {
                            if event_out.send(msg).await.is_err() {
                                break;
                            }
                        }
                        Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                        Err(_) => break,
                    }
                }
            }
        }
    });

    let mut viewport_task: Option<tokio::task::JoinHandle<()>> = None;
    let subscribed_session: std::sync::Arc<std::sync::Mutex<Option<String>>> =
        std::sync::Arc::new(std::sync::Mutex::new(None));
    while let Some(frame_res) = reader.next().await {
        let message = match frame_res {
            Ok(msg) => msg,
            Err(e) => {
                tracing::warn!("Client frame decode error: {e}");
                break;
            }
        };

        // Forks are lowered to the regular spawn path only after validating a real
        // adapter capability and a captured native conversation identity.
        let mut fork_parent: Option<String> = None;
        let mut fork_subtab = false;
        let message = if let IpcMessage::WorkspaceRequest(agmux_proto::WorkspaceRequest::Fork {
            session_id,
            new_session_id,
            subtab,
        }) = &message
        {
            let resolved = {
                let db = storage.lock().map_err(|e| anyhow::anyhow!(e.to_string()))?;
                match (
                    db.get_session(session_id)?,
                    db.get_cli_session_uuid(session_id)?,
                ) {
                    (None, _) => Err("Session not found"),
                    (Some(parent), uuid) => {
                        let command = parent.command.split_whitespace().next().unwrap_or("");
                        let name = std::path::Path::new(command)
                            .file_stem()
                            .and_then(|n| n.to_str())
                            .unwrap_or("");
                        let registry = agmux_core::plugin_api::PluginRegistry::with_builtins();
                        match (
                            registry.all().into_iter().find(|p| p.binary_name() == name),
                            uuid,
                        ) {
                            (Some(plugin), Some(uuid)) => match plugin.fork_args(&uuid) {
                                Some(args) => Ok(agmux_proto::SpawnSessionRequest {
                                    session_id: new_session_id.clone(),
                                    command: command.to_owned(),
                                    args,
                                    cwd: Some(parent.cwd),
                                    worktree_branch: None,
                                    detached: true,
                                    cols: None,
                                    rows: None,
                                }),
                                None => Err("This adapter does not support conversation forking"),
                            },
                            (Some(plugin), None) if plugin.fork_args("").is_some() => {
                                Err("No native conversation ID captured for this session")
                            }
                            _ => Err("This adapter does not support conversation forking"),
                        }
                    }
                }
            };
            match resolved {
                Ok(req) => {
                    if *subtab && !new_session_id.starts_with(&format!("{session_id}:fork-")) {
                        out_tx
                            .send(IpcMessage::WorkspaceResponse(
                                agmux_proto::WorkspaceResponse {
                                    sessions: vec![],
                                    metadata: Default::default(),
                                    error: Some("Invalid sub-tab identity".into()),
                                },
                            ))
                            .await?;
                        continue;
                    }
                    fork_parent = Some(session_id.clone());
                    fork_subtab = *subtab;
                    IpcMessage::SpawnSessionRequest(req)
                }
                Err(error) => {
                    out_tx
                        .send(IpcMessage::WorkspaceResponse(
                            agmux_proto::WorkspaceResponse {
                                sessions: vec![],
                                metadata: Default::default(),
                                error: Some(error.into()),
                            },
                        ))
                        .await?;
                    continue;
                }
            }
        } else {
            message
        };
        match message {
            IpcMessage::CompanionRequest(agmux_proto::CompanionRequest::Chat {
                session_id,
                operation,
                payload,
            }) => {
                let session = {
                    let db = storage.lock().map_err(|e| anyhow::anyhow!(e.to_string()))?;
                    db.get_session(&session_id)?
                };
                let result = if let Some(session) = session {
                    if session.command.split_whitespace().next().is_some_and(|c| {
                        c.replace('\\', "/")
                            .rsplit('/')
                            .next()
                            .is_some_and(|n| matches!(n, "gemini" | "gemini.exe" | "gemini.cmd"))
                    }) {
                        crate::gemini_chat::request(
                            &session_id,
                            std::path::Path::new(&session.cwd),
                            &operation,
                            &payload,
                            storage.clone(),
                            matches!(
                                session.state.as_str(),
                                "DEAD" | "ARCHIVED" | "COMPLETED" | "EXITED"
                            ),
                        )
                        .await
                    } else {
                        Err(anyhow::anyhow!(
                            "Structured chat is available for Gemini sessions"
                        ))
                    }
                } else {
                    Err(anyhow::anyhow!("Session not found"))
                };
                let (content, error) = match result {
                    Ok(v) => (Some(v), None),
                    Err(e) => (None, Some(e.to_string())),
                };
                out_tx
                    .send(IpcMessage::CompanionResponse(
                        agmux_proto::CompanionResponse {
                            notifications: vec![],
                            selected_panel: None,
                            panels: vec![],
                            selected: None,
                            content,
                            error,
                        },
                    ))
                    .await?;
            }
            IpcMessage::CompanionRequest(agmux_proto::CompanionRequest::StartChat {
                session_id,
                cwd,
                model,
            }) => {
                let result = async {
                    let root = tokio::fs::canonicalize(cwd).await?;
                    anyhow::ensure!(root.is_dir(), "Project must be a directory");
                    let info = SessionInfo {
                        session_id: session_id.clone(),
                        pid: 0,
                        cwd: root.to_string_lossy().into_owned(),
                        command: "gemini --acp".into(),
                        created_at: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)?
                            .as_secs(),
                        state: "IDLE".into(),
                    };
                    {
                        let db = storage.lock().map_err(|e| anyhow::anyhow!(e.to_string()))?;
                        db.insert_session(&info)?;
                        db.select_companion_session(&session_id)?;
                    }
                    crate::gemini_chat::create(&storage, &session_id, model)?;
                    let git_root = root.clone();
                    let sha = tokio::task::spawn_blocking(move || {
                        std::process::Command::new("git")
                            .current_dir(git_root)
                            .args(["rev-parse", "--verify", "HEAD"])
                            .output()
                            .ok()
                            .filter(|o| o.status.success())
                            .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned())
                    })
                    .await?;
                    storage
                        .lock()
                        .map_err(|e| anyhow::anyhow!(e.to_string()))?
                        .set_start_git_sha(&session_id, sha.as_deref())?;
                    Ok::<_, anyhow::Error>(serde_json::json!({"session_id":session_id}).to_string())
                }
                .await;
                let (content, error) = match result {
                    Ok(c) => (Some(c), None),
                    Err(e) => (None, Some(e.to_string())),
                };
                out_tx
                    .send(IpcMessage::CompanionResponse(
                        agmux_proto::CompanionResponse {
                            notifications: vec![],
                            selected_panel: None,
                            panels: vec![],
                            selected: None,
                            content,
                            error,
                        },
                    ))
                    .await?;
            }
            IpcMessage::CompanionRequest(agmux_proto::CompanionRequest::ChatStates) => {
                let content = Some(crate::gemini_chat::statuses(storage.clone()).await?);
                out_tx
                    .send(IpcMessage::CompanionResponse(
                        agmux_proto::CompanionResponse {
                            notifications: vec![],
                            selected_panel: None,
                            panels: vec![],
                            selected: None,
                            content,
                            error: None,
                        },
                    ))
                    .await?;
            }
            IpcMessage::CompanionRequest(agmux_proto::CompanionRequest::Models { cwd }) => {
                let result = crate::gemini_chat::models(std::path::Path::new(&cwd)).await;
                let (content, error) = match result {
                    Ok(v) => (Some(v), None),
                    Err(e) => (None, Some(e.to_string())),
                };
                out_tx
                    .send(IpcMessage::CompanionResponse(
                        agmux_proto::CompanionResponse {
                            notifications: vec![],
                            selected_panel: None,
                            panels: vec![],
                            selected: None,
                            content,
                            error,
                        },
                    ))
                    .await?;
            }
            IpcMessage::CompanionRequest(agmux_proto::CompanionRequest::Reply {
                session_id,
                message,
            }) => {
                let managed = managed_prompt(&storage, &session_id, &message).await;
                let error = if let Some(result) = managed {
                    result.err()
                } else if message.len() > 65536 {
                    Some("Reply too large".to_owned())
                } else {
                    match pty_manager.get_session(&session_id).await {
                        Ok(session) => session.submit_prompt(&message).err().map(|e| e.to_string()),
                        Err(e) => Some(e.to_string()),
                    }
                };
                out_tx
                    .send(IpcMessage::CompanionResponse(
                        agmux_proto::CompanionResponse {
                            notifications: vec![],
                            selected_panel: None,
                            panels: vec![],
                            selected: None,
                            content: None,
                            error,
                        },
                    ))
                    .await?;
            }
            IpcMessage::CompanionRequest(req) => {
                let db = storage.clone();
                let response = tokio::task::spawn_blocking(move || {
                    use agmux_proto::CompanionRequest as R;
                    let mut response = agmux_proto::CompanionResponse {
                        notifications: vec![],
                        selected_panel: None,
                        panels: vec![],
                        selected: None,
                        content: None,
                        error: None,
                    };
                    let result = (|| -> anyhow::Result<()> {
                        let db = db.lock().map_err(|e| anyhow::anyhow!(e.to_string()))?;
                        match req {
                            R::Open { session_id, target } => {
                                let session = db
                                    .get_session(&session_id)?
                                    .ok_or_else(|| anyhow::anyhow!("Session not found"))?;
                                let (kind, target) = if target.starts_with("http://")
                                    || target.starts_with("https://")
                                {
                                    let authority = target
                                        .split_once("://")
                                        .map(|(_, v)| v.split('/').next().unwrap_or(""))
                                        .unwrap_or("");
                                    anyhow::ensure!(
                                        !authority.is_empty()
                                            && !authority.contains(['@', '\n', '\r', ' ']),
                                        "Invalid URL"
                                    );
                                    ("web", target)
                                } else {
                                    anyhow::ensure!(!target.contains("://"), "Unsupported scheme");
                                    let path = std::path::Path::new(&target);
                                    anyhow::ensure!(
                                        !path
                                            .components()
                                            .any(|c| matches!(c, std::path::Component::ParentDir)),
                                        "Path traversal rejected"
                                    );
                                    let root = std::fs::canonicalize(&session.cwd)?;
                                    let file = std::fs::canonicalize(if path.is_absolute() {
                                        path.to_path_buf()
                                    } else {
                                        root.join(path)
                                    })?;
                                    anyhow::ensure!(
                                        file.starts_with(&root) && file.is_file(),
                                        "File outside session workspace"
                                    );
                                    ("file", file.to_string_lossy().into_owned())
                                };
                                anyhow::ensure!(
                                    db.register_panel(&session_id, kind, &target)?,
                                    "Session not found"
                                );
                                db.select_companion_session(&session_id)?;
                            }
                            R::Select { session_id } => {
                                anyhow::ensure!(
                                    db.select_companion_session(&session_id)?,
                                    "Session not found"
                                );
                            }
                            R::SelectPanel {
                                session_id,
                                panel_id,
                            } => {
                                anyhow::ensure!(
                                    db.select_companion_panel(&session_id, panel_id)?,
                                    "Panel not found"
                                );
                                db.select_companion_session(&session_id)?;
                            }
                            R::Read { panel_id } | R::Write { panel_id, .. } => {
                                let panel = db
                                    .companion_panels()?
                                    .into_iter()
                                    .find(|p| p.id == panel_id && p.kind == "file")
                                    .ok_or_else(|| anyhow::anyhow!("File panel not found"))?;
                                let session = db
                                    .get_session(&panel.session_id)?
                                    .ok_or_else(|| anyhow::anyhow!("Session not found"))?;
                                let root = std::fs::canonicalize(session.cwd)?;
                                let file = std::fs::canonicalize(&panel.target)?;
                                anyhow::ensure!(
                                    file.starts_with(root)
                                        && file.to_string_lossy() == panel.target
                                        && file.is_file(),
                                    "Panel file changed or outside workspace"
                                );
                                match req {
                                    R::Write { content, .. } => {
                                        anyhow::ensure!(
                                            content.len() <= 1_000_000,
                                            "File too large"
                                        );
                                        std::fs::write(file, content)?;
                                    }
                                    _ => {
                                        anyhow::ensure!(
                                            std::fs::metadata(&file)?.len() <= 1_000_000,
                                            "File too large"
                                        );
                                        response.content = Some(std::fs::read_to_string(file)?);
                                    }
                                }
                            }
                            R::List => {
                                response.content =
                                    Some(agmux_proto::COMPANION_ACP_CAPABILITY.into());
                            }
                            R::Reply { .. }
                            | R::Chat { .. }
                            | R::Models { .. }
                            | R::ChatStates
                            | R::StartChat { .. } => {
                                unreachable!()
                            }
                        }
                        response.notifications = db.companion_notifications()?;
                        response.panels = db.companion_panels()?;
                        response.selected = db.companion_selection()?;
                        response.selected_panel = db.companion_selected_panel()?;
                        Ok(())
                    })();
                    if let Err(error) = result {
                        response.error = Some(error.to_string());
                    }
                    response
                })
                .await?;
                out_tx.send(IpcMessage::CompanionResponse(response)).await?;
            }
            IpcMessage::WorkspaceRequest(req) => {
                let changed = matches!(req, agmux_proto::WorkspaceRequest::Set { .. });
                let (mut sessions, metadata, error) = {
                    let db = storage.lock().map_err(|e| anyhow::anyhow!(e.to_string()))?;
                    let error = match req {
                        agmux_proto::WorkspaceRequest::Set {
                            session_id,
                            metadata,
                        } => match db.set_workspace_metadata(&session_id, &metadata) {
                            Ok(true) => None,
                            Ok(false) => Some("Session not found".to_string()),
                            Err(e) => Some(e.to_string()),
                        },
                        agmux_proto::WorkspaceRequest::List => None,
                        agmux_proto::WorkspaceRequest::Fork { .. } => unreachable!(),
                    };
                    (
                        db.list_workspace_sessions()?,
                        db.workspace_metadata()?,
                        error,
                    )
                };
                sessions.extend(ssh_federation.list_mounted_sessions().await);
                out_tx
                    .send(IpcMessage::WorkspaceResponse(
                        agmux_proto::WorkspaceResponse {
                            sessions,
                            metadata,
                            error,
                        },
                    ))
                    .await?;
                if changed {
                    event_broadcaster.broadcast_session_list().await;
                }
            }
            IpcMessage::ReadSessionRequest(req) => {
                let result = if let Ok(active) = pty_manager.get_session(&req.session_id).await {
                    Ok(active.capture_screen())
                } else {
                    let db = storage.lock().map_err(|e| anyhow::anyhow!(e.to_string()))?;
                    match db.load_snapshot(&req.session_id) {
                        Ok(Some(snapshot)) => {
                            agmux_core::snapshot_compressor::decompress_snapshot(&snapshot)
                                .map_err(|e| e.to_string())
                        }
                        Ok(None) => Err("Session or snapshot not found".to_string()),
                        Err(e) => Err(e.to_string()),
                    }
                };
                let response = match result {
                    Ok(bytes) => agmux_proto::ReadSessionResponse {
                        success: true,
                        screen: String::from_utf8_lossy(&bytes).into_owned(),
                        error: None,
                    },
                    Err(error) => agmux_proto::ReadSessionResponse {
                        success: false,
                        screen: String::new(),
                        error: Some(error),
                    },
                };
                out_tx
                    .send(IpcMessage::ReadSessionResponse(response))
                    .await?;
            }
            IpcMessage::TeardownRequest(req) => {
                let result = teardown_scheduler
                    .request(&req.session_id, &req.action)
                    .await;
                out_tx
                    .send(IpcMessage::TeardownResponse(
                        agmux_proto::TeardownResponse {
                            success: result.is_ok(),
                            error: result.err(),
                        },
                    ))
                    .await?;
            }
            IpcMessage::SpawnSessionRequest(mut req) => {
                // Claude exposes a deterministic native session ID on fresh launches.
                let is_claude = std::path::Path::new(&req.command)
                    .file_stem()
                    .is_some_and(|n| n == "claude");
                let native_uuid = if is_claude
                    && !req
                        .args
                        .iter()
                        .any(|a| matches!(a.as_str(), "--resume" | "--session-id" | "--continue"))
                {
                    let uuid = agmux_core::session_tracker::SessionTracker::mint_uuid_v4();
                    req.args
                        .splice(0..0, ["--session-id".to_string(), uuid.clone()]);
                    Some(uuid)
                } else if is_claude {
                    req.args
                        .windows(2)
                        .find(|w| w[0] == "--session-id")
                        .map(|w| w[1].clone())
                } else {
                    None
                };
                let session_id = req.session_id.clone();
                let is_detached = req.detached;
                let req_cols = req.cols;
                let req_rows = req.rows;
                let command = std::iter::once(req.command.as_str())
                    .chain(req.args.iter().map(String::as_str))
                    .collect::<Vec<_>>()
                    .join(" ");
                let mut cwd = req.cwd.map(PathBuf::from);
                let mut port = None;
                if let Some(branch) = req.worktree_branch {
                    let base = cwd.clone().unwrap_or(std::env::current_dir()?);
                    let result = tokio::task::spawn_blocking(move || -> anyhow::Result<_> {
                        let output = std::process::Command::new("git")
                            .current_dir(base)
                            .args(["rev-parse", "--show-toplevel"])
                            .output()?;
                        if !output.status.success() {
                            anyhow::bail!(
                                "Git repository discovery failed: {}",
                                String::from_utf8_lossy(&output.stderr).trim()
                            );
                        }
                        let root = PathBuf::from(String::from_utf8(output.stdout)?.trim());
                        Ok(agmux_core::worktree_manager::provision_worktree(
                            &root, &branch,
                        )?)
                    })
                    .await;
                    match result {
                        Ok(Ok(worktree)) => {
                            cwd = Some(worktree.path);
                            port = Some(worktree.port);
                        }
                        failure => {
                            let error = match failure {
                                Ok(Err(e)) => e.to_string(),
                                Err(e) => e.to_string(),
                                _ => unreachable!(),
                            };
                            let _ = out_tx
                                .send(IpcMessage::SpawnSessionResponse(SpawnSessionResponse {
                                    session_id,
                                    success: false,
                                    error: Some(error),
                                }))
                                .await;
                            continue;
                        }
                    }
                }
                let cwd_text = cwd
                    .clone()
                    .unwrap_or(std::env::current_dir()?)
                    .display()
                    .to_string();
                let git_cwd = cwd_text.clone();
                let start_sha = tokio::task::spawn_blocking(move || {
                    std::process::Command::new("git")
                        .current_dir(git_cwd)
                        .args(["rev-parse", "--verify", "HEAD"])
                        .output()
                        .ok()
                        .filter(|o| o.status.success())
                        .map(|o| String::from_utf8_lossy(&o.stdout).trim().to_owned())
                })
                .await?;
                if fork_subtab && port.is_none() {
                    if let Some(parent) = fork_parent.as_deref() {
                        if let Ok(parent_session) = pty_manager.get_session(parent).await {
                            port = parent_session.port;
                        }
                    }
                }
                let env = if fork_subtab {
                    vec![(
                        "AGMUX_PARENT_SESSION_ID".to_owned(),
                        fork_parent.clone().unwrap_or_default(),
                    )]
                } else {
                    Vec::new()
                };
                match pty_manager
                    .spawn_with_env(session_id.clone(), req.command, req.args, cwd, port, env)
                    .await
                {
                    Ok(session) => {
                        if let (Some(c), Some(r)) = (req_cols, req_rows) {
                            if c != 80 || r != 24 {
                                let _ = pty_resizer.resize_session(&session_id, c, r).await;
                            }
                        }
                        let info = SessionInfo {
                            session_id: session_id.clone(),
                            pid: session.pid,
                            cwd: cwd_text,
                            command,
                            created_at: std::time::SystemTime::now()
                                .duration_since(std::time::UNIX_EPOCH)?
                                .as_secs(),
                            state: "RUNNING".into(),
                        };
                        let db = storage.clone();
                        let inserted = tokio::task::spawn_blocking(move || {
                            let db = db.lock().map_err(|e| e.to_string())?;
                            db.insert_session(&info).map_err(|e| e.to_string())?;
                            if let Some(parent) = fork_parent {
                                let mut metadata = db
                                    .workspace_metadata()
                                    .map_err(|e| e.to_string())?
                                    .remove(&parent)
                                    .unwrap_or_else(|| agmux_proto::WorkspaceMetadata {
                                        project: info.cwd.clone(),
                                        ..Default::default()
                                    });
                                metadata.favorite = false;
                                metadata.archived = false;
                                db.set_workspace_metadata(&info.session_id, &metadata)
                                    .map_err(|e| e.to_string())?;
                            }
                            db.set_start_git_sha(&info.session_id, start_sha.as_deref())
                                .map_err(|e| e.to_string())?;
                            db.set_cli_session_uuid(&info.session_id, native_uuid.as_deref())
                                .map_err(|e| e.to_string())
                        })
                        .await?;
                        if let Err(error) = inserted {
                            tracing::error!("Failed to persist session: {error}");
                            let _ = out_tx
                                .send(IpcMessage::SpawnSessionResponse(SpawnSessionResponse {
                                    session_id,
                                    success: false,
                                    error: Some(error),
                                }))
                                .await;
                            continue;
                        }
                        let resp = IpcMessage::SpawnSessionResponse(SpawnSessionResponse {
                            session_id: session_id.clone(),
                            success: true,
                            error: None,
                        });
                        let _ = out_tx.send(resp).await;

                        // Attach streaming forwarder
                        let out_tx_fwd = out_tx.clone();
                        let output_db = storage.clone();
                        let fwd_session_id = session_id.clone();
                        let state_events = events.clone();
                        let pty_fwd = pty_manager.clone();
                        let session_clone = session.clone();
                        let lock_mgr_fwd = lock_manager.clone();
                        let file_res_fwd = file_reservations.clone();
                        let kanban_mgr_fwd = kanban_manager.clone();
                        let journal_fwd = journal.clone();
                        let bcast_fwd = event_broadcaster.clone();
                        let cfg_fwd = config_watcher.clone();
                        let teardown_fwd = teardown_scheduler.clone();
                        let subscribed_check = subscribed_session.clone();
                        tokio::spawn(async move {
                            let mut output_rx = session.subscribe_output();
                            let mut exit_rx = session.subscribe_exit();
                            let mut input_rx = session.subscribe_input();
                            let threshold = cfg_fwd.get().idle_after_ms;
                            let mut classifier = agmux_core::state_machine::Classifier::new(
                                std::time::Duration::from_millis(threshold),
                            );

                            let mut history =
                                agmux_core::fts_indexer::TerminalHistory::new(1024 * 1024);
                            let mut line_no = 0u64;
                            loop {
                                let finished = if exit_rx.borrow().is_some() {
                                    true
                                } else {
                                    tokio::select! {
                                        result = output_rx.recv() => {
                                            match result {
                                                Ok(data) => {
                                                    publish_transition(output_db.clone(), &state_events, &fwd_session_id, classifier.output(&data), Some(&session_clone), Some(&journal_fwd), Some(&bcast_fwd)).await;
                                                    let lines = history.ingest(&data);
                                                    let partial = history.pending_line();
                                                    let indexed = !lines.is_empty() || partial.is_some();
                                                    index_lines(output_db.clone(), &fwd_session_id, &mut line_no, lines).await;
                                                    if let Some(partial) = partial { let mut partial_line = line_no; index_lines(output_db.clone(), &fwd_session_id, &mut partial_line, vec![partial]).await; }
                                                    if indexed { bcast_fwd.transcript_indexed(&fwd_session_id); }
                                                    let should_forward = !is_detached && subscribed_check.lock().unwrap().as_deref() != Some(&fwd_session_id);
                                                    if should_forward {
                                                        let _ = out_tx_fwd.send(IpcMessage::TerminalOutputFrame(TerminalOutputFrame { session_id: fwd_session_id.clone(), data })).await;
                                                    }
                                                    false
                                                }
                                                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => { tracing::warn!("Lost {n} terminal output frames"); false }
                                                Err(_) => true,
                                            }
                                        }
                                        _ = input_rx.recv() => {
                                            publish_transition(output_db.clone(), &state_events, &fwd_session_id, classifier.input(), Some(&session_clone), Some(&journal_fwd), Some(&bcast_fwd)).await;
                                            false
                                        }
                                        _ = tokio::time::sleep_until(classifier.deadline().into()), if classifier.state() == agmux_core::state_machine::SessionState::Running => {
                                            publish_transition(output_db.clone(), &state_events, &fwd_session_id, classifier.timeout(), Some(&session_clone), Some(&journal_fwd), Some(&bcast_fwd)).await;
                                            false
                                        }
                                        _ = exit_rx.changed() => true,
                                    }
                                };
                                if finished {
                                    // The reader has stopped before exit is signaled; drain queued frames first.
                                    while let Ok(data) = output_rx.try_recv() {
                                        publish_transition(
                                            output_db.clone(),
                                            &state_events,
                                            &fwd_session_id,
                                            classifier.output(&data),
                                            Some(&session_clone),
                                            Some(&journal_fwd),
                                            Some(&bcast_fwd),
                                        )
                                        .await;
                                        let lines = history.ingest(&data);
                                        index_lines(
                                            output_db.clone(),
                                            &fwd_session_id,
                                            &mut line_no,
                                            lines,
                                        )
                                        .await;
                                        let should_forward = !is_detached
                                            && subscribed_check.lock().unwrap().as_deref()
                                                != Some(&fwd_session_id);
                                        if should_forward {
                                            let _ = out_tx_fwd
                                                .send(IpcMessage::TerminalOutputFrame(
                                                    TerminalOutputFrame {
                                                        session_id: fwd_session_id.clone(),
                                                        data,
                                                    },
                                                ))
                                                .await;
                                        }
                                    }
                                    if let Some(line) = history.finish() {
                                        index_lines(
                                            output_db.clone(),
                                            &fwd_session_id,
                                            &mut line_no,
                                            vec![line],
                                        )
                                        .await;
                                    }
                                    let code = *exit_rx.borrow();
                                    let _ = teardown_fwd.complete(&fwd_session_id).await;
                                    let is_torn_down = output_db
                                        .lock()
                                        .ok()
                                        .and_then(|db| {
                                            db.get_session(&fwd_session_id).ok().flatten()
                                        })
                                        .is_some_and(|s| {
                                            matches!(s.state.as_str(), "DEAD" | "ARCHIVED")
                                        });
                                    if let Some(code) = code.filter(|_| !is_torn_down) {
                                        publish_transition(
                                            output_db.clone(),
                                            &state_events,
                                            &fwd_session_id,
                                            classifier.exit(code),
                                            Some(&session_clone),
                                            Some(&journal_fwd),
                                            Some(&bcast_fwd),
                                        )
                                        .await;
                                    }
                                    let _ = out_tx_fwd
                                        .send(IpcMessage::SessionExitEvent(SessionExitEvent {
                                            session_id: fwd_session_id.clone(),
                                            exit_code: code,
                                        }))
                                        .await;

                                    let _ = journal_fwd
                                        .publish_event(
                                            "session_exit",
                                            Some(&fwd_session_id),
                                            &format!("exit_code: {code:?}"),
                                        )
                                        .await;

                                    // Evict named locks, file reservations, and Kanban tasks held by the session
                                    lock_mgr_fwd.evict_session(&fwd_session_id).await;
                                    file_res_fwd.evict_session(&fwd_session_id).await;
                                    let _ = kanban_mgr_fwd.evict_session(&fwd_session_id).await;

                                    // Snapshot terminal grid and drop active PTY session memory
                                    let screen = session_clone.capture_screen();
                                    let uncompressed_size = screen.len();
                                    if let Ok(compressed) =
                                        agmux_core::snapshot_compressor::compress_snapshot(&screen)
                                    {
                                        let db = output_db.clone();
                                        let sid = fwd_session_id.clone();
                                        let _ = tokio::task::spawn_blocking(move || {
                                            if let Ok(db) = db.lock() {
                                                let _ = db.save_snapshot(
                                                    &sid,
                                                    &compressed,
                                                    uncompressed_size,
                                                );
                                            }
                                        })
                                        .await;
                                    }
                                    let _ = pty_fwd.remove_session(&fwd_session_id).await;
                                    break;
                                }
                            }
                        });
                    }
                    Err(err) => {
                        let resp = IpcMessage::SpawnSessionResponse(SpawnSessionResponse {
                            session_id,
                            success: false,
                            error: Some(err.to_string()),
                        });
                        let _ = out_tx.send(resp).await;
                    }
                }
            }
            IpcMessage::AddReviewCommentRequest(req) => {
                let db = storage.clone();
                let result = tokio::task::spawn_blocking(move || -> Result<String, String> {
                    use std::hash::{Hash, Hasher};
                    let db = db.lock().map_err(|e| e.to_string())?;
                    let session = db
                        .get_session(&req.session_id)
                        .map_err(|e| e.to_string())?
                        .ok_or("Session not found")?;
                    let bytes =
                        agmux_core::generate_random_32_bytes().map_err(|e| e.to_string())?;
                    let id = format!(
                        "rev_{}",
                        bytes.iter().map(|b| format!("{b:02x}")).collect::<String>()
                    );
                    let text = std::fs::read_to_string(
                        std::path::Path::new(&session.cwd).join(&req.file_path),
                    )
                    .unwrap_or_default();
                    let mut hasher = std::collections::hash_map::DefaultHasher::new();
                    text.lines()
                        .nth(req.line_number.saturating_sub(1) as usize)
                        .unwrap_or("")
                        .hash(&mut hasher);
                    let comment = agmux_proto::ReviewComment {
                        id: id.clone(),
                        session_id: req.session_id,
                        file_path: req.file_path,
                        line_number: req.line_number,
                        line_hash: format!("{:016x}", hasher.finish()),
                        content: req.content,
                        status: "draft".into(),
                        created_at: std::time::SystemTime::now()
                            .duration_since(std::time::UNIX_EPOCH)
                            .unwrap_or_default()
                            .as_secs() as i64,
                    };
                    db.add_review_comment(&comment).map_err(|e| e.to_string())?;
                    Ok(id)
                })
                .await?;
                let response = match result {
                    Ok(id) => agmux_proto::AddReviewCommentResponse {
                        success: true,
                        comment_id: Some(id),
                        error: None,
                    },
                    Err(error) => agmux_proto::AddReviewCommentResponse {
                        success: false,
                        comment_id: None,
                        error: Some(error),
                    },
                };
                out_tx
                    .send(IpcMessage::AddReviewCommentResponse(response))
                    .await?;
            }
            IpcMessage::ResolveReviewCommentRequest(req) => {
                let db = storage.clone();
                let result = tokio::task::spawn_blocking(move || -> Result<bool, String> {
                    let db = db.lock().map_err(|e| e.to_string())?;
                    if let Some(session_id) = req.session_id {
                        let comments = db
                            .list_review_comments(&session_id, None)
                            .map_err(|e| e.to_string())?;
                        if !comments.iter().any(|c| c.id == req.comment_id) {
                            return Ok(false);
                        }
                    }
                    db.resolve_review_comment(&req.comment_id)
                        .map_err(|e| e.to_string())
                })
                .await?;
                let response = match result {
                    Ok(true) => agmux_proto::ResolveReviewCommentResponse {
                        success: true,
                        error: None,
                    },
                    Ok(false) => agmux_proto::ResolveReviewCommentResponse {
                        success: false,
                        error: Some("Comment not found".into()),
                    },
                    Err(error) => agmux_proto::ResolveReviewCommentResponse {
                        success: false,
                        error: Some(error),
                    },
                };
                out_tx
                    .send(IpcMessage::ResolveReviewCommentResponse(response))
                    .await?;
            }
            IpcMessage::ListReviewCommentsRequest(req) => {
                let db = storage.clone();
                let result = tokio::task::spawn_blocking(move || {
                    db.lock()
                        .map_err(|e| e.to_string())?
                        .list_review_comments(&req.session_id, req.status.as_deref())
                        .map_err(|e| e.to_string())
                })
                .await?;
                let response = match result {
                    Ok(comments) => agmux_proto::ListReviewCommentsResponse {
                        success: true,
                        comments,
                        error: None,
                    },
                    Err(error) => agmux_proto::ListReviewCommentsResponse {
                        success: false,
                        comments: vec![],
                        error: Some(error),
                    },
                };
                out_tx
                    .send(IpcMessage::ListReviewCommentsResponse(response))
                    .await?;
            }
            IpcMessage::DispatchReviewRoundRequest(req) => {
                let session = pty_manager.get_session(&req.session_id).await;
                let db = storage.clone();
                let sid = req.session_id.clone();
                let result = tokio::task::spawn_blocking(move || -> Result<_, String> {
                    let db = db.lock().map_err(|e| e.to_string())?;
                    let comments = db
                        .list_review_comments(&sid, Some("draft"))
                        .map_err(|e| e.to_string())?;
                    if comments.is_empty() {
                        return Err("No draft comments to dispatch".into());
                    }
                    let session = session.map_err(|e| e.to_string())?;
                    if session.subscribe_exit().borrow().is_some() {
                        return Err("Session is not active".into());
                    }
                    let round = db.next_review_round(&sid).map_err(|e| e.to_string())?;
                    let prompt =
                        agmux_core::prompt_flattener::flatten_review_comments(round, &comments);
                    session.submit_prompt(&prompt).map_err(|e| e.to_string())?;
                    let ids: Vec<_> = comments.iter().map(|c| c.id.clone()).collect();
                    let old = db
                        .complete_review_round(&sid, round, &ids)
                        .map_err(|e| e.to_string())?;
                    Ok((round, comments.len(), prompt, old))
                })
                .await?;
                let response = match result {
                    Ok((round_number, count, prompt, old_state)) => {
                        let _ = events.send(SessionStateChangedEvent {
                            session_id: req.session_id,
                            old_state,
                            new_state: "WORKING".into(),
                            reason: "review round dispatched".into(),
                        });
                        agmux_proto::DispatchReviewRoundResponse {
                            success: true,
                            round_number,
                            count,
                            prompt,
                            error: None,
                        }
                    }
                    Err(error) => agmux_proto::DispatchReviewRoundResponse {
                        success: false,
                        round_number: 0,
                        count: 0,
                        prompt: String::new(),
                        error: Some(error),
                    },
                };
                out_tx
                    .send(IpcMessage::DispatchReviewRoundResponse(response))
                    .await?;
            }
            IpcMessage::GetDiffRequest(req) => {
                let db = storage.clone();
                let result = tokio::task::spawn_blocking(move || -> Result<_, String> {
                    let (session, baseline) = {
                        let db = db.lock().map_err(|e| e.to_string())?;
                        let session = db
                            .get_session(&req.session_id)
                            .map_err(|e| e.to_string())?
                            .ok_or_else(|| format!("Session not found: {}", req.session_id))?;
                        let baseline = db
                            .start_git_sha(&req.session_id)
                            .map_err(|e| e.to_string())?;
                        (session, baseline)
                    };
                    let track = matches!(
                        req.scope,
                        agmux_proto::DiffScope::FullUncommitted
                            | agmux_proto::DiffScope::LastCommit
                    );
                    let files = agmux_core::diff_calculator::calculate_diff(
                        std::path::Path::new(&session.cwd),
                        req.scope,
                        baseline.as_deref(),
                        req.target.as_deref(),
                    )
                    .map_err(|e| e.to_string())?;
                    if track {
                        let db = db.lock().map_err(|e| e.to_string())?;
                        let comments = db
                            .list_review_comments(&req.session_id, Some("sent"))
                            .map_err(|e| e.to_string())?;
                        let ids =
                            agmux_core::comment_tracker::check_auto_resolutions(&comments, &files);
                        db.update_comments_status(&ids, "resolved")
                            .map_err(|e| e.to_string())?;
                    }
                    Ok(files)
                })
                .await?;
                let response = match result {
                    Ok(files) => agmux_proto::GetDiffResponse { files, error: None },
                    Err(error) => agmux_proto::GetDiffResponse {
                        files: Vec::new(),
                        error: Some(error),
                    },
                };
                out_tx.send(IpcMessage::GetDiffResponse(response)).await?;
            }
            IpcMessage::SearchNotesRequest(req) => {
                let db = storage.clone();
                let request = req.clone();
                let response = tokio::task::spawn_blocking(move || {
                    let mut response = agmux_proto::SearchNotesResponse {
                        request: req.clone(),
                        pages: Vec::new(),
                        hits: Vec::new(),
                        session_ids: Vec::new(),
                        error: None,
                    };
                    let result = (|| -> Result<(), String> {
                        let db = db.lock().map_err(|e| e.to_string())?;
                        match req {
                            agmux_proto::SearchNotesRequest::Set {
                                session_id,
                                page,
                                content,
                            } => {
                                if db
                                    .get_session(&session_id)
                                    .map_err(|e| e.to_string())?
                                    .is_none()
                                {
                                    return Err("Unknown session".into());
                                }
                                db.set_note(&session_id, page, &content)
                                    .map_err(|e| e.to_string())?;
                            }
                            agmux_proto::SearchNotesRequest::Get { session_id, page } => {
                                if let Some(content) =
                                    db.get_note(&session_id, page).map_err(|e| e.to_string())?
                                {
                                    response.pages.push((page, content));
                                }
                            }
                            agmux_proto::SearchNotesRequest::List { session_id } => {
                                response.pages =
                                    db.list_notes(&session_id).map_err(|e| e.to_string())?;
                            }
                            agmux_proto::SearchNotesRequest::Search { query } => {
                                for hit in db
                                    .search_terminal_logs(&query, 100)
                                    .map_err(|e| e.to_string())?
                                {
                                    let lines = db
                                        .transcript_context(&hit.session_id, hit.line_no)
                                        .map_err(|e| e.to_string())?;
                                    let context_match = lines
                                        .iter()
                                        .position(|(n, _)| *n == hit.line_no)
                                        .unwrap_or(0);
                                    response.hits.push(agmux_proto::GlobalSearchHit {
                                        session_id: hit.session_id,
                                        page: None,
                                        line_no: hit.line_no,
                                        snippet: hit.snippet,
                                        context: lines.into_iter().map(|(_, line)| line).collect(),
                                        context_match,
                                    });
                                }
                                for (session_id, page, content) in
                                    db.search_notes(&query, 100).map_err(|e| e.to_string())?
                                {
                                    let lines: Vec<_> =
                                        content.lines().map(str::to_owned).collect();
                                    let needle = query.to_lowercase();
                                    let index = lines
                                        .iter()
                                        .position(|line| line.to_lowercase().contains(&needle))
                                        .unwrap_or(0);
                                    response.hits.push(agmux_proto::GlobalSearchHit {
                                        session_id,
                                        page: Some(page),
                                        line_no: index as u64 + 1,
                                        snippet: lines.get(index).cloned().unwrap_or_default(),
                                        context: lines,
                                        context_match: index,
                                    });
                                }
                            }
                            agmux_proto::SearchNotesRequest::Filter { query } => {
                                let needle = query.to_lowercase();
                                let note_sessions: std::collections::HashSet<_> = db
                                    .search_notes(&query, 10000)
                                    .map_err(|e| e.to_string())?
                                    .into_iter()
                                    .map(|(id, _, _)| id)
                                    .collect();
                                for session in
                                    db.list_active_sessions().map_err(|e| e.to_string())?
                                {
                                    let branch = std::process::Command::new("git")
                                        .args(["-C", &session.cwd, "branch", "--show-current"])
                                        .output()
                                        .ok()
                                        .filter(|o| o.status.success())
                                        .map(|o| String::from_utf8_lossy(&o.stdout).to_lowercase())
                                        .unwrap_or_default();
                                    if needle.is_empty()
                                        || session.session_id.to_lowercase().contains(&needle)
                                        || session.command.to_lowercase().contains(&needle)
                                        || branch.contains(&needle)
                                        || note_sessions.contains(&session.session_id)
                                    {
                                        response.session_ids.push(session.session_id);
                                    }
                                }
                            }
                        }
                        Ok(())
                    })();
                    response.error = result.err();
                    response
                })
                .await?;
                let changed = matches!(request, agmux_proto::SearchNotesRequest::Set { .. })
                    && response.error.is_none();
                out_tx
                    .send(IpcMessage::SearchNotesResponse(response))
                    .await?;
                if changed {
                    event_broadcaster.broadcast_session_list().await;
                }
            }
            IpcMessage::SearchLogsRequest(req) => {
                let db = storage.clone();
                let matches = tokio::task::spawn_blocking(move || {
                    db.lock()
                        .map_err(|e| e.to_string())?
                        .search_terminal_logs_for_session(
                            &req.query,
                            req.session_id.as_deref(),
                            req.limit,
                        )
                        .map_err(|e| e.to_string())
                })
                .await?
                .map_err(anyhow::Error::msg)?;
                out_tx
                    .send(IpcMessage::SearchLogsResponse(SearchLogsResponse {
                        matches,
                    }))
                    .await?;
            }
            IpcMessage::SubscribeViewport { session_id } => {
                if let Some(task) = viewport_task.take() {
                    task.abort();
                }
                *subscribed_session.lock().unwrap() = Some(session_id.clone());
                let native = crate::gemini_chat::native_session(&session_id).await;
                let mut native_exit = native.as_ref().map(|session| session.subscribe_exit());
                let subscription = if let Some(native) = native {
                    Ok(native.subscribe_viewport_sync())
                } else {
                    subscription_manager.subscribe_viewport(&session_id).await
                };
                if let Ok((sync_frame, mut output)) = subscription {
                    out_tx.send(IpcMessage::ScreenSyncFrame(sync_frame)).await?;
                    let tx = out_tx.clone();
                    viewport_task = Some(tokio::spawn(async move {
                        loop {
                            let output = tokio::select! {
                                code=async {
                                    if let Some(exit)=native_exit.as_mut() {
                                        if exit.borrow().is_none(){let _=exit.changed().await;}
                                        *exit.borrow()
                                    } else { std::future::pending::<Option<i32>>().await }
                                }=>{
                                    let _=tx.send(IpcMessage::SessionExitEvent(agmux_proto::SessionExitEvent{session_id:session_id.clone(),exit_code:code})).await;
                                    break;
                                },
                                output=output.recv()=>output,
                            };
                            match output {
                                Ok(data) => {
                                    if tx
                                        .send(IpcMessage::TerminalOutputFrame(
                                            TerminalOutputFrame {
                                                session_id: session_id.clone(),
                                                data,
                                            },
                                        ))
                                        .await
                                        .is_err()
                                    {
                                        break;
                                    }
                                }
                                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => {
                                    continue;
                                }
                                Err(_) => break,
                            }
                        }
                    }));
                }
            }
            IpcMessage::RemoveSessionRequest(req) => {
                let db = storage.clone();
                let sid = req.session_id.clone();
                let force = req.force;
                let result =
                    tokio::task::spawn_blocking(move || -> Result<Option<String>, String> {
                        let session = db
                            .lock()
                            .map_err(|e| e.to_string())?
                            .get_session(&sid)
                            .map_err(|e| e.to_string())?
                            .ok_or_else(|| format!("session not found: {sid}"))?;
                        let cwd = PathBuf::from(session.cwd);
                        let stash = if cwd.join(".git").is_file() {
                            let output = std::process::Command::new("git")
                                .current_dir(&cwd)
                                .args(["rev-parse", "--path-format=absolute", "--git-common-dir"])
                                .output()
                                .map_err(|e| e.to_string())?;
                            if !output.status.success() {
                                return Err(String::from_utf8_lossy(&output.stderr)
                                    .trim()
                                    .to_owned());
                            }
                            let common = PathBuf::from(
                                String::from_utf8(output.stdout)
                                    .map_err(|e| e.to_string())?
                                    .trim(),
                            );
                            let root = common
                                .parent()
                                .ok_or_else(|| "invalid git common directory".to_string())?;
                            agmux_core::worktree_manager::safe_remove_worktree(root, &cwd, force)
                                .map_err(|e| e.to_string())?
                        } else {
                            None
                        };
                        db.lock()
                            .map_err(|e| e.to_string())?
                            .remove_session(&sid)
                            .map_err(|e| e.to_string())?;
                        Ok(stash)
                    })
                    .await;
                let response = match result {
                    Ok(Ok(stash_created)) => {
                        lock_manager.evict_session(&req.session_id).await;
                        file_reservations.evict_session(&req.session_id).await;
                        let _ = kanban_manager.evict_session(&req.session_id).await;
                        if let Some(session) = pty_manager.remove_session(&req.session_id).await {
                            session.kill();
                        }
                        agmux_proto::RemoveSessionResponse {
                            success: true,
                            stash_created,
                            error: None,
                        }
                    }
                    Ok(Err(error)) => agmux_proto::RemoveSessionResponse {
                        success: false,
                        stash_created: None,
                        error: Some(error),
                    },
                    Err(error) => agmux_proto::RemoveSessionResponse {
                        success: false,
                        stash_created: None,
                        error: Some(error.to_string()),
                    },
                };
                out_tx
                    .send(IpcMessage::RemoveSessionResponse(response))
                    .await?;
            }
            IpcMessage::ListSessionsRequest(_) => {
                let db = storage.clone();
                let mut sessions = tokio::task::spawn_blocking(move || {
                    db.lock()
                        .map_err(|e| e.to_string())?
                        .list_active_sessions()
                        .map_err(|e| e.to_string())
                })
                .await?
                .map_err(anyhow::Error::msg)?;
                let remote_sessions = ssh_federation.list_mounted_sessions().await;
                sessions.extend(remote_sessions);
                let teardowns = storage
                    .lock()
                    .map_err(|e| anyhow::anyhow!(e.to_string()))?
                    .list_teardowns()?;
                out_tx
                    .send(IpcMessage::ListSessionsResponse(ListSessionsResponse {
                        sessions,
                        teardowns,
                    }))
                    .await?;
            }
            IpcMessage::WatchEventsRequest(req) => {
                let j = journal.clone();
                let start_seq = if let Some(seq) = req.since_seq {
                    seq
                } else if let Some(ref cid) = req.cursor_id {
                    j.get_cursor(cid).await.unwrap_or(0)
                } else {
                    0
                };

                let mut current_seq = start_seq;
                let missed_events = j.read_since(start_seq).await.unwrap_or_default();
                let had_missed = !missed_events.is_empty();
                for event in missed_events {
                    current_seq = current_seq.max(event.seq);
                    let frame = IpcMessage::WatchEventFrame(agmux_proto::WatchEventFrame {
                        event: Some(event),
                        changed_while_away: had_missed,
                        is_done: false,
                    });
                    if out_tx.send(frame).await.is_err() {
                        return Ok(());
                    }
                }

                if let Some(ref cid) = req.cursor_id {
                    let _ = j.set_cursor(cid, current_seq).await;
                }

                if req.follow {
                    let mut live_rx = j.subscribe();
                    while let Ok(mut event) = live_rx.recv().await {
                        if event.seq <= current_seq {
                            continue;
                        }
                        current_seq = event.seq;
                        event.replay = false;
                        let frame = IpcMessage::WatchEventFrame(agmux_proto::WatchEventFrame {
                            event: Some(event),
                            changed_while_away: false,
                            is_done: false,
                        });
                        if out_tx.send(frame).await.is_err() {
                            break;
                        }
                        if let Some(ref cid) = req.cursor_id {
                            let _ = j.set_cursor(cid, current_seq).await;
                        }
                    }
                } else {
                    let _ = out_tx
                        .send(IpcMessage::WatchEventFrame(agmux_proto::WatchEventFrame {
                            event: None,
                            changed_while_away: had_missed,
                            is_done: true,
                        }))
                        .await;
                }
            }
            IpcMessage::AckCursorRequest(req) => {
                let j = journal.clone();
                let res = j.set_cursor(&req.cursor_id, req.seq).await;
                out_tx
                    .send(IpcMessage::AckCursorResponse(
                        agmux_proto::AckCursorResponse {
                            success: res.is_ok(),
                            error: res.err(),
                        },
                    ))
                    .await?;
            }
            IpcMessage::ReserveFilesRequest(req) => {
                let res_table = file_reservations.clone();
                match res_table.reserve(&req.session_id, &req.paths).await {
                    Ok(Ok(())) => {
                        out_tx
                            .send(IpcMessage::ReserveFilesResponse(
                                agmux_proto::ReserveFilesResponse {
                                    success: true,
                                    conflicting_path: None,
                                    holder_session_id: None,
                                    error: None,
                                },
                            ))
                            .await?;
                    }
                    Ok(Err((conflict, holder))) => {
                        out_tx
                            .send(IpcMessage::ReserveFilesResponse(
                                agmux_proto::ReserveFilesResponse {
                                    success: false,
                                    conflicting_path: Some(conflict.clone()),
                                    holder_session_id: Some(holder.clone()),
                                    error: Some(format!(
                                        "Active reservation conflict: path overlaps with '{conflict}' held by session '{holder}'"
                                    )),
                                },
                            ))
                            .await?;
                    }
                    Err(error) => {
                        out_tx
                            .send(IpcMessage::ReserveFilesResponse(
                                agmux_proto::ReserveFilesResponse {
                                    success: false,
                                    conflicting_path: None,
                                    holder_session_id: None,
                                    error: Some(error),
                                },
                            ))
                            .await?;
                    }
                }
            }
            IpcMessage::ReleaseFilesRequest(req) => {
                let res_table = file_reservations.clone();
                match res_table.release(&req.session_id, &req.paths).await {
                    Ok(released_count) => {
                        out_tx
                            .send(IpcMessage::ReleaseFilesResponse(
                                agmux_proto::ReleaseFilesResponse {
                                    success: true,
                                    released_count,
                                    error: None,
                                },
                            ))
                            .await?;
                    }
                    Err(error) => {
                        out_tx
                            .send(IpcMessage::ReleaseFilesResponse(
                                agmux_proto::ReleaseFilesResponse {
                                    success: false,
                                    released_count: 0,
                                    error: Some(error),
                                },
                            ))
                            .await?;
                    }
                }
            }
            IpcMessage::ListReservationsRequest(_) => {
                let res_table = file_reservations.clone();
                match res_table.list().await {
                    Ok(reservations) => {
                        out_tx
                            .send(IpcMessage::ListReservationsResponse(
                                agmux_proto::ListReservationsResponse {
                                    reservations,
                                    error: None,
                                },
                            ))
                            .await?;
                    }
                    Err(error) => {
                        out_tx
                            .send(IpcMessage::ListReservationsResponse(
                                agmux_proto::ListReservationsResponse {
                                    reservations: Vec::new(),
                                    error: Some(error),
                                },
                            ))
                            .await?;
                    }
                }
            }
            IpcMessage::AcquireLockRequest(req) => {
                let lock_mgr = lock_manager.clone();
                match lock_mgr
                    .acquire(&req.lock_name, &req.session_id, req.wait)
                    .await
                {
                    Ok((acquired, holder)) => {
                        out_tx
                            .send(IpcMessage::AcquireLockResponse(
                                agmux_proto::AcquireLockResponse {
                                    success: true,
                                    acquired,
                                    holder_session_id: holder,
                                    error: None,
                                },
                            ))
                            .await?;
                    }
                    Err(error) => {
                        out_tx
                            .send(IpcMessage::AcquireLockResponse(
                                agmux_proto::AcquireLockResponse {
                                    success: false,
                                    acquired: false,
                                    holder_session_id: None,
                                    error: Some(error),
                                },
                            ))
                            .await?;
                    }
                }
            }
            IpcMessage::ReleaseLockRequest(req) => {
                let lock_mgr = lock_manager.clone();
                match lock_mgr.release(&req.lock_name, &req.session_id).await {
                    Ok(released) => {
                        out_tx
                            .send(IpcMessage::ReleaseLockResponse(
                                agmux_proto::ReleaseLockResponse {
                                    success: released,
                                    error: if released {
                                        None
                                    } else {
                                        Some("Lock was not held by this session".into())
                                    },
                                },
                            ))
                            .await?;
                    }
                    Err(error) => {
                        out_tx
                            .send(IpcMessage::ReleaseLockResponse(
                                agmux_proto::ReleaseLockResponse {
                                    success: false,
                                    error: Some(error),
                                },
                            ))
                            .await?;
                    }
                }
            }
            IpcMessage::ListLocksRequest(_) => {
                let lock_mgr = lock_manager.clone();
                match lock_mgr.list_locks().await {
                    Ok(locks) => {
                        out_tx
                            .send(IpcMessage::ListLocksResponse(
                                agmux_proto::ListLocksResponse { locks, error: None },
                            ))
                            .await?;
                    }
                    Err(error) => {
                        out_tx
                            .send(IpcMessage::ListLocksResponse(
                                agmux_proto::ListLocksResponse {
                                    locks: Vec::new(),
                                    error: Some(error),
                                },
                            ))
                            .await?;
                    }
                }
            }
            IpcMessage::SendMessageRequest(req) => {
                if let Some(result) =
                    managed_prompt(&storage, &req.target_session_id, &req.message).await
                {
                    out_tx
                        .send(IpcMessage::SendMessageResponse(
                            agmux_proto::SendMessageResponse {
                                success: result.is_ok(),
                                queued: false,
                                error: result.err(),
                            },
                        ))
                        .await?;
                    continue;
                }
                let session = pty_manager.get_session(&req.target_session_id).await;
                let db = storage.clone();
                let result = tokio::task::spawn_blocking(move || -> Result<bool, String> {
                    let db = db.lock().map_err(|e| e.to_string())?;
                    let info = db
                        .get_session(&req.target_session_id)
                        .map_err(|e| e.to_string())?
                        .ok_or_else(|| format!("Session not found: {}", req.target_session_id))?;
                    if matches!(info.state.as_str(), "WAITING" | "WAITING_INPUT") {
                        db.enqueue_message(&req.target_session_id, &req.message)
                            .map_err(|e| e.to_string())?;
                        Ok(true)
                    } else {
                        let session = session.map_err(|e| e.to_string())?;
                        if session.subscribe_exit().borrow().is_some() {
                            return Err("Session is not active".into());
                        }
                        session
                            .submit_prompt(&req.message)
                            .map_err(|e| e.to_string())?;
                        Ok(false)
                    }
                })
                .await;
                let response = match result {
                    Ok(Ok(queued)) => agmux_proto::SendMessageResponse {
                        success: true,
                        queued,
                        error: None,
                    },
                    Ok(Err(error)) => agmux_proto::SendMessageResponse {
                        success: false,
                        queued: false,
                        error: Some(error),
                    },
                    Err(error) => agmux_proto::SendMessageResponse {
                        success: false,
                        queued: false,
                        error: Some(error.to_string()),
                    },
                };
                out_tx
                    .send(IpcMessage::SendMessageResponse(response))
                    .await?;
            }
            IpcMessage::TerminalInputFrame(input) => {
                if crate::ssh_federation::SshFederationManager::parse_mounted_id(&input.session_id)
                    .is_some()
                {
                    if let Err(e) = ssh_federation
                        .forward_input(&input.session_id, &input.data)
                        .await
                    {
                        tracing::warn!(
                            "Failed forwarding input to remote session {}: {e}",
                            input.session_id
                        );
                    }
                } else if crate::gemini_chat::native_input(&input.session_id, &input.data).await? {
                    // Native Gemini input belongs to the same persisted conversation.
                } else if let Ok(session) = pty_manager.get_session(&input.session_id).await {
                    if let Err(e) = session.send_input(&input.data) {
                        tracing::warn!(
                            "Failed delivering input to session {}: {e}",
                            input.session_id
                        );
                    }
                }
            }
            IpcMessage::FreezeSessionRequest(req) => {
                let managed = {
                    storage
                        .lock()
                        .map_err(|e| anyhow::anyhow!(e.to_string()))?
                        .conversation(&req.session_id)?
                        .is_some()
                };
                if managed {
                    let result = crate::gemini_chat::end(storage.clone(), &req.session_id).await;
                    if let Err(e) = &result {
                        tracing::error!(%e,"Ending managed session failed");
                    }
                    out_tx
                        .send(IpcMessage::FreezeSessionResponse(
                            agmux_proto::FreezeSessionResponse {
                                success: result.is_ok(),
                                compressed_size: 0,
                            },
                        ))
                        .await?;
                    continue;
                }
                crate::gemini_chat::close(&req.session_id).await;
                let session_opt = pty_manager.remove_session(&req.session_id).await;
                let (success, compressed_size) = if let Some(session) = session_opt {
                    session.kill();
                    let screen = session.capture_screen();
                    let uncompressed_size = screen.len();
                    match agmux_core::snapshot_compressor::compress_snapshot(&screen) {
                        Ok(compressed) => {
                            let size = compressed.len();
                            let db = storage.clone();
                            let sid = req.session_id.clone();
                            let save_res = tokio::task::spawn_blocking(move || {
                                let db = db.lock().map_err(|e| e.to_string())?;
                                db.save_snapshot(&sid, &compressed, uncompressed_size)
                                    .map_err(|e| e.to_string())?;
                                db.update_session_state(&sid, "DEAD")
                                    .map_err(|e| e.to_string())?;
                                Ok::<(), String>(())
                            })
                            .await;
                            match save_res {
                                Ok(Ok(())) => (true, size),
                                _ => (false, 0),
                            }
                        }
                        Err(_) => (false, 0),
                    }
                } else {
                    let db = storage.clone();
                    let sid = req.session_id.clone();
                    let snap_res = tokio::task::spawn_blocking(move || {
                        let db = db.lock().map_err(|e| e.to_string())?;
                        db.load_snapshot(&sid).map_err(|e| e.to_string())
                    })
                    .await;
                    match snap_res {
                        Ok(Ok(Some(snap))) => (true, snap.len()),
                        _ => (false, 0),
                    }
                };
                out_tx
                    .send(IpcMessage::FreezeSessionResponse(
                        agmux_proto::FreezeSessionResponse {
                            success,
                            compressed_size,
                        },
                    ))
                    .await?;
            }
            IpcMessage::ResumeSessionRequest(req) => {
                let db = storage.clone();
                let sid = req.session_id.clone();
                let snap_res = tokio::task::spawn_blocking(move || {
                    let db = db.lock().map_err(|e| e.to_string())?;
                    db.load_snapshot(&sid).map_err(|e| e.to_string())
                })
                .await;
                let (success, screen_data) = match snap_res {
                    Ok(Ok(Some(blob))) => {
                        match agmux_core::snapshot_compressor::decompress_snapshot(&blob) {
                            Ok(data) => (true, data),
                            Err(_) => (false, Vec::new()),
                        }
                    }
                    _ => (false, Vec::new()),
                };
                out_tx
                    .send(IpcMessage::ResumeSessionResponse(
                        agmux_proto::ResumeSessionResponse {
                            success,
                            screen_data,
                        },
                    ))
                    .await?;
            }
            IpcMessage::AcknowledgeSessionRequest(req) => {
                let db = storage.clone();
                let sid = req.session_id.clone();
                let active = pty_manager.get_session(&sid).await.ok();
                let result = tokio::task::spawn_blocking(move || -> Result<(), String> {
                    let db = db.lock().map_err(|e| e.to_string())?;
                    db.update_session_state(&sid, "IDLE")
                        .map_err(|e| e.to_string())?;
                    if let Some(session) = active.filter(|s| s.subscribe_exit().borrow().is_none())
                    {
                        for (id, message) in db.pending_messages(&sid).map_err(|e| e.to_string())? {
                            session.submit_prompt(&message).map_err(|e| e.to_string())?;
                            db.mark_message_delivered(id).map_err(|e| e.to_string())?;
                        }
                    }
                    Ok(())
                })
                .await;
                let response = match result {
                    Ok(Ok(())) => {
                        let _ = events.send(SessionStateChangedEvent {
                            session_id: req.session_id.clone(),
                            old_state: "FINISHED".into(),
                            new_state: "IDLE".into(),
                            reason: "user acknowledged".into(),
                        });
                        agmux_proto::AcknowledgeSessionResponse {
                            success: true,
                            error: None,
                        }
                    }
                    Ok(Err(e)) => agmux_proto::AcknowledgeSessionResponse {
                        success: false,
                        error: Some(e),
                    },
                    Err(e) => agmux_proto::AcknowledgeSessionResponse {
                        success: false,
                        error: Some(e.to_string()),
                    },
                };
                if response.success {
                    if let Err(error) = teardown_scheduler.complete(&req.session_id).await {
                        tracing::error!("Teardown failed: {error}");
                    }
                }
                out_tx
                    .send(IpcMessage::AcknowledgeSessionResponse(response))
                    .await?;
            }
            IpcMessage::CreateTaskRequest(req) => {
                let km = kanban_manager.clone();
                let resp = match km
                    .create_task(&req.task_id, &req.title, &req.description)
                    .await
                {
                    Ok(task) => agmux_proto::CreateTaskResponse {
                        success: true,
                        task: Some(task),
                        error: None,
                    },
                    Err(error) => agmux_proto::CreateTaskResponse {
                        success: false,
                        task: None,
                        error: Some(error),
                    },
                };
                out_tx.send(IpcMessage::CreateTaskResponse(resp)).await?;
            }
            IpcMessage::ListTasksRequest(req) => {
                let km = kanban_manager.clone();
                let resp = match km.list_tasks(req.status.as_deref()).await {
                    Ok(tasks) => agmux_proto::ListTasksResponse { tasks, error: None },
                    Err(error) => agmux_proto::ListTasksResponse {
                        tasks: Vec::new(),
                        error: Some(error),
                    },
                };
                out_tx.send(IpcMessage::ListTasksResponse(resp)).await?;
            }
            IpcMessage::ClaimTaskRequest(req) => {
                let km = kanban_manager.clone();
                let resp = match km.claim_task(&req.task_id, &req.session_id).await {
                    Ok(task) => agmux_proto::ClaimTaskResponse {
                        success: true,
                        task: Some(task),
                        error: None,
                    },
                    Err(error) => agmux_proto::ClaimTaskResponse {
                        success: false,
                        task: None,
                        error: Some(error),
                    },
                };
                out_tx.send(IpcMessage::ClaimTaskResponse(resp)).await?;
            }
            IpcMessage::FinishTaskRequest(req) => {
                let km = kanban_manager.clone();
                let resp = match km.finish_task(&req.task_id, &req.session_id).await {
                    Ok(task) => agmux_proto::FinishTaskResponse {
                        success: true,
                        task: Some(task),
                        error: None,
                    },
                    Err(error) => agmux_proto::FinishTaskResponse {
                        success: false,
                        task: None,
                        error: Some(error),
                    },
                };
                out_tx.send(IpcMessage::FinishTaskResponse(resp)).await?;
            }
            IpcMessage::ReleaseTaskRequest(req) => {
                let km = kanban_manager.clone();
                let resp = match km.release_task(&req.task_id, &req.session_id).await {
                    Ok(task) => agmux_proto::ReleaseTaskResponse {
                        success: true,
                        task: Some(task),
                        error: None,
                    },
                    Err(error) => agmux_proto::ReleaseTaskResponse {
                        success: false,
                        task: None,
                        error: Some(error),
                    },
                };
                out_tx.send(IpcMessage::ReleaseTaskResponse(resp)).await?;
            }
            IpcMessage::PostReportRequest(req) => {
                let km = kanban_manager.clone();
                let resp = match km
                    .post_report(
                        &req.task_id,
                        &req.session_id,
                        &req.summary,
                        req.pr_url.as_deref(),
                        req.commit_sha.as_deref(),
                    )
                    .await
                {
                    Ok(report) => agmux_proto::PostReportResponse {
                        success: true,
                        report: Some(report),
                        error: None,
                    },
                    Err(error) => agmux_proto::PostReportResponse {
                        success: false,
                        report: None,
                        error: Some(error),
                    },
                };
                out_tx.send(IpcMessage::PostReportResponse(resp)).await?;
            }
            IpcMessage::ViewReportRequest(req) => {
                let km = kanban_manager.clone();
                let resp = match km.view_reports(&req.task_id).await {
                    Ok(reports) => agmux_proto::ViewReportResponse {
                        success: true,
                        reports,
                        error: None,
                    },
                    Err(error) => agmux_proto::ViewReportResponse {
                        success: false,
                        reports: Vec::new(),
                        error: Some(error),
                    },
                };
                out_tx.send(IpcMessage::ViewReportResponse(resp)).await?;
            }
            IpcMessage::ResizeSessionRequest(req) => {
                let resizer = pty_resizer.clone();
                let result = if let Some(native) =
                    crate::gemini_chat::native_session(&req.session_id).await
                {
                    native
                        .resize(req.cols, req.rows)
                        .map_err(anyhow::Error::from)
                } else {
                    resizer
                        .resize_session(&req.session_id, req.cols, req.rows)
                        .await
                        .map_err(anyhow::Error::from)
                };
                let resp = match result {
                    Ok(()) => agmux_proto::ResizeSessionResponse {
                        success: true,
                        cols: req.cols,
                        rows: req.rows,
                        error: None,
                    },
                    Err(e) => agmux_proto::ResizeSessionResponse {
                        success: false,
                        cols: 0,
                        rows: 0,
                        error: Some(e.to_string()),
                    },
                };
                out_tx.send(IpcMessage::ResizeSessionResponse(resp)).await?;
            }
            IpcMessage::SpawnSubtabRequest(req) => {
                let spawner = subtab_spawner.clone();
                let db = storage.clone();
                let cfg_res = spawner
                    .build_config(&req.parent_session_id, &req.subtab_id, req.shell, None)
                    .await;
                let resp = match cfg_res {
                    Ok(cfg) => match spawner.spawn_subtab(&cfg).await {
                        Ok(session) => {
                            let cwd_str = cfg.cwd.display().to_string();
                            let info = SessionInfo {
                                session_id: req.subtab_id.clone(),
                                pid: session.pid,
                                cwd: cwd_str.clone(),
                                command: cfg.shell.clone(),
                                created_at: std::time::SystemTime::now()
                                    .duration_since(std::time::UNIX_EPOCH)
                                    .unwrap_or_default()
                                    .as_secs(),
                                state: "IDLE".into(),
                            };
                            let _ = tokio::task::spawn_blocking(move || {
                                if let Ok(db) = db.lock() {
                                    let _ = db.insert_session(&info);
                                }
                            })
                            .await;
                            let subtab_scheduler = teardown_scheduler.clone();
                            let subtab_id = req.subtab_id.clone();
                            let mut subtab_exit = session.subscribe_exit();
                            tokio::spawn(async move {
                                if subtab_exit.borrow().is_none() {
                                    let _ = subtab_exit.changed().await;
                                }
                                if let Err(error) = subtab_scheduler.complete(&subtab_id).await {
                                    tracing::error!("Subtab teardown failed: {error}");
                                }
                            });
                            event_broadcaster.broadcast_session_list().await;
                            agmux_proto::SpawnSubtabResponse {
                                success: true,
                                subtab_id: req.subtab_id,
                                cwd: cwd_str,
                                port: cfg.port,
                                error: None,
                            }
                        }
                        Err(e) => agmux_proto::SpawnSubtabResponse {
                            success: false,
                            subtab_id: req.subtab_id,
                            cwd: String::new(),
                            port: 0,
                            error: Some(e.to_string()),
                        },
                    },
                    Err(e) => agmux_proto::SpawnSubtabResponse {
                        success: false,
                        subtab_id: req.subtab_id,
                        cwd: String::new(),
                        port: 0,
                        error: Some(e.to_string()),
                    },
                };
                out_tx.send(IpcMessage::SpawnSubtabResponse(resp)).await?;
            }
            IpcMessage::InternalHookRequest(req) => {
                let hl = hook_listener.clone();
                let resp = match hl.handle_hook(&req.session_id, &req.event).await {
                    Ok(new_state) => agmux_proto::InternalHookResponse {
                        success: true,
                        new_state,
                        error: None,
                    },
                    Err(error) => agmux_proto::InternalHookResponse {
                        success: false,
                        new_state: String::new(),
                        error: Some(error),
                    },
                };
                if resp.success
                    && matches!(resp.new_state.as_str(), "COMPLETED" | "IDLE" | "FINISHED")
                {
                    if let Err(error) = teardown_scheduler.complete(&req.session_id).await {
                        tracing::error!("Teardown failed: {error}");
                    }
                }
                out_tx.send(IpcMessage::InternalHookResponse(resp)).await?;
            }
            IpcMessage::ConnectRemoteRequest(req) => {
                let fed = ssh_federation.clone();
                let resp = match fed
                    .connect_remote(
                        &req.target,
                        req.remote_socket.map(PathBuf::from),
                        req.remote_token,
                    )
                    .await
                {
                    Ok((host_label, mounted_sessions)) => agmux_proto::ConnectRemoteResponse {
                        success: true,
                        host_label,
                        mounted_sessions,
                        error: None,
                    },
                    Err(error) => agmux_proto::ConnectRemoteResponse {
                        success: false,
                        host_label: String::new(),
                        mounted_sessions: Vec::new(),
                        error: Some(error),
                    },
                };
                out_tx.send(IpcMessage::ConnectRemoteResponse(resp)).await?;
            }
            IpcMessage::ListRemotesRequest(_) => {
                let remotes = ssh_federation.list_remotes();
                out_tx
                    .send(IpcMessage::ListRemotesResponse(
                        agmux_proto::ListRemotesResponse { remotes },
                    ))
                    .await?;
            }
            IpcMessage::ShutdownDaemonRequest(_) => {
                let terminated_sessions = shutdown_coordinator.shutdown_all().await;
                let _ = out_tx
                    .send(IpcMessage::ShutdownDaemonResponse(
                        agmux_proto::ShutdownDaemonResponse {
                            success: true,
                            terminated_sessions,
                        },
                    ))
                    .await;
                tokio::time::sleep(std::time::Duration::from_millis(30)).await;
                let _ = shutdown_tx.send(true);
                break;
            }
            IpcMessage::CompanionResponse(_)
            | IpcMessage::ConnectRemoteResponse(_)
            | IpcMessage::WorkspaceResponse(_)
            | IpcMessage::ListRemotesResponse(_)
            | IpcMessage::ReadSessionResponse(_)
            | IpcMessage::TeardownResponse(_)
            | IpcMessage::ResizeSessionResponse(_)
            | IpcMessage::SpawnSubtabResponse(_)
            | IpcMessage::InternalHookResponse(_)
            | IpcMessage::ShutdownDaemonResponse(_)
            | IpcMessage::ScreenSyncFrame(_)
            | IpcMessage::NotificationEvent(_)
            | IpcMessage::SessionListUpdate(_)
            | IpcMessage::CreateTaskResponse(_)
            | IpcMessage::ListTasksResponse(_)
            | IpcMessage::ClaimTaskResponse(_)
            | IpcMessage::FinishTaskResponse(_)
            | IpcMessage::ReleaseTaskResponse(_)
            | IpcMessage::PostReportResponse(_)
            | IpcMessage::ViewReportResponse(_)
            | IpcMessage::WatchEventFrame(_)
            | IpcMessage::AckCursorResponse(_)
            | IpcMessage::ReserveFilesResponse(_)
            | IpcMessage::ReleaseFilesResponse(_)
            | IpcMessage::ListReservationsResponse(_)
            | IpcMessage::AcquireLockResponse(_)
            | IpcMessage::ReleaseLockResponse(_)
            | IpcMessage::ListLocksResponse(_)
            | IpcMessage::SendMessageResponse(_)
            | IpcMessage::ResolveReviewCommentResponse(_)
            | IpcMessage::AcknowledgeSessionResponse(_)
            | IpcMessage::AddReviewCommentResponse(_)
            | IpcMessage::ListReviewCommentsResponse(_)
            | IpcMessage::DispatchReviewRoundResponse(_)
            | IpcMessage::HandshakeRequest(_)
            | IpcMessage::GetDiffResponse(_)
            | IpcMessage::RemoveSessionResponse(_)
            | IpcMessage::FreezeSessionResponse(_)
            | IpcMessage::ResumeSessionResponse(_)
            | IpcMessage::HandshakeResponse(_)
            | IpcMessage::SearchNotesResponse(_)
            | IpcMessage::SearchLogsResponse(_)
            | IpcMessage::ListSessionsResponse(_)
            | IpcMessage::SpawnSessionResponse(_)
            | IpcMessage::TerminalOutputFrame(_)
            | IpcMessage::SessionExitEvent(_)
            | IpcMessage::SessionStateChangedEvent(_) => {
                tracing::debug!("Unhandled message in active session state: {message:?}");
            }
        }
    }

    if let Some(task) = viewport_task {
        task.abort();
    }
    event_task.abort();
    writer_task.abort();
    Ok(())
}

async fn index_lines(
    db: Arc<std::sync::Mutex<agmux_core::storage_engine::StorageEngine>>,
    id: &str,
    line_no: &mut u64,
    lines: Vec<String>,
) {
    if lines.is_empty() {
        return;
    }
    let first = *line_no + 1;
    *line_no += lines.len() as u64;
    let id = id.to_owned();
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    let result = tokio::task::spawn_blocking(move || {
        db.lock()
            .map_err(|e| e.to_string())?
            .index_terminal_lines(&id, first, timestamp, &lines)
            .map_err(|e| e.to_string())
    })
    .await;
    if !matches!(result, Ok(Ok(()))) {
        tracing::error!("Failed to index terminal lines: {result:?}");
    }
}

async fn publish_transition(
    db: Arc<std::sync::Mutex<agmux_core::storage_engine::StorageEngine>>,
    events: &tokio::sync::broadcast::Sender<SessionStateChangedEvent>,
    id: &str,
    transition: Option<(
        agmux_core::state_machine::SessionState,
        agmux_core::state_machine::SessionState,
        &'static str,
    )>,
    session: Option<&Arc<agmux_core::pty_manager::ActivePtySession>>,
    journal: Option<&crate::journal_watcher::JournalWatcher>,
    broadcaster: Option<&crate::event_broadcaster::EventBroadcaster>,
) {
    if let Some((old, new, reason)) = transition {
        let session_id = id.to_owned();
        let db_id = session_id.clone();
        let db_update = db.clone();
        let result = tokio::task::spawn_blocking(move || {
            db_update
                .lock()
                .map_err(|e| e.to_string())?
                .update_live_session_state(&db_id, new.as_str())
                .map_err(|e| e.to_string())
        })
        .await;
        if matches!(result, Ok(Ok(true))) {
            let _ = events.send(SessionStateChangedEvent {
                session_id: session_id.clone(),
                old_state: old.as_str().into(),
                new_state: new.as_str().into(),
                reason: reason.into(),
            });

            if let Some(b) = broadcaster {
                b.notify_transition(
                    &session_id,
                    old.as_str(),
                    new.as_str(),
                    reason,
                    &session
                        .map(|s| {
                            s.subscribe_viewport_sync()
                                .0
                                .lines
                                .join("\n")
                                .trim_end()
                                .to_owned()
                        })
                        .unwrap_or_default(),
                )
                .await;
            }

            if let Some(j) = journal {
                let _ = j
                    .publish_event(
                        "session_state_changed",
                        Some(&session_id),
                        &format!("{} -> {} ({})", old.as_str(), new.as_str(), reason),
                    )
                    .await;
            }

            // Auto-flush queued messages when prompt is resolved or session transitions out of WaitingInput
            if (old == agmux_core::state_machine::SessionState::WaitingInput
                || new == agmux_core::state_machine::SessionState::Idle)
                && new != agmux_core::state_machine::SessionState::WaitingInput
            {
                if let Some(active_session) = session {
                    let db_clone = db.clone();
                    let sid = session_id.clone();
                    if let Ok(Ok(msgs)) = tokio::task::spawn_blocking(move || {
                        db_clone
                            .lock()
                            .map_err(|e| e.to_string())?
                            .pop_pending_messages(&sid)
                            .map_err(|e| e.to_string())
                    })
                    .await
                    {
                        for (_id, msg) in msgs {
                            let _ = active_session.submit_prompt(&msg);
                        }
                    }
                }
            }
        } else {
            tracing::error!("Failed to persist state transition: {result:?}");
        }
    }
}

/// Route legacy prompt entry points into the same managed provider context.
async fn managed_prompt(
    storage: &Arc<std::sync::Mutex<agmux_core::storage_engine::StorageEngine>>,
    id: &str,
    message: &str,
) -> Option<Result<(), String>> {
    let session = {
        let db = match storage.lock() {
            Ok(db) => db,
            Err(e) => return Some(Err(e.to_string())),
        };
        match db.conversation(id) {
            Ok(None) => return None,
            Err(e) => return Some(Err(e.to_string())),
            Ok(Some(_)) => {}
        }
        match db.get_session(id) {
            Ok(Some(s)) => s,
            Ok(None) => return Some(Err("Session not found".into())),
            Err(e) => return Some(Err(e.to_string())),
        }
    };
    let payload=serde_json::json!({"message":message,"request_id":agmux_core::session_tracker::SessionTracker::mint_uuid_v4()}).to_string();
    Some(
        crate::gemini_chat::request(
            id,
            std::path::Path::new(&session.cwd),
            "send",
            &payload,
            storage.clone(),
            matches!(
                session.state.as_str(),
                "DEAD" | "ARCHIVED" | "COMPLETED" | "EXITED"
            ),
        )
        .await
        .map(|_| ())
        .map_err(|e| e.to_string()),
    )
}
