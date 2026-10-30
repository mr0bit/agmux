//! SSH and Tailscale remote daemon federation bridge (`REQ-REMOTE-FEDERATION-SSH-001`).

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};

use agmux_proto::{
    HandshakeRequest, IpcMessage, ListSessionsRequest, SessionInfo, TerminalInputFrame,
    connect_ipc, new_framed,
};
use futures_util::{SinkExt, StreamExt};

/// Represents a mounted remote daemon endpoint and its cached session list.
#[derive(Debug, Clone)]
pub struct FederatedRemoteHost {
    pub host_label: String,
    pub target_uri: String,
    pub remote_socket: Option<PathBuf>,
    pub remote_token: Option<String>,
    pub sessions: Vec<SessionInfo>,
}

/// Manages federated remote hosts (`ssh://user@host` or Tailscale targets), mounting remote sessions
/// with `[<host_label>] <session_id>` and bridging I/O frames across IPC.
#[derive(Clone, Default)]
pub struct SshFederationManager {
    hosts: Arc<Mutex<HashMap<String, FederatedRemoteHost>>>,
}

impl SshFederationManager {
    /// Create a new `SshFederationManager`.
    pub fn new() -> Self {
        Self::default()
    }

    /// Normalize a target URI (`ssh://user@box:22`, `user@box`, `tailscale://devbox`) into a concise host label.
    pub fn parse_host_label(target: &str) -> String {
        let stripped = target
            .trim()
            .strip_prefix("ssh://")
            .or_else(|| target.trim().strip_prefix("tailscale://"))
            .unwrap_or(target.trim());
        let without_user = stripped
            .split_once('@')
            .map(|(_, host)| host)
            .unwrap_or(stripped);
        without_user
            .split_once(':')
            .map(|(h, _)| h)
            .unwrap_or(without_user)
            .trim_end_matches('/')
            .to_string()
    }

    /// Format a remote session ID under a foldable host prefix (`[host] session_id`).
    pub fn format_mounted_id(host_label: &str, remote_session_id: &str) -> String {
        format!("[{host_label}] {remote_session_id}")
    }

    /// Split a mounted session ID (`[host] session_id`) into `(host_label, remote_session_id)`.
    pub fn parse_mounted_id(mounted_id: &str) -> Option<(String, String)> {
        let rest = mounted_id.strip_prefix('[')?;
        let (host, after) = rest.split_once("] ")?;
        if host.is_empty() || after.is_empty() {
            return None;
        }
        Some((host.to_string(), after.to_string()))
    }

    /// Connect to a remote daemon (via explicit `remote_socket`/`remote_token` or `AGMUX_REMOTE_BRIDGE_SOCK`),
    /// perform the authenticated handshake, fetch active sessions, and mount them under `[host_label]`.
    pub async fn connect_remote(
        &self,
        target: &str,
        remote_socket: Option<PathBuf>,
        remote_token: Option<String>,
    ) -> Result<(String, Vec<SessionInfo>), String> {
        let host_label = Self::parse_host_label(target);
        if host_label.is_empty() {
            return Err("Invalid remote target URI".to_string());
        }

        let resolved_socket = remote_socket.or_else(|| {
            std::env::var("AGMUX_REMOTE_BRIDGE_SOCK")
                .ok()
                .filter(|s| !s.is_empty())
                .map(PathBuf::from)
        });
        let resolved_token = remote_token.or_else(|| {
            std::env::var("AGMUX_REMOTE_TOKEN_PATH")
                .ok()
                .and_then(|p| std::fs::read_to_string(p).ok())
                .map(|s| s.trim().to_string())
        });

        let mounted_sessions = if let Some(ref sock) = resolved_socket {
            let token = resolved_token.clone().unwrap_or_default();
            let remote_list = Self::fetch_remote_sessions(sock, &token).await?;
            remote_list
                .into_iter()
                .map(|mut s| {
                    s.session_id = Self::format_mounted_id(&host_label, &s.session_id);
                    s
                })
                .collect()
        } else {
            Vec::new()
        };

        let host_entry = FederatedRemoteHost {
            host_label: host_label.clone(),
            target_uri: target.to_string(),
            remote_socket: resolved_socket,
            remote_token: resolved_token,
            sessions: mounted_sessions.clone(),
        };

        if let Ok(mut guard) = self.hosts.lock() {
            guard.insert(host_label.clone(), host_entry);
        }

        Ok((host_label, mounted_sessions))
    }

