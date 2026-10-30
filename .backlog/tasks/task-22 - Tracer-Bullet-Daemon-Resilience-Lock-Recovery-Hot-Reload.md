---
id: TASK-22
title: 'Tracer Bullet: Daemon Resilience, Single-Instance Lock, Crash Recovery & Config Hot-Reload'
status: Done
assignee:
  - '@codex'
  - '@agy'
created_date: '2026-10-30T00:00:00Z'
labels:
  - tracer-bullet
  - daemon
  - lifecycle
  - recovery
  - hot-reload
milestone: m-2
dependencies:
  - TASK-21
references:
  - docs/specs/01_core_daemon_lifecycle_spec.md
  - docs/specs/06_storage_persistence_fts_spec.md
  - REQ-DAEMON-SHUTDOWN-SIG-003
  - REQ-DAEMON-LOCKFILE-004
  - REQ-DAEMON-RECOVERY-005
  - REQ-STORAGE-CONFIG-HOTRELOAD-005
priority: high
type: feature
ordinal: 22000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Implements `REQ-DAEMON-SHUTDOWN-SIG-003`, `REQ-DAEMON-LOCKFILE-004`, `REQ-DAEMON-RECOVERY-005`, and `REQ-STORAGE-CONFIG-HOTRELOAD-005`: graceful SIGHUP/SIGTERM propagation to all active child session process groups on daemon shutdown, single-instance lockfile enforcement (`agmuxd.pid`), startup crash recovery transitioning orphaned `WORKING`/`WAITING` sessions to `DEAD` while preserving snapshots, and dynamic `config.toml` hot-reloading.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Graceful shutdown coordinator propagates SIGHUP/SIGTERM to child process groups within 3s
- [x] #2 Single-instance lockfile prevents duplicate daemon instances cleanly
- [x] #3 Startup recovery manager transitions orphaned active sessions in SQLite to DEAD
- [x] #4 Config watcher hot-reloads config.toml parameters dynamically without dropping sessions
<!-- AC:END -->
