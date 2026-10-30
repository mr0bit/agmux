//! Asynchronous push event broadcaster (`NotificationEvent` and `SessionListUpdate`).

use std::sync::{Arc, Mutex};
use tokio::sync::broadcast;

use agmux_core::storage_engine::StorageEngine;
use agmux_proto::{IpcMessage, NotificationEvent, SessionListUpdate};

/// Broadcasts `NotificationEvent` and `SessionListUpdate` frames to connected IPC clients.
#[derive(Clone)]
pub struct EventBroadcaster {
    db: Arc<Mutex<StorageEngine>>,
    tx: broadcast::Sender<IpcMessage>,
}

impl EventBroadcaster {
    /// Create a new `EventBroadcaster`.
    pub fn new(db: Arc<Mutex<StorageEngine>>) -> Self {
        let (tx, _) = broadcast::channel(512);
        Self { db, tx }
    }

    /// Subscribe to asynchronous push messages (`NotificationEvent`, `SessionListUpdate`).
    pub fn subscribe(&self) -> broadcast::Receiver<IpcMessage> {
        self.tx.subscribe()
    }

    /// Broadcast an updated `SessionListUpdate` snapshot to all connected clients.
    pub async fn broadcast_session_list(&self) {
        let db = self.db.clone();
        if let Ok(Ok(sessions)) = tokio::task::spawn_blocking(move || {
            db.lock()
                .map_err(|e| e.to_string())?
                .list_active_sessions()
                .map_err(|e| e.to_string())
        })
        .await
        {
            let _ = self
                .tx
                .send(IpcMessage::SessionListUpdate(SessionListUpdate {
                    sessions,
                }));
        }
    }

    /// Broadcast a `NotificationEvent` and accompanying `SessionListUpdate`.
    pub async fn notify_transition(
        &self,
        session_id: &str,
        old_state: &str,
        new_state: &str,
        reason: &str,
    ) {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let event = NotificationEvent {
            session_id: session_id.to_string(),
            kind: format!("state.{new_state}"),
            title: format!("Session {session_id} -> {new_state}"),
            body: format!("{old_state} -> {new_state} ({reason})"),
            timestamp,
        };
        let _ = self.tx.send(IpcMessage::NotificationEvent(event));
        self.broadcast_session_list().await;
    }
}
