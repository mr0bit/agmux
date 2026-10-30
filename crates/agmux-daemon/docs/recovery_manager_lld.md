# LLD-DAEMON-RECOVERY-005: Orphaned Session Crash Recovery Manager

* **Unique ID:** `LLD-DAEMON-RECOVERY-005`
* **Upstream Requirement:** `REQ-DAEMON-RECOVERY-005`
* **Downstream Verification Test:** `TEST-VER-DAEMON-005` (`tests/e2e_daemon_resilience_test.rs`)
* **SW Component:** `crates/agmux-daemon::recovery_manager`

## 1. Overview
`RecoveryManager` runs during `DaemonServer` startup after opening `StorageEngine` and before accepting client connections. It scans for sessions left in active states (`RUNNING`, `WORKING`, `WAITING`, `WAITING_INPUT`, `STUCK`) from an unclean prior daemon termination (`kill -9` or power loss).

## 2. Recovery Steps
1. Queries all sessions in `sessions` whose state is in `('RUNNING', 'WORKING', 'WAITING', 'WAITING_INPUT', 'STUCK')`.
2. Transitions each orphaned session's state to `'DEAD'` while preserving its stored `session_snapshots` and FTS logs.
3. Releases any `named_locks` and `file_reservations` held by the orphaned session ID.
4. Appends a `session.recovered_dead` event to `event_journal` for each recovered session.
