//! SQLite session metadata repository.
use agmux_proto::SessionInfo;
use rusqlite::{Connection, params};
use std::path::{Path, PathBuf};

/// SQLite repository. The daemon owns the connection.
pub struct StorageEngine {
    connection: Connection,
}

impl StorageEngine {
    /// Open the database and apply the initial migration atomically.
    pub fn open(path: &Path) -> Result<Self, StorageError> {
        if let Some(parent) = path.parent().filter(|p| !p.as_os_str().is_empty()) {
            std::fs::create_dir_all(parent)?;
        }
        let mut connection = Connection::open(path)?;
        connection.execute_batch(
            "PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL; PRAGMA temp_store=MEMORY;",
        )?;
        let tx = connection.transaction()?;
        let version: u32 = tx.pragma_query_value(None, "user_version", |row| row.get(0))?;
        if version == 0 {
            tx.execute_batch("CREATE TABLE IF NOT EXISTS sessions (id TEXT PRIMARY KEY, pid INTEGER, cwd TEXT, command TEXT, created_at INTEGER, state TEXT); PRAGMA user_version=1;")?;
        } else if version > 1 {
            return Err(StorageError::UnsupportedVersion(version));
        }
        tx.commit()?;
        Ok(Self { connection })
    }
    pub fn insert_session(&self, s: &SessionInfo) -> Result<(), StorageError> {
        self.connection.execute(
            "INSERT INTO sessions (id,pid,cwd,command,created_at,state) VALUES (?1,?2,?3,?4,?5,?6)",
            params![s.session_id, s.pid, s.cwd, s.command, s.created_at, s.state],
        )?;
        Ok(())
    }
    pub fn update_session_state(&self, id: &str, state: &str) -> Result<(), StorageError> {
        self.connection.execute(
            "UPDATE sessions SET state=?1 WHERE id=?2",
            params![state, id],
        )?;
        Ok(())
    }
    pub fn list_active_sessions(&self) -> Result<Vec<SessionInfo>, StorageError> {
        let mut stmt = self.connection.prepare("SELECT id,pid,cwd,command,created_at,state FROM sessions WHERE state NOT IN ('DEAD','ERRORED') ORDER BY created_at,id")?;
        let rows = stmt.query_map([], |r| {
            Ok(SessionInfo {
                session_id: r.get(0)?,
                pid: r.get(1)?,
                cwd: r.get(2)?,
                command: r.get(3)?,
                created_at: r.get(4)?,
                state: r.get(5)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }
    pub fn remove_session(&self, id: &str) -> Result<(), StorageError> {
        self.connection
            .execute("DELETE FROM sessions WHERE id=?1", [id])?;
        Ok(())
    }
}
/// Storage failures.
#[derive(Debug, thiserror::Error)]
pub enum StorageError {
    #[error("SQLite error: {0}")]
    Sqlite(#[from] rusqlite::Error),
    #[error("Storage I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("Unsupported database version {0}")]
    UnsupportedVersion(u32),
}
/// Platform data directory for the daemon database.
pub fn default_database_path() -> PathBuf {
    if let Some(path) = std::env::var_os("AGMUX_DB_PATH") {
        return path.into();
    }
    #[cfg(windows)]
    let base = std::env::var_os("LOCALAPPDATA").map(PathBuf::from);
    #[cfg(not(windows))]
    let base = std::env::var_os("XDG_DATA_HOME")
        .filter(|p| !p.is_empty())
        .map(PathBuf::from)
        .or_else(|| std::env::var_os("HOME").map(|p| PathBuf::from(p).join(".local/share")));
    base.unwrap_or_else(std::env::temp_dir)
        .join("agmux/state.db")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn session_lifecycle() {
        let db = StorageEngine::open(Path::new(":memory:")).expect("open database");
        let info = SessionInfo {
            session_id: "one".into(),
            pid: 42,
            cwd: "/tmp".into(),
            command: "sleep 2".into(),
            created_at: 1,
            state: "WORKING".into(),
        };
        db.insert_session(&info).expect("insert");
        assert_eq!(db.list_active_sessions().expect("list"), vec![info]);
        db.update_session_state("one", "DEAD").expect("update");
        assert!(db.list_active_sessions().expect("list").is_empty());
        db.remove_session("one").expect("remove");
    }
}
