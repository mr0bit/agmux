# Software Requirements Specification: Core Daemon Lifecycle & Process Supervision

## 1. Overview
This specification defines the lifecycle, process supervision, signal propagation, and auto-spawning requirements for the background daemon binary (`agmuxd`).

---

## 2. Requirements

### REQ-DAEMON-AUTOSPAWN-001
* **Unique ID:** `REQ-DAEMON-AUTOSPAWN-001`
* **Text:** As a user invoking any `agmux` frontend or CLI subcommand when the background daemon is not running, the system shall automatically spawn `agmuxd` detached as an independent background daemon process before establishing the IPC connection, requiring zero manual daemon launch commands.
* **Criticality:** High
* **SW Component:** `crates/agmux-cli::daemon_spawner`
* **Implementation Status:** Verified
* **Verification Criteria:** Verify that invoking `agmux list` or `agmux` TUI on a clean system with no running `agmuxd` successfully starts `agmuxd` detached, exits the launch helper, and successfully connects over IPC within 500ms without leaving terminal lockups.
* **Downstream Req:** `LLD-DAEMON-AUTOSPAWN-001`
* **Downstream Test:** `TEST-VER-DAEMON-001`
* **Test Status:** Passed

---

### REQ-DAEMON-PROCGROUP-002
* **Unique ID:** `REQ-DAEMON-PROCGROUP-002`
* **Text:** As a user running multiple concurrent CLI agents, the system shall isolate each agent session and its child processes into a dedicated operating system process group (`setpgid` on Unix, Job Objects on Windows), ensuring that process operations on one session do not affect sibling sessions or the daemon.
* **Criticality:** High
* **SW Component:** `crates/agmux-daemon::process_supervisor`
* **Implementation Status:** Verified
* **Verification Criteria:** Spawn two agent sessions; send a SIGKILL to the process group of session A. Verify that all child processes of session A terminate, while session B and `agmuxd` continue running with zero state disruption.
* **Downstream Req:** `LLD-DAEMON-PROCGROUP-002`
* **Downstream Test:** `TEST-VER-DAEMON-002`
* **Test Status:** Passed

---

### REQ-DAEMON-SHUTDOWN-SIG-003
* **Unique ID:** `REQ-DAEMON-SHUTDOWN-SIG-003`
* **Text:** As a system administrator terminating `agmuxd` via SIGTERM or SIGINT, the system shall gracefully propagate SIGHUP followed by SIGTERM to all active child agent process groups before exiting, preventing orphaned runaway agent processes that leak memory and API tokens.
* **Criticality:** High
* **SW Component:** `crates/agmux-daemon::shutdown_coordinator`
* **Implementation Status:** Not implemented
* **Verification Criteria:** Launch 3 active agent sessions with running child build processes; send SIGTERM to `agmuxd`. Verify via OS process inspection that within 3 seconds, all 3 agent processes and their child build trees are reaped and no zombie processes remain.
* **Downstream Req:** `LLD-DAEMON-SHUTDOWN-SIG-003`
* **Downstream Test:** `TEST-VER-DAEMON-003`
* **Test Status:** Failed

---

### REQ-DAEMON-LOCKFILE-004
* **Unique ID:** `REQ-DAEMON-LOCKFILE-004`
* **Text:** As a user starting `agmuxd`, the system shall enforce single-instance execution per user account using a cross-platform filesystem lock (`agmuxd.pid` / flock / LockFileEx), refusing duplicate daemon instances and routing calls to the existing daemon.
* **Criticality:** Medium
* **SW Component:** `crates/agmux-daemon::instance_lock`
* **Implementation Status:** Not implemented
* **Verification Criteria:** Attempt to run `agmuxd` while an instance is already active. Verify that the second command exits cleanly with code 0 or an informative message without corrupting sockets or SQLite locks.
* **Downstream Req:** `LLD-DAEMON-LOCKFILE-004`
* **Downstream Test:** `TEST-VER-DAEMON-004`
* **Test Status:** Failed

---

### REQ-DAEMON-RECOVERY-005
* **Unique ID:** `REQ-DAEMON-RECOVERY-005`
* **Text:** As a user recovering from an ungraceful daemon crash or system reboot, `agmuxd` on startup shall scan the SQLite database for orphaned sessions previously marked `WORKING` or `WAITING` and transition their state to `DEAD`, preserving their ANSI scrollback snapshots for post-mortem inspection.
* **Criticality:** High
* **SW Component:** `crates/agmux-daemon::recovery_manager`
* **Implementation Status:** Not implemented
* **Verification Criteria:** Simulate a sudden power kill (`kill -9`) of `agmuxd` during an active agent turn. Restart `agmuxd`. Verify that all orphaned sessions are safely marked `DEAD` with snapshot inspection available and zero hanging lock records.
* **Downstream Req:** `LLD-DAEMON-RECOVERY-005`
* **Downstream Test:** `TEST-VER-DAEMON-005`
* **Test Status:** Failed
