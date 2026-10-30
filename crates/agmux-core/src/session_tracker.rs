//! Automatic native conversation UUID capture via pre-minted flags and store watchers (`REQ-PLUGIN-UUID-CAPTURE-005`).

use std::path::Path;

use crate::storage_engine::{StorageEngine, StorageError};

/// Coordinates pre-minting UUID v4 identifiers, scanning agent session store directories, and persisting
/// captured conversation UUIDs into SQLite (`sessions.cli_session_uuid`).
#[derive(Debug, Clone, Default)]
pub struct SessionTracker;

impl SessionTracker {
    /// Generate a RFC 4122 v4 UUID string using cryptographic OS randomness.
    pub fn mint_uuid_v4() -> String {
        let bytes = crate::generate_random_32_bytes().unwrap_or([0x42; 32]);
        let mut b = [0u8; 16];
        b.copy_from_slice(&bytes[..16]);
        // Set version 4 and RFC 4122 variant bits
        b[6] = (b[6] & 0x0f) | 0x40;
        b[8] = (b[8] & 0x3f) | 0x80;
        format!(
            "{:02x}{:02x}{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}-{:02x}{:02x}{:02x}{:02x}{:02x}{:02x}",
            b[0],
            b[1],
            b[2],
            b[3],
            b[4],
            b[5],
            b[6],
            b[7],
            b[8],
            b[9],
            b[10],
            b[11],
            b[12],
            b[13],
            b[14],
            b[15]
        )
    }

    /// Pre-mint a UUID for a session, persist it to `sessions.cli_session_uuid`, and return `(uuid, spawn_args)`.
    pub fn inject_and_persist_uuid(
        db: &StorageEngine,
        session_id: &str,
        flag: &str,
        extra_args: &[String],
    ) -> Result<(String, Vec<String>), StorageError> {
        let uuid = Self::mint_uuid_v4();
        db.set_cli_session_uuid(session_id, Some(&uuid))?;
        let mut args = vec![flag.to_string(), uuid.clone()];
        args.extend_from_slice(extra_args);
        Ok((uuid, args))
    }

    /// Inspect a session store directory (e.g. `~/.gemini/antigravity-cli/brain` or `.codex/sessions`)
    /// for the most recently modified UUID-named entry, and persist it into `sessions.cli_session_uuid`.
    pub fn capture_from_store_dir(
        db: &StorageEngine,
        session_id: &str,
        store_dir: &Path,
    ) -> Result<Option<String>, StorageError> {
        let Ok(entries) = std::fs::read_dir(store_dir) else {
            return Ok(None);
        };
        let mut newest: Option<(std::time::SystemTime, String)> = None;
        for entry in entries.flatten() {
            let file_name = entry.file_name().to_string_lossy().to_string();
            let candidate = file_name
                .strip_suffix(".json")
                .or_else(|| file_name.strip_suffix(".jsonl"))
                .unwrap_or(&file_name)
                .to_string();
            if Self::looks_like_uuid_or_session_token(&candidate) {
                let mtime = entry
                    .metadata()
                    .and_then(|m| m.modified())
                    .unwrap_or(std::time::SystemTime::UNIX_EPOCH);
                if newest.as_ref().is_none_or(|(prev_t, _)| mtime >= *prev_t) {
                    newest = Some((mtime, candidate));
                }
            }
        }
        if let Some((_, uuid)) = newest {
            db.set_cli_session_uuid(session_id, Some(&uuid))?;
            Ok(Some(uuid))
        } else {
            Ok(None)
        }
    }

    /// Extract a UUID v4 from a line of terminal stdout if present.
    pub fn extract_uuid_from_line(line: &str) -> Option<String> {
        for token in line.split(|c: char| !c.is_ascii_hexdigit() && c != '-') {
            if Self::is_uuid_format(token) {
                return Some(token.to_string());
            }
        }
        None
    }

    /// Check if `s` has standard 36-char `8-4-4-4-12` hex UUID structure.
    pub fn is_uuid_format(s: &str) -> bool {
        if s.len() != 36 {
            return false;
        }
        let parts: Vec<&str> = s.split('-').collect();
        if parts.len() != 5 {
            return false;
        }
        let lens = [8, 4, 4, 4, 12];
        parts
            .iter()
            .zip(lens.iter())
            .all(|(p, &len)| p.len() == len && p.chars().all(|c| c.is_ascii_hexdigit()))
    }

    fn looks_like_uuid_or_session_token(s: &str) -> bool {
        Self::is_uuid_format(s) || (s.len() >= 8 && !s.starts_with('.'))
    }
}
