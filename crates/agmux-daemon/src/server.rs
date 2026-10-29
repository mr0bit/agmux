//! IPC server listening on local domain sockets and routing client requests.

use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use futures_util::{SinkExt, StreamExt};
use tokio::net::{UnixListener, UnixStream};

use agmux_core::pty_manager::PtyManager;
use agmux_proto::{
    HandshakeResponse, IpcMessage, ListSessionsResponse, SearchLogsResponse, SessionExitEvent,
    SessionInfo, SessionStateChangedEvent, SpawnSessionResponse, TerminalOutputFrame, new_framed,
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
        if self.socket_path.exists() {
            // Check if another daemon is already responding
            if UnixStream::connect(&self.socket_path).await.is_ok() {
                anyhow::bail!(
                    "Active daemon already running on socket {}",
                    self.socket_path.display()
                );
            }
            // Remove stale socket
            let _ = fs::remove_file(&self.socket_path);
        }

        if let Some(parent) = self.socket_path.parent() {
            fs::create_dir_all(parent)
                .with_context(|| format!("Failed to create socket dir {}", parent.display()))?;
        }

        let storage = Arc::new(std::sync::Mutex::new(
            agmux_core::storage_engine::StorageEngine::open(
                &agmux_core::storage_engine::default_database_path(),
            )
            .context("Failed to open session database")?,
        ));

        let listener = UnixListener::bind(&self.socket_path)
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

        loop {
            match listener.accept().await {
                Ok((stream, _addr)) => {
                    let pty_mgr = pty_manager.clone();
                    let expected_token = auth_token.clone();
                    let storage = storage.clone();
                    let events = events.clone();
                    tokio::spawn(async move {
                        if let Err(e) =
                            handle_client(stream, pty_mgr, expected_token, storage, events).await
                        {
                            tracing::debug!("Client session ended: {e:#}");
                        }
                    });
                }
                Err(e) => {
                    tracing::error!("Error accepting IPC client connection: {e}");
                }
            }
        }
    }
}

