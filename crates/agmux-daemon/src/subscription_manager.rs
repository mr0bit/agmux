//! Viewport subscription coordinator emitting atomic `ScreenSyncFrame` snapshots.

use tokio::sync::broadcast;

use agmux_core::pty_manager::{PtyError, PtyManager};
use agmux_proto::ScreenSyncFrame;

/// Coordinates client `SubscribeViewport` requests and builds atomic `ScreenSyncFrame` snapshots.
#[derive(Clone)]
pub struct SubscriptionManager {
    pty_manager: PtyManager,
}

impl SubscriptionManager {
    /// Create a new `SubscriptionManager`.
    pub fn new(pty_manager: PtyManager) -> Self {
        Self { pty_manager }
    }

    /// Subscribe to a session's viewport, returning an atomic `ScreenSyncFrame` and live output receiver.
    pub async fn subscribe_viewport(
        &self,
        session_id: &str,
    ) -> Result<(ScreenSyncFrame, broadcast::Receiver<Vec<u8>>), PtyError> {
        let session = self.pty_manager.get_session(session_id).await?;
        Ok(session.subscribe_viewport_sync())
    }
}
