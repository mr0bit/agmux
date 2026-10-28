//! Core domain types and error boundary.

/// The current package version.
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Errors exposed by the core library.
#[derive(Debug, thiserror::Error)]
pub enum AgmuxError {
    /// An underlying input/output operation failed.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}
