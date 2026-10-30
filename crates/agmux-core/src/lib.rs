//! Core domain types, PTY process management, and token authentication.

pub mod agent_discovery;
pub mod auth;
pub mod config_manager;
pub mod diff_calculator;
pub mod prompt_flattener;
pub mod pty_manager;
pub mod scratchpad_notes;
pub mod snapshot_compressor;
pub mod state_machine;
pub mod storage_engine;
pub mod worktree_manager;

use std::path::PathBuf;

/// Default socket path resolution across Linux/macOS/Windows.
///
/// Priority:
/// 1. `AGMUX_SOCK` environment variable if set.
/// 2. `\\.\pipe\agmuxd-%USERNAME%` on Windows, or `/tmp/agmux-$USER.sock` on Unix.
/// 3. Fallback to `/tmp/agmux-default.sock` or `\\.\pipe\agmuxd-default`.
pub fn default_socket_path() -> PathBuf {
    if let Ok(val) = std::env::var("AGMUX_SOCK") {
        if !val.is_empty() {
            return PathBuf::from(val);
        }
    }

    #[cfg(windows)]
    {
        let user = std::env::var("USERNAME").unwrap_or_else(|_| "default".to_string());
        PathBuf::from(format!(r"\\.\pipe\agmuxd-{user}"))
    }

    #[cfg(not(windows))]
    {
        let user = std::env::var("USER")
            .or_else(|_| std::env::var("UID"))
            .unwrap_or_else(|_| "default".to_string());

        PathBuf::from(format!("/tmp/agmux-{user}.sock"))
    }
}

pub use auth::{
    AuthError, default_token_path, generate_random_32_bytes, get_or_create_token, validate_token,
};
pub use pty_manager::{ActivePtySession, PtyError, PtyManager};

/// The current package version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Errors exposed by the core library.
#[derive(Debug, thiserror::Error)]
pub enum AgmuxError {
    /// An underlying input/output operation failed.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Authentication failure.
    #[error("Authentication error: {0}")]
    Auth(#[from] AuthError),

    /// PTY allocation or process failure.
    #[error("PTY error: {0}")]
    Pty(#[from] PtyError),
}

pub mod fts_indexer;

pub mod builtin_plugins;
pub mod child_process_monitor;
pub mod cli_detector;
pub mod comment_tracker;
pub mod debounce_timer;
pub mod declarative_plugin;
pub mod plugin_api;
pub mod pty_resizer;
pub mod session_tracker;
pub mod shell_classifier;
pub mod stuck_detector;
pub mod subtab_spawner;
pub mod tail_matcher;
pub mod vt_parser;
pub mod wsl_bridge;
