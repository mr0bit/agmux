//! Autonomous Kanban board and structured worker reporting coordinator.

use std::sync::{Arc, Mutex};

use agmux_core::storage_engine::StorageEngine;
use agmux_proto::{KanbanTask, WorkerReport};

use crate::journal_watcher::JournalWatcher;

/// Coordinates shared Kanban task lifecycles and structured worker reports.
#[derive(Clone)]
pub struct KanbanManager {
    db: Arc<Mutex<StorageEngine>>,
    journal: Option<JournalWatcher>,
}

impl KanbanManager {
    /// Create a new `KanbanManager`.
    pub fn new(db: Arc<Mutex<StorageEngine>>) -> Self {
        Self { db, journal: None }
    }

    /// Create a new `KanbanManager` wired to a `JournalWatcher` for live event broadcasting.
    pub fn with_journal(db: Arc<Mutex<StorageEngine>>, journal: JournalWatcher) -> Self {
        Self {
            db,
            journal: Some(journal),
        }
    }

    async fn emit_event(&self, event_type: &str, session_id: Option<&str>, payload: &str) {
        if let Some(ref j) = self.journal {
            let _ = j.publish_event(event_type, session_id, payload).await;
        } else {
            let db = self.db.clone();
            let et = event_type.to_string();
            let sid = session_id.map(|s| s.to_string());
            let pl = payload.to_string();
            let _ = tokio::task::spawn_blocking(move || {
                if let Ok(db) = db.lock() {
                    let _ = db.record_event(&et, sid.as_deref(), &pl);
                }
            })
            .await;
        }
    }

    /// Create a new Kanban task in `"todo"` state and emit `task.created`.
    pub async fn create_task(
        &self,
        task_id: &str,
        title: &str,
        description: &str,
    ) -> Result<KanbanTask, String> {
        let db = self.db.clone();
        let tid = task_id.to_string();
        let t_title = title.to_string();
        let t_desc = description.to_string();
        let task = tokio::task::spawn_blocking(move || {
            let db = db.lock().map_err(|e| e.to_string())?;
            db.create_kanban_task(&tid, &t_title, &t_desc)
                .map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| e.to_string())??;
        self.emit_event("task.created", None, &format!("task_id={task_id}"))
            .await;
        Ok(task)
    }

    /// List Kanban tasks, optionally filtered by status.
    pub async fn list_tasks(&self, status: Option<&str>) -> Result<Vec<KanbanTask>, String> {
        let db = self.db.clone();
        let st = status.map(|s| s.to_string());
        tokio::task::spawn_blocking(move || {
            let db = db.lock().map_err(|e| e.to_string())?;
            db.list_kanban_tasks(st.as_deref())
                .map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| e.to_string())?
    }

    /// Claim a Kanban task for `session_id` (auto-creating if absent) and emit `task.claimed`.
    pub async fn claim_task(&self, task_id: &str, session_id: &str) -> Result<KanbanTask, String> {
        let db = self.db.clone();
        let tid = task_id.to_string();
        let sid = session_id.to_string();
        let task = tokio::task::spawn_blocking(move || {
            let db = db.lock().map_err(|e| e.to_string())?;
            db.claim_kanban_task(&tid, &sid).map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| e.to_string())??;
        self.emit_event(
            "task.claimed",
            Some(session_id),
            &format!("task_id={task_id}"),
        )
        .await;
        Ok(task)
    }

    /// Mark a Kanban task as finished (`"done"`) and emit `task.finished`.
    pub async fn finish_task(&self, task_id: &str, session_id: &str) -> Result<KanbanTask, String> {
        let db = self.db.clone();
        let tid = task_id.to_string();
        let sid = session_id.to_string();
        let task = tokio::task::spawn_blocking(move || {
            let db = db.lock().map_err(|e| e.to_string())?;
            db.finish_kanban_task(&tid, &sid).map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| e.to_string())??;
        self.emit_event(
            "task.finished",
            Some(session_id),
            &format!("task_id={task_id}"),
        )
        .await;
        Ok(task)
    }

    /// Release a claimed Kanban task back to `"todo"` and emit `task.released`.
    pub async fn release_task(
        &self,
        task_id: &str,
        session_id: &str,
    ) -> Result<KanbanTask, String> {
        let db = self.db.clone();
        let tid = task_id.to_string();
        let sid = session_id.to_string();
        let task = tokio::task::spawn_blocking(move || {
            let db = db.lock().map_err(|e| e.to_string())?;
            db.release_kanban_task(&tid, &sid)
                .map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| e.to_string())??;
        self.emit_event(
            "task.released",
            Some(session_id),
            &format!("task_id={task_id}"),
        )
        .await;
        Ok(task)
    }

    /// Evict all `in_progress` Kanban tasks claimed by a terminated or dead session back to `"todo"`.
    pub async fn evict_session(&self, session_id: &str) -> Vec<String> {
        let db = self.db.clone();
        let sid = session_id.to_string();
        let ids = tokio::task::spawn_blocking(move || {
            if let Ok(db) = db.lock() {
                db.release_session_kanban_tasks(&sid).unwrap_or_default()
            } else {
                Vec::new()
            }
        })
        .await
        .unwrap_or_default();
        for tid in &ids {
            self.emit_event("task.released", Some(session_id), &format!("task_id={tid}"))
                .await;
        }
        ids
    }

    /// Post a structured worker report for `task_id` and emit `report.posted`.
    pub async fn post_report(
        &self,
        task_id: &str,
        session_id: &str,
        summary: &str,
        pr_url: Option<&str>,
        commit_sha: Option<&str>,
    ) -> Result<WorkerReport, String> {
        let db = self.db.clone();
        let tid = task_id.to_string();
        let sid = session_id.to_string();
        let sum = summary.to_string();
        let pr = pr_url.map(|s| s.to_string());
        let sha = commit_sha.map(|s| s.to_string());
        let report = tokio::task::spawn_blocking(move || {
            let db = db.lock().map_err(|e| e.to_string())?;
            db.post_worker_report(&tid, &sid, &sum, pr.as_deref(), sha.as_deref())
                .map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| e.to_string())??;
        self.emit_event(
            "report.posted",
            Some(session_id),
            &format!("task_id={task_id}"),
        )
        .await;
        Ok(report)
    }

    /// Retrieve all structured worker reports for `task_id`.
    pub async fn view_reports(&self, task_id: &str) -> Result<Vec<WorkerReport>, String> {
        let db = self.db.clone();
        let tid = task_id.to_string();
        tokio::task::spawn_blocking(move || {
            let db = db.lock().map_err(|e| e.to_string())?;
            db.get_worker_reports(&tid).map_err(|e| e.to_string())
        })
        .await
        .map_err(|e| e.to_string())?
    }
}
