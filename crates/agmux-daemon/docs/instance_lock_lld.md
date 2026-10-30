# LLD-DAEMON-LOCKFILE-004: Cross-Platform Single-Instance Lockfile

* **Unique ID:** `LLD-DAEMON-LOCKFILE-004`
* **Upstream Requirement:** `REQ-DAEMON-LOCKFILE-004`
* **Downstream Verification Test:** `TEST-VER-DAEMON-004` (`tests/e2e_daemon_resilience_test.rs`)
* **SW Component:** `crates/agmux-daemon::instance_lock`

## 1. Overview
`InstanceLock` enforces single-instance execution per user account / socket path using an OS-level exclusive lockfile (`agmuxd.pid` adjacent to the socket or configured path), holding the open `std::fs::File` descriptor for the entire lifetime of the daemon.

## 2. OS Lock Semantics & Ordering Invariant
- **Unix:** Opens or creates the lockfile and invokes `libc::flock(file.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB)`. If `flock` returns `EWOULDBLOCK` / failure, another `agmuxd` holds the lock and `InstanceLock::acquire` returns an error immediately.
- **Windows:** Opens the lockfile with `OpenOptionsExt::share_mode(0)` so concurrent open attempts fail immediately.
- **Socket Safety Invariant:** `DaemonServer::run` acquires `InstanceLock` **before** unlinking any stale Unix socket file (`fs::remove_file`), guaranteeing that a duplicate `agmuxd` invocation never removes an active daemon's socket.
