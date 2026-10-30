---
id: TASK-23
title: 'Tracer Bullet: Atomic ScreenSyncFrame, Push Event Broadcaster & Single-Writer IPC'
status: Done
assignee:
  - '@codex'
  - '@agy'
created_date: '2026-10-30T00:00:00Z'
labels:
  - tracer-bullet
  - ipc
  - sync-frame
  - broadcaster
milestone: m-2
dependencies:
  - TASK-22
references:
  - docs/specs/02_ipc_transport_protocol_spec.md
  - REQ-IPC-SINGLE-WRITER-003
  - REQ-IPC-SYNC-FRAME-004
  - REQ-IPC-EVENT-DISPATCH-005
priority: high
type: feature
ordinal: 23000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Implements `REQ-IPC-SINGLE-WRITER-003`, `REQ-IPC-SYNC-FRAME-004`, and `REQ-IPC-EVENT-DISPATCH-005`: guarantees all CLI subcommands execute exclusively over framed IPC without touching `state.db` directly, returns an atomic `ScreenSyncFrame` (VT100 grid, cursor position, scrollback offset) upon `SubscribeViewport` for zero-flicker reattachment, and dispatches asynchronous `NotificationEvent` and `SessionListUpdate` push frames on state transitions.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 50 concurrent CLI requests execute over IPC with zero SQLITE_BUSY errors
- [x] #2 SubscribeViewport immediately delivers atomic ScreenSyncFrame with VT100 grid, cursor coordinates, and scrollback offset
- [x] #3 Event broadcaster pushes NotificationEvent and SessionListUpdate asynchronously on state transitions
<!-- AC:END -->
