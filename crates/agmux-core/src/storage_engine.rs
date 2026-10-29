//! SQLite session metadata repository.
use agmux_proto::{SearchMatch, SessionInfo};
use rusqlite::{Connection, OptionalExtension, params};
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
        tx.execute_batch("CREATE VIRTUAL TABLE IF NOT EXISTS terminal_logs_fts USING fts5(session_id UNINDEXED, line_no UNINDEXED, timestamp UNINDEXED, content, tokenize='trigram');")?;
        tx.execute_batch("CREATE TABLE IF NOT EXISTS session_snapshots (session_id TEXT PRIMARY KEY, snapshot BLOB, compressed_size INTEGER, uncompressed_size INTEGER, timestamp INTEGER);")?;
        tx.commit()?;
        Ok(Self { connection })
    }
    /// Insert one clean terminal line.
    pub fn index_terminal_line(
        &self,
        session_id: &str,
        line_no: u64,
        timestamp: u64,
        content: &str,
    ) -> Result<(), StorageError> {
        self.connection.execute("INSERT INTO terminal_logs_fts (session_id,line_no,timestamp,content) VALUES (?1,?2,?3,?4)", params![session_id,line_no,timestamp,content])?;
        Ok(())
    }
    /// Atomically insert a batch of consecutive terminal lines.
    pub fn index_terminal_lines(
        &mut self,
        session_id: &str,
        first_line: u64,
        timestamp: u64,
        lines: &[String],
    ) -> Result<(), StorageError> {
        let tx = self.connection.transaction()?;
        for (offset, line) in lines.iter().enumerate() {
            tx.execute("INSERT INTO terminal_logs_fts (session_id,line_no,timestamp,content) VALUES (?1,?2,?3,?4)", params![session_id, first_line + offset as u64, timestamp, line])?;
        }
        tx.commit()?;
        Ok(())
    }
    /// Search terminal lines for a literal substring, optionally within a session.
    pub fn search_terminal_logs(
        &self,
        query: &str,
        limit: u32,
    ) -> Result<Vec<SearchMatch>, StorageError> {
        self.search_terminal_logs_for_session(query, None, limit)
    }
    /// Search terminal lines within an optional session.
    pub fn search_terminal_logs_for_session(
        &self,
        query: &str,
        session_id: Option<&str>,
        limit: u32,
    ) -> Result<Vec<SearchMatch>, StorageError> {
        if query.is_empty() {
            return Ok(Vec::new());
        }
        let (sql, pattern) = if query.chars().count() >= 3 {
            (
                "SELECT session_id,line_no,timestamp,content FROM terminal_logs_fts WHERE terminal_logs_fts MATCH ?1 AND (?2 IS NULL OR session_id=?2) ORDER BY rowid DESC LIMIT ?3",
                format!("\"{}\"", query.replace('"', "\"\"")),
            )
        } else {
            (
                "SELECT session_id,line_no,timestamp,content FROM terminal_logs_fts WHERE instr(lower(content),lower(?1)) > 0 AND (?2 IS NULL OR session_id=?2) ORDER BY rowid DESC LIMIT ?3",
                query.to_string(),
            )
        };
        let mut stmt = self.connection.prepare(sql)?;
        let rows = stmt.query_map(params![pattern, session_id, limit], |r| {
            Ok(SearchMatch {
                session_id: r.get(0)?,
                line_no: r.get(1)?,
                timestamp: r.get(2)?,
                snippet: r.get(3)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
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
        let mut stmt = self.connection.prepare("SELECT id,pid,cwd,command,created_at,state FROM sessions WHERE state NOT IN ('DEAD','ERRORED','COMPLETED','FAILED') ORDER BY created_at,id")?;
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
    /// Look up a session regardless of its lifecycle state.
    pub fn get_session(&self, id: &str) -> Result<Option<SessionInfo>, StorageError> {
        Ok(self
            .connection
            .query_row(
                "SELECT id,pid,cwd,command,created_at,state FROM sessions WHERE id=?1",
                [id],
                |r| {
                    Ok(SessionInfo {
                        session_id: r.get(0)?,
                        pid: r.get(1)?,
                        cwd: r.get(2)?,
                        command: r.get(3)?,
                        created_at: r.get(4)?,
                        state: r.get(5)?,
                    })
                },
            )
            .optional()?)
    }
    pub fn remove_session(&self, id: &str) -> Result<(), StorageError> {
        self.connection
            .execute("DELETE FROM sessions WHERE id=?1", [id])?;
        Ok(())
    }
    /// Save a compressed snapshot for a session.
    pub fn save_snapshot(
        &self,
        session_id: &str,
        compressed_snapshot: &[u8],
        uncompressed_size: usize,
    ) -> Result<(), StorageError> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        self.connection.execute(
            "INSERT OR REPLACE INTO session_snapshots (session_id, snapshot, compressed_size, uncompressed_size, timestamp) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![session_id, compressed_snapshot, compressed_snapshot.len() as u64, uncompressed_size as u64, now],
        )?;
        Ok(())
    }
    /// Load a compressed snapshot for a session.
    pub fn load_snapshot(&self, session_id: &str) -> Result<Option<Vec<u8>>, StorageError> {
        let mut stmt = self
            .connection
            .prepare("SELECT snapshot FROM session_snapshots WHERE session_id = ?1")?;
        let mut rows = stmt.query(params![session_id])?;
        if let Some(row) = rows.next()? {
            let blob: Vec<u8> = row.get(0)?;
            Ok(Some(blob))
        } else {
            Ok(None)
        }
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
    fn search_literal_and_session_filter() {
        let db = StorageEngine::open(Path::new(":memory:")).expect("open");
        db.index_terminal_line("one", 1, 42, "CRITICAL_AGMUX_TOKEN_XYZ123")
            .expect("index");
        db.index_terminal_line("two", 2, 43, "TOKEN-XYZ")
            .expect("index");
        let matches = db.search_terminal_logs("TOKEN_XYZ", 10).expect("search");
        assert_eq!(matches.len(), 1);
        assert_eq!(matches[0].session_id, "one");
        assert!(
            db.search_terminal_logs_for_session("TOKEN", Some("missing"), 10)
                .expect("search")
                .is_empty()
        );
    }
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
