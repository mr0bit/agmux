//! Core domain types, PTY process management, and token authentication.

pub mod auth;
pub mod pty_manager;
pub mod snapshot_compressor;
pub mod state_machine;
pub mod storage_engine;

use std::path::PathBuf;

/// Default socket path resolution across Linux/macOS.
///
/// Priority:
/// 1. `AGMUX_SOCK` environment variable if set.
/// 2. `/tmp/agmux-$USER.sock` (or `/tmp/agmux-$UID.sock`).
/// 3. Fallback to `/tmp/agmux-default.sock`.
pub fn default_socket_path() -> PathBuf {
    if let Ok(val) = std::env::var("AGMUX_SOCK") {
        if !val.is_empty() {
            return PathBuf::from(val);
        }
    }

    let user = std::env::var("USER")
        .or_else(|_| std::env::var("UID"))
        .unwrap_or_else(|_| "default".to_string());

    PathBuf::from(format!("/tmp/agmux-{user}.sock"))
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