    /// Refresh and return all mounted remote sessions across all connected remote hosts.
    pub async fn list_mounted_sessions(&self) -> Vec<SessionInfo> {
        let hosts: Vec<FederatedRemoteHost> = self
            .hosts
            .lock()
            .map(|g| g.values().cloned().collect())
            .unwrap_or_default();

        let mut all = Vec::new();
        for host in hosts {
            if let Some(ref sock) = host.remote_socket {
                let token = host.remote_token.clone().unwrap_or_default();
                if let Ok(remote_list) = Self::fetch_remote_sessions(sock, &token).await {
                    let mapped: Vec<SessionInfo> = remote_list
                        .into_iter()
                        .map(|mut s| {
                            s.session_id = Self::format_mounted_id(&host.host_label, &s.session_id);
                            s
                        })
                        .collect();
                    if let Ok(mut g) = self.hosts.lock()
                        && let Some(h) = g.get_mut(&host.host_label)
                    {
                        h.sessions = mapped.clone();
                    }
                    all.extend(mapped);
                    continue;
                }
            }
            all.extend(host.sessions);
        }
        all
    }

    /// Forward terminal input to a mounted remote session (`[host] session_id`) over the IPC bridge.
    pub async fn forward_input(
        &self,
        mounted_session_id: &str,
        data: &[u8],
    ) -> Result<bool, String> {
        let Some((host_label, remote_session_id)) = Self::parse_mounted_id(mounted_session_id)
        else {
            return Ok(false);
        };
        let host = self
            .hosts
            .lock()
            .ok()
            .and_then(|g| g.get(&host_label).cloned())
            .ok_or_else(|| format!("Remote host '{host_label}' not connected"))?;
        let sock = host
            .remote_socket
            .ok_or_else(|| format!("Remote host '{host_label}' has no active bridge socket"))?;
        let token = host.remote_token.unwrap_or_default();

        let stream = connect_ipc(&sock)
            .await
            .map_err(|e| format!("Failed connecting to remote bridge {}: {e}", sock.display()))?;
        let mut framed = new_framed(stream);
        framed
            .send(IpcMessage::HandshakeRequest(HandshakeRequest {
                client_version: env!("CARGO_PKG_VERSION").to_string(),
                token,
            }))
            .await
            .map_err(|e| e.to_string())?;
        match framed.next().await {
            Some(Ok(IpcMessage::HandshakeResponse(resp))) if resp.success => {}
            other => return Err(format!("Remote handshake failed: {other:?}")),
        }
        framed
            .send(IpcMessage::TerminalInputFrame(TerminalInputFrame {
                session_id: remote_session_id,
                data: data.to_vec(),
            }))
            .await
            .map_err(|e| e.to_string())?;
        Ok(true)
    }

    /// Return connected remote host labels.
    pub fn list_remotes(&self) -> Vec<String> {
        let mut names: Vec<String> = self
            .hosts
            .lock()
            .map(|g| g.keys().cloned().collect())
            .unwrap_or_default();
        names.sort();
        names
    }

    async fn fetch_remote_sessions(
        socket_path: &Path,
        token: &str,
    ) -> Result<Vec<SessionInfo>, String> {
        let stream = connect_ipc(socket_path).await.map_err(|e| {
            format!(
                "Failed to connect to remote daemon socket {}: {e}",
                socket_path.display()
            )
        })?;
        let mut framed = new_framed(stream);
        framed
            .send(IpcMessage::HandshakeRequest(HandshakeRequest {
                client_version: env!("CARGO_PKG_VERSION").to_string(),
                token: token.to_string(),
            }))
            .await
            .map_err(|e| e.to_string())?;
        match framed.next().await {
            Some(Ok(IpcMessage::HandshakeResponse(resp))) if resp.success => {}
            Some(Ok(IpcMessage::HandshakeResponse(resp))) => {
                return Err(resp
                    .error
                    .unwrap_or_else(|| "Remote authentication rejected".to_string()));
            }
            other => return Err(format!("Unexpected remote handshake response: {other:?}")),
        }
        framed
            .send(IpcMessage::ListSessionsRequest(ListSessionsRequest {}))
            .await
            .map_err(|e| e.to_string())?;
        while let Some(frame) = framed.next().await {
            match frame.map_err(|e| e.to_string())? {
                IpcMessage::ListSessionsResponse(resp) => return Ok(resp.sessions),
                IpcMessage::SessionStateChangedEvent(_)
                | IpcMessage::NotificationEvent(_)
                | IpcMessage::SessionListUpdate(_) => continue,
                other => return Err(format!("Unexpected remote list response: {other:?}")),
            }
        }
        Err("Remote connection closed before ListSessionsResponse".to_string())
    }
}
