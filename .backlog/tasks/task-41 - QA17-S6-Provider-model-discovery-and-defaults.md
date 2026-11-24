---
id: TASK-41
title: 'QA17 S6: Provider model discovery and defaults'
status: In Progress
assignee: []
created_date: '2026-11-17T23:16:00Z'
updated_date: '2026-11-17T23:52:00Z'
labels: []
dependencies:
  - TASK-39
  - TASK-40
references:
  - docs/specs/12_companion_acceptance_spec.md
  - https://github.com/mr0bit/agmux/issues/17
  - tests/reports/qa17_acceptance_report.md
modified_files:
  - crates/agmux-cli/src/companion_gui/ui/src/backend.ts
  - crates/agmux-cli/src/companion_gui/ui/src/LiveApp.tsx
priority: medium
type: enhancement
ordinal: 41000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Approved slice S6 for GitHub issue #17 (QA-01).
What it delivers: The session picker discovers provider-advertised models and defaults with an honest fallback.
Requirements: REQ-UX-PROVIDER-MODELS-108, REQ-UX-MODEL-DEFAULTS-109.
Source: https://github.com/mr0bit/agmux/issues/17
Blocked by: TASK-39, TASK-40.
Use the five-level V-cycle: specification, registered architecture, LLD, unit tests, external verification and user-view proof.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 The session picker discovers provider-advertised models and defaults with an honest fallback.
- [x] #2 Requirement-to-LLD-to-test mapping and reproducible verification evidence are recorded.
- [x] #3 Native Windows acceptance is recorded separately from fixture/Linux results; incomplete user acceptance remains visible.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implementation and traceability recorded in tests/reports/qa17_acceptance_report.md. Linux/public-protocol evidence is separate from native Windows/authenticated user acceptance. Refs GitHub issue #17; branch fix/qa17-vcycle.
<!-- SECTION:NOTES:END -->
