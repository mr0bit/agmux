---
id: TASK-37
title: 'QA17 S2: Non-Git workspaces and bounded errors'
status: Done
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
  - crates/agmux-core/src/diff_calculator.rs
  - crates/agmux-cli/src/companion_gui/live.rs
  - crates/agmux-cli/src/companion_gui/ui/src/LiveApp.tsx
  - crates/agmux-cli/src/companion_gui/ui/src/live.css
  - crates/agmux-core/tests/diff_calculator_test.rs
  - crates/agmux-core/docs/diff_calculator_lld.md
priority: high
type: bug
ordinal: 37000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Approved slice S2 for GitHub issue #17 (QA-03).
What it delivers: Plain folders remain usable; Git review explains unavailable diffs without flooding or blocking Chat.
Requirements: REQ-UX-NONGIT-DIFF-102, REQ-UX-BOUNDED-ERROR-103.
Source: https://github.com/mr0bit/agmux/issues/17
Blocked by: None (can start immediately).
Use the five-level V-cycle: specification, registered architecture, LLD, unit tests, external verification and user-view proof.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Plain folders remain usable; Git review explains unavailable diffs without flooding or blocking Chat.
- [x] #2 Requirement-to-LLD-to-test mapping and reproducible verification evidence are recorded.
- [x] #3 Native Windows acceptance is recorded separately from fixture/Linux results; incomplete user acceptance remains visible.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implementation and traceability recorded in tests/reports/qa17_acceptance_report.md. Linux/public-protocol evidence is separate from native Windows/authenticated user acceptance. Refs GitHub issue #17; branch fix/qa17-vcycle.
<!-- SECTION:NOTES:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Public browser/daemon verification passed; design, requirements and screenshots recorded. Windows/user acceptance stays open under TASK-42 and GitHub #17.
<!-- SECTION:FINAL_SUMMARY:END -->
