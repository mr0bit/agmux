//! Framed IPC client routing CLI subcommands and streaming terminal I/O.

use std::io::Write;
use std::path::Path;

use anyhow::{Context, Result, bail};
use futures_util::{SinkExt, StreamExt};
use tokio::net::UnixStream;

use agmux_proto::{HandshakeRequest, IpcMessage, SpawnSessionRequest, new_framed};

/// Connect to daemon IPC, spawn a command inside a new PTY session,
/// and stream child stdout/stderr directly to the user's terminal until exit.
pub async fn run_command(
    socket_path: &Path,
    token: &str,
    command: String,
    args: Vec<String>,
    worktree_branch: Option<String>,
) -> Result<i32> {
    let stream = UnixStream::connect(socket_path).await.with_context(|| {
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
        if !matches!(message, IpcMessage::SessionStateChangedEvent(_)) {
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
    let stream = UnixStream::connect(socket_path)
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
        if !matches!(message, IpcMessage::SessionStateChangedEvent(_)) {
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
    let stream = UnixStream::connect(socket_path)
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
        if !matches!(message, IpcMessage::SessionStateChangedEvent(_)) {
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
    let stream = UnixStream::connect(socket_path)
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
        if !matches!(message, IpcMessage::SessionStateChangedEvent(_)) {
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
    let stream = UnixStream::connect(socket_path)
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
        if !matches!(message, IpcMessage::SessionStateChangedEvent(_)) {
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
    let stream = UnixStream::connect(socket_path)
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
            IpcMessage::SessionStateChangedEvent(_) => continue,
            other => bail!("Unexpected removal response: {other:?}"),
        }
    }
}
