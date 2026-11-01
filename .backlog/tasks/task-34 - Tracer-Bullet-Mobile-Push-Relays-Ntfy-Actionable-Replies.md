---
id: TASK-34
title: >-
  Tracer Bullet: Mobile Push Relays (ntfy.sh & Webhooks with Actionable Inline Replies)
status: Done
assignee:
  - '@agy'
created_date: '2026-11-01T06:15:00Z'
updated_date: '2026-11-01T06:48:00Z'
labels:
  - tracer-bullet
  - notifications
  - push
  - phase-3
milestone: m-3
dependencies:
  - TASK-33
references:
  - docs/specs/11_desktop_gui_and_notifications_spec.md
priority: medium
type: feature
ordinal: 34000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Implements External Mobile Push Relays and Actionable Push Replies (`FR-P3-03`):
1. `REQ-GUI-MOBILE-PUSH-004` (`crates/agmux-daemon::notification_dispatcher`):
   - Configurable push relays in `config.toml` (`[notifications.push]`, `provider = "ntfy"|"webhook"`, `url`, `topic`, `token`).
   - Dispatches rich push notifications on `WAITING` and `ERRORED` agent transitions.
   - Attaches actionable inline reply buttons allowing 1-tap mobile input responses routed back to the session via the companion bridge endpoint.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 `notification_dispatcher` reads push relay configuration from `config.toml`
- [x] #2 Transitions to `WAITING` and `ERRORED` dispatch formatted HTTP requests to configured push endpoints (with 120-char quote and action buttons)
- [x] #3 Integration test (`tests/e2e_mobile_push_relay_test.rs`) verifies webhook / ntfy payload generation and action routing
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Dispatched to Google Antigravity (Lane: feature2, gemini-3.8-flash-medium).
<!-- SECTION:NOTES:END -->
