---
id: TASK-32
title: >-
  Tracer Bullet: Rich Notification Hooks & Phase 3 Desktop/Web Companion Bridge
  (agmux gui & panels open)
status: Done
assignee:
  - '@agy'
created_date: '2026-10-30T20:30:00Z'
updated_date: '2026-10-31T01:30:00Z'
labels:
  - tracer-bullet
  - notifications
  - gui
  - phase-3
milestone: m-3
dependencies:
  - TASK-31
references:
  - docs/specs/11_desktop_gui_and_notifications_spec.md
priority: high
type: feature
ordinal: 32000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Implements Rich Notifications & Custom Shell Notification Hooks (`FR-P0-08`) and the Phase 3 Desktop/Web Companion Bridge (`FR-P3-01`–`FR-P3-03`):
1. `REQ-GUI-NOTIFICATION-HOOKS-001` (`crates/agmux-daemon::notification_dispatcher`): Emits structured notifications (with 120-character output quote) on transitions to `WAITING`, `FINISHED`, or `ERRORED`, and executes custom shell notification hooks with `AGMUX_NOTIFY_KIND`, `AGMUX_SESSION_ID`, `AGMUX_SESSION_NAME`, `AGMUX_TOOL`, `AGMUX_CWD`, `AGMUX_BRANCH`, `AGMUX_TITLE`, and `AGMUX_BODY` environment variables under a 15-second timeout.
2. `REQ-GUI-COMPANION-PANELS-002` (`crates/agmux-cli::companion_gui`): Provides `agmux panels open <url|file>` (docking/registering file editor and web preview panels in the session workspace), `agmux://` URI deep-link resolution (`agmux open agmux://session/<id>`), and `agmux gui` (Phase 3 companion GUI bridge & HTML/JSON dashboard with persistent Attention Counter badge, docked WebView/Monaco panel state, and 1-tap inline reply endpoint).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 `notification_dispatcher` formats a 120-char quote on `WAITING`, `FINISHED`, and `ERRORED` transitions and runs configured shell hook scripts with all 8 `AGMUX_*` env vars and a 15s timeout
- [x] #2 `agmux panels open <url|file>` registers and opens file/URL panels for the target session and `agmux://` deep-links resolve to target sessions/panels
- [x] #3 `agmux gui` provides the Phase 3 companion GUI state/dashboard with live Attention Counter badge, docked panels, and 1-tap inline reply support
- [x] #4 Level 3 LLDs (`notification_dispatcher_lld.md`, `companion_gui_lld.md`) and Level 5 SVT (`tests/e2e_notifications_and_companion_gui_test.rs`) pass
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Finalized the Phase 3 Rich Notifications and Companion GUI bridge:
- Resolved the command string vs executable tool profile normalization in `event_broadcaster.rs` ensuring `$AGMUX_TOOL` passes the clean tool profile (`sh`).
- Verified all quality gates: `cargo check --workspace`, `cargo test --workspace` (85 passed, 0 failed), `cargo clippy --workspace -- -D warnings`, `cargo fmt --check`, and `node .soul/tools/v-cycle-checker.mjs` (60/60 requirements, 100% Implemented & Verified).
- Visual proof artifact captured at `docs/assets/screenshots/task-32_verification.png`.
<!-- SECTION:NOTES:END -->
