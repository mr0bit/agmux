//! Framed IPC client routing CLI subcommands and streaming terminal I/O.

use std::io::Write;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result, bail};
use futures_util::{SinkExt, StreamExt};

use agmux_proto::{HandshakeRequest, IpcMessage, SpawnSessionRequest, connect_ipc, new_framed};

/// Connect to daemon IPC, spawn a command inside a new PTY session,
/// and stream child stdout/stderr directly to the user's terminal until exit.
pub async fn run_command(
    socket_path: &Path,
    token: &str,
    command: String,
    args: Vec<String>,
    worktree_branch: Option<String>,
) -> Result<i32> {
    let stream = connect_ipc(socket_path).await.with_context(|| {
        format!(
            "Failed to connect to daemon socket {}",
            socket_path.display()
        )
    })?;

    let framed = new_framed(stream);
    let (mut sink, mut reader) = framed.split();

    // 1. Send Handshake
    let handshake = IpcMessage::HandshakeRequest(HandshakeRequest {
        client_version: env!("CARGO_PKG_VERSION").to_string(),
        token: token.to_string(),
    });

    sink.send(handshake)
        .await
        .context("Failed transmitting handshake request to daemon")?;

    let handshake_res = reader
        .next()
        .await
        .context("Connection closed by daemon during handshake")?
        .context("Framing error reading handshake response")?;

    match handshake_res {
        IpcMessage::HandshakeResponse(resp) => {
            if !resp.success {
                bail!(
                    "Authentication failed: {}",
                    resp.error.unwrap_or_else(|| "Unknown error".into())
                );
            }
        }
        other => {
            bail!("Unexpected response to handshake: {other:?}");
        }
    }

    // 2. Send SpawnSessionRequest
    let session_id = format!("run-{}", std::process::id());
    let cwd = std::env::current_dir()
        .ok()
        .map(|p| p.display().to_string());

    let spawn_req = IpcMessage::SpawnSessionRequest(SpawnSessionRequest {
        session_id: session_id.clone(),
        command,
        args,
        cwd,
        worktree_branch,
    });

    sink.send(spawn_req)
        .await
        .context("Failed sending spawn request to daemon")?;

    let spawn_res = loop {
        let message = reader
            .next()
            .await
            .context("Connection closed by daemon during spawn request")?
            .context("Framing error reading spawn response")?;
        if !matches!(
            message,
            IpcMessage::SessionStateChangedEvent(_)
                | IpcMessage::NotificationEvent(_)
                | IpcMessage::SessionListUpdate(_)
        ) {
            break message;
        }
    };

    match spawn_res {
        IpcMessage::SpawnSessionResponse(resp) => {
            if !resp.success {
                bail!(
                    "Failed to spawn session: {}",
                    resp.error.unwrap_or_else(|| "Unknown error".into())
                );
            }
        }
        other => {
            bail!("Unexpected response to spawn request: {other:?}");
        }
    }

    // 3. Stream terminal output until child exits
    let mut stdout = std::io::stdout();
    let mut final_exit_code = 0;

    while let Some(msg_res) = reader.next().await {
        let msg = msg_res.context("Error receiving framed message from daemon")?;
        match msg {
            IpcMessage::TerminalOutputFrame(frame) if frame.session_id == session_id => {
                let _ = stdout.write_all(&frame.data);
                let _ = stdout.flush();
            }
            IpcMessage::SessionExitEvent(event) if event.session_id == session_id => {
                final_exit_code = event.exit_code.unwrap_or(0);
                break;
            }
            _ => {}
        }
    }

    Ok(final_exit_code)
}

