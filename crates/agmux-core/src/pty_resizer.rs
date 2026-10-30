//! Terminal resize and VT100 grid reflow coordinator.

use crate::pty_manager::{ActivePtySession, PtyError, PtyManager};

/// Coordinates dynamic terminal resizing across OS PTY masters and in-memory VT100 buffers.
#[derive(Clone)]
pub struct PtyResizer {
    pty_manager: PtyManager,
}

impl PtyResizer {
    /// Create a new `PtyResizer`.
    pub fn new(pty_manager: PtyManager) -> Self {
        Self { pty_manager }
    }

    /// Resize a session's PTY master and in-memory screen grid to `(cols, rows)`.
    pub async fn resize_session(
        &self,
        session_id: &str,
        cols: u16,
        rows: u16,
    ) -> Result<(), PtyError> {
        let session = self.pty_manager.get_session(session_id).await?;
        Self::resize_active(&session, cols, rows)
    }

    /// Resize an `ActivePtySession` directly.
    pub fn resize_active(session: &ActivePtySession, cols: u16, rows: u16) -> Result<(), PtyError> {
        session.resize(cols, rows)
    }
}
