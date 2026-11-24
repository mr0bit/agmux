---
id: TASK-39
title: 'QA17 S4: Gemini conversation and turn lifecycle'
status: In Progress
assignee: []
created_date: '2026-11-17T23:16:00Z'
updated_date: '2026-11-17T23:54:00Z'
labels: []
dependencies:
  - TASK-36
references:
  - docs/specs/12_companion_acceptance_spec.md
  - https://github.com/mr0bit/agmux/issues/17
  - tests/reports/qa17_acceptance_report.md
modified_files:
  - crates/agmux-daemon/src/gemini_chat.rs
  - crates/agmux-daemon/src/lib.rs
  - crates/agmux-proto/src/messages.rs
  - crates/agmux-daemon/src/server.rs
  - crates/agmux-cli/src/companion_gui/live.rs
  - crates/agmux-cli/src/companion_gui/ui/src/GeminiChat.tsx
  - crates/agmux-cli/src/companion_gui/ui/src/LiveApp.tsx
  - crates/agmux-cli/src/companion_gui/ui/src/live.css
  - crates/agmux-daemon/src/shutdown_coordinator.rs
  - crates/agmux-cli/src/companion_gui/ui/src/backend.ts
  - crates/agmux-daemon/docs/gemini_chat_lld.md
  - crates/agmux-daemon/Cargo.toml
  - Cargo.lock
priority: high
type: bug
ordinal: 39000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Approved slice S4 for GitHub issue #17 (QA-05, QA-06).
What it delivers: Chat uses structured provider messages and explicit lifecycle/errors; Terminal stays a separate native view.
Requirements: REQ-UX-CHAT-MESSAGES-105, REQ-UX-TURN-LIFECYCLE-106, REQ-UX-CHAT-ISOLATION-111.
Source: https://github.com/mr0bit/agmux/issues/17
Blocked by: TASK-36.
Use the five-level V-cycle: specification, registered architecture, LLD, unit tests, external verification and user-view proof.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Chat uses structured provider messages and explicit lifecycle/errors; Terminal stays a separate native view.
- [x] #2 Requirement-to-LLD-to-test mapping and reproducible verification evidence are recorded.
- [x] #3 Native Windows acceptance is recorded separately from fixture/Linux results; incomplete user acceptance remains visible.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implementation and traceability recorded in tests/reports/qa17_acceptance_report.md. Linux/public-protocol evidence is separate from native Windows/authenticated user acceptance. Refs GitHub issue #17; branch fix/qa17-vcycle.
<!-- SECTION:NOTES:END -->
