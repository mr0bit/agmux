//! In-memory waiter coordination and persistent named resource lock manager.

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;
use tokio::sync::{Mutex, oneshot};

use agmux_core::storage_engine::StorageEngine;
use agmux_proto::LockEntry;

/// Waiter awaiting a named lock grant.
struct LockWaiter {
    session_id: String,
    tx: oneshot::Sender<bool>,
}

/// Coordinates named resource locks and FIFO waiter queues with auto-eviction.
#[derive(Clone)]
pub struct LockManager {
    db: Arc<std::sync::Mutex<StorageEngine>>,
    waiters: Arc<Mutex<HashMap<String, VecDeque<LockWaiter>>>>,
}

impl LockManager {
    /// Create a new `LockManager`.
    pub fn new(db: Arc<std::sync::Mutex<StorageEngine>>) -> Self {
        Self {
            db,
            waiters: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    /// Attempts to acquire a lock. If wait is true and the lock is currently held,
    /// suspends the caller until the lock is acquired or the session is evicted/cancelled.
    pub async fn acquire(
        &self,
        name: &str,
        session_id: &str,
        wait: bool,
    ) -> Result<(bool, Option<String>), String> {
        // First try non-blocking acquisition
        {
            let db = self.db.lock().map_err(|e| e.to_string())?;
            if db
                .acquire_lock(name, session_id)
                .map_err(|e| e.to_string())?
            {
                return Ok((true, Some(session_id.to_string())));
            }
        }

        let current_holder = {
            let db = self.db.lock().map_err(|e| e.to_string())?;
            db.get_lock_holder(name).map_err(|e| e.to_string())?
        };

        if !wait {
            return Ok((false, current_holder));
        }

        // Register waiter
        let (tx, rx) = oneshot::channel();
        {
            let mut waiters = self.waiters.lock().await;
            waiters
                .entry(name.to_string())
                .or_default()
                .push_back(LockWaiter {
                    session_id: session_id.to_string(),
                    tx,
                });
        }

        // Wait to be woken
        match rx.await {
            Ok(acquired) => {
                if acquired {
                    Ok((true, Some(session_id.to_string())))
                } else {
                    Ok((false, None))
                }
            }
            Err(_) => {
                // Waiter was dropped or cancelled
                Ok((false, None))
            }
        }
    }

    /// Releases a lock held by session_id. If there are queued waiters for this lock,
    /// wakes the next waiter in FIFO order and grants them the lock.
    pub async fn release(&self, name: &str, session_id: &str) -> Result<bool, String> {
        let released = {
            let db = self.db.lock().map_err(|e| e.to_string())?;
            db.release_lock(name, session_id)
                .map_err(|e| e.to_string())?
        };

        if released {
            self.wake_next_waiter(name).await;
        }

        Ok(released)
    }

    /// Evicts all locks and pending waits for a session (e.g. on exit or error).
    /// For each released lock, wakes the next waiter in FIFO order.
    pub async fn evict_session(&self, session_id: &str) -> Vec<String> {
        // 1. Release held locks in DB
        let released_locks = {
            if let Ok(db) = self.db.lock() {
                db.release_session_locks(session_id).unwrap_or_default()
            } else {
                Vec::new()
            }
        };

        // 2. Remove session from any waiting queues
        {
            let mut waiters = self.waiters.lock().await;
            for q in waiters.values_mut() {
                q.retain(|w| w.session_id != session_id);
            }
        }

        // 3. For each released lock, wake the next waiter in FIFO order
        for name in &released_locks {
            self.wake_next_waiter(name).await;
        }

        released_locks
    }

    async fn wake_next_waiter(&self, name: &str) {
        let mut waiters = self.waiters.lock().await;
        if let Some(queue) = waiters.get_mut(name) {
            while let Some(waiter) = queue.pop_front() {
                // Try to acquire the lock for this waiter
                let acquired = {
                    if let Ok(db) = self.db.lock() {
                        db.acquire_lock(name, &waiter.session_id).unwrap_or(false)
                    } else {
                        false
                    }
                };

                if acquired {
                    if waiter.tx.send(true).is_ok() {
                        return;
                    } else {
                        // The waiter dropped before receiving grant; release and try next waiter
                        if let Ok(db) = self.db.lock() {
                            let _ = db.release_lock(name, &waiter.session_id);
                        }
                    }
                }
            }
        }
    }

    /// List all active locks.
    pub async fn list_locks(&self) -> Result<Vec<LockEntry>, String> {
        let db = self.db.lock().map_err(|e| e.to_string())?;
        let locks = db.list_locks().map_err(|e| e.to_string())?;
        Ok(locks
            .into_iter()
            .map(|(name, holder_session_id, acquired_at)| LockEntry {
                name,
                holder_session_id,
                acquired_at,
            })
            .collect())
    }
}
