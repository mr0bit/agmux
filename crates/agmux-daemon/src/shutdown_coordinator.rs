//! Graceful child process group shutdown coordinator (`SIGHUP` -> `SIGTERM`).

use std::collections::HashSet;
use std::sync::{Arc, Mutex};
use std::time::Duration;

use agmux_core::pty_manager::PtyManager;

/// Tracks active child session PIDs and propagates `SIGHUP` followed by `SIGTERM` on daemon shutdown.
#[derive(Clone)]
pub struct ShutdownCoordinator {
    pty_manager: PtyManager,
    extra_pids: Arc<Mutex<HashSet<u32>>>,
}

impl ShutdownCoordinator {
    /// Create a new `ShutdownCoordinator` bound to the daemon's `PtyManager`.
    pub fn new(pty_manager: PtyManager) -> Self {
        Self {
            pty_manager,
            extra_pids: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    /// Explicitly register an active child PID to be terminated on daemon shutdown.
    pub fn register_pid(&self, pid: u32) {
        if let Ok(mut set) = self.extra_pids.lock() {
            set.insert(pid);
        }
    }

    /// Unregister a child PID after normal termination.
    pub fn unregister_pid(&self, pid: u32) {
        if let Ok(mut set) = self.extra_pids.lock() {
            set.remove(&pid);
        }
    }

    /// Gracefully propagate `SIGHUP` followed by `SIGTERM` to all active child process groups.
    /// Returns the number of active sessions signaled.
    pub async fn shutdown_all(&self) -> usize {
        let active = self.pty_manager.active_pids().await;
        let mut pids: Vec<u32> = active.into_iter().map(|(_, pid)| pid).collect();
        if let Ok(set) = self.extra_pids.lock() {
            for &pid in set.iter() {
                if !pids.contains(&pid) {
                    pids.push(pid);
                }
            }
        }

        if pids.is_empty() {
            return 0;
        }

        #[cfg(unix)]
        {
            for &pid in &pids {
                let p = pid as libc::pid_t;
                unsafe {
                    let _ = libc::kill(-p, libc::SIGHUP);
                    let _ = libc::kill(p, libc::SIGHUP);
                }
            }

            tokio::time::sleep(Duration::from_millis(60)).await;

            for &pid in &pids {
                let p = pid as libc::pid_t;
                unsafe {
                    let _ = libc::kill(-p, libc::SIGTERM);
                    let _ = libc::kill(p, libc::SIGTERM);
                }
            }
        }

        #[cfg(windows)]
        {
            for &pid in &pids {
                let _ = std::process::Command::new("taskkill")
                    .args(["/F", "/T", "/PID", &pid.to_string()])
                    .output();
            }
        }

        pids.len()
    }
}