/// Request active session metadata over authenticated IPC.
pub async fn list_sessions(
    socket_path: &Path,
    token: &str,
) -> Result<Vec<agmux_proto::SessionInfo>> {
    let stream = connect_ipc(socket_path)
        .await
        .context("Failed to connect to daemon")?;
    let mut framed = new_framed(stream);
    framed
        .send(IpcMessage::HandshakeRequest(HandshakeRequest {
            client_version: env!("CARGO_PKG_VERSION").into(),
            token: token.into(),
        }))
        .await?;
    match framed
        .next()
        .await
        .context("Connection closed during handshake")??
    {
        IpcMessage::HandshakeResponse(response) if response.success => {}
        other => bail!("Authentication failed: {other:?}"),
    }
    framed
        .send(IpcMessage::ListSessionsRequest(
            agmux_proto::ListSessionsRequest {},
        ))
        .await?;
    let response = loop {
        let message = framed
            .next()
            .await
            .context("Connection closed during list request")??;
        if !matches!(
            message,
            IpcMessage::SessionStateChangedEvent(_)
                | IpcMessage::NotificationEvent(_)
                | IpcMessage::SessionListUpdate(_)
        ) {
            break message;
        }
    };
    match response {
        IpcMessage::ListSessionsResponse(response) => Ok(response.sessions),
        other => bail!("Unexpected list response: {other:?}"),
    }
}

/// Search stored terminal output over authenticated IPC.
pub async fn search_logs(
    socket_path: &Path,
    token: &str,
    query: String,
    session_id: Option<String>,
    limit: u32,
) -> Result<Vec<agmux_proto::SearchMatch>> {
    let stream = connect_ipc(socket_path)
        .await
        .context("Failed to connect to daemon")?;
    let mut framed = new_framed(stream);
    framed
        .send(IpcMessage::HandshakeRequest(HandshakeRequest {
            client_version: env!("CARGO_PKG_VERSION").into(),
            token: token.into(),
        }))
        .await?;
    match framed
        .next()
        .await
        .context("Connection closed during handshake")??
    {
        IpcMessage::HandshakeResponse(response) if response.success => {}
        other => bail!("Authentication failed: {other:?}"),
    }
    framed
        .send(IpcMessage::SearchLogsRequest(
            agmux_proto::SearchLogsRequest {
                query,
                session_id,
                limit,
            },
        ))
        .await?;
    let response = loop {
        let message = framed
            .next()
            .await
            .context("Connection closed during search")??;
        if !matches!(
            message,
            IpcMessage::SessionStateChangedEvent(_)
                | IpcMessage::NotificationEvent(_)
                | IpcMessage::SessionListUpdate(_)
        ) {
            break message;
        }
    };
    match response {
        IpcMessage::SearchLogsResponse(response) => Ok(response.matches),
        other => bail!("Unexpected search response: {other:?}"),
    }
}

/// Freeze an active or finished session and persist snapshot.
pub async fn freeze_session(
    socket_path: &Path,
    token: &str,
    session_id: String,
) -> Result<agmux_proto::FreezeSessionResponse> {
    let stream = connect_ipc(socket_path)
        .await
        .context("Failed to connect to daemon")?;
    let mut framed = new_framed(stream);
    framed
        .send(IpcMessage::HandshakeRequest(HandshakeRequest {
            client_version: env!("CARGO_PKG_VERSION").into(),
            token: token.into(),
        }))
        .await?;
    match framed
        .next()
        .await
        .context("Connection closed during handshake")??
    {
        IpcMessage::HandshakeResponse(response) if response.success => {}
        other => bail!("Authentication failed: {other:?}"),
    }
    framed
        .send(IpcMessage::FreezeSessionRequest(
            agmux_proto::FreezeSessionRequest { session_id },
        ))
        .await?;
    let response = loop {
        let message = framed
            .next()
            .await
            .context("Connection closed during freeze request")??;
        if !matches!(
            message,
            IpcMessage::SessionStateChangedEvent(_)
                | IpcMessage::NotificationEvent(_)
                | IpcMessage::SessionListUpdate(_)
        ) {
            break message;
        }
    };
    match response {
        IpcMessage::FreezeSessionResponse(response) => Ok(response),
        other => bail!("Unexpected freeze response: {other:?}"),
    }
}

