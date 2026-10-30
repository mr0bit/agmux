//! Deferred teardown coordinated with session completion and process exit.
use crate::{
    event_broadcaster::EventBroadcaster, file_reservation_table::FileReservationTable,
    kanban_manager::KanbanManager, lock_manager::LockManager,
};
use agmux_core::{pty_manager::PtyManager, storage_engine::StorageEngine};
use std::sync::{Arc, Mutex};

#[derive(Clone)]
pub struct TeardownScheduler {
    db: Arc<Mutex<StorageEngine>>,
    ptys: PtyManager,
    gate: Arc<tokio::sync::Mutex<()>>,
    broadcaster: EventBroadcaster,
    locks: LockManager,
    files: FileReservationTable,
    tasks: KanbanManager,
}
impl TeardownScheduler {
    pub fn new(
        db: Arc<Mutex<StorageEngine>>,
        ptys: PtyManager,
        broadcaster: EventBroadcaster,
        locks: LockManager,
        files: FileReservationTable,
        tasks: KanbanManager,
    ) -> Self {
        Self {
            db,
            ptys,
            gate: Arc::new(tokio::sync::Mutex::new(())),
            broadcaster,
            locks,
            files,
            tasks,
        }
    }
    pub async fn request(&self, id: &str, action: &str) -> Result<(), String> {
        if !matches!(action, "archive" | "kill" | "cancel") {
            return Err("Invalid teardown action".into());
        }
        let guard = self.gate.lock().await;
        let active = self
            .ptys
            .get_session(id)
            .await
            .map_err(|_| format!("Session is not active: {id}"))?;
        if active.subscribe_exit().borrow().is_some() {
            return Err(format!("Session is not active: {id}"));
        }
        let state = {
            let db = self.db.lock().map_err(|e| e.to_string())?;
            let session = db
                .get_session(id)
                .map_err(|e| e.to_string())?
                .ok_or_else(|| format!("Session not found: {id}"))?;
            if matches!(
                session.state.as_str(),
                "DEAD" | "ARCHIVED" | "FAILED" | "ERRORED"
            ) {
                return Err("Session is not active".into());
            }
            db.set_teardown(
                id,
                if action == "cancel" {
                    None
                } else {
                    Some(action)
                },
            )
            .map_err(|e| e.to_string())?;
            session.state
        };
        drop(guard);
        self.broadcaster.broadcast_session_list().await;
        if action != "cancel" && matches!(state.as_str(), "IDLE" | "FINISHED" | "COMPLETED") {
            self.complete(id).await?;
        }
        Ok(())
    }
    /// Consume the pending action once and persist a snapshot and final state.
    pub async fn complete(&self, id: &str) -> Result<bool, String> {
        let _guard = self.gate.lock().await;
        let session = self.ptys.get_session(id).await.ok();
        let action = {
            let db = self.db.lock().map_err(|e| e.to_string())?;
            db.list_teardowns().map_err(|e| e.to_string())?.remove(id)
        };
        let Some(action) = action else {
            return Ok(false);
        };
        if let Some(session) = session.as_ref() {
            let screen = session.capture_screen();
            let compressed = agmux_core::snapshot_compressor::compress_snapshot(&screen)
                .map_err(|e| e.to_string())?;
            let db = self.db.lock().map_err(|e| e.to_string())?;
            db.save_snapshot(id, &compressed, screen.len())
                .map_err(|e| e.to_string())?;
        }
        {
            let db = self.db.lock().map_err(|e| e.to_string())?;
            db.finish_teardown(
                id,
                if action == "archive" {
                    "ARCHIVED"
                } else {
                    "DEAD"
                },
            )
            .map_err(|e| e.to_string())?;
        }
        if let Some(session) = session.filter(|s| s.subscribe_exit().borrow().is_none()) {
            #[cfg(unix)]
            {
                let _ = crate::process_supervisor::kill_process_group(session.pid, libc::SIGKILL);
            }
            session.kill();
        }
        self.ptys.remove_session(id).await;
        self.locks.evict_session(id).await;
        self.files.evict_session(id).await;
        let _ = self.tasks.evict_session(id).await;
        self.broadcaster.broadcast_session_list().await;
        Ok(true)
    }
}