/// Process an individual client connection through handshake and session streaming.
async fn handle_client(
    stream: UnixStream,
    pty_manager: PtyManager,
    expected_token: Arc<String>,
    storage: Arc<std::sync::Mutex<agmux_core::storage_engine::StorageEngine>>,
    events: tokio::sync::broadcast::Sender<SessionStateChangedEvent>,
) -> Result<()> {
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
            let is_valid = agmux_core::validate_token(&expected_token, &req.token);
            if !is_valid {
                let response = IpcMessage::HandshakeResponse(HandshakeResponse {
                    success: false,
                    daemon_version: env!("CARGO_PKG_VERSION").to_string(),
                    error: Some("Unauthorized: invalid authentication token".to_string()),
                });
                let _ = sink.send(response).await;
                anyhow::bail!("Client authentication failed: invalid token");
            }

            let response = IpcMessage::HandshakeResponse(HandshakeResponse {
                success: true,
                daemon_version: env!("CARGO_PKG_VERSION").to_string(),
                error: None,
            });
            sink.send(response)
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
    let event_out = out_tx.clone();
    let event_task = tokio::spawn(async move {
        loop {
            match event_rx.recv().await {
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
    });

    let mut viewport_task: Option<tokio::task::JoinHandle<()>> = None;
    while let Some(frame_res) = reader.next().await {
        let message = match frame_res {
            Ok(msg) => msg,
            Err(e) => {
                tracing::warn!("Client frame decode error: {e}");
                break;
            }
        };

        match message {
            IpcMessage::SpawnSessionRequest(req) => {
                let session_id = req.session_id.clone();
                let cwd_text = req.cwd.clone().unwrap_or_else(|| {
                    std::env::current_dir()
                        .unwrap_or_default()
                        .display()
                        .to_string()
                });
                let command = std::iter::once(req.command.as_str())
                    .chain(req.args.iter().map(String::as_str))
                    .collect::<Vec<_>>()
                    .join(" ");
                let cwd = req.cwd.map(PathBuf::from);

                match pty_manager
                    .spawn(session_id.clone(), req.command, req.args, cwd)
                    .await
                {
                    Ok(session) => {
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
                            db.lock()
                                .map_err(|e| e.to_string())?
                                .insert_session(&info)
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
                        tokio::spawn(async move {
                            let mut output_rx = session.subscribe_output();
                            let mut exit_rx = session.subscribe_exit();
                            let mut input_rx = session.subscribe_input();
                            let threshold = std::env::var("AGMUX_STUCK_TIMEOUT_MS")
                                .ok()
                                .and_then(|v| v.parse::<u64>().ok())
                                .filter(|v| *v > 0)
                                .unwrap_or(120_000);
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
                                                    publish_transition(output_db.clone(), &state_events, &fwd_session_id, classifier.output(&data)).await;
                                                    let lines = history.ingest(&data);
                                                    index_lines(output_db.clone(), &fwd_session_id, &mut line_no, lines).await;
                                                    let _ = out_tx_fwd.send(IpcMessage::TerminalOutputFrame(TerminalOutputFrame { session_id: fwd_session_id.clone(), data })).await;
                                                    false
                                                }
                                                Err(tokio::sync::broadcast::error::RecvError::Lagged(n)) => { tracing::warn!("Lost {n} terminal output frames"); false }
                                                Err(_) => true,
                                            }
                                        }
                                        _ = input_rx.recv() => {
                                            publish_transition(output_db.clone(), &state_events, &fwd_session_id, classifier.input()).await;
                                            false
                                        }
                                        _ = tokio::time::sleep_until(classifier.deadline().into()), if classifier.state() == agmux_core::state_machine::SessionState::Running => {
                                            publish_transition(output_db.clone(), &state_events, &fwd_session_id, classifier.timeout()).await;
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
                                        let _ = out_tx_fwd
                                            .send(IpcMessage::TerminalOutputFrame(
                                                TerminalOutputFrame {
                                                    session_id: fwd_session_id.clone(),
                                                    data,
                                                },
                                            ))
                                            .await;
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
                                    if let Some(code) = code {
                                        publish_transition(
                                            output_db.clone(),
                                            &state_events,
                                            &fwd_session_id,
                                            classifier.exit(code),
                                        )
                                        .await;
                                    }
                                    let _ = out_tx_fwd
                                        .send(IpcMessage::SessionExitEvent(SessionExitEvent {
                                            session_id: fwd_session_id.clone(),
                                            exit_code: code,
                                        }))
                                        .await;

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
                if let Ok(session) = pty_manager.get_session(&session_id).await {
                    let (snapshot, mut output) = session.subscribe_viewport();
                    out_tx
                        .send(IpcMessage::TerminalOutputFrame(TerminalOutputFrame {
                            session_id: session_id.clone(),
                            data: snapshot,
                        }))
                        .await?;
                    let tx = out_tx.clone();
                    viewport_task = Some(tokio::spawn(async move {
                        loop {
                            match output.recv().await {
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
            IpcMessage::ListSessionsRequest(_) => {
                let db = storage.clone();
                let sessions = tokio::task::spawn_blocking(move || {
                    db.lock()
                        .map_err(|e| e.to_string())?
                        .list_active_sessions()
                        .map_err(|e| e.to_string())
                })
                .await?
                .map_err(anyhow::Error::msg)?;
                out_tx
                    .send(IpcMessage::ListSessionsResponse(ListSessionsResponse {
                        sessions,
                    }))
                    .await?;
            }
            IpcMessage::TerminalInputFrame(input) => {
                if let Ok(session) = pty_manager.get_session(&input.session_id).await {
                    if let Err(e) = session.send_input(&input.data) {
                        tracing::warn!(
                            "Failed delivering input to session {}: {e}",
                            input.session_id
                        );
                    }
                }
            }
            IpcMessage::FreezeSessionRequest(req) => {
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
            IpcMessage::HandshakeRequest(_)
            | IpcMessage::FreezeSessionResponse(_)
            | IpcMessage::ResumeSessionResponse(_)
            | IpcMessage::HandshakeResponse(_)
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
) {
    if let Some((old, new, reason)) = transition {
        let session_id = id.to_owned();
        let db_id = session_id.clone();
        let result = tokio::task::spawn_blocking(move || {
            db.lock()
                .map_err(|e| e.to_string())?
                .update_session_state(&db_id, new.as_str())
                .map_err(|e| e.to_string())
        })
        .await;
        if matches!(result, Ok(Ok(()))) {
            let _ = events.send(SessionStateChangedEvent {
                session_id,
                old_state: old.as_str().into(),
                new_state: new.as_str().into(),
                reason: reason.into(),
            });
        } else {
            tracing::error!("Failed to persist state transition: {result:?}");
        }
    }
}
