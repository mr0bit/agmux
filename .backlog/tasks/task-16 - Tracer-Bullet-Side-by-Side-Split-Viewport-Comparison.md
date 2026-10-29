---
id: TASK-16
title: 'Tracer Bullet: Side-by-Side Split Viewport Comparison'
status: Done
assignee:
  - '@agy'
created_date: '2026-10-29T01:18:00Z'
labels:
  - tracer-bullet
  - tui
  - split
  - comparison
milestone: m-1
dependencies:
  - TASK-7
references:
  - docs/specs/07_tui_viewport_command_palette_spec.md
priority: medium
type: feature
ordinal: 16000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Implements `REQ-TUI-SPLIT-COMPARE-005`: pressing `v` toggles a side-by-side split viewport. Pressing `m` pins the primary session on the left pane while arrow keys cycle through target agent sessions on the right pane.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Pressing `v` in TUI splits main viewport into two side-by-side 50/50 panes
- [x] #2 Pressing `m` pins the current session into the primary left pane
- [x] #3 Cycling sidebar selection with arrow keys updates the secondary right pane live
- [x] #4 Integration test asserts split rendering and pin-and-compare behavior
<!-- AC:END -->
