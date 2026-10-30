---
id: TASK-21
title: 'Tracer Bullet: Autonomous Kanban Board & Worker Reporting'
status: Done
assignee:
  - '@codex'
  - '@agy'
created_date: '2026-10-29T01:25:00Z'
labels:
  - tracer-bullet
  - orchestration
  - kanban
  - report
milestone: m-2
dependencies:
  - TASK-20
references:
  - docs/specs/09_multi_agent_orchestration_bus_spec.md
priority: medium
type: feature
ordinal: 21000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Implements `REQ-ORCH-KANBAN-REPORT-005`: provides a shared SQLite-backed Kanban board (`agmux task {list|create|claim|finish|release}`) and structured worker reports (`agmux report {post|view}`) for multi-agent autonomous coordination.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Kanban tasks managed with atomic claim, finish, and release operations in SQLite
- [x] #2 Structured reports with PR URL, commit SHA, and metrics attached to tasks
- [x] #3 CLI commands `agmux task` and `agmux report` supported over IPC
- [x] #4 Integration test asserts multi-agent task claiming, report posting, and status transitions
<!-- AC:END -->
