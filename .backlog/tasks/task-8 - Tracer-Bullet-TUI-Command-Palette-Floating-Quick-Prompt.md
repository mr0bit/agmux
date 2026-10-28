---
id: TASK-8
title: 'Tracer Bullet: TUI Command Palette & Floating Quick-Prompt'
status: To Do
assignee: []
created_date: '2026-10-28T23:40:00Z'
labels:
  - tracer-bullet
  - palette
  - quick-prompt
  - ux
milestone: m-0
dependencies:
  - TASK-7
references:
  - docs/specs/07_tui_viewport_command_palette_spec.md
priority: high
type: feature
ordinal: 8000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
In TUI, Ctrl+P opens a fuzzy-filtered command palette with universal fallbacks. Pressing Space or hotkey opens a floating modal quick-prompt bar to inject prompt text directly into the focused agent session.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Ctrl+P displays fuzzy command palette with action execution
- [ ] #2 Quick-prompt modal accepts user input and dispatches to active session PTY
- [ ] #3 Fallback mechanisms handle unmapped hotkeys gracefully
- [ ] #4 UI seam test verifies keyboard focus capture and event dispatch
<!-- AC:END -->
