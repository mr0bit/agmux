//! IPC server listening on local domain sockets and routing client requests.

use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use futures_util::{SinkExt, StreamExt};

use agmux_core::pty_manager::PtyManager;
use agmux_proto::{
    HandshakeResponse, IpcListener, IpcMessage, IpcServerStream, ListSessionsResponse,
    SearchLogsResponse, SessionExitEvent, SessionInfo, SessionStateChangedEvent,
    SpawnSessionResponse, TerminalOutputFrame, connect_ipc, new_framed,
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

        #[cfg(unix)]
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

        let ctx = DaemonContext {
            pty_manager,
            expected_token: auth_token,
            storage,
            events,
            lock_manager,
            file_reservations,
            journal,
        };

        loop {
            match listener.accept_stream().await {
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
}

/// Process an individual client connection through handshake and session streaming.
async fn handle_client(stream: IpcServerStream, ctx: DaemonContext) -> Result<()> {
    let DaemonContext {
        pty_manager,
        expected_token,
        storage,
        events,
        lock_manager,
        file_reservations,
        journal,
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
                match pty_manager
                    .spawn(session_id.clone(), req.command, req.args, cwd, port)
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
                            let db = db.lock().map_err(|e| e.to_string())?;
                            db.insert_session(&info).map_err(|e| e.to_string())?;
                            db.set_start_git_sha(&info.session_id, start_sha.as_deref())
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
                        let journal_fwd = journal.clone();
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
                                                    publish_transition(output_db.clone(), &state_events, &fwd_session_id, classifier.output(&data), Some(&session_clone), Some(&journal_fwd)).await;
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
                                            publish_transition(output_db.clone(), &state_events, &fwd_session_id, classifier.input(), Some(&session_clone), Some(&journal_fwd)).await;
                                            false
                                        }
                                        _ = tokio::time::sleep_until(classifier.deadline().into()), if classifier.state() == agmux_core::state_machine::SessionState::Running => {
                                            publish_transition(output_db.clone(), &state_events, &fwd_session_id, classifier.timeout(), Some(&session_clone), Some(&journal_fwd)).await;
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
                                            Some(&session_clone),
                                            Some(&journal_fwd),
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

                                    // Evict named locks and file reservations held by the session
                                    lock_mgr_fwd.evict_session(&fwd_session_id).await;
                                    file_res_fwd.evict_session(&fwd_session_id).await;

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
                    session
                        .send_input(format!("{prompt}\n").as_bytes())
                        .map_err(|e| e.to_string())?;
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
                            .send_input(format!("{}\n", req.message).as_bytes())
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
                            session
                                .send_input(format!("{message}\n").as_bytes())
                                .map_err(|e| e.to_string())?;
                            db.mark_message_delivered(id).map_err(|e| e.to_string())?;
                        }
                    }
                    Ok(())
                })
                .await;
                let response = match result {
                    Ok(Ok(())) => {
                        let _ = events.send(SessionStateChangedEvent {
                            session_id: req.session_id,
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
                out_tx
                    .send(IpcMessage::AcknowledgeSessionResponse(response))
                    .await?;
            }
            IpcMessage::WatchEventFrame(_)
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
) {
    if let Some((old, new, reason)) = transition {
        let session_id = id.to_owned();
        let db_id = session_id.clone();
        let db_update = db.clone();
        let result = tokio::task::spawn_blocking(move || {
            db_update
                .lock()
                .map_err(|e| e.to_string())?
                .update_session_state(&db_id, new.as_str())
                .map_err(|e| e.to_string())
        })
        .await;
        if matches!(result, Ok(Ok(()))) {
            let _ = events.send(SessionStateChangedEvent {
                session_id: session_id.clone(),
                old_state: old.as_str().into(),
                new_state: new.as_str().into(),
                reason: reason.into(),
            });

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
                            let _ = active_session.send_input(format!("{msg}\n").as_bytes());
                        }
                    }
                }
            }
        } else {
            tracing::error!("Failed to persist state transition: {result:?}");
        }
    }
}
