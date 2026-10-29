---
id: TASK-15
title: 'Tracer Bullet: TUI Attention Filter & Session Acknowledgment'
status: Done
assignee:
  - '@agy'
created_date: '2026-10-29T01:18:00Z'
labels:
  - tracer-bullet
  - tui
  - filter
  - attention
milestone: m-1
dependencies:
  - TASK-8
references:
  - docs/specs/07_tui_viewport_command_palette_spec.md
priority: medium
type: feature
ordinal: 15000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Implements `REQ-TUI-ATTENTION-FILTER-004`: pressing `w` toggles an Attention Filter hiding `IDLE` and `DEAD` sessions, displaying only `WAITING`, `FINISHED`, `ERRORED`, or `STUCK` sessions. Pressing `.` acknowledges a `FINISHED` session and transitions it to `IDLE`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Pressing `w` in TUI toggles attention filter hiding IDLE and DEAD sessions
- [x] #2 Pressing `.` acknowledges selected FINISHED session, transitioning state to IDLE
- [x] #3 Sidebar UI updates visible session list and indicator badges immediately
- [x] #4 Integration test asserts filtering and acknowledgment transitions
<!-- AC:END -->
