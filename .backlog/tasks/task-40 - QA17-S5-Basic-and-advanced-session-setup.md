---
id: TASK-40
title: 'QA17 S5: Basic and advanced session setup'
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
  - crates/agmux-cli/src/companion_gui/ui/src/LiveApp.tsx
  - crates/agmux-cli/docs/companion_gui_lld.md
priority: medium
type: enhancement
ordinal: 40000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Approved slice S5 for GitHub issue #17 (QA-02).
What it delivers: Basic launch works without JSON; technical CLI options live in an accessible collapsed section.
Requirements: REQ-UX-ADVANCED-SETUP-107.
Source: https://github.com/mr0bit/agmux/issues/17
Blocked by: None (can start immediately).
Use the five-level V-cycle: specification, registered architecture, LLD, unit tests, external verification and user-view proof.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Basic launch works without JSON; technical CLI options live in an accessible collapsed section.
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
