---
id: TASK-3
title: 'Tracer Bullet: Session Persistence & agmux list'
status: Done
assignee: []
created_date: '2026-10-28T23:40:00Z'
updated_date: '2026-10-29T00:14:00Z'
labels:
  - tracer-bullet
  - storage
  - cli
  - persistence
milestone: m-0
dependencies:
  - TASK-2
references:
  - docs/specs/01_core_daemon_lifecycle_spec.md
  - docs/specs/06_storage_persistence_fts_spec.md
priority: high
type: feature
ordinal: 3000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Daemon initializes SQLite in WAL mode with atomic schema migrations and registers new sessions. CLI 'agmux list' connects via IPC, queries active sessions, and prints formatted table with session ID, PID, working directory, and start time.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 SQLite WAL database initializes with atomic migration table
- [x] #2 Spawned PTY sessions persist metadata into sessions table
- [x] #3 CLI agmux list displays table of running sessions queried via IPC
- [x] #4 SUT tests verify database transactions and CLI list rendering
<!-- AC:END -->
