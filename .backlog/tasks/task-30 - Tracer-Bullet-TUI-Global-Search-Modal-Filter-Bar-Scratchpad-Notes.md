---
id: TASK-30
title: >-
  Tracer Bullet: TUI Global Search Modal (Ctrl+F), Sidebar Filter (/), &
  Multi-Page Session Scratchpad Notes
status: Done
assignee:
  - '@agy'
created_date: '2026-10-30T20:30:00Z'
updated_date: '2026-10-30T21:01:00Z'
labels:
  - tracer-bullet
  - tui
  - search
  - notes
milestone: m-1
dependencies:
  - TASK-29
references:
  - docs/specs/07_tui_viewport_command_palette_spec.md
priority: high
type: feature
ordinal: 30000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Implements interactive global search, quick filtering, and persistent multi-page scratchpad notes (`FR-P0-09`, `FR-P0-10`, `FR-P1-04`):
1. `REQ-TUI-GLOBAL-SEARCH-007` (`crates/agmux-tui::search_modal`): Interactive `Ctrl+F` Global Search modal in `agmux-tui` with live search query input, matched transcript results list across active and `DEAD` sessions, and a right-hand read-only preview pane auto-scrolled to the matched line, plus a `/` Vim-style quick sidebar filter bar filtering sessions by name, branch, or attached notes.
2. `REQ-TUI-SCRATCHPAD-NOTES-008` (`crates/agmux-core::scratchpad_notes`): Persistent multi-page session scratchpad notes stored in SQLite (`session_notes`), accessible via `agmux note {set|get|list}` CLI / IPC and `Ctrl+PgUp` / `Ctrl+PgDn` (`N` modal fallback) in `agmux-tui`, and searchable via `/` and `Ctrl+F`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Pressing `Ctrl+F` in `agmux-tui` opens the Global Search modal showing FTS5 search hits on the left and a read-only context preview pane auto-scrolled to the matched line on the right
- [x] #2 Pressing `/` in `agmux-tui` opens the quick filter bar filtering visible sidebar sessions by title, branch, or attached scratchpad notes
- [x] #3 Multi-page scratchpad notes are persisted in SQLite (`crates/agmux-core::scratchpad_notes`), queryable/editable via `agmux note` CLI, IPC, and TUI (`Ctrl+PgUp`/`Ctrl+PgDn`)
- [x] #4 Level 3 LLDs (`search_modal_lld.md`, `scratchpad_notes_lld.md`) and Level 5 SVT (`tests/e2e_tui_search_and_notes_test.rs`) pass
<!-- AC:END -->
