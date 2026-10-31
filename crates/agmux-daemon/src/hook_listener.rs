//! Tier 1 explicit IPC hook listener (`agmux internal-hook --session <id> --event <event>`).

use std::sync::{Arc, Mutex};

use agmux_core::storage_engine::StorageEngine;
use agmux_proto::SessionStateChangedEvent;

use crate::event_broadcaster::EventBroadcaster;
use crate::journal_watcher::JournalWatcher;

/// Processes Tier 1 explicit agent lifecycle hook events (`start`, `stop`, `wait`, `error`).
#[derive(Clone)]
pub struct HookListener {
    db: Arc<Mutex<StorageEngine>>,
    events: tokio::sync::broadcast::Sender<SessionStateChangedEvent>,
    journal: JournalWatcher,
    broadcaster: EventBroadcaster,
    pty_manager: agmux_core::pty_manager::PtyManager,
}

impl HookListener {
    /// Create a new `HookListener`.
    pub fn new(
        db: Arc<Mutex<StorageEngine>>,
        events: tokio::sync::broadcast::Sender<SessionStateChangedEvent>,
        journal: JournalWatcher,
        broadcaster: EventBroadcaster,
        pty_manager: agmux_core::pty_manager::PtyManager,
    ) -> Self {
        Self {
            db,
            events,
            journal,
            broadcaster,
            pty_manager,
        }
    }

    /// Map a hook event name (`start`, `stop`, `wait`, `error`) to canonical session state.
    pub fn map_event_to_state(event: &str) -> Result<&'static str, String> {
        match event.trim().to_lowercase().as_str() {
            "start" | "working" | "running" => Ok("RUNNING"),
            "stop" | "finish" | "finished" | "completed" | "idle" => Ok("COMPLETED"),
            "wait" | "waiting" | "waiting_input" | "prompt" => Ok("WAITING_INPUT"),
            "error" | "errored" | "failed" | "fail" => Ok("FAILED"),
            other => Err(format!("Unsupported hook event '{other}'")),
        }
    }

    /// Handle an incoming Tier 1 hook event and broadcast the transition immediately.
    pub async fn handle_hook(&self, session_id: &str, event: &str) -> Result<String, String> {
        let new_state = Self::map_event_to_state(event)?;
        let db = self.db.clone();
        let sid = session_id.to_string();
        let ns = new_state.to_string();
        let old_state = tokio::task::spawn_blocking(move || -> Result<String, String> {
            let db = db.lock().map_err(|e| e.to_string())?;
            let old = db
                .get_session(&sid)
                .map_err(|e| e.to_string())?
                .map(|s| s.state)
                .ok_or_else(|| format!("Session not found: {sid}"))?;
            if matches!(old.as_str(), "DEAD" | "ARCHIVED" | "FAILED" | "ERRORED") {
                return Err("Session is not active".into());
            }
            db.update_session_state(&sid, &ns)
                .map_err(|e| e.to_string())?;
            Ok(old)
        })
        .await
        .map_err(|e| e.to_string())??;

        if old_state == new_state {
            return Ok(new_state.to_string());
        }

        let _ = self.events.send(SessionStateChangedEvent {
            session_id: session_id.to_string(),
            old_state: old_state.clone(),
            new_state: new_state.to_string(),
            reason: format!("tier1_hook:{event}"),
        });
        let output = match self.pty_manager.get_session(session_id).await {
            Ok(session) => session
                .subscribe_viewport_sync()
                .0
                .lines
                .join("\n")
                .trim_end()
                .to_owned(),
            Err(_) => String::new(),
        };
        self.broadcaster
            .notify_transition(
                session_id,
                &old_state,
                new_state,
                &format!("tier1_hook:{event}"),
                &output,
            )
            .await;
        let _ = self
            .journal
            .publish_event(
                "hook.transition",
                Some(session_id),
                &format!("{old_state} -> {new_state} ({event})"),
            )
            .await;

        Ok(new_state.to_string())
    }
}
