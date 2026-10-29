---
id: TASK-7
title: 'Tracer Bullet: Ratatui TUI & Live Viewport Streaming'
status: Done
assignee: []
created_date: '2026-10-28T23:40:00Z'
updated_date: '2026-10-29T00:40:00Z'
labels:
  - tracer-bullet
  - tui
  - ratatui
  - viewport
milestone: m-0
dependencies:
  - TASK-2
  - TASK-5
references:
  - docs/specs/07_tui_viewport_command_palette_spec.md
priority: high
type: feature
ordinal: 7000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Running 'agmux' launches full-screen interactive Ratatui TUI. Subscribes to daemon's ScreenSyncFrame stream, rendering live 60 FPS zero-flicker VT100 screen grid alongside a sidebar showing active sessions with colored state badges.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Ratatui TUI establishes delta subscription stream with daemon
- [x] #2 Main viewport renders 60 FPS VT100 grid with zero flicker
- [x] #3 Sidebar displays sessions with colored state badges (RUNNING, WAITING_INPUT, STUCK, etc.)
- [x] #4 Keyboard navigation switches active pane and session smoothly
<!-- AC:END -->
