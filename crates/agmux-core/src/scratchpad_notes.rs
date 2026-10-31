//! Persistent, numbered scratchpad pages attached to sessions.

use rusqlite::{OptionalExtension, params};

use crate::storage_engine::{StorageEngine, StorageError};

impl StorageEngine {
    /// Return transcript context around a matching line.
    pub fn transcript_context(
        &self,
        session_id: &str,
        line_no: u64,
    ) -> Result<Vec<(u64, String)>, StorageError> {
        let mut statement = self.connection.prepare("SELECT line_no, content FROM terminal_logs_fts WHERE session_id=?1 AND CAST(line_no AS INTEGER) BETWEEN ?2 AND ?3 ORDER BY CAST(line_no AS INTEGER), rowid LIMIT 31")?;
        let rows = statement.query_map(
            params![
                session_id,
                line_no.saturating_sub(7),
                line_no.saturating_add(7)
            ],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }
    /// Replace a page's contents, creating the page when necessary.
    pub fn set_note(&self, session_id: &str, page: u32, content: &str) -> Result<(), StorageError> {
        self.connection.execute(
            "INSERT INTO session_notes (session_id, page, content) VALUES (?1, ?2, ?3) ON CONFLICT(session_id, page) DO UPDATE SET content=excluded.content",
            params![session_id, page, content],
        )?;
        Ok(())
    }

    /// Retrieve a page, returning `None` when it does not exist.
    pub fn get_note(&self, session_id: &str, page: u32) -> Result<Option<String>, StorageError> {
        Ok(self
            .connection
            .query_row(
                "SELECT content FROM session_notes WHERE session_id=?1 AND page=?2",
                params![session_id, page],
                |row| row.get(0),
            )
            .optional()?)
    }

    /// List a session's pages in page order.
    pub fn list_notes(&self, session_id: &str) -> Result<Vec<(u32, String)>, StorageError> {
        let mut statement = self
            .connection
            .prepare("SELECT page, content FROM session_notes WHERE session_id=?1 ORDER BY page")?;
        let rows = statement.query_map([session_id], |row| Ok((row.get(0)?, row.get(1)?)))?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }

    /// Search page contents as literal text, returning session, page and content.
    pub fn search_notes(
        &self,
        query: &str,
        limit: u32,
    ) -> Result<Vec<(String, u32, String)>, StorageError> {
        if query.is_empty() {
            return Ok(Vec::new());
        }
        let (sql, pattern) = if query.chars().count() >= 3 {
            (
                "SELECT session_id, page, content FROM session_notes_fts WHERE session_notes_fts MATCH ?1 LIMIT ?2",
                format!("\"{}\"", query.replace('"', "\"\"")),
            )
        } else {
            (
                "SELECT session_id, page, content FROM session_notes_fts WHERE instr(lower(content), lower(?1)) > 0 LIMIT ?2",
                query.to_owned(),
            )
        };
        let mut statement = self.connection.prepare(sql)?;
        let rows = statement.query_map(params![pattern, limit], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?))
        })?;
        Ok(rows.collect::<Result<Vec<_>, _>>()?)
    }
}
