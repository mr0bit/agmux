---
id: TASK-38
title: 'QA17 S3: Terminal cell geometry and resize fidelity'
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
  - crates/agmux-cli/src/companion_gui/ui/src/LiveTerminal.tsx
  - crates/agmux-core/src/snapshot_compressor.rs
  - crates/agmux-core/docs/snapshot_compressor_lld.md
priority: high
type: bug
ordinal: 38000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Approved slice S3 for GitHub issue #17 (QA-04).
What it delivers: Native terminal glyphs, borders and cursor stay aligned across resizing and display scaling.
Requirements: REQ-UX-TERMINAL-CELLS-104.
Source: https://github.com/mr0bit/agmux/issues/17
Blocked by: None (can start immediately).
Use the five-level V-cycle: specification, registered architecture, LLD, unit tests, external verification and user-view proof.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Native terminal glyphs, borders and cursor stay aligned across resizing and display scaling.
- [x] #2 Requirement-to-LLD-to-test mapping and reproducible verification evidence are recorded.
- [x] #3 Native Windows acceptance is recorded separately from fixture/Linux results; incomplete user acceptance remains visible.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implementation and traceability recorded in tests/reports/qa17_acceptance_report.md. Linux/public-protocol evidence is separate from native Windows/authenticated user acceptance. Refs GitHub issue #17; branch fix/qa17-vcycle.
<!-- SECTION:NOTES:END -->
