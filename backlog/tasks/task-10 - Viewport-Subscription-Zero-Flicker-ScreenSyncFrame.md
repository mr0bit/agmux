---
id: TASK-10
title: Viewport Subscription & Zero-Flicker ScreenSyncFrame
status: To Do
assignee: []
created_date: '2026-10-28T22:39:00Z'
labels:
  - ipc
  - viewport
  - sync
milestone: m-0
dependencies:
  - TASK-2
  - TASK-4
references:
  - docs/specs/02_ipc_transport_protocol_spec.md
priority: high
type: feature
ordinal: 10000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Implement daemon viewport subscription manager streaming atomic ScreenSyncFrame upon connection for zero-flicker reattachment and dirty delta diffs.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 ScreenSyncFrame with complete VT100 character matrix and cursor
- [ ] #2 Incremental dirty cell delta broadcasting
<!-- AC:END -->
