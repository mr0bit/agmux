# LLD-DAEMON-LOCKFILE-004: Cross-Platform Single-Instance Lockfile

* **Unique ID:** `LLD-DAEMON-LOCKFILE-004`
* **Upstream Requirement:** `REQ-DAEMON-LOCKFILE-004`
* **Downstream Verification Test:** `TEST-VER-DAEMON-004` (`tests/e2e_daemon_resilience_test.rs`)
* **SW Component:** `crates/agmux-daemon::instance_lock`

## 1. Overview
`InstanceLock` enforces single-instance execution per user account / socket path using an OS-level exclusive lockfile (`agmuxd.pid` adjacent to a filesystem socket, a data-directory lock for a named pipe, or an explicitly configured path), holding the open `std::fs::File` descriptor for the entire lifetime of the daemon.

## 2. OS Lock Semantics & Ordering Invariant
- **Unix:** Opens or creates the lockfile and invokes `libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB)`. If `flock` returns `EWOULDBLOCK` / failure, another `agmuxd` holds the lock and `InstanceLock::acquire` returns an error immediately.
- **Windows:** Opens the lockfile with `OpenOptionsExt::share_mode(0)` so concurrent open attempts fail immediately.
- **Socket Safety Invariant:** `DaemonServer::run` acquires `InstanceLock` **before** unlinking any stale Unix socket file (`fs::remove_file`), guaranteeing that a duplicate `agmuxd` invocation never removes an active daemon's socket.

## Windows named-pipe startup

A `\\.\pipe\...` address is not a normal filesystem path. Never derive a lock
or database file by changing its extension. For named pipes, lock files live in
`run/` beside the configured/default database, with a deterministic case-insensitive
endpoint identity in the filename. A 64-bit FNV-1a identifier keeps arbitrary pipe
names out of filesystem names; it is not an authentication mechanism. The default
Windows location is `%LOCALAPPDATA%/agmux/run/agmuxd-<identity>.pid`.
`AGMUX_LOCK_PATH` still overrides this location. All literal named-pipe endpoints
use the configured/default database path, including custom named-pipe addresses.
Normal Unix socket/file mappings retain their existing behavior.

Windows share/lock violations report an already-running daemon; other open errors
preserve their actual filesystem cause instead of claiming lock contention.
The path regression test acquires a real filesystem lock for a named-pipe identity,
checks identity separation and case normalization, and excludes the pipe namespace.
Release CI launches the executables with default endpoint/token/database/lock
resolution and isolated Windows data directories, not a filesystem AGMUX_SOCK.
