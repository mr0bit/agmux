# LLD-DAEMON-SHUTDOWN-SIG-003: Graceful Child Process Group Shutdown Coordinator

* **Unique ID:** `LLD-DAEMON-SHUTDOWN-SIG-003`
* **Upstream Requirement:** `REQ-DAEMON-SHUTDOWN-SIG-003`
* **Downstream Verification Test:** `TEST-VER-DAEMON-003` (`tests/e2e_daemon_resilience_test.rs`)
* **SW Component:** `crates/agmux-daemon::shutdown_coordinator`

## 1. Overview
`ShutdownCoordinator` tracks active child process identifiers (PIDs) spawned inside PTY sessions and orchestrates graceful two-stage termination (`SIGHUP` followed by `SIGTERM` on Unix; `taskkill /F /T /PID` on Windows) before the daemon exits.

## 2. Termination Sequence
1. On receiving a daemon termination signal (`SIGTERM`/`SIGINT`) or `ShutdownDaemonRequest` IPC message, `ShutdownCoordinator::shutdown_all` snapshots all registered active session PIDs.
2. **Stage 1 (`SIGHUP`):** Sends `SIGHUP` to both the negative process group ID (`-(pid as pid_t)`) and direct process ID (`pid as pid_t`) on Unix, allowing shells and agent CLIs to flush state.
3. **Grace Period:** Waits a brief grace interval (e.g., 50ms) for processes to exit cleanly.
4. **Stage 2 (`SIGTERM`):** Sends `SIGTERM` to remaining active child process groups and PIDs (or `taskkill /F /T /PID <pid>` on Windows) so no zombie or orphan agent processes remain.