/// Resume and inspect a persisted session snapshot.
pub async fn resume_session(
    socket_path: &Path,
    token: &str,
    session_id: String,
) -> Result<agmux_proto::ResumeSessionResponse> {
    let stream = connect_ipc(socket_path)
        .await
        .context("Failed to connect to daemon")?;
    let mut framed = new_framed(stream);
    framed
        .send(IpcMessage::HandshakeRequest(HandshakeRequest {
            client_version: env!("CARGO_PKG_VERSION").into(),
            token: token.into(),
        }))
        .await?;
    match framed
        .next()
        .await
        .context("Connection closed during handshake")??
    {
        IpcMessage::HandshakeResponse(response) if response.success => {}
        other => bail!("Authentication failed: {other:?}"),
    }
    framed
        .send(IpcMessage::ResumeSessionRequest(
            agmux_proto::ResumeSessionRequest { session_id },
        ))
        .await?;
    let response = loop {
        let message = framed
            .next()
            .await
            .context("Connection closed during resume request")??;
        if !matches!(
            message,
            IpcMessage::SessionStateChangedEvent(_)
                | IpcMessage::NotificationEvent(_)
                | IpcMessage::SessionListUpdate(_)
        ) {
            break message;
        }
    };
    match response {
        IpcMessage::ResumeSessionResponse(response) => Ok(response),
        other => bail!("Unexpected resume response: {other:?}"),
    }
}

/// Remove a session over authenticated IPC.
pub async fn remove_session(
    socket_path: &Path,
    token: &str,
    session_id: String,
    force: bool,
) -> Result<agmux_proto::RemoveSessionResponse> {
    let stream = connect_ipc(socket_path)
        .await
        .context("Failed to connect to daemon")?;
    let mut framed = new_framed(stream);
    framed
        .send(IpcMessage::HandshakeRequest(HandshakeRequest {
            client_version: env!("CARGO_PKG_VERSION").into(),
            token: token.into(),
        }))
        .await?;
    match framed
        .next()
        .await
        .context("Connection closed during handshake")??
    {
        IpcMessage::HandshakeResponse(response) if response.success => {}
        other => bail!("Authentication failed: {other:?}"),
    }
    framed
        .send(IpcMessage::RemoveSessionRequest(
            agmux_proto::RemoveSessionRequest { session_id, force },
        ))
        .await?;
    loop {
        match framed
            .next()
            .await
            .context("Connection closed during removal")??
        {
            IpcMessage::RemoveSessionResponse(response) => return Ok(response),
            IpcMessage::SessionStateChangedEvent(_)
            | IpcMessage::NotificationEvent(_)
            | IpcMessage::SessionListUpdate(_) => continue,
            other => bail!("Unexpected removal response: {other:?}"),
        }
    }
}

/// Request a structured review diff.
pub async fn get_diff(
    socket_path: &Path,
    token: &str,
    request: agmux_proto::GetDiffRequest,
) -> Result<Vec<agmux_proto::FileDiff>> {
    let mut framed = new_framed(connect_ipc(socket_path).await?);
    framed
        .send(IpcMessage::HandshakeRequest(HandshakeRequest {
            client_version: env!("CARGO_PKG_VERSION").into(),
            token: token.into(),
        }))
        .await?;
    match framed
        .next()
        .await
        .context("Connection closed during handshake")??
    {
        IpcMessage::HandshakeResponse(response) if response.success => {}
        other => bail!("Authentication failed: {other:?}"),
    }
    framed.send(IpcMessage::GetDiffRequest(request)).await?;
    loop {
        match framed
            .next()
            .await
            .context("Connection closed during diff request")??
        {
            IpcMessage::GetDiffResponse(response) => {
                if let Some(error) = response.error {
                    bail!("{error}");
                }
                return Ok(response.files);
            }
            IpcMessage::SessionStateChangedEvent(_)
            | IpcMessage::NotificationEvent(_)
            | IpcMessage::SessionListUpdate(_) => continue,
            other => bail!("Unexpected diff response: {other:?}"),
        }
    }
}

