---
id: TASK-5
title: 'Tracer Bullet: 4-Signal State Classifier & Agent STUCK Detection'
status: To Do
assignee: []
created_date: '2026-10-28T23:40:00Z'
labels:
  - tracer-bullet
  - classifier
  - agent
  - stuck
milestone: m-0
dependencies:
  - TASK-4
references:
  - docs/specs/04_state_machine_and_classifier_spec.md
  - docs/specs/05_cli_adapter_plugin_system_spec.md
priority: high
type: feature
ordinal: 5000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Daemon evaluates terminal byte stream, child exit codes, and timing signals through the 7-state classification engine (IDLE, RUNNING, WAITING_INPUT, COMPLETED, FAILED, STUCK, FROZEN). Pluggable STUCK detector identifies agent inactivity past threshold and emits state transition event.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 4-signal engine classifies process states with zero CPU polling
- [ ] #2 Inactive agent session transitions to STUCK when threshold elapsed
- [ ] #3 agmux list and JSON IPC payload reflect real-time session state
- [ ] #4 SVT test verifies state transitions across simulated terminal sessions
<!-- AC:END -->
