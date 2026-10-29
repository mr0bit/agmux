//! Background daemon auto-spawner on socket absence.

use std::path::{Path, PathBuf};
use std::time::Duration;

use anyhow::{Context, Result, bail};
use tokio::net::UnixStream;

/// Locate the `agmuxd` daemon binary on the host system.
pub fn find_daemon_binary() -> Result<PathBuf> {
    if let Ok(bin) = std::env::var("AGMUXD_BIN") {
        let p = PathBuf::from(bin);
        if p.exists() {
            return Ok(p);
        }
    }

    // Check directory of the currently running executable
    if let Ok(current_exe) = std::env::current_exe() {
        if let Some(parent) = current_exe.parent() {
            let candidate = parent.join("agmuxd");
            if candidate.exists() {
                return Ok(candidate);
            }
        }
    }

    // Check target/debug or target/release if developing locally
    for fallback in &["target/debug/agmuxd", "target/release/agmuxd"] {
        let p = PathBuf::from(fallback);
        if p.exists() {
            return Ok(p);
        }
    }

    // Fall back to searching PATH
    Ok(PathBuf::from("agmuxd"))
}

/// Ensure the background daemon process is alive, spawning it detached if inactive.
///
/// If the daemon socket is not responsive, this spawns `agmuxd` detached
/// and polls for readiness every 50ms up to 3 seconds.
pub async fn ensure_daemon_running(socket_path: &Path, token_path: &Path) -> Result<()> {
    if UnixStream::connect(socket_path).await.is_ok() {
        return Ok(());
    }

    let daemon_bin = find_daemon_binary()?;
    tracing::info!(
        "Daemon inactive on {}. Auto-spawning {}...",
        socket_path.display(),
        daemon_bin.display()
    );

    let mut cmd = std::process::Command::new(&daemon_bin);
    cmd.arg("--socket").arg(socket_path);
    cmd.arg("--token-path").arg(token_path);
    cmd.stdin(std::process::Stdio::null());
    cmd.stdout(std::process::Stdio::null());
    cmd.stderr(std::process::Stdio::null());

    let mut child = cmd.spawn().with_context(|| {
        format!(
            "Failed to auto-spawn daemon binary {}",
            daemon_bin.display()
        )
    })?;

    // Poll socket readiness with 50ms intervals up to 3 seconds (60 attempts)
    let max_attempts = 60;
    let poll_interval = Duration::from_millis(50);

    for attempt in 1..=max_attempts {
        tokio::time::sleep(poll_interval).await;

        if UnixStream::connect(socket_path).await.is_ok() {
            tracing::info!(
                "Daemon successfully auto-spawned and ready after {}ms",
                attempt * 50
            );
            return Ok(());
        }

        // Check if child exited prematurely
        if let Ok(Some(status)) = child.try_wait() {
            bail!("Daemon process exited prematurely with status: {status}");
        }
    }

    bail!(
        "Timed out waiting 3s for daemon socket {} to become ready",
        socket_path.display()
    );
}
