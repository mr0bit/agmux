# Software Requirements Specification: Storage Persistence, FTS5 Search & Snapshots

## 1. Overview
This specification details the single-writer SQLite WAL database, embedded atomic schema migrations, ANSI strip ingestion, rolling FTS5 trigram search index, and zstd level 3 dead-session snapshot compression.

---

## 2. Requirements

### REQ-STORAGE-SQLITE-WAL-001
* **Unique ID:** `REQ-STORAGE-SQLITE-WAL-001`
* **Text:** As `agmuxd` persists state, the system shall operate a bundled SQLite database in WAL mode (`journal_mode = WAL`, `synchronous = NORMAL`, `temp_store = MEMORY`) located at `~/.local/share/agmux/state.db` (or `%LOCALAPPDATA%\agmux\state.db`), acting as the sole writer process.
* **Criticality:** High
* **SW Component:** `crates/agmux-core::storage_engine`
* **Implementation Status:** Verified
* **Verification Criteria:** Perform 1,000 concurrent read transactions while writing batches; verify zero database corruptions and sub-millisecond query execution.
* **Downstream Req:** `LLD-STORAGE-SQLITE-WAL-001`
* **Downstream Test:** `TEST-VER-STORAGE-001`
* **Test Status:** Passed

---

### REQ-STORAGE-ATOMIC-MIGRATION-002
* **Unique ID:** `REQ-STORAGE-ATOMIC-MIGRATION-002`
* **Text:** As new versions of Agmux are deployed, `agmuxd` on boot shall inspect `PRAGMA user_version` and apply embedded incremental SQL migrations within a single atomic transaction before serving any IPC requests.
* **Criticality:** High
* **SW Component:** `crates/agmux-core::migration_manager`
* **Implementation Status:** Verified
* **Verification Criteria:** Seed a version 0 database with mock data; start the daemon with version 1 schema migrations; verify that all tables, columns, and indexes migrate cleanly and `PRAGMA user_version` updates atomically.
* **Downstream Req:** `LLD-STORAGE-ATOMIC-MIGRATION-002`
* **Downstream Test:** `TEST-VER-STORAGE-002`
* **Test Status:** Passed

---

### REQ-STORAGE-FTS5-RETENTION-003
* **Unique ID:** `REQ-STORAGE-FTS5-RETENTION-003`
* **Text:** As terminal output is generated, the reader task shall strip ANSI escapes and batch lines every 250ms into the `terminal_logs_fts` FTS5 virtual table (`tokenize = 'trigram'`), enforcing a configurable rolling retention limit (default 50,000 lines per session) loaded from `config.toml`.
* **Criticality:** High
* **SW Component:** `crates/agmux-core::fts_indexer`
* **Implementation Status:** Not implemented
* **Verification Criteria:** Feed 100,000 terminal lines into a session; verify that lines exceeding the 50,000-line cap are pruned in FIFO order, keeping trigram search fast and disk growth bounded.
* **Downstream Req:** `LLD-STORAGE-FTS5-RETENTION-003`
* **Downstream Test:** `TEST-VER-STORAGE-003`
* **Test Status:** Failed

---

### REQ-STORAGE-ZSTD-SNAPSHOT-004
* **Unique ID:** `REQ-STORAGE-ZSTD-SNAPSHOT-004`
* **Text:** As a user kills a session (`x` -> `DEAD`), the system shall capture the full colored ANSI scrollback from the `vt100::Parser`, compress it with zstd level 3 into `sessions.ansi_snapshot`, and drop the in-memory parser and PTY master, reducing the session's active RAM footprint to 0 bytes.
* **Criticality:** High
* **SW Component:** `crates/agmux-core::snapshot_compressor`
* **Implementation Status:** Not implemented
* **Verification Criteria:** Kill a 10,000-line active agent session; verify that the ANSI buffer compresses to under 60 KB in less than 2ms, writes to SQLite, and daemon memory decreases accordingly.
* **Downstream Req:** `LLD-STORAGE-ZSTD-SNAPSHOT-004`
* **Downstream Test:** `TEST-VER-STORAGE-004`
* **Test Status:** Failed

---

### REQ-STORAGE-CONFIG-HOTRELOAD-005
* **Unique ID:** `REQ-STORAGE-CONFIG-HOTRELOAD-005`
* **Text:** As a user edits `~/.config/agmux/config.toml` (or `%APPDATA%\agmux\config.toml` on Windows), `agmuxd` shall watch the file using the `notify` crate and hot-reload retention caps, settle debounce windows, and notification rules without restarting or dropping sessions.
* **Criticality:** Medium
* **SW Component:** `crates/agmux-daemon::config_watcher`
* **Implementation Status:** Not implemented
* **Verification Criteria:** Modify `idle_after_ms` from 1000 to 2000 in `config.toml` while `agmuxd` is running; verify via daemon log output that the configuration is reloaded and the new debounce window is immediately applied.
* **Downstream Req:** `LLD-STORAGE-CONFIG-HOTRELOAD-005`
* **Downstream Test:** `TEST-VER-STORAGE-005`
* **Test Status:** Failed