/// Send a review request over authenticated IPC.
pub async fn review_request(
    socket_path: &Path,
    token: &str,
    request: IpcMessage,
) -> Result<IpcMessage> {
    let mut framed = new_framed(connect_ipc(socket_path).await?);
    framed
        .send(IpcMessage::HandshakeRequest(HandshakeRequest {
            client_version: env!("CARGO_PKG_VERSION").into(),
            token: token.into(),
        }))
        .await?;
    match framed
        .next()
        .await
        .context("Connection closed during handshake")??
    {
        IpcMessage::HandshakeResponse(response) if response.success => {}
        other => bail!("Authentication failed: {other:?}"),
    }
    framed.send(request).await?;
    loop {
        let message = framed
            .next()
            .await
            .context("Connection closed during review request")??;
        if !matches!(
            message,
            IpcMessage::SessionStateChangedEvent(_)
                | IpcMessage::NotificationEvent(_)
                | IpcMessage::SessionListUpdate(_)
        ) {
            return Ok(message);
        }
    }
}

/// Request lock acquisition over authenticated IPC.
pub async fn acquire_lock(
    socket_path: &Path,
    token: &str,
    lock_name: String,
    session_id: String,
    wait: bool,
) -> Result<agmux_proto::AcquireLockResponse> {
    let mut framed = new_framed(connect_ipc(socket_path).await?);
    framed
        .send(IpcMessage::HandshakeRequest(HandshakeRequest {
            client_version: env!("CARGO_PKG_VERSION").into(),
            token: token.into(),
        }))
        .await?;
    match framed
        .next()
        .await
        .context("Connection closed during handshake")??
    {
        IpcMessage::HandshakeResponse(response) if response.success => {}
        other => bail!("Authentication failed: {other:?}"),
    }
    framed
        .send(IpcMessage::AcquireLockRequest(
            agmux_proto::AcquireLockRequest {
                lock_name,
                session_id,
                wait,
            },
        ))
        .await?;
    loop {
        match framed
            .next()
            .await
            .context("Connection closed waiting for lock response")??
        {
            IpcMessage::AcquireLockResponse(resp) => return Ok(resp),
            IpcMessage::SessionStateChangedEvent(_)
            | IpcMessage::NotificationEvent(_)
            | IpcMessage::SessionListUpdate(_) => continue,
            other => bail!("Unexpected response: {other:?}"),
        }
    }
}

/// Request lock release over authenticated IPC.
pub async fn release_lock(
    socket_path: &Path,
    token: &str,
    lock_name: String,
    session_id: String,
) -> Result<agmux_proto::ReleaseLockResponse> {
    let mut framed = new_framed(connect_ipc(socket_path).await?);
    framed
        .send(IpcMessage::HandshakeRequest(HandshakeRequest {
            client_version: env!("CARGO_PKG_VERSION").into(),
            token: token.into(),
        }))
        .await?;
    match framed
        .next()
        .await
        .context("Connection closed during handshake")??
    {
        IpcMessage::HandshakeResponse(response) if response.success => {}
        other => bail!("Authentication failed: {other:?}"),
    }
    framed
        .send(IpcMessage::ReleaseLockRequest(
            agmux_proto::ReleaseLockRequest {
                lock_name,
                session_id,
            },
        ))
        .await?;
    loop {
        match framed
            .next()
            .await
            .context("Connection closed waiting for lock release response")??
        {
            IpcMessage::ReleaseLockResponse(resp) => return Ok(resp),
            IpcMessage::SessionStateChangedEvent(_)
            | IpcMessage::NotificationEvent(_)
            | IpcMessage::SessionListUpdate(_) => continue,
            other => bail!("Unexpected response: {other:?}"),
        }
    }
}

