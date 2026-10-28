---
id: TASK-12
title: Command Palette with Universal Fallbacks
status: To Do
assignee: []
created_date: '2026-10-28T22:39:00Z'
labels:
  - tui
  - command-palette
  - nucleo
milestone: m-0
dependencies:
  - TASK-11
references:
  - docs/specs/07_tui_viewport_command_palette_spec.md
priority: high
type: feature
ordinal: 12000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Implement VS Code-style Command Palette modal in agmux-tui triggered by Ctrl+Shift+P, F1, and : searching static actions and dynamic targets with nucleo.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Command Palette modal overlay
- [ ] #2 Universal terminal fallbacks (F1 and :)
- [ ] #3 Nucleo fuzzy matching across static commands and session targets
<!-- AC:END -->
