---
id: TASK-46
title: 'QA17 B4: Inactive sessions and archive'
status: Done
assignee: []
created_date: '2026-11-18T01:54:00Z'
updated_date: '2026-11-18T03:08:00Z'
labels: []
dependencies:
  - TASK-43
references:
  - https://github.com/mr0bit/agmux/issues/17
  - docs/specs/13_session_workflow_spec.md
documentation:
  - tests/reports/qa17_batch2_acceptance_report.md
modified_files:
  - crates/agmux-cli/src/companion_gui/ui/src/LiveApp.tsx
  - crates/agmux-core/src/storage_engine.rs
  - crates/agmux-core/docs/storage_engine_lld.md
  - crates/agmux-cli/docs/companion_gui_lld.md
  - docs/assets/screenshots/qa17_b2_inactive_verification.png
priority: high
type: bug
ordinal: 46000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Approved 2026-11-18 batch 2, GitHub issue #17 (QA-12).
Delivery: Ended sessions retain history under Inactive and support explicit persistent archiving.
Requirement numbers: 118, 124 in docs/specs/13_session_workflow_spec.md.
Blocked by: TASK-43.
Use one live provider conversation with two presentations; no second independent Gemini context.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Ended sessions retain history under Inactive and support explicit persistent archiving.
- [x] #2 Requirements, LLD, public-seam regression tests and user-view screenshots are linked.
- [x] #3 Native/authenticated acceptance is distinguished from external provider fixtures; unfinished acceptance stays visible.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implemented and public-seam regressions passed. See tests/reports/qa17_batch2_acceptance_report.md, docs/specs/13_session_workflow_spec.md and the linked screenshot. External ACP fixture results are distinct from signed-in Gemini/native user acceptance, retained under TASK-50 and issue #17 / draft PR #18. Rust 126, workflow browser 13, official CLI/application 6, UI state/design/motion 11 passed.
<!-- SECTION:NOTES:END -->