/// Request active lock list over authenticated IPC.
pub async fn list_locks(socket_path: &Path, token: &str) -> Result<agmux_proto::ListLocksResponse> {
    let mut framed = new_framed(connect_ipc(socket_path).await?);
    framed
        .send(IpcMessage::HandshakeRequest(HandshakeRequest {
            client_version: env!("CARGO_PKG_VERSION").into(),
            token: token.into(),
        }))
        .await?;
    match framed
        .next()
        .await
        .context("Connection closed during handshake")??
    {
        IpcMessage::HandshakeResponse(response) if response.success => {}
        other => bail!("Authentication failed: {other:?}"),
    }
    framed
        .send(IpcMessage::ListLocksRequest(
            agmux_proto::ListLocksRequest {},
        ))
        .await?;
    loop {
        match framed
            .next()
            .await
            .context("Connection closed waiting for lock list response")??
        {
            IpcMessage::ListLocksResponse(resp) => return Ok(resp),
            IpcMessage::SessionStateChangedEvent(_)
            | IpcMessage::NotificationEvent(_)
            | IpcMessage::SessionListUpdate(_) => continue,
            other => bail!("Unexpected response: {other:?}"),
        }
    }
}

/// Canonicalize a path relative to the primary git repository root, even across worktrees.
pub fn canonicalize_repo_path(path_str: &str, cwd: &Path) -> String {
    let trimmed = path_str.trim();
    if trimmed.is_empty() || trimmed == "." || trimmed == "./" {
        return String::new();
    }

    let primary_root = (|| -> Option<PathBuf> {
        let output = std::process::Command::new("git")
            .current_dir(cwd)
            .args(["rev-parse", "--path-format=absolute", "--git-common-dir"])
            .output()
            .ok()?;
        if output.status.success() {
            let common = String::from_utf8(output.stdout).ok()?;
            let common_path = PathBuf::from(common.trim());
            return common_path.parent().map(|p| p.to_path_buf());
        }
        let toplevel = std::process::Command::new("git")
            .current_dir(cwd)
            .args(["rev-parse", "--show-toplevel"])
            .output()
            .ok()?;
        if toplevel.status.success() {
            let top = String::from_utf8(toplevel.stdout).ok()?;
            return Some(PathBuf::from(top.trim()));
        }
        None
    })();

    let path = Path::new(trimmed);
    let abs_path = if path.is_absolute() {
        path.to_path_buf()
    } else {
        cwd.join(path)
    };

    if let Some(root) = primary_root {
        if let Ok(rel) = abs_path.strip_prefix(&root) {
            let rel_str = rel.to_string_lossy();
            if let Some(stripped) = rel_str.strip_prefix(".agmux-worktrees/") {
                if let Some(idx) = stripped.find('/') {
                    return stripped[idx + 1..].trim_matches('/').to_string();
                }
            } else {
                return rel_str.trim_matches('/').to_string();
            }
        }
    }

    trimmed
        .trim_start_matches("./")
        .trim_matches('/')
        .to_string()
}

/// Request file reservations over authenticated IPC.
pub async fn reserve_files(
    socket_path: &Path,
    token: &str,
    session_id: String,
    paths: Vec<String>,
) -> Result<agmux_proto::ReserveFilesResponse> {
    let mut framed = new_framed(connect_ipc(socket_path).await?);
    framed
        .send(IpcMessage::HandshakeRequest(HandshakeRequest {
            client_version: env!("CARGO_PKG_VERSION").into(),
            token: token.into(),
        }))
        .await?;
    match framed
        .next()
        .await
        .context("Connection closed during handshake")??
    {
        IpcMessage::HandshakeResponse(response) if response.success => {}
        other => bail!("Authentication failed: {other:?}"),
    }
    framed
        .send(IpcMessage::ReserveFilesRequest(
            agmux_proto::ReserveFilesRequest { session_id, paths },
        ))
        .await?;
    loop {
        match framed
            .next()
            .await
            .context("Connection closed waiting for reserve response")??
        {
            IpcMessage::ReserveFilesResponse(resp) => return Ok(resp),
            IpcMessage::SessionStateChangedEvent(_)
            | IpcMessage::NotificationEvent(_)
            | IpcMessage::SessionListUpdate(_) => continue,
            other => bail!("Unexpected response: {other:?}"),
        }
    }
}

