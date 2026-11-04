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
    dispatcher: crate::notification_dispatcher::NotificationDispatcher,
    config: crate::config_watcher::ConfigWatcher,
}

impl EventBroadcaster {
    /// Signal that transcript search data changed without rebuilding the session list.
    pub fn transcript_indexed(&self, session_id: &str) {
        let _ = self
            .tx
            .send(IpcMessage::NotificationEvent(NotificationEvent {
                session_id: session_id.into(),
                kind: "search.index".into(),
                title: String::new(),
                body: String::new(),
                timestamp: 0,
            }));
    }
    /// Create a new `EventBroadcaster`.
    pub fn new(
        db: Arc<Mutex<StorageEngine>>,
        config: crate::config_watcher::ConfigWatcher,
    ) -> Self {
        let (tx, _) = broadcast::channel(512);
        Self {
            db,
            tx,
            dispatcher: Default::default(),
            config,
        }
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
            let teardowns = self
                .db
                .lock()
                .ok()
                .and_then(|db| db.list_teardowns().ok())
                .unwrap_or_default();
            let _ = self
                .tx
                .send(IpcMessage::SessionListUpdate(SessionListUpdate {
                    sessions,
                    teardowns,
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
        output: &str,
    ) {
        let kind = match new_state {
            "WAITING_INPUT" => Some("WAITING"),
            "COMPLETED" => Some("FINISHED"),
            "FAILED" => Some("ERRORED"),
            _ => None,
        };
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let body = crate::notification_dispatcher::latest_quote(output);
        let title = format!("Session {session_id} -> {}", kind.unwrap_or(new_state));
        let event = NotificationEvent {
            session_id: session_id.to_string(),
            kind: format!("state.{}", kind.unwrap_or(new_state)),
            title: title.clone(),
            body: if kind.is_some() {
                body.clone()
            } else {
                format!("{old_state} -> {new_state} ({reason})")
            },
            timestamp,
        };
        if kind.is_some() {
            let db = self.db.clone();
            let saved = event.clone();
            let _ = tokio::task::spawn_blocking(move || {
                if let Ok(db) = db.lock() {
                    let _ = db.save_companion_notification(&saved);
                }
            })
            .await;
        }
        let _ = self.tx.send(IpcMessage::NotificationEvent(event));
        if let (Some(kind), Some(permit)) = (kind, self.dispatcher.try_reserve()) {
            let db = self.db.clone();
            let id = session_id.to_owned();
            let config_path = self.config.path().to_owned();
            tokio::spawn(async move {
                let _permit = permit;
                let data = tokio::task::spawn_blocking(move || {
                    let config = std::fs::read_to_string(config_path).ok();
                    let hook = config
                        .as_deref()
                        .and_then(crate::config_watcher::notification_hook);
                    let push_cfg = config
                        .as_deref()
                        .and_then(crate::config_watcher::push_relay_config);
                    if hook.is_none() && push_cfg.is_none() {
                        return None;
                    }
                    let db = db.lock().ok()?;
                    let info = db.get_session(&id).ok().flatten()?;
                    let metadata = db.workspace_metadata().ok()?.remove(&id);
                    drop(db);
                    let branch = crate::notification_dispatcher::workspace_branch(&info.cwd);
                    Some((hook, push_cfg, info, metadata, branch))
                })
                .await
                .ok()
                .flatten();
                if let Some((hook, push_cfg, info, metadata, branch)) = data {
                    let tool = info
                        .command
                        .split_whitespace()
                        .next()
                        .unwrap_or(&info.command)
                        .to_owned();
                    let notification = crate::notification_dispatcher::HookNotification {
                        kind: kind.into(),
                        session_id: info.session_id.clone(),
                        session_name: metadata
                            .map(|m| m.project)
                            .filter(|s| !s.is_empty())
                            .unwrap_or(info.session_id),
                        tool,
                        cwd: info.cwd,
                        branch,
                        title,
                        body,
                    };
                    let hook_task = async {
                        if let Some(hook) = hook {
                            if let Err(error) =
                                crate::notification_dispatcher::run_hook(&hook, &notification).await
                            {
                                tracing::warn!(%error, "notification hook failed");
                            }
                        }
                    };
                    let push_task = async {
                        if let Some(push_cfg) = push_cfg {
                            if kind == "WAITING" || kind == "ERRORED" {
                                if let Err(error) =
                                    crate::notification_dispatcher::dispatch_push_relay(
                                        &push_cfg,
                                        &notification,
                                    )
                                    .await
                                {
                                    tracing::warn!(%error, "push relay failed");
                                }
                            }
                        }
                    };
                    tokio::join!(hook_task, push_task);
                }
            });
        }
        self.broadcast_session_list().await;
    }
}
