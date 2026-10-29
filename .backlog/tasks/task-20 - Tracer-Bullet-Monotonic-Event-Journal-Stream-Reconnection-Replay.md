---
id: TASK-20
title: 'Tracer Bullet: Monotonic Event Journal Stream & Reconnection Replay'
status: Done
assignee: []
created_date: '2026-10-29T01:25:00Z'
labels:
  - tracer-bullet
  - orchestration
  - journal
  - events
milestone: m-2
dependencies:
  - TASK-19
references:
  - docs/specs/09_multi_agent_orchestration_bus_spec.md
priority: medium
type: feature
ordinal: 20000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Implements `REQ-ORCH-EVENT-WATCH-004`: streams journaled workspace events backed by SQLite monotonic sequence numbers via `agmux watch [--as <cursor>] [--follow]`, enabling clients to reconnect and replay missed events without loss.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 All workspace events persisted with monotonic sequence IDs in SQLite event journal
- [x] #2 CLI `agmux watch --as <cursor> --follow` streams live events and records cursor position
- [x] #3 Disconnected cursor reconnecting replays all missed events in exact monotonic order
- [x] #4 Integration test asserts sequential event logging, live follow, and cursor replay
<!-- AC:END -->
