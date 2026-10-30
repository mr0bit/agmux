---
id: TASK-25
title: 'Tracer Bullet: Multi-Tier State Classifier (Hooks, Child PIDs, Tail Matcher, Debounce & Shell Tracking)'
status: Done
assignee:
  - '@codex'
  - '@agy'
created_date: '2026-10-30T00:00:00Z'
labels:
  - tracer-bullet
  - state
  - classifier
  - hooks
  - debounce
milestone: m-2
dependencies:
  - TASK-24
references:
  - docs/specs/04_state_machine_and_classifier_spec.md
  - REQ-STATE-TIER1-HOOKS-002
  - REQ-STATE-TIER2-CHILDPID-003
  - REQ-STATE-TIER3-TAILMATCH-004
  - REQ-STATE-DEBOUNCE-TIMERS-005
  - REQ-STATE-SHELL-TRACKING-006
priority: high
type: feature
ordinal: 25000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Implements `REQ-STATE-TIER1-HOOKS-002`, `REQ-STATE-TIER2-CHILDPID-003`, `REQ-STATE-TIER3-TAILMATCH-004`, `REQ-STATE-DEBOUNCE-TIMERS-005`, and `REQ-STATE-SHELL-TRACKING-006`: Tier-1 IPC lifecycle hooks (`agmux internal-hook --session <id> --event <start|stop|wait|error>`), Tier-2 OS child PID tree inspection to lock state to `WORKING` during silent builds, Tier-3 tail matcher scanning bottom 5 visible rows for Braille spinners (`WORKING`), rate limits (`ERRORED`), and prompts (`WAITING`), configurable debounce timers (`idle_after_ms` and `blocked_settle_ms`), and dynamic shell sub-tab tracking.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 CLI `agmux internal-hook` immediately transitions session state over IPC
- [x] #2 Child process tree monitor detects active child processes during silent builds
- [x] #3 Tail matcher inspects bottom 5 rows for Braille spinners, rate-limit banners, and confirmation prompts
- [x] #4 Debounce timer enforces idle_after_ms and blocked_settle_ms windows without state flutter
- [x] #5 Shell classifier dynamically tracks plain shell sub-tab WORKING vs IDLE state
<!-- AC:END -->
