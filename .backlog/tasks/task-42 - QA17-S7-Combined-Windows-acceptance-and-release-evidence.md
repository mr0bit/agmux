---
id: TASK-42
title: 'QA17 S7: Combined Windows acceptance and release evidence'
status: In Progress
assignee: []
created_date: '2026-11-17T23:16:00Z'
updated_date: '2026-11-17T23:52:00Z'
labels: []
dependencies:
  - TASK-36
  - TASK-37
  - TASK-38
  - TASK-39
  - TASK-40
  - TASK-41
references:
  - docs/specs/12_companion_acceptance_spec.md
  - https://github.com/mr0bit/agmux/issues/17
  - tests/reports/qa17_acceptance_report.md
modified_files:
  - docs/archi/module-list.md
  - crates/agmux-cli/src/companion_gui/ui/tests/fixtures/acp-provider.mjs
  - crates/agmux-cli/src/companion_gui/ui/scripts/live-test-server.mjs
  - crates/agmux-cli/src/companion_gui/ui/playwright.qa17.config.ts
  - crates/agmux-cli/src/companion_gui/ui/tests/qa17.spec.ts
  - crates/agmux-cli/src/companion_gui/ui/tests/live.spec.ts
  - .github/workflows/ci.yml
  - crates/agmux-proto/src/lib.rs
  - crates/agmux-daemon/src/server.rs
  - crates/agmux-cli/src/companion_gui/live.rs
  - crates/agmux-cli/docs/companion_gui_lld.md
  - tests/e2e_desktop_companion_gui_test.rs
  - tests/reports/qa17_acceptance_report.md
  - docs/specs/12_companion_acceptance_spec.md
  - docs/specs/README.md
  - docs/specs/TRACEABILITY_MATRIX.md
  - groma/systems/agmux/components/gemini-chat.md
  - groma/systems/agmux/containers/agmux-ui/components/geminichat.md
  - docs/assets/screenshots/qa17_auth_verification.png
  - docs/assets/screenshots/qa17_setup_verification.png
  - docs/assets/screenshots/qa17_chat_verification.png
  - docs/assets/screenshots/qa17_nongit_verification.png
  - docs/assets/screenshots/qa17_terminal_verification.png
  - docs/assets/screenshots/embedded_companion_verification.png
  - docs/assets/screenshots/gemini_provider_verification.png
priority: high
type: bug
ordinal: 42000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Approved slice S7 for GitHub issue #17 (QA-08).
What it delivers: Run the complete Windows regression flow, including startup and authenticated two-turn Gemini acceptance, before release sign-off.
Requirements: REQ-UX-WINDOWS-ACCEPTANCE-110, REQ-UX-STARTUP-DIAGNOSTICS-112.
Source: https://github.com/mr0bit/agmux/issues/17
Blocked by: TASK-36, TASK-37, TASK-38, TASK-39, TASK-40, TASK-41.
Use the five-level V-cycle: specification, registered architecture, LLD, unit tests, external verification and user-view proof.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Run the complete Windows regression flow, including startup and authenticated two-turn Gemini acceptance, before release sign-off.
- [x] #2 Requirement-to-LLD-to-test mapping and reproducible verification evidence are recorded.
- [x] #3 Native Windows acceptance is recorded separately from fixture/Linux results; incomplete user acceptance remains visible.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implementation and traceability recorded in tests/reports/qa17_acceptance_report.md. Linux/public-protocol evidence is separate from native Windows/authenticated user acceptance. Refs GitHub issue #17; branch fix/qa17-vcycle.
<!-- SECTION:NOTES:END -->
