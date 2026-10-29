//! IPC server listening on local domain sockets and routing client requests.

use std::fs;
use std::path::PathBuf;
use std::sync::Arc;

use anyhow::{Context, Result};
use futures_util::{SinkExt, StreamExt};
use tokio::net::{UnixListener, UnixStream};

use agmux_core::pty_manager::PtyManager;
use agmux_proto::{
    HandshakeResponse, IpcMessage, SessionExitEvent, SpawnSessionResponse, TerminalOutputFrame,
    new_framed,
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

        loop {
            match listener.accept().await {
                Ok((stream, _addr)) => {
                    let pty_mgr = pty_manager.clone();
                    let expected_token = auth_token.clone();
                    tokio::spawn(async move {
                        if let Err(e) = handle_client(stream, pty_mgr, expected_token).await {
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
                let cwd = req.cwd.map(PathBuf::from);

                match pty_manager
                    .spawn(session_id.clone(), req.command, req.args, cwd)
                    .await
                {
                    Ok(session) => {
                        let resp = IpcMessage::SpawnSessionResponse(SpawnSessionResponse {
                            session_id: session_id.clone(),
                            success: true,
                            error: None,
                        });
                        if out_tx.send(resp).await.is_err() {
                            break;
                        }

                        // Attach streaming forwarder
                        let out_tx_fwd = out_tx.clone();
                        let fwd_session_id = session_id.clone();
                        tokio::spawn(async move {
                            let mut output_rx = session.subscribe_output();
                            let mut exit_rx = session.subscribe_exit();

                            loop {
                                tokio::select! {
                                    Ok(data) = output_rx.recv() => {
                                        let out_frame = IpcMessage::TerminalOutputFrame(TerminalOutputFrame {
                                            session_id: fwd_session_id.clone(),
                                            data,
                                        });
                                        if out_tx_fwd.send(out_frame).await.is_err() {
                                            break;
                                        }
                                    }
                                    _ = exit_rx.changed() => {
                                        let code_opt = {
                                            *exit_rx.borrow()
                                        };
                                        if let Some(code) = code_opt {
                                            let exit_event = IpcMessage::SessionExitEvent(SessionExitEvent {
                                                session_id: fwd_session_id.clone(),
                                                exit_code: Some(code),
                                            });
                                            let _ = out_tx_fwd.send(exit_event).await;
                                            break;
                                        }
                                    }
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
            IpcMessage::HandshakeRequest(_)
            | IpcMessage::HandshakeResponse(_)
            | IpcMessage::SpawnSessionResponse(_)
            | IpcMessage::TerminalOutputFrame(_)
            | IpcMessage::SessionExitEvent(_) => {
                tracing::debug!("Unhandled message in active session state: {message:?}");
            }
        }
    }

    writer_task.abort();
    Ok(())
}
