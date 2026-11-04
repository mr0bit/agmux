---
id: TASK-35
title: >-
  Tracer Bullet: Desktop Companion GUI, Real-Time SSE Viewport & Split Monaco Panels
status: Done
assignee:
  - '@codex'
created_date: '2026-11-01T06:15:00Z'
updated_date: '2026-11-01T07:08:00Z'
labels:
  - tracer-bullet
  - desktop
  - gui
  - phase-3
milestone: m-3
dependencies:
  - TASK-34
references:
  - docs/specs/11_desktop_gui_and_notifications_spec.md
priority: high
type: feature
ordinal: 35000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Implements Real-time Desktop Companion GUI Streaming & Split Panels (`FR-P3-01`, `FR-P3-02`):
1. `REQ-GUI-DESKTOP-APP-005` (`crates/agmux-cli::companion_gui`):
   - Server-Sent Events (`/api/events`) real-time terminal viewport streaming (<50ms latency).
   - Side-by-side split layout: terminal session viewport alongside live Monaco/CodeMirror code editor and web previews.
   - OS desktop `.desktop` and protocol handler registration for `agmux://` deep links.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 `/api/events` delivers low-latency SSE push updates for session output and state transitions
- [x] #2 Desktop companion dashboard renders a side-by-side split view with syntax-highlighted code editor and web preview panels
- [x] #3 Integration test (`tests/e2e_desktop_companion_gui_test.rs`) verifies SSE stream delivery, split layout, and deep-link activation
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implemented by OpenAI Codex (Lane: feature1, gpt-6.1-terra). Verified via `tests/e2e_desktop_companion_gui_test.rs`.
<!-- SECTION:NOTES:END -->
