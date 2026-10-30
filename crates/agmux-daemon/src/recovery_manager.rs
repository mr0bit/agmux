//! Orphaned session crash recovery on daemon startup.

use agmux_core::storage_engine::{StorageEngine, StorageError};

/// Scans SQLite on daemon startup to transition orphaned active sessions to `DEAD`.
pub struct RecoveryManager;

impl RecoveryManager {
    /// Transition any sessions left in `RUNNING`, `WORKING`, `WAITING`, `WAITING_INPUT`, or `STUCK`
    /// to `DEAD`, release their held locks and file reservations, and record `session.recovered_dead`
    /// journal events. Preserves all compressed ANSI scrollback snapshots.
    pub fn recover_orphaned_sessions(db: &StorageEngine) -> Result<Vec<String>, StorageError> {
        let active = db.list_active_sessions()?;
        let mut recovered = Vec::new();
        for session in active {
            if matches!(
                session.state.as_str(),
                "RUNNING" | "WORKING" | "WAITING" | "WAITING_INPUT" | "STUCK"
            ) {
                db.update_session_state(&session.session_id, "DEAD")?;
                let _ = db.release_session_locks(&session.session_id)?;
                let _ = db.release_session_reservations(&session.session_id)?;
                let _ = db.release_session_kanban_tasks(&session.session_id)?;
                let _ = db.record_event(
                    "session.recovered_dead",
                    Some(&session.session_id),
                    &format!("prior_state={}", session.state),
                )?;
                recovered.push(session.session_id);
            }
        }
        Ok(recovered)
    }
}
