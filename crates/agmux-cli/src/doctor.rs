//! Environment diagnostic analyzer (`agmux doctor`) and automated permission/lockfile repair (`agmux daemon repair`) (`REQ-REMOTE-DOCTOR-REPAIR-004`).

use std::path::Path;

use portable_pty::{PtySize, native_pty_system};
use serde::{Deserialize, Serialize};

/// Diagnostic report produced by `agmux doctor`.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct DoctorReport {
    pub healthy: bool,
    pub pty_available: bool,
    pub token_permissions_ok: bool,
    pub token_mode: u32,
    pub db_accessible: bool,
    pub discovered_agents: Vec<String>,
    pub active_remotes: Vec<String>,
    pub issues: Vec<String>,
}

/// Run environment diagnostics on PTY allocation, `daemon.token` permissions (`0600` on Unix), and SQLite `state.db`.
pub fn run_doctor(token_path: &Path, db_path: &Path, active_remotes: Vec<String>) -> DoctorReport {
    let mut issues = Vec::new();

    // 1. Check local PTY allocation
    let pty_available = native_pty_system()
        .openpty(PtySize {
            rows: 24,
            cols: 80,
            pixel_width: 0,
            pixel_height: 0,
        })
        .is_ok();
    if !pty_available {
        issues.push("Local PTY allocation failed".to_string());
    }

    // 2. Check token file existence and permissions (0600 on Unix)
    let (token_permissions_ok, token_mode) = check_token_permissions(token_path);
    if !token_permissions_ok {
        issues.push(format!(
            "Token file {} has insecure or missing permissions (mode {:04o}; expected 0600). Run `agmux daemon repair`.",
            token_path.display(),
            token_mode
        ));
    }

    // 3. Check SQLite database accessibility
    let db_accessible = agmux_core::storage_engine::StorageEngine::open(db_path).is_ok();
    if !db_accessible {
        issues.push(format!(
            "Database {} could not be opened in WAL mode",
            db_path.display()
        ));
    }

    let discovered_agents: Vec<String> = agmux_core::agent_discovery::discover_installed_agents()
        .into_iter()
        .filter(|a| a.available)
        .map(|a| a.binary_name)
        .collect();

    DoctorReport {
        healthy: issues.is_empty(),
        pty_available,
        token_permissions_ok,
        token_mode,
        db_accessible,
        discovered_agents,
        active_remotes,
        issues,
    }
}

/// Repair broken `daemon.token` file permissions (`0600` on Unix) and clean up stale lockfiles.
pub fn repair_daemon_env(
    token_path: &Path,
    socket_path: &Path,
) -> Result<Vec<String>, std::io::Error> {
    let mut actions = Vec::new();

    if !token_path.exists() {
        let _ = agmux_core::get_or_create_token(token_path)
            .map_err(|e| std::io::Error::other(e.to_string()))?;
        actions.push(format!(
            "Created missing token file {}",
            token_path.display()
        ));
    }

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        std::fs::set_permissions(token_path, std::fs::Permissions::from_mode(0o600))?;
        actions.push(format!(
            "Restored 0600 permissions on {}",
            token_path.display()
        ));
    }

    let lock_path = socket_path.with_extension("lock");
    if lock_path.exists()
        && let Ok(content) = std::fs::read_to_string(&lock_path)
        && let Ok(pid) = content.trim().parse::<u32>()
        && !is_pid_alive(pid)
    {
        let _ = std::fs::remove_file(&lock_path);
        actions.push(format!(
            "Removed stale lockfile {} (dead PID {})",
            lock_path.display(),
            pid
        ));
    }

    Ok(actions)
}

fn check_token_permissions(token_path: &Path) -> (bool, u32) {
    let Ok(meta) = std::fs::metadata(token_path) else {
        return (false, 0);
    };
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mode = meta.permissions().mode() & 0o777;
        (mode == 0o600, mode)
    }
    #[cfg(not(unix))]
    {
        let _ = meta;
        (true, 0o600)
    }
}

fn is_pid_alive(pid: u32) -> bool {
    if pid == 0 {
        return false;
    }
    #[cfg(unix)]
    unsafe {
        libc::kill(pid as libc::pid_t, 0) == 0
            || std::io::Error::last_os_error().raw_os_error() == Some(libc::EPERM)
    }
    #[cfg(not(unix))]
    {
        let _ = pid;
        false
    }
}
