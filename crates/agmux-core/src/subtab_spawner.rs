//! Interactive shell sub-tab spawner inheriting parent session worktree and environment.

use std::path::PathBuf;
use std::sync::Arc;

use crate::pty_manager::{ActivePtySession, PtyError, PtyManager};
use crate::worktree_manager::port_for_branch;

/// Resolved configuration for spawning a plain shell sub-tab attached to a parent session.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SubtabConfig {
    pub parent_session_id: String,
    pub subtab_id: String,
    pub shell: String,
    pub cwd: PathBuf,
    pub port: u16,
}

/// Spawns interactive shell sub-tabs inheriting parent worktree CWD and deterministic PORT.
#[derive(Clone)]
pub struct SubtabSpawner {
    pty_manager: PtyManager,
}

impl SubtabSpawner {
    /// Create a new `SubtabSpawner`.
    pub fn new(pty_manager: PtyManager) -> Self {
        Self { pty_manager }
    }

    /// Resolve the default interactive shell for the host OS.
    pub fn default_shell() -> String {
        #[cfg(windows)]
        {
            std::env::var("COMSPEC").unwrap_or_else(|_| "powershell.exe".to_string())
        }
        #[cfg(not(windows))]
        {
            std::env::var("SHELL")
                .ok()
                .filter(|s| !s.trim().is_empty())
                .unwrap_or_else(|| "sh".to_string())
        }
    }

    /// Build a `SubtabConfig` inheriting the parent session's `cwd` and deterministic `port`.
    pub async fn build_config(
        &self,
        parent_session_id: &str,
        subtab_id: &str,
        shell: Option<String>,
        fallback_cwd: Option<PathBuf>,
    ) -> Result<SubtabConfig, PtyError> {
        let (cwd, port) = if let Ok(parent) = self.pty_manager.get_session(parent_session_id).await
        {
            let dir =
                parent.cwd.clone().or(fallback_cwd).unwrap_or_else(|| {
                    std::env::current_dir().unwrap_or_else(|_| PathBuf::from("."))
                });
            let p = parent
                .port
                .unwrap_or_else(|| port_for_branch(parent_session_id));
            (dir, p)
        } else {
            let dir = fallback_cwd
                .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| PathBuf::from(".")));
            let p = port_for_branch(parent_session_id);
            (dir, p)
        };

        Ok(SubtabConfig {
            parent_session_id: parent_session_id.to_string(),
            subtab_id: subtab_id.to_string(),
            shell: shell
                .filter(|s| !s.trim().is_empty())
                .unwrap_or_else(Self::default_shell),
            cwd,
            port,
        })
    }

    /// Spawn the interactive shell sub-tab inside `PtyManager`.
    pub async fn spawn_subtab(
        &self,
        config: &SubtabConfig,
    ) -> Result<Arc<ActivePtySession>, PtyError> {
        self.pty_manager
            .spawn_with_env(
                config.subtab_id.clone(),
                config.shell.clone(),
                Vec::new(),
                Some(config.cwd.clone()),
                Some(config.port),
                vec![(
                    "AGMUX_PARENT_SESSION_ID".to_string(),
                    config.parent_session_id.clone(),
                )],
            )
            .await
    }
}
