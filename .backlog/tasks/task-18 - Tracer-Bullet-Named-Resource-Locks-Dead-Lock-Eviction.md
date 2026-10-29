---
id: TASK-18
title: 'Tracer Bullet: Named Resource Locks & Dead-Lock Auto-Eviction'
status: Done
assignee: []
created_date: '2026-10-29T01:25:00Z'
labels:
  - tracer-bullet
  - orchestration
  - locks
  - eviction
milestone: m-2
dependencies:
  - TASK-17
references:
  - docs/specs/09_multi_agent_orchestration_bus_spec.md
priority: high
type: feature
ordinal: 18000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Implements `REQ-ORCH-DEAD-LOCK-EVICT-002`: manages named resource locks (`agmux lock acquire <name> [--wait]`). When a lock-holding session transitions to `DEAD` or `ERRORED`, automatically evicts all of that session's held locks and wakes queued sessions in FIFO order.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Named lock acquisition and release supported via CLI and IPC
- [x] #2 Lock contention queues callers in FIFO order when `--wait` is requested
- [x] #3 Termination or error of lock holder automatically evicts locks and wakes next waiter
- [x] #4 Integration test asserts lock mutual exclusion, wait queuing, and dead-lock auto-eviction
<!-- AC:END -->