/// Request release of file reservations over authenticated IPC.
pub async fn release_files(
    socket_path: &Path,
    token: &str,
    session_id: String,
    paths: Vec<String>,
) -> Result<agmux_proto::ReleaseFilesResponse> {
    let mut framed = new_framed(connect_ipc(socket_path).await?);
    framed
        .send(IpcMessage::HandshakeRequest(HandshakeRequest {
            client_version: env!("CARGO_PKG_VERSION").into(),
            token: token.into(),
        }))
        .await?;
    match framed
        .next()
        .await
        .context("Connection closed during handshake")??
    {
        IpcMessage::HandshakeResponse(response) if response.success => {}
        other => bail!("Authentication failed: {other:?}"),
    }
    framed
        .send(IpcMessage::ReleaseFilesRequest(
            agmux_proto::ReleaseFilesRequest { session_id, paths },
        ))
        .await?;
    loop {
        match framed
            .next()
            .await
            .context("Connection closed waiting for release response")??
        {
            IpcMessage::ReleaseFilesResponse(resp) => return Ok(resp),
            IpcMessage::SessionStateChangedEvent(_)
            | IpcMessage::NotificationEvent(_)
            | IpcMessage::SessionListUpdate(_) => continue,
            other => bail!("Unexpected response: {other:?}"),
        }
    }
}

/// Request active file reservations list over authenticated IPC.
pub async fn list_reservations(
    socket_path: &Path,
    token: &str,
) -> Result<agmux_proto::ListReservationsResponse> {
    let mut framed = new_framed(connect_ipc(socket_path).await?);
    framed
        .send(IpcMessage::HandshakeRequest(HandshakeRequest {
            client_version: env!("CARGO_PKG_VERSION").into(),
            token: token.into(),
        }))
        .await?;
    match framed
        .next()
        .await
        .context("Connection closed during handshake")??
    {
        IpcMessage::HandshakeResponse(response) if response.success => {}
        other => bail!("Authentication failed: {other:?}"),
    }
    framed
        .send(IpcMessage::ListReservationsRequest(
            agmux_proto::ListReservationsRequest {},
        ))
        .await?;
    loop {
        match framed
            .next()
            .await
            .context("Connection closed waiting for reservations list")??
        {
            IpcMessage::ListReservationsResponse(resp) => return Ok(resp),
            IpcMessage::SessionStateChangedEvent(_)
            | IpcMessage::NotificationEvent(_)
            | IpcMessage::SessionListUpdate(_) => continue,
            other => bail!("Unexpected response: {other:?}"),
        }
    }
}

/// Stream events from daemon event journal.
pub async fn watch_events<F>(
    socket_path: &Path,
    token: &str,
    cursor_id: Option<String>,
    since_seq: Option<u64>,
    follow: bool,
    mut on_event: F,
) -> Result<()>
where
    F: FnMut(agmux_proto::WatchEventFrame) -> Result<bool>,
{
    let mut framed = new_framed(connect_ipc(socket_path).await?);
    framed
        .send(IpcMessage::HandshakeRequest(HandshakeRequest {
            client_version: env!("CARGO_PKG_VERSION").into(),
            token: token.into(),
        }))
        .await?;
    match framed
        .next()
        .await
        .context("Connection closed during handshake")??
    {
        IpcMessage::HandshakeResponse(response) if response.success => {}
        other => bail!("Authentication failed: {other:?}"),
    }
    framed
        .send(IpcMessage::WatchEventsRequest(
            agmux_proto::WatchEventsRequest {
                cursor_id,
                since_seq,
                follow,
            },
        ))
        .await?;
    while let Some(msg) = framed.next().await {
        match msg? {
            IpcMessage::WatchEventFrame(frame) => {
                if frame.is_done {
                    break;
                }
                let keep_going = on_event(frame)?;
                if !keep_going {
                    break;
                }
            }
            IpcMessage::SessionStateChangedEvent(_)
            | IpcMessage::NotificationEvent(_)
            | IpcMessage::SessionListUpdate(_) => continue,
            other => bail!("Unexpected frame in watch stream: {other:?}"),
        }
    }
    Ok(())
}

