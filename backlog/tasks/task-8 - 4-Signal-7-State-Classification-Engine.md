---
id: TASK-8
title: 4-Signal 7-State Classification Engine
status: To Do
assignee: []
created_date: '2026-10-28T22:38:00Z'
labels:
  - state
  - classifier
  - engine
milestone: m-0
dependencies:
  - TASK-4
references:
  - docs/specs/04_state_machine_and_classifier_spec.md
priority: high
type: feature
ordinal: 8000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Implement event-driven state classifier supporting 7 states (WORKING, WAITING, STUCK, FINISHED, IDLE, ERRORED, DEAD) via hooks, sysinfo, and SIMD tail matcher.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 7-state typed state machine transitions
- [ ] #2 sysinfo child PID tree compilation detection
- [ ] #3 SIMD aho-corasick Braille, rate-limit, and prompt matching
- [ ] #4 Debounce settle timers (--idle-after-ms, --blocked-settle-ms)
<!-- AC:END -->
