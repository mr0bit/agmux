//! Local companion web bridge over authenticated daemon IPC.
use agmux_proto::{CompanionRequest, CompanionResponse, IpcMessage};
use anyhow::{Context, Result, bail};
use serde_json::{Value, json};
use std::path::{Path, PathBuf};

/// Browser API and static frontend served over the existing authenticated IPC seam.
pub mod live;

pub async fn request(
    socket: &Path,
    token: &str,
    req: CompanionRequest,
) -> Result<CompanionResponse> {
    match crate::ipc_client::review_request(socket, token, IpcMessage::CompanionRequest(req))
        .await?
    {
        IpcMessage::CompanionResponse(r) => {
            if let Some(e) = &r.error {
                bail!("{e}");
            }
            Ok(r)
        }
        other => bail!("Unexpected companion response: {other:?}"),
    }
}

pub async fn snapshot(socket: &Path, token: &str) -> Result<Value> {
    let panels = request(socket, token, CompanionRequest::List).await?;
    let workspace = crate::ipc_client::review_request(
        socket,
        token,
        IpcMessage::WorkspaceRequest(agmux_proto::WorkspaceRequest::List),
    )
    .await?;
    let IpcMessage::WorkspaceResponse(workspace) = workspace else {
        bail!("Unexpected workspace response");
    };
    let attention = workspace
        .sessions
        .iter()
        .filter(|s| matches!(s.state.as_str(), "WAITING_INPUT" | "FAILED" | "STUCK"))
        .count();
    let screen = if let Some(id) = &panels.selected {
        match crate::ipc_client::review_request(
            socket,
            token,
            IpcMessage::ReadSessionRequest(agmux_proto::ReadSessionRequest {
                session_id: id.clone(),
            }),
        )
        .await
        {
            Ok(IpcMessage::ReadSessionResponse(r)) => {
                let mut history = agmux_core::fts_indexer::TerminalHistory::new(1_000_000);
                history.ingest(r.screen.as_bytes());
                String::from_utf8_lossy(&history.clean()).into_owned()
            }
            _ => String::new(),
        }
    } else {
        String::new()
    };
    Ok(
        json!({"sessions":workspace.sessions,"metadata":workspace.metadata,"panels":panels.panels,"notifications":panels.notifications,"selected_panel":panels.selected_panel,"selected":panels.selected,"attention":attention,"screen":screen}),
    )
}

/// Resolve strictly structured session and panel links through the daemon registry.
pub async fn open_link(socket: &Path, token: &str, uri: &str) -> Result<()> {
    let rest = uri
        .strip_prefix("agmux://session/")
        .context("Invalid agmux link")?;
    let parts: Vec<_> = rest.split('/').collect();
    if parts.is_empty()
        || parts[0].is_empty()
        || !parts[0]
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || "-_".contains(c))
    {
        bail!("Invalid session ID");
    }
    match parts.as_slice() {
        [id] => {
            request(
                socket,
                token,
                CompanionRequest::Select {
                    session_id: (*id).into(),
                },
            )
            .await?;
        }
        [id, "panel", panel] => {
            let panel: i64 = panel.parse()?;
            request(
                socket,
                token,
                CompanionRequest::SelectPanel {
                    session_id: (*id).into(),
                    panel_id: panel,
                },
            )
            .await?;
        }
        _ => bail!("Invalid agmux link"),
    }
    Ok(())
}

/// Create a desktop entry for the `agmux://` URI scheme.
#[cfg(target_os = "linux")]
pub fn register_desktop_protocol(app_path: Option<&Path>) -> Result<PathBuf> {
    let executable = match app_path {
        Some(path) => path.to_path_buf(),
        None => std::env::current_exe()?,
    };
    let executable = executable
        .canonicalize()
        .context("Invalid executable path")?;
    let executable = executable
        .to_str()
        .context("Executable path is not UTF-8")?;
    let escaped = executable
        .replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('%', "%%")
        .replace('$', "\\$")
        .replace('`', "\\`");
    let home = std::env::var_os("HOME").context("HOME is not set")?;
    let dir = PathBuf::from(home).join(".local/share/applications");
    std::fs::create_dir_all(&dir)?;
    let path = dir.join("agmux-scheme-handler.desktop");
    std::fs::write(
        &path,
        format!(
            "[Desktop Entry]\nType=Application\nName=Agmux\nExec=\"{escaped}\" open %u\nStartupNotify=false\nTerminal=false\nMimeType=x-scheme-handler/agmux;\n"
        ),
    )?;
    let status = std::process::Command::new("xdg-mime")
        .args([
            "default",
            "agmux-scheme-handler.desktop",
            "x-scheme-handler/agmux",
        ])
        .status()
        .context("Failed to run xdg-mime")?;
    if !status.success() {
        bail!("xdg-mime failed: {status}");
    }
    Ok(path)
}

/// Registering a desktop URI handler is currently supported on Linux.
#[cfg(not(target_os = "linux"))]
pub fn register_desktop_protocol(_app_path: Option<&Path>) -> Result<PathBuf> {
    bail!("Desktop protocol registration is only supported on Linux")
}

/// Run the full companion application, using assets embedded in agmux itself.
pub async fn serve(socket: PathBuf, daemon_token: String) -> Result<()> {
    live::serve(socket, daemon_token).await
}
