---
id: TASK-50
title: 'QA17 B8: Windows batch 2 acceptance and evidence'
status: In Progress
assignee: []
created_date: '2026-11-18T01:54:00Z'
updated_date: '2026-11-18T03:27:00Z'
labels: []
dependencies:
  - TASK-45
  - TASK-49
references:
  - https://github.com/mr0bit/agmux/issues/17
  - docs/specs/13_session_workflow_spec.md
documentation:
  - tests/reports/qa17_batch2_acceptance_report.md
modified_files:
  - docs/specs/13_session_workflow_spec.md
  - docs/specs/README.md
  - crates/agmux-cli/src/companion_gui/ui/tests/fixtures/acp-provider.mjs
  - crates/agmux-cli/src/companion_gui/ui/scripts/live-test-server.mjs
  - crates/agmux-cli/src/companion_gui/ui/tests/workflow.spec.ts
  - crates/agmux-cli/src/companion_gui/ui/playwright.qa17.config.ts
  - crates/agmux-cli/src/companion_gui/ui/tests/qa17.spec.ts
  - crates/agmux-cli/src/companion_gui/ui/tests/live.spec.ts
  - tests/reports/qa17_batch2_acceptance_report.md
  - tests/reports/qa17_acceptance_report.md
  - docs/assets/screenshots/gemini_provider_verification.png
  - docs/assets/screenshots/qa17_auth_verification.png
  - docs/assets/screenshots/qa17_chat_verification.png
  - docs/assets/screenshots/qa17_nongit_verification.png
  - docs/assets/screenshots/qa17_setup_verification.png
  - docs/assets/screenshots/qa17_terminal_verification.png
  - docs/specs/TRACEABILITY_MATRIX.md
  - docs/assets/screenshots/embedded_companion_verification.png
  - crates/agmux-proto/docs/transport_lld.md
  - docs/archi/module-list.md
  - crates/agmux-proto/src/ipc.rs
  - .github/workflows/ci.yml
priority: medium
type: enhancement
ordinal: 50000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Approved 2026-11-18 batch 2, GitHub issue #17 (QA-09–QA-16).
Delivery: Packaged native Windows regressions and explicit shared-context, file approval and user-view evidence qualify the new behavior.
Requirement numbers: 123 in docs/specs/13_session_workflow_spec.md.
Blocked by: TASK-45, TASK-49.
Use one live provider conversation with two presentations; no second independent Gemini context.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Packaged native Windows regressions and explicit shared-context, file approval and user-view evidence qualify the new behavior.
- [x] #2 Requirements, LLD, public-seam regression tests and user-view screenshots are linked.
- [x] #3 Native/authenticated acceptance is distinguished from external provider fixtures; unfinished acceptance stays visible.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
All local gates pass and source-backed screenshots/traceability are committed with the implementation. Native Windows package CI and signed-in user acceptance remain explicit. AC1 stays open; do not infer account-backed Gemini or DPI acceptance from deterministic protocol fixture screenshots. PR #18 stays draft; release v0.1.0 unchanged. Native runtime SHA/archive/checksum results are recorded in PR #18 and issue #17.

Windows CI run 97 exposed ERROR_PIPE_BUSY (231) during concurrent GUI IPC connections. Add REQ-IPC-CONNECT-CONTENTION-125 under the existing transport owner, bounded pre-request retry and native pipe recovery/deadline regression. Qualification remains open until the corrected Windows build passes.
<!-- SECTION:NOTES:END -->
