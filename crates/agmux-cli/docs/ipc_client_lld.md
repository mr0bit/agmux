# LLD-IPC-SINGLE-WRITER-003: Single-Writer IPC Client Architecture

* **Unique ID:** `LLD-IPC-SINGLE-WRITER-003`
* **Upstream Requirement:** `REQ-IPC-SINGLE-WRITER-003`
* **Downstream Verification Test:** `TEST-VER-IPC-003` (`tests/e2e_ipc_sync_broadcast_test.rs`)
* **SW Component:** `crates/agmux-cli::ipc_client`

## 1. Overview
All CLI subcommands in `agmux-cli::ipc_client` execute state reads and mutations exclusively by sending framed `IpcMessage` requests over the authenticated daemon socket. The CLI never opens `state.db` directly, eliminating multi-process SQLite lock contention (`SQLITE_BUSY`).

## 2. Concurrency & Serialization Invariant
- `agmuxd` is the sole process holding an open `rusqlite::Connection` (`StorageEngine`), serialized behind an internal `Arc<Mutex<StorageEngine>>` in WAL mode (`PRAGMA journal_mode=WAL`).
- Concurrent IPC client requests are multiplexed over independent Tokio tasks and serialized cleanly at the daemon's storage mutex, supporting 50+ simultaneous CLI operations alongside background FTS log indexing with zero `SQLITE_BUSY` errors.
