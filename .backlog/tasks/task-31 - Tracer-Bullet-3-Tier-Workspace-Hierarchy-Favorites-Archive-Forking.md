---
id: TASK-31
title: >-
  Tracer Bullet: 3-Tier Workspace Hierarchy, Favorites, Archive/Restore, Forking
  & Quick Prompt Modifiers
status: Done
assignee:
  - '@codex'
created_date: '2026-10-30T20:30:00Z'
updated_date: '2026-10-30T21:16:00Z'
labels:
  - tracer-bullet
  - tui
  - hierarchy
  - workflow
milestone: m-1
dependencies:
  - TASK-30
references:
  - docs/specs/07_tui_viewport_command_palette_spec.md
priority: high
type: feature
ordinal: 31000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Implements the Phase 1 Power Workflow TUI hierarchy and session lifecycle controls (`FR-P1-01`–`FR-P1-06`, `FR-P0-16`):
1. `REQ-TUI-WORKSPACE-HIERARCHY-009` (`crates/agmux-tui::workspace_hierarchy`):
   - 3-Tier Workspace Hierarchy (`Project` -> nested `Groups/Folders` via `g` -> `Session`), plus `⭐ Favorites` pinning (`*`, auto-hidden when empty) and Compact (1-line) vs. Comfortable (3-line) row density toggle (`D`).
   - Session Archive (`a`), toggle archived view (`t`), and Restore (`u`) in both TUI and daemon metadata.
   - 1-key Conversation Forking menu (`f` -> `s` for New Standalone Session or `t` for New Sub-Tab).
   - Quick Prompt (`Space`) inline spawn/reply modifiers cycling CLI tool (`Tab`), model (`Ctrl+L`), reasoning effort (`Ctrl+X`), and worktree toggle (`Ctrl+T`).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 3-Tier Workspace Hierarchy (`Project` / `Group` `g` / `Session`), `⭐ Favorites` (`*`), and Compact vs. Comfortable 3-line row density (`D`) render and navigate cleanly in `agmux-tui`
- [x] #2 Archiving (`a`), toggling archived view (`t`), and restoring (`u`) sessions work end-to-end
- [x] #3 Pressing `f` opens the Conversation Fork menu supporting standalone session (`s`) and sub-tab (`t`) forks
- [x] #4 Quick Prompt (`Space`) supports inline modifier cycling for CLI tool (`Tab`), model (`Ctrl+L`), reasoning effort (`Ctrl+X`), and worktree (`Ctrl+T`), verified by SVT (`tests/e2e_tui_workspace_hierarchy_test.rs`)
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implemented workspace metadata and hierarchy, archive and restore, adapter-backed fork dispatch, and quick prompt modifiers. Verification: cargo check --workspace, cargo test --workspace, cargo clippy --workspace -- -D warnings, cargo fmt --check, and node .soul/tools/v-cycle-checker.mjs passed; see tests/reports/tui_workspace_hierarchy_test_report.md and docs/assets/screenshots/task-31_verification.png.
<!-- SECTION:NOTES:END -->
