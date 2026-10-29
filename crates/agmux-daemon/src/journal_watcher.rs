//! Journal event streaming and replay coordinator.

use std::sync::Arc;
use tokio::sync::broadcast;

use agmux_core::storage_engine::StorageEngine;
use agmux_proto::JournalEvent;

/// Coordinates monotonic journal persistence, live broadcasting, and cursor replay.
#[derive(Clone)]
pub struct JournalWatcher {
    db: Arc<std::sync::Mutex<StorageEngine>>,
    live_tx: broadcast::Sender<JournalEvent>,
}

impl JournalWatcher {
    /// Create a new `JournalWatcher`.
    pub fn new(db: Arc<std::sync::Mutex<StorageEngine>>) -> Self {
        let (live_tx, _) = broadcast::channel(1024);
        Self { db, live_tx }
    }

    /// Record an event into the SQLite event journal and broadcast it live to subscribers.
    pub async fn publish_event(
        &self,
        event_type: &str,
        session_id: Option<&str>,
        payload: &str,
    ) -> Result<JournalEvent, String> {
        let db = self.db.clone();
        let et = event_type.to_string();
        let sid = session_id.map(|s| s.to_string());
        let pl = payload.to_string();
        let event = tokio::task::spawn_blocking(move || {
            let db = db.lock().map_err(|e| e.to_string())?;
            db.record_event(&et, sid.as_deref(), &pl)
                .map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| e.to_string())??;

        let _ = self.live_tx.send(event.clone());
        Ok(event)
    }

    /// Subscribe to real-time live events.
    pub fn subscribe(&self) -> broadcast::Receiver<JournalEvent> {
        self.live_tx.subscribe()
    }

    /// Read historical events since a sequence number.
    pub async fn read_since(&self, since_seq: u64) -> Result<Vec<JournalEvent>, String> {
        let db = self.db.clone();
        tokio::task::spawn_blocking(move || {
            let db = db.lock().map_err(|e| e.to_string())?;
            db.read_events_since(since_seq, None)
                .map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| e.to_string())?
    }

    /// Retrieve the last recorded sequence number for a cursor.
    pub async fn get_cursor(&self, cursor_id: &str) -> Result<u64, String> {
        let db = self.db.clone();
        let cid = cursor_id.to_string();
        tokio::task::spawn_blocking(move || {
            let db = db.lock().map_err(|e| e.to_string())?;
            db.get_cursor(&cid).map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| e.to_string())?
    }

    /// Persist cursor sequence position.
    pub async fn set_cursor(&self, cursor_id: &str, seq: u64) -> Result<(), String> {
        let db = self.db.clone();
        let cid = cursor_id.to_string();
        tokio::task::spawn_blocking(move || {
            let db = db.lock().map_err(|e| e.to_string())?;
            db.set_cursor(&cid, seq).map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| e.to_string())?
    }
}
