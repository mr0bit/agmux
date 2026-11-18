//! SQLite session metadata repository.
use agmux_proto::{
    JournalEvent, KanbanTask, ReviewComment, SearchMatch, SessionInfo, WorkerReport,
};
use rusqlite::{Connection, OptionalExtension, params};
use std::path::{Path, PathBuf};

/// Helper to determine if two relative file paths overlap hierarchically.
pub fn path_conflicts(a: &str, b: &str) -> bool {
    let clean_a = a.trim_matches('/');
    let clean_b = b.trim_matches('/');
    if clean_a.is_empty() || clean_b.is_empty() {
        return true;
    }
    if clean_a == clean_b {
        return true;
    }
    clean_a.starts_with(&format!("{clean_b}/")) || clean_b.starts_with(&format!("{clean_a}/"))
}

/// SQLite repository. The daemon owns the connection.
pub struct StorageEngine {
    pub(crate) connection: Connection,
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
        } else if version > 2 {
            return Err(StorageError::UnsupportedVersion(version));
        }
        if version < 2 {
            tx.execute_batch(
                "ALTER TABLE sessions ADD COLUMN start_git_sha TEXT; PRAGMA user_version=2;",
            )?;
        }
        let _ = tx.execute("ALTER TABLE sessions ADD COLUMN cli_session_uuid TEXT", []);
        tx.execute_batch("CREATE VIRTUAL TABLE IF NOT EXISTS terminal_logs_fts USING fts5(session_id UNINDEXED, line_no UNINDEXED, timestamp UNINDEXED, content, tokenize='trigram');")?;
        tx.execute_batch("CREATE TABLE IF NOT EXISTS session_snapshots (session_id TEXT PRIMARY KEY, snapshot BLOB, compressed_size INTEGER, uncompressed_size INTEGER, timestamp INTEGER);")?;
        tx.execute_batch("CREATE TABLE IF NOT EXISTS review_comments (id TEXT PRIMARY KEY, session_id TEXT NOT NULL, file_path TEXT NOT NULL, line_number INTEGER NOT NULL, line_hash TEXT NOT NULL, content TEXT NOT NULL, status TEXT NOT NULL, created_at INTEGER NOT NULL); CREATE INDEX IF NOT EXISTS idx_review_session_status ON review_comments(session_id, status); CREATE TABLE IF NOT EXISTS review_rounds (session_id TEXT NOT NULL, round_number INTEGER NOT NULL, PRIMARY KEY(session_id, round_number));")?;
        tx.execute_batch("CREATE TABLE IF NOT EXISTS queued_messages (id INTEGER PRIMARY KEY AUTOINCREMENT, target_session_id TEXT NOT NULL, message TEXT NOT NULL, created_at INTEGER NOT NULL, delivered_at INTEGER); CREATE INDEX IF NOT EXISTS idx_queued_messages ON queued_messages(target_session_id, delivered_at);")?;
        tx.execute_batch("CREATE TABLE IF NOT EXISTS named_locks (name TEXT PRIMARY KEY, holder_session_id TEXT NOT NULL, acquired_at INTEGER NOT NULL); CREATE INDEX IF NOT EXISTS idx_named_locks_holder ON named_locks(holder_session_id);")?;
        tx.execute_batch("CREATE TABLE IF NOT EXISTS file_reservations (path TEXT PRIMARY KEY, holder_session_id TEXT NOT NULL, acquired_at INTEGER NOT NULL); CREATE INDEX IF NOT EXISTS idx_file_reservations_holder ON file_reservations(holder_session_id);")?;
        tx.execute_batch("CREATE TABLE IF NOT EXISTS event_journal (seq INTEGER PRIMARY KEY AUTOINCREMENT, event_type TEXT NOT NULL, session_id TEXT, payload TEXT NOT NULL, timestamp INTEGER NOT NULL); CREATE INDEX IF NOT EXISTS idx_event_journal_seq ON event_journal(seq); CREATE TABLE IF NOT EXISTS event_cursors (cursor_id TEXT PRIMARY KEY, last_seq INTEGER NOT NULL, updated_at INTEGER NOT NULL);")?;
        tx.execute_batch("CREATE TABLE IF NOT EXISTS kanban_tasks (id TEXT PRIMARY KEY, title TEXT NOT NULL, description TEXT NOT NULL, status TEXT NOT NULL, claimed_by TEXT, created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL); CREATE INDEX IF NOT EXISTS idx_kanban_tasks_status ON kanban_tasks(status); CREATE TABLE IF NOT EXISTS worker_reports (id INTEGER PRIMARY KEY AUTOINCREMENT, task_id TEXT NOT NULL, session_id TEXT NOT NULL, summary TEXT NOT NULL, pr_url TEXT, commit_sha TEXT, created_at INTEGER NOT NULL); CREATE INDEX IF NOT EXISTS idx_worker_reports_task ON worker_reports(task_id);")?;
        tx.execute_batch("CREATE TABLE IF NOT EXISTS pending_teardowns (session_id TEXT PRIMARY KEY, action TEXT NOT NULL);")?;
        tx.execute_batch("CREATE TABLE IF NOT EXISTS workspace_metadata (session_id TEXT PRIMARY KEY, project TEXT NOT NULL, group_path TEXT NOT NULL, favorite INTEGER NOT NULL, archived INTEGER NOT NULL);")?;
        tx.execute_batch("CREATE TABLE IF NOT EXISTS companion_notifications (id INTEGER PRIMARY KEY, session_id TEXT NOT NULL, kind TEXT NOT NULL, title TEXT NOT NULL, body TEXT NOT NULL, timestamp INTEGER NOT NULL);")?;
        tx.execute_batch("CREATE TABLE IF NOT EXISTS companion_panels (id INTEGER PRIMARY KEY, session_id TEXT NOT NULL, kind TEXT NOT NULL, target TEXT NOT NULL, UNIQUE(session_id, kind, target)); CREATE TABLE IF NOT EXISTS companion_selection (id INTEGER PRIMARY KEY CHECK(id=1), session_id TEXT NOT NULL); CREATE TABLE IF NOT EXISTS companion_panel_selection (id INTEGER PRIMARY KEY CHECK(id=1), panel_id INTEGER NOT NULL);")?;
        tx.execute_batch("CREATE TABLE IF NOT EXISTS managed_conversations (session_id TEXT PRIMARY KEY, content TEXT NOT NULL, status TEXT NOT NULL, updated_at INTEGER NOT NULL);")?;
        tx.execute_batch("CREATE TABLE IF NOT EXISTS session_notes (session_id TEXT NOT NULL, page INTEGER NOT NULL, content TEXT NOT NULL, PRIMARY KEY(session_id, page));")?;
        tx.execute_batch("CREATE VIRTUAL TABLE IF NOT EXISTS session_notes_fts USING fts5(session_id UNINDEXED, page UNINDEXED, content, tokenize='trigram'); CREATE TRIGGER IF NOT EXISTS session_notes_insert AFTER INSERT ON session_notes BEGIN INSERT INTO session_notes_fts(rowid, session_id, page, content) VALUES (new.rowid, new.session_id, new.page, new.content); END; CREATE TRIGGER IF NOT EXISTS session_notes_update AFTER UPDATE ON session_notes BEGIN DELETE FROM session_notes_fts WHERE rowid=old.rowid; INSERT INTO session_notes_fts(rowid, session_id, page, content) VALUES (new.rowid, new.session_id, new.page, new.content); END; CREATE TRIGGER IF NOT EXISTS session_notes_delete AFTER DELETE ON session_notes BEGIN DELETE FROM session_notes_fts WHERE rowid=old.rowid; END;")?;
        tx.execute_batch("INSERT INTO session_notes_fts(rowid, session_id, page, content) SELECT rowid, session_id, page, content FROM session_notes WHERE rowid NOT IN (SELECT rowid FROM session_notes_fts);")?;
        tx.commit()?;
        Ok(Self { connection })
    }
    /// Persist a bounded notification history for companion clients.
    pub fn save_companion_notification(
        &self,
        event: &agmux_proto::NotificationEvent,
    ) -> Result<(), StorageError> {
        self.connection.execute("INSERT INTO companion_notifications(session_id,kind,title,body,timestamp) VALUES(?1,?2,?3,?4,?5)", params![event.session_id,event.kind,event.title,event.body,event.timestamp])?;
        self.connection.execute("DELETE FROM companion_notifications WHERE id NOT IN (SELECT id FROM companion_notifications ORDER BY id DESC LIMIT 500)", [])?;
        Ok(())
    }
    pub fn companion_notifications(
        &self,
    ) -> Result<Vec<agmux_proto::NotificationEvent>, StorageError> {
        let mut stmt = self.connection.prepare("SELECT session_id,kind,title,body,timestamp FROM companion_notifications ORDER BY id DESC LIMIT 500")?;
        let rows = stmt.query_map([], |r| {
            Ok(agmux_proto::NotificationEvent {
                session_id: r.get(0)?,
                kind: r.get(1)?,
                title: r.get(2)?,
                body: r.get(3)?,
                timestamp: r.get(4)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }
    /// Register a panel for an existing session.
    pub fn register_panel(
        &self,
        session_id: &str,
        kind: &str,
        target: &str,
    ) -> Result<bool, StorageError> {
        Ok(self.connection.execute("INSERT OR IGNORE INTO companion_panels(session_id,kind,target) SELECT id,?2,?3 FROM sessions WHERE id=?1", params![session_id,kind,target])? != 0 || self.connection.query_row("SELECT EXISTS(SELECT 1 FROM companion_panels WHERE session_id=?1 AND kind=?2 AND target=?3)", params![session_id,kind,target], |r| r.get(0))?)
    }
    /// Persist the selected session, rejecting unknown sessions.
    pub fn select_companion_session(&self, session_id: &str) -> Result<bool, StorageError> {
        Ok(self.connection.execute("INSERT INTO companion_selection(id,session_id) SELECT 1,id FROM sessions WHERE id=?1 ON CONFLICT(id) DO UPDATE SET session_id=excluded.session_id", [session_id])? != 0)
    }
    pub fn select_companion_panel(
        &self,
        session_id: &str,
        panel_id: i64,
    ) -> Result<bool, StorageError> {
        Ok(self.connection.execute("INSERT INTO companion_panel_selection(id,panel_id) SELECT 1,id FROM companion_panels WHERE id=?1 AND session_id=?2 ON CONFLICT(id) DO UPDATE SET panel_id=excluded.panel_id", params![panel_id, session_id])? != 0)
    }
    pub fn companion_selected_panel(&self) -> Result<Option<i64>, StorageError> {
        Ok(self
            .connection
            .query_row(
                "SELECT panel_id FROM companion_panel_selection WHERE id=1",
                [],
                |r| r.get(0),
            )
            .optional()?)
    }
    pub fn companion_selection(&self) -> Result<Option<String>, StorageError> {
        Ok(self
            .connection
            .query_row(
                "SELECT session_id FROM companion_selection WHERE id=1",
                [],
                |r| r.get(0),
            )
            .optional()?)
    }
    pub fn companion_panels(&self) -> Result<Vec<agmux_proto::CompanionPanel>, StorageError> {
        let mut stmt = self
            .connection
            .prepare("SELECT id,session_id,kind,target FROM companion_panels ORDER BY id")?;
        let rows = stmt.query_map([], |r| {
            Ok(agmux_proto::CompanionPanel {
                id: r.get(0)?,
                session_id: r.get(1)?,
                kind: r.get(2)?,
                target: r.get(3)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }
    /// Persist a message for a session.
    pub fn enqueue_message(
        &self,
        target_session_id: &str,
        message: &str,
    ) -> Result<u64, StorageError> {
        self.connection.execute("INSERT INTO queued_messages (target_session_id,message,created_at) VALUES (?1,?2,unixepoch())", params![target_session_id,message])?;
        Ok(self.connection.last_insert_rowid() as u64)
    }
    /// Return pending messages in FIFO order without removing them.
    pub fn pending_messages(
        &self,
        target_session_id: &str,
    ) -> Result<Vec<(u64, String)>, StorageError> {
        let mut stmt = self.connection.prepare("SELECT id,message FROM queued_messages WHERE target_session_id=?1 AND delivered_at IS NULL ORDER BY id")?;
        let rows = stmt.query_map([target_session_id], |r| Ok((r.get(0)?, r.get(1)?)))?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }
    /// Mark a successfully delivered message.
    pub fn mark_message_delivered(&self, id: u64) -> Result<(), StorageError> {
        self.connection.execute("UPDATE queued_messages SET delivered_at=unixepoch() WHERE id=?1 AND delivered_at IS NULL", [id])?;
        Ok(())
    }
    /// Atomically claim all currently pending messages.
    pub fn pop_pending_messages(
        &self,
        target_session_id: &str,
    ) -> Result<Vec<(u64, String)>, StorageError> {
        let tx = self.connection.unchecked_transaction()?;
        let messages = {
            let mut stmt = tx.prepare("SELECT id,message FROM queued_messages WHERE target_session_id=?1 AND delivered_at IS NULL ORDER BY id")?;
            let rows = stmt.query_map([target_session_id], |r| Ok((r.get(0)?, r.get(1)?)))?;
            rows.collect::<Result<Vec<_>, _>>()?
        };
        tx.execute("UPDATE queued_messages SET delivered_at=unixepoch() WHERE target_session_id=?1 AND delivered_at IS NULL", [target_session_id])?;
        tx.commit()?;
        Ok(messages)
    }
    /// Count pending messages for a session.
    pub fn pending_message_count(&self, target_session_id: &str) -> Result<usize, StorageError> {
        Ok(self.connection.query_row("SELECT count(*) FROM queued_messages WHERE target_session_id=?1 AND delivered_at IS NULL", [target_session_id], |r| r.get(0))?)
    }
    /// Atomically acquire a named lock. Returns true if acquired, false if held by another session.
    pub fn acquire_lock(&self, name: &str, session_id: &str) -> Result<bool, StorageError> {
        let mut stmt = self
            .connection
            .prepare("SELECT holder_session_id FROM named_locks WHERE name = ?1")?;
        let mut rows = stmt.query([name])?;
        if let Some(row) = rows.next()? {
            let holder: String = row.get(0)?;
            return Ok(holder == session_id);
        }
        self.connection.execute(
            "INSERT INTO named_locks (name, holder_session_id, acquired_at) VALUES (?1, ?2, unixepoch())",
            params![name, session_id],
        )?;
        Ok(true)
    }
    /// Release a named lock held by session_id.
    pub fn release_lock(&self, name: &str, session_id: &str) -> Result<bool, StorageError> {
        let count = self.connection.execute(
            "DELETE FROM named_locks WHERE name = ?1 AND holder_session_id = ?2",
            params![name, session_id],
        )?;
        Ok(count > 0)
    }
    /// Release all named locks held by a session and return their names.
    pub fn release_session_locks(&self, session_id: &str) -> Result<Vec<String>, StorageError> {
        let mut stmt = self
            .connection
            .prepare("SELECT name FROM named_locks WHERE holder_session_id = ?1")?;
        let rows = stmt.query_map([session_id], |r| r.get(0))?;
        let names = rows.collect::<Result<Vec<String>, _>>()?;
        self.connection.execute(
            "DELETE FROM named_locks WHERE holder_session_id = ?1",
            [session_id],
        )?;
        Ok(names)
    }
    /// Query the current holder of a named lock.
    pub fn get_lock_holder(&self, name: &str) -> Result<Option<String>, StorageError> {
        let mut stmt = self
            .connection
            .prepare("SELECT holder_session_id FROM named_locks WHERE name = ?1")?;
        let mut rows = stmt.query([name])?;
        if let Some(row) = rows.next()? {
            Ok(Some(row.get(0)?))
        } else {
            Ok(None)
        }
    }
    /// List all currently active named locks.
    pub fn list_locks(&self) -> Result<Vec<(String, String, u64)>, StorageError> {
        let mut stmt = self.connection.prepare(
            "SELECT name, holder_session_id, acquired_at FROM named_locks ORDER BY acquired_at ASC",
        )?;
        let rows = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }
    /// Attempt to reserve a file or directory path. Returns Ok(Ok(())) on success,
    /// or Ok(Err((conflicting_path, holder_session))) if an active conflicting reservation exists.
    pub fn reserve_file(
        &self,
        path: &str,
        session_id: &str,
    ) -> Result<Result<(), (String, String)>, StorageError> {
        let clean_path = path.trim_matches('/');
        let mut stmt = self
            .connection
            .prepare("SELECT path, holder_session_id FROM file_reservations")?;
        let rows = stmt.query_map([], |r| Ok((r.get::<_, String>(0)?, r.get::<_, String>(1)?)))?;
        for row in rows {
            let (existing_path, holder) = row?;
            if path_conflicts(&existing_path, clean_path) {
                if holder == session_id {
                    // Already held by this session
                    return Ok(Ok(()));
                } else {
                    return Ok(Err((existing_path, holder)));
                }
            }
        }
        self.connection.execute(
            "INSERT INTO file_reservations (path, holder_session_id, acquired_at) VALUES (?1, ?2, unixepoch())",
            params![clean_path, session_id],
        )?;
        Ok(Ok(()))
    }
    /// Release a specific reserved path held by session_id.
    pub fn release_file(&self, path: &str, session_id: &str) -> Result<bool, StorageError> {
        let clean_path = path.trim_matches('/');
        let count = self.connection.execute(
            "DELETE FROM file_reservations WHERE path = ?1 AND holder_session_id = ?2",
            params![clean_path, session_id],
        )?;
        Ok(count > 0)
    }
    /// Release all file reservations held by a session.
    pub fn release_session_reservations(
        &self,
        session_id: &str,
    ) -> Result<Vec<String>, StorageError> {
        let mut stmt = self
            .connection
            .prepare("SELECT path FROM file_reservations WHERE holder_session_id = ?1")?;
        let rows = stmt.query_map([session_id], |r| r.get(0))?;
        let paths = rows.collect::<Result<Vec<String>, _>>()?;
        self.connection.execute(
            "DELETE FROM file_reservations WHERE holder_session_id = ?1",
            [session_id],
        )?;
        Ok(paths)
    }
    /// List all currently active file reservations.
    pub fn list_reservations(&self) -> Result<Vec<(String, String, u64)>, StorageError> {
        let mut stmt = self.connection.prepare(
            "SELECT path, holder_session_id, acquired_at FROM file_reservations ORDER BY acquired_at ASC",
        )?;
        let rows = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }
    /// Record an event into the monotonic journal and return its sequence ID.
    pub fn record_event(
        &self,
        event_type: &str,
        session_id: Option<&str>,
        payload: &str,
    ) -> Result<JournalEvent, StorageError> {
        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        self.connection.execute(
            "INSERT INTO event_journal (event_type, session_id, payload, timestamp) VALUES (?1, ?2, ?3, ?4)",
            params![event_type, session_id, payload, timestamp],
        )?;
        let seq = self.connection.last_insert_rowid() as u64;
        Ok(JournalEvent {
            seq,
            event_type: event_type.to_string(),
            session_id: session_id.map(|s| s.to_string()),
            payload: payload.to_string(),
            timestamp,
            replay: false,
        })
    }
    /// Read events with seq > since_seq in exact monotonic sequence order.
    pub fn read_events_since(
        &self,
        since_seq: u64,
        limit: Option<usize>,
    ) -> Result<Vec<JournalEvent>, StorageError> {
        let sql = if let Some(limit) = limit {
            format!(
                "SELECT seq, event_type, session_id, payload, timestamp FROM event_journal WHERE seq > ?1 ORDER BY seq ASC LIMIT {limit}"
            )
        } else {
            "SELECT seq, event_type, session_id, payload, timestamp FROM event_journal WHERE seq > ?1 ORDER BY seq ASC".to_string()
        };
        let mut stmt = self.connection.prepare(&sql)?;
        let rows = stmt.query_map([since_seq], |r| {
            Ok(JournalEvent {
                seq: r.get(0)?,
                event_type: r.get(1)?,
                session_id: r.get(2)?,
                payload: r.get(3)?,
                timestamp: r.get(4)?,
                replay: true,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }
    /// Get last sequence position for a cursor.
    pub fn get_cursor(&self, cursor_id: &str) -> Result<u64, StorageError> {
        let mut stmt = self
            .connection
            .prepare("SELECT last_seq FROM event_cursors WHERE cursor_id = ?1")?;
        let mut rows = stmt.query([cursor_id])?;
        if let Some(row) = rows.next()? {
            Ok(row.get(0)?)
        } else {
            Ok(0)
        }
    }
    /// Update or create cursor position.
    pub fn set_cursor(&self, cursor_id: &str, last_seq: u64) -> Result<(), StorageError> {
        self.connection.execute(
            "INSERT INTO event_cursors (cursor_id, last_seq, updated_at) VALUES (?1, ?2, unixepoch())
             ON CONFLICT(cursor_id) DO UPDATE SET last_seq = excluded.last_seq, updated_at = excluded.updated_at",
            params![cursor_id, last_seq],
        )?;
        Ok(())
    }
    pub fn add_review_comment(&self, comment: &ReviewComment) -> Result<(), StorageError> {
        self.connection.execute(
            "INSERT INTO review_comments VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
            params![
                comment.id,
                comment.session_id,
                comment.file_path,
                comment.line_number,
                comment.line_hash,
                comment.content,
                comment.status,
                comment.created_at
            ],
        )?;
        Ok(())
    }
    pub fn resolve_review_comment(&self, comment_id: &str) -> Result<bool, StorageError> {
        Ok(self.connection.execute(
            "UPDATE review_comments SET status='resolved' WHERE id=?1",
            [comment_id],
        )? > 0)
    }
    pub fn list_review_comments(
        &self,
        session_id: &str,
        status: Option<&str>,
    ) -> Result<Vec<ReviewComment>, StorageError> {
        let mut stmt = self.connection.prepare("SELECT id,session_id,file_path,line_number,line_hash,content,status,created_at FROM review_comments WHERE session_id=?1 AND (?2 IS NULL OR status=?2) ORDER BY rowid")?;
        let rows = stmt.query_map(params![session_id, status], |r| {
            Ok(ReviewComment {
                id: r.get(0)?,
                session_id: r.get(1)?,
                file_path: r.get(2)?,
                line_number: r.get(3)?,
                line_hash: r.get(4)?,
                content: r.get(5)?,
                status: r.get(6)?,
                created_at: r.get(7)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }
    pub fn update_comments_status(
        &self,
        ids: &[String],
        new_status: &str,
    ) -> Result<(), StorageError> {
        let tx = self.connection.unchecked_transaction()?;
        for id in ids {
            tx.execute(
                "UPDATE review_comments SET status=?1 WHERE id=?2",
                params![new_status, id],
            )?;
        }
        tx.commit()?;
        Ok(())
    }
    pub fn next_review_round(&self, session_id: &str) -> Result<u32, StorageError> {
        Ok(self.connection.query_row(
            "SELECT COALESCE(MAX(round_number),0)+1 FROM review_rounds WHERE session_id=?1",
            [session_id],
            |r| r.get(0),
        )?)
    }
    pub fn complete_review_round(
        &self,
        session_id: &str,
        round: u32,
        ids: &[String],
    ) -> Result<String, StorageError> {
        let tx = self.connection.unchecked_transaction()?;
        let old: String = tx.query_row(
            "SELECT state FROM sessions WHERE id=?1",
            [session_id],
            |r| r.get(0),
        )?;
        for id in ids {
            tx.execute("UPDATE review_comments SET status='sent' WHERE id=?1 AND session_id=?2 AND status='draft'", params![id,session_id])?;
        }
        tx.execute(
            "INSERT INTO review_rounds VALUES (?1,?2)",
            params![session_id, round],
        )?;
        tx.execute(
            "UPDATE sessions SET state='WORKING' WHERE id=?1",
            [session_id],
        )?;
        tx.commit()?;
        Ok(old)
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
        if !lines.is_empty() {
            tx.execute(
                "DELETE FROM terminal_logs_fts WHERE session_id=?1 AND line_no=?2",
                params![session_id, first_line],
            )?;
        }
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
    /// Persist a managed provider transcript without storing provider credentials.
    pub fn save_conversation(
        &self,
        id: &str,
        content: &str,
        status: &str,
        updated: u64,
    ) -> Result<(), StorageError> {
        self.connection.execute("INSERT INTO managed_conversations(session_id,content,status,updated_at) VALUES(?1,?2,?3,?4) ON CONFLICT(session_id) DO UPDATE SET content=excluded.content,status=excluded.status,updated_at=excluded.updated_at",params![id,content,status,updated])?;
        Ok(())
    }
    pub fn conversation(&self, id: &str) -> Result<Option<String>, StorageError> {
        Ok(self
            .connection
            .query_row(
                "SELECT content FROM managed_conversations WHERE session_id=?1",
                [id],
                |r| r.get(0),
            )
            .optional()?)
    }
    pub fn conversation_index(&self) -> Result<Vec<(String, String, u64)>, StorageError> {
        let mut q = self
            .connection
            .prepare("SELECT session_id,status,updated_at FROM managed_conversations")?;
        Ok(q.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))?
            .collect::<Result<Vec<_>, _>>()?)
    }
    pub fn set_session_pid(&self, id: &str, pid: u32) -> Result<(), StorageError> {
        self.connection
            .execute("UPDATE sessions SET pid=?1 WHERE id=?2", params![pid, id])?;
        Ok(())
    }
    /// Record the Git commit captured at session startup.
    pub fn set_start_git_sha(&self, id: &str, sha: Option<&str>) -> Result<(), StorageError> {
        self.connection.execute(
            "UPDATE sessions SET start_git_sha=?1 WHERE id=?2",
            params![sha, id],
        )?;
        Ok(())
    }
    /// Read the initial Git commit for an existing session.
    pub fn start_git_sha(&self, id: &str) -> Result<Option<String>, StorageError> {
        Ok(self
            .connection
            .query_row(
                "SELECT start_git_sha FROM sessions WHERE id=?1",
                [id],
                |r| r.get(0),
            )
            .optional()?
            .flatten())
    }
    /// Record the native CLI conversation UUID captured for a session (`REQ-PLUGIN-UUID-CAPTURE-005`).
    pub fn set_cli_session_uuid(&self, id: &str, uuid: Option<&str>) -> Result<(), StorageError> {
        self.connection.execute(
            "UPDATE sessions SET cli_session_uuid=?1 WHERE id=?2",
            params![uuid, id],
        )?;
        Ok(())
    }
    /// Read the native CLI conversation UUID captured for a session (`REQ-PLUGIN-UUID-CAPTURE-005`).
    pub fn get_cli_session_uuid(&self, id: &str) -> Result<Option<String>, StorageError> {
        Ok(self
            .connection
            .query_row(
                "SELECT cli_session_uuid FROM sessions WHERE id=?1",
                [id],
                |r| r.get(0),
            )
            .optional()?
            .flatten())
    }
    pub fn update_session_state(&self, id: &str, state: &str) -> Result<(), StorageError> {
        self.connection.execute(
            "UPDATE sessions SET state=?1 WHERE id=?2",
            params![state, id],
        )?;
        Ok(())
    }
    /// All persisted sessions, including archived and exited sessions.
    pub fn list_workspace_sessions(&self) -> Result<Vec<SessionInfo>, StorageError> {
        let mut stmt = self.connection.prepare(
            "SELECT id,pid,cwd,command,created_at,state FROM sessions ORDER BY created_at,id",
        )?;
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
    /// Persist organization independently of lifecycle state.
    pub fn set_workspace_metadata(
        &self,
        id: &str,
        metadata: &agmux_proto::WorkspaceMetadata,
    ) -> Result<bool, StorageError> {
        Ok(self.connection.execute("INSERT INTO workspace_metadata (session_id,project,group_path,favorite,archived) SELECT id,?2,?3,?4,?5 FROM sessions WHERE id=?1 ON CONFLICT(session_id) DO UPDATE SET project=excluded.project,group_path=excluded.group_path,favorite=excluded.favorite,archived=excluded.archived", params![id, metadata.project, metadata.group, metadata.favorite, metadata.archived])? != 0)
    }
    /// Return organization for all sessions.
    pub fn workspace_metadata(
        &self,
    ) -> Result<std::collections::HashMap<String, agmux_proto::WorkspaceMetadata>, StorageError>
    {
        let mut stmt = self.connection.prepare(
            "SELECT session_id,project,group_path,favorite,archived FROM workspace_metadata",
        )?;
        let rows = stmt.query_map([], |r| {
            Ok((
                r.get(0)?,
                agmux_proto::WorkspaceMetadata {
                    project: r.get(1)?,
                    group: r.get(2)?,
                    favorite: r.get(3)?,
                    archived: r.get(4)?,
                },
            ))
        })?;
        Ok(rows.collect::<Result<_, _>>()?)
    }
    pub fn list_active_sessions(&self) -> Result<Vec<SessionInfo>, StorageError> {
        let mut stmt = self.connection.prepare("SELECT id,pid,cwd,command,created_at,state FROM sessions WHERE state NOT IN ('DEAD','ERRORED','COMPLETED','FAILED','ARCHIVED') ORDER BY created_at,id")?;
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
        self.set_teardown(id, None)?;
        self.connection.execute("DELETE FROM companion_panel_selection WHERE panel_id IN (SELECT id FROM companion_panels WHERE session_id=?1)", [id])?;
        self.connection
            .execute("DELETE FROM companion_panels WHERE session_id=?1", [id])?;
        self.connection
            .execute("DELETE FROM companion_selection WHERE session_id=?1", [id])?;
        self.connection
            .execute("DELETE FROM workspace_metadata WHERE session_id=?1", [id])?;
        self.connection
            .execute("DELETE FROM sessions WHERE id=?1", [id])?;
        Ok(())
    }
    /// Persist ordinary transitions without reviving a torn-down session.
    pub fn update_live_session_state(&self, id: &str, state: &str) -> Result<bool, StorageError> {
        Ok(self.connection.execute(
            "UPDATE sessions SET state=?2 WHERE id=?1 AND state NOT IN ('DEAD','ARCHIVED')",
            params![id, state],
        )? > 0)
    }

    /// Commit the teardown state and consume its request in one transaction.
    pub fn finish_teardown(&self, id: &str, state: &str) -> Result<(), StorageError> {
        let tx = self.connection.unchecked_transaction()?;
        tx.execute(
            "UPDATE sessions SET state=?2 WHERE id=?1",
            params![id, state],
        )?;
        tx.execute("DELETE FROM pending_teardowns WHERE session_id=?1", [id])?;
        tx.commit()?;
        Ok(())
    }

    /// Return pending actions for presentation and completion.
    pub fn list_teardowns(
        &self,
    ) -> Result<std::collections::HashMap<String, String>, StorageError> {
        let mut stmt = self
            .connection
            .prepare("SELECT session_id, action FROM pending_teardowns")?;
        let rows = stmt.query_map([], |r| Ok((r.get(0)?, r.get(1)?)))?;
        Ok(rows.collect::<Result<_, _>>()?)
    }

    /// Set, replace, or cancel a pending teardown.
    pub fn set_teardown(&self, id: &str, action: Option<&str>) -> Result<(), StorageError> {
        if let Some(action) = action {
            self.connection.execute(
                "INSERT OR REPLACE INTO pending_teardowns(session_id,action) VALUES (?1,?2)",
                params![id, action],
            )?;
        } else {
            self.connection
                .execute("DELETE FROM pending_teardowns WHERE session_id=?1", [id])?;
        }
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
    /// Create a new Kanban task in `"todo"` status.
    pub fn create_kanban_task(
        &self,
        id: &str,
        title: &str,
        description: &str,
    ) -> Result<KanbanTask, StorageError> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        self.connection.execute(
            "INSERT INTO kanban_tasks (id, title, description, status, claimed_by, created_at, updated_at)
             VALUES (?1, ?2, ?3, 'todo', NULL, ?4, ?4)
             ON CONFLICT(id) DO UPDATE SET title = excluded.title, description = excluded.description, updated_at = excluded.updated_at",
            params![id, title, description, now],
        )?;
        self.get_kanban_task(id)?
            .ok_or_else(|| StorageError::TaskError(format!("Task '{id}' not found after insert")))
    }
    /// Retrieve a single Kanban task by ID.
    pub fn get_kanban_task(&self, id: &str) -> Result<Option<KanbanTask>, StorageError> {
        Ok(self
            .connection
            .query_row(
                "SELECT id, title, description, status, claimed_by, created_at, updated_at FROM kanban_tasks WHERE id = ?1",
                [id],
                |r| {
                    Ok(KanbanTask {
                        id: r.get(0)?,
                        title: r.get(1)?,
                        description: r.get(2)?,
                        status: r.get(3)?,
                        claimed_by: r.get(4)?,
                        created_at: r.get(5)?,
                        updated_at: r.get(6)?,
                    })
                },
            )
            .optional()?)
    }
    /// List Kanban tasks, optionally filtered by status.
    pub fn list_kanban_tasks(&self, status: Option<&str>) -> Result<Vec<KanbanTask>, StorageError> {
        let mut stmt = self.connection.prepare(
            "SELECT id, title, description, status, claimed_by, created_at, updated_at
             FROM kanban_tasks
             WHERE (?1 IS NULL OR status = ?1)
             ORDER BY created_at ASC, id ASC",
        )?;
        let rows = stmt.query_map([status], |r| {
            Ok(KanbanTask {
                id: r.get(0)?,
                title: r.get(1)?,
                description: r.get(2)?,
                status: r.get(3)?,
                claimed_by: r.get(4)?,
                created_at: r.get(5)?,
                updated_at: r.get(6)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }
    /// Claim a Kanban task for a session. If the task does not yet exist, auto-creates it in `"in_progress"`.
    pub fn claim_kanban_task(
        &self,
        id: &str,
        session_id: &str,
    ) -> Result<KanbanTask, StorageError> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        if let Some(existing) = self.get_kanban_task(id)? {
            if existing.status == "in_progress"
                && existing.claimed_by.as_deref() != Some(session_id)
            {
                return Err(StorageError::TaskError(format!(
                    "Task '{id}' is already claimed by '{}'",
                    existing.claimed_by.unwrap_or_default()
                )));
            }
            if existing.status == "done" {
                return Err(StorageError::TaskError(format!(
                    "Task '{id}' is already finished"
                )));
            }
            self.connection.execute(
                "UPDATE kanban_tasks SET status = 'in_progress', claimed_by = ?1, updated_at = ?2 WHERE id = ?3",
                params![session_id, now, id],
            )?;
        } else {
            self.connection.execute(
                "INSERT INTO kanban_tasks (id, title, description, status, claimed_by, created_at, updated_at)
                 VALUES (?1, ?1, '', 'in_progress', ?2, ?3, ?3)",
                params![id, session_id, now],
            )?;
        }
        self.get_kanban_task(id)?
            .ok_or_else(|| StorageError::TaskError(format!("Task '{id}' not found")))
    }
    /// Mark a Kanban task as `"done"`.
    pub fn finish_kanban_task(
        &self,
        id: &str,
        session_id: &str,
    ) -> Result<KanbanTask, StorageError> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let existing = self
            .get_kanban_task(id)?
            .ok_or_else(|| StorageError::TaskError(format!("Task '{id}' not found")))?;
        if let Some(ref holder) = existing.claimed_by
            && holder != session_id
        {
            return Err(StorageError::TaskError(format!(
                "Task '{id}' is claimed by '{holder}', cannot finish from '{session_id}'"
            )));
        }
        self.connection.execute(
            "UPDATE kanban_tasks SET status = 'done', claimed_by = COALESCE(claimed_by, ?1), updated_at = ?2 WHERE id = ?3",
            params![session_id, now, id],
        )?;
        self.get_kanban_task(id)?
            .ok_or_else(|| StorageError::TaskError(format!("Task '{id}' not found")))
    }
    /// Release a claimed Kanban task back to `"todo"`.
    pub fn release_kanban_task(
        &self,
        id: &str,
        session_id: &str,
    ) -> Result<KanbanTask, StorageError> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let existing = self
            .get_kanban_task(id)?
            .ok_or_else(|| StorageError::TaskError(format!("Task '{id}' not found")))?;
        if let Some(ref holder) = existing.claimed_by
            && holder != session_id
        {
            return Err(StorageError::TaskError(format!(
                "Task '{id}' is claimed by '{holder}', cannot release from '{session_id}'"
            )));
        }
        self.connection.execute(
            "UPDATE kanban_tasks SET status = 'todo', claimed_by = NULL, updated_at = ?1 WHERE id = ?2",
            params![now, id],
        )?;
        self.get_kanban_task(id)?
            .ok_or_else(|| StorageError::TaskError(format!("Task '{id}' not found")))
    }
    /// Release all `in_progress` Kanban tasks claimed by `session_id` back to `todo` and return their IDs.
    pub fn release_session_kanban_tasks(
        &self,
        session_id: &str,
    ) -> Result<Vec<String>, StorageError> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        let mut stmt = self.connection.prepare(
            "SELECT id FROM kanban_tasks WHERE status = 'in_progress' AND claimed_by = ?1",
        )?;
        let rows = stmt.query_map([session_id], |r| r.get(0))?;
        let ids = rows.collect::<Result<Vec<String>, _>>()?;
        if !ids.is_empty() {
            self.connection.execute(
                "UPDATE kanban_tasks SET status = 'todo', claimed_by = NULL, updated_at = ?1 WHERE status = 'in_progress' AND claimed_by = ?2",
                params![now, session_id],
            )?;
        }
        Ok(ids)
    }
    /// Record a structured worker completion report.
    pub fn post_worker_report(
        &self,
        task_id: &str,
        session_id: &str,
        summary: &str,
        pr_url: Option<&str>,
        commit_sha: Option<&str>,
    ) -> Result<WorkerReport, StorageError> {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_secs();
        self.connection.execute(
            "INSERT INTO worker_reports (task_id, session_id, summary, pr_url, commit_sha, created_at)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![task_id, session_id, summary, pr_url, commit_sha, now],
        )?;
        Ok(WorkerReport {
            task_id: task_id.to_string(),
            session_id: session_id.to_string(),
            summary: summary.to_string(),
            pr_url: pr_url.map(|s| s.to_string()),
            commit_sha: commit_sha.map(|s| s.to_string()),
            created_at: now,
        })
    }
    /// Retrieve all structured worker reports for a task in chronological order.
    pub fn get_worker_reports(&self, task_id: &str) -> Result<Vec<WorkerReport>, StorageError> {
        let mut stmt = self.connection.prepare(
            "SELECT task_id, session_id, summary, pr_url, commit_sha, created_at
             FROM worker_reports
             WHERE task_id = ?1
             ORDER BY id ASC",
        )?;
        let rows = stmt.query_map([task_id], |r| {
            Ok(WorkerReport {
                task_id: r.get(0)?,
                session_id: r.get(1)?,
                summary: r.get(2)?,
                pr_url: r.get(3)?,
                commit_sha: r.get(4)?,
                created_at: r.get(5)?,
            })
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
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
    #[error("{0}")]
    TaskError(String),
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

#[cfg(test)]
mod conversation_retention_tests {
    #[test]
    fn retained_history_survives_database_reopen() {
        let root = std::env::temp_dir().join(format!("agmux-history-{}", std::process::id()));
        std::fs::create_dir_all(&root).unwrap();
        let path = root.join("db");
        {
            let db = super::StorageEngine::open(&path).unwrap();
            db.save_conversation("one", r#"{"messages":[{"text":"kept"}]}"#, "ended", 123)
                .unwrap();
        }
        {
            let db = super::StorageEngine::open(&path).unwrap();
            assert!(db.conversation("one").unwrap().unwrap().contains("kept"));
            assert_eq!(
                db.conversation_index().unwrap(),
                vec![("one".into(), "ended".into(), 123)]
            );
        }
        std::fs::remove_dir_all(root).unwrap();
    }
}
