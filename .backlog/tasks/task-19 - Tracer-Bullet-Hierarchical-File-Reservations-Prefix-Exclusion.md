---
id: TASK-19
title: 'Tracer Bullet: Hierarchical File Reservations & Prefix Collision Detection'
status: Done
assignee: []
created_date: '2026-10-29T01:25:00Z'
labels:
  - tracer-bullet
  - orchestration
  - reservations
  - files
milestone: m-2
dependencies:
  - TASK-18
references:
  - docs/specs/09_multi_agent_orchestration_bus_spec.md
priority: high
type: feature
ordinal: 19000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Implements `REQ-ORCH-FILE-RESERVATION-003`: coordinates parallel agents working across worktrees by canonicalizing file reservation paths relative to the primary repository root and enforcing directory/file prefix exclusion (e.g. reserving `src/` blocks reserving `src/lib.rs` and vice versa).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 File reservation commands `agmux reserve <path>...` and release supported via CLI and IPC
- [x] #2 Reservation paths canonicalized relative to git repo root regardless of worktree location
- [x] #3 Prefix conflict detection rejects overlapping file and directory reservations
- [x] #4 Integration test asserts cross-worktree reservation conflict rejection and clean release
<!-- AC:END -->