/// Create a Kanban task over authenticated IPC.
pub async fn create_task(
    socket_path: &Path,
    token: &str,
    task_id: String,
    title: String,
    description: String,
) -> Result<agmux_proto::CreateTaskResponse> {
    match review_request(
        socket_path,
        token,
        IpcMessage::CreateTaskRequest(agmux_proto::CreateTaskRequest {
            task_id,
            title,
            description,
        }),
    )
    .await?
    {
        IpcMessage::CreateTaskResponse(resp) => Ok(resp),
        other => bail!("Unexpected response: {other:?}"),
    }
}

/// List Kanban tasks over authenticated IPC.
pub async fn list_tasks(
    socket_path: &Path,
    token: &str,
    status: Option<String>,
) -> Result<agmux_proto::ListTasksResponse> {
    match review_request(
        socket_path,
        token,
        IpcMessage::ListTasksRequest(agmux_proto::ListTasksRequest { status }),
    )
    .await?
    {
        IpcMessage::ListTasksResponse(resp) => Ok(resp),
        other => bail!("Unexpected response: {other:?}"),
    }
}

/// Claim a Kanban task over authenticated IPC.
pub async fn claim_task(
    socket_path: &Path,
    token: &str,
    task_id: String,
    session_id: String,
) -> Result<agmux_proto::ClaimTaskResponse> {
    match review_request(
        socket_path,
        token,
        IpcMessage::ClaimTaskRequest(agmux_proto::ClaimTaskRequest {
            task_id,
            session_id,
        }),
    )
    .await?
    {
        IpcMessage::ClaimTaskResponse(resp) => Ok(resp),
        other => bail!("Unexpected response: {other:?}"),
    }
}

/// Finish a Kanban task over authenticated IPC.
pub async fn finish_task(
    socket_path: &Path,
    token: &str,
    task_id: String,
    session_id: String,
) -> Result<agmux_proto::FinishTaskResponse> {
    match review_request(
        socket_path,
        token,
        IpcMessage::FinishTaskRequest(agmux_proto::FinishTaskRequest {
            task_id,
            session_id,
        }),
    )
    .await?
    {
        IpcMessage::FinishTaskResponse(resp) => Ok(resp),
        other => bail!("Unexpected response: {other:?}"),
    }
}

/// Release a Kanban task over authenticated IPC.
pub async fn release_task(
    socket_path: &Path,
    token: &str,
    task_id: String,
    session_id: String,
) -> Result<agmux_proto::ReleaseTaskResponse> {
    match review_request(
        socket_path,
        token,
        IpcMessage::ReleaseTaskRequest(agmux_proto::ReleaseTaskRequest {
            task_id,
            session_id,
        }),
    )
    .await?
    {
        IpcMessage::ReleaseTaskResponse(resp) => Ok(resp),
        other => bail!("Unexpected response: {other:?}"),
    }
}

/// Post a structured worker report over authenticated IPC.
pub async fn post_report(
    socket_path: &Path,
    token: &str,
    task_id: String,
    session_id: String,
    summary: String,
    pr_url: Option<String>,
    commit_sha: Option<String>,
) -> Result<agmux_proto::PostReportResponse> {
    match review_request(
        socket_path,
        token,
        IpcMessage::PostReportRequest(agmux_proto::PostReportRequest {
            task_id,
            session_id,
            summary,
            pr_url,
            commit_sha,
        }),
    )
    .await?
    {
        IpcMessage::PostReportResponse(resp) => Ok(resp),
        other => bail!("Unexpected response: {other:?}"),
    }
}

/// View structured worker reports for a task over authenticated IPC.
pub async fn view_reports(
    socket_path: &Path,
    token: &str,
    task_id: String,
) -> Result<agmux_proto::ViewReportResponse> {
    match review_request(
        socket_path,
        token,
        IpcMessage::ViewReportRequest(agmux_proto::ViewReportRequest { task_id }),
    )
    .await?
    {
        IpcMessage::ViewReportResponse(resp) => Ok(resp),
        other => bail!("Unexpected response: {other:?}"),
    }
}

