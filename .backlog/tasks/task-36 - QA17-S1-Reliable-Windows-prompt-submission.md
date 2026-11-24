---
id: TASK-36
title: 'QA17 S1: Reliable Windows prompt submission'
status: In Progress
assignee: []
created_date: '2026-11-17T23:16:00Z'
updated_date: '2026-11-17T23:52:00Z'
labels: []
dependencies: []
references:
  - docs/specs/12_companion_acceptance_spec.md
  - https://github.com/mr0bit/agmux/issues/17
  - tests/reports/qa17_acceptance_report.md
modified_files:
  - crates/agmux-core/src/pty_manager.rs
  - crates/agmux-daemon/src/server.rs
  - crates/agmux-core/src/snapshot_compressor.rs
  - crates/agmux-core/docs/pty_manager_lld.md
priority: high
type: bug
ordinal: 36000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Approved slice S1 for GitHub issue #17 (QA-07).
What it delivers: A Chat send submits one prompt once, including the provider submit key; two prompts remain distinct turns.
Requirements: REQ-UX-PROMPT-SUBMIT-101.
Source: https://github.com/mr0bit/agmux/issues/17
Blocked by: None (can start immediately).
Use the five-level V-cycle: specification, registered architecture, LLD, unit tests, external verification and user-view proof.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 A Chat send submits one prompt once, including the provider submit key; two prompts remain distinct turns.
- [x] #2 Requirement-to-LLD-to-test mapping and reproducible verification evidence are recorded.
- [x] #3 Native Windows acceptance is recorded separately from fixture/Linux results; incomplete user acceptance remains visible.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implementation and traceability recorded in tests/reports/qa17_acceptance_report.md. Linux/public-protocol evidence is separate from native Windows/authenticated user acceptance. Refs GitHub issue #17; branch fix/qa17-vcycle.
<!-- SECTION:NOTES:END -->
