//! Multi-agent hierarchical file reservation coordinator.

use std::sync::Arc;

use agmux_core::storage_engine::StorageEngine;
use agmux_proto::FileReservationEntry;

/// Coordinates file reservations with prefix exclusion.
#[derive(Clone)]
pub struct FileReservationTable {
    db: Arc<std::sync::Mutex<StorageEngine>>,
}

impl FileReservationTable {
    /// Create a new `FileReservationTable`.
    pub fn new(db: Arc<std::sync::Mutex<StorageEngine>>) -> Self {
        Self { db }
    }

    /// Reserves a batch of paths for a session. If any path conflicts with an active reservation
    /// held by another session, fails and returns the conflict details.
    pub async fn reserve(
        &self,
        session_id: &str,
        paths: &[String],
    ) -> Result<Result<(), (String, String)>, String> {
        let db = self.db.lock().map_err(|e| e.to_string())?;
        for path in paths {
            match db
                .reserve_file(path, session_id)
                .map_err(|e| e.to_string())?
            {
                Ok(()) => {}
                Err((conflict, holder)) => return Ok(Err((conflict, holder))),
            }
        }
        Ok(Ok(()))
    }

    /// Release specific paths for a session.
    pub async fn release(&self, session_id: &str, paths: &[String]) -> Result<usize, String> {
        let db = self.db.lock().map_err(|e| e.to_string())?;
        let mut count = 0;
        for path in paths {
            if db
                .release_file(path, session_id)
                .map_err(|e| e.to_string())?
            {
                count += 1;
            }
        }
        Ok(count)
    }

    /// Evicts all file reservations held by a session (e.g. on exit or error).
    pub async fn evict_session(&self, session_id: &str) -> Vec<String> {
        if let Ok(db) = self.db.lock() {
            db.release_session_reservations(session_id)
                .unwrap_or_default()
        } else {
            Vec::new()
        }
    }

    /// List all active reservations.
    pub async fn list(&self) -> Result<Vec<FileReservationEntry>, String> {
        let db = self.db.lock().map_err(|e| e.to_string())?;
        let reservations = db.list_reservations().map_err(|e| e.to_string())?;
        Ok(reservations
            .into_iter()
            .map(
                |(path, holder_session_id, acquired_at)| FileReservationEntry {
                    path,
                    holder_session_id,
                    acquired_at,
                },
            )
            .collect())
    }
}
