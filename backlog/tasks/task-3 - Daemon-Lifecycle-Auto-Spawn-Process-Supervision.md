---
id: TASK-3
title: Daemon Lifecycle, Auto-Spawn & Process Supervision
status: To Do
assignee: []
created_date: '2026-10-28T22:38:00Z'
labels:
  - daemon
  - process
  - lifecycle
milestone: m-0
dependencies:
  - TASK-2
references:
  - docs/specs/01_core_daemon_lifecycle_spec.md
priority: high
type: feature
ordinal: 3000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Implement background daemon auto-spawn on client connect, process group isolation (setpgid), and clean SIGHUP/SIGTERM signal propagation on shutdown.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Auto-spawn agmuxd detached if socket is absent
- [ ] #2 Child sessions isolated in OS process groups
- [ ] #3 Clean signal propagation on daemon SIGTERM/SIGINT
- [ ] #4 Lockfile enforcement (agmuxd.pid)
<!-- AC:END -->