/// Request graceful daemon shutdown over authenticated IPC.
pub async fn shutdown_daemon(
    socket_path: &Path,
    token: &str,
) -> Result<agmux_proto::ShutdownDaemonResponse> {
    match review_request(
        socket_path,
        token,
        IpcMessage::ShutdownDaemonRequest(agmux_proto::ShutdownDaemonRequest {}),
    )
    .await?
    {
        IpcMessage::ShutdownDaemonResponse(resp) => Ok(resp),
        other => bail!("Unexpected response: {other:?}"),
    }
}

/// Request a PTY session resize over authenticated IPC.
pub async fn resize_session(
    socket_path: &Path,
    token: &str,
    session_id: String,
    cols: u16,
    rows: u16,
) -> Result<agmux_proto::ResizeSessionResponse> {
    match review_request(
        socket_path,
        token,
        IpcMessage::ResizeSessionRequest(agmux_proto::ResizeSessionRequest {
            session_id,
            cols,
            rows,
        }),
    )
    .await?
    {
        IpcMessage::ResizeSessionResponse(resp) => Ok(resp),
        other => bail!("Unexpected response: {other:?}"),
    }
}

/// Spawn an interactive shell sub-tab inheriting a parent session's CWD and PORT over authenticated IPC.
pub async fn spawn_subtab(
    socket_path: &Path,
    token: &str,
    parent_session_id: String,
    subtab_id: String,
    shell: Option<String>,
) -> Result<agmux_proto::SpawnSubtabResponse> {
    match review_request(
        socket_path,
        token,
        IpcMessage::SpawnSubtabRequest(agmux_proto::SpawnSubtabRequest {
            parent_session_id,
            subtab_id,
            shell,
        }),
    )
    .await?
    {
        IpcMessage::SpawnSubtabResponse(resp) => Ok(resp),
        other => bail!("Unexpected response: {other:?}"),
    }
}

/// Emit a Tier 1 deterministic CLI hook event (`working`, `permission_request`, `stop`) over authenticated IPC.
pub async fn send_internal_hook(
    socket_path: &Path,
    token: &str,
    session_id: String,
    event: String,
) -> Result<agmux_proto::InternalHookResponse> {
    match review_request(
        socket_path,
        token,
        IpcMessage::InternalHookRequest(agmux_proto::InternalHookRequest { session_id, event }),
    )
    .await?
    {
        IpcMessage::InternalHookResponse(resp) => Ok(resp),
        other => bail!("Unexpected response: {other:?}"),
    }
}

/// Federate and mount a remote daemon over authenticated IPC (`REQ-REMOTE-FEDERATION-SSH-001`).
pub async fn connect_remote(
    socket_path: &Path,
    token: &str,
    target: String,
    remote_socket: Option<String>,
    remote_token: Option<String>,
) -> Result<agmux_proto::ConnectRemoteResponse> {
    match review_request(
        socket_path,
        token,
        IpcMessage::ConnectRemoteRequest(agmux_proto::ConnectRemoteRequest {
            target,
            remote_socket,
            remote_token,
        }),
    )
    .await?
    {
        IpcMessage::ConnectRemoteResponse(resp) => Ok(resp),
        other => bail!("Unexpected response: {other:?}"),
    }
}

/// List connected remote daemon host labels over authenticated IPC.
pub async fn list_remotes(
    socket_path: &Path,
    token: &str,
) -> Result<agmux_proto::ListRemotesResponse> {
    match review_request(
        socket_path,
        token,
        IpcMessage::ListRemotesRequest(agmux_proto::ListRemotesRequest {}),
    )
    .await?
    {
        IpcMessage::ListRemotesResponse(resp) => Ok(resp),
        other => bail!("Unexpected response: {other:?}"),
    }
}
