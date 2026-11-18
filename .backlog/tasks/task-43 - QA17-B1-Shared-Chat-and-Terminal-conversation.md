---
id: TASK-43
title: 'QA17 B1: Shared Chat and Terminal conversation'
status: Done
assignee: []
created_date: '2026-11-18T01:54:00Z'
updated_date: '2026-11-18T20:29:00Z'
labels: []
dependencies: []
references:
  - https://github.com/mr0bit/agmux/issues/17
  - docs/specs/13_session_workflow_spec.md
documentation:
  - tests/reports/qa17_batch2_acceptance_report.md
  - tests/reports/qa17_native_terminal_report.md
modified_files:
  - crates/agmux-daemon/docs/gemini_chat_lld.md
  - docs/archi/module-list.md
  - crates/agmux-core/src/storage_engine.rs
  - crates/agmux-proto/src/messages.rs
  - crates/agmux-proto/src/lib.rs
  - crates/agmux-daemon/src/gemini_chat.rs
  - crates/agmux-daemon/src/server.rs
  - crates/agmux-cli/src/companion_gui/live.rs
  - crates/agmux-cli/src/companion_gui/ui/src/backend.ts
  - crates/agmux-cli/src/companion_gui/ui/src/GeminiChat.tsx
  - crates/agmux-cli/src/companion_gui/ui/src/LiveApp.tsx
  - crates/agmux-cli/src/companion_gui/ui/src/live.css
  - groma/systems/agmux/containers/agmux-ui/components/geminichat.md
  - docs/assets/screenshots/qa17_b2_shared_verification.png
  - crates/agmux-daemon/src/main.rs
  - crates/agmux-cli/src/companion_gui/ui/tests/fixtures/acp-provider.mjs
  - crates/agmux-cli/src/companion_gui/ui/tests/workflow.spec.ts
  - docs/specs/13_session_workflow_spec.md
  - crates/agmux-cli/docs/companion_gui_lld.md
  - crates/agmux-cli/src/companion_gui/ui/tests/live.spec.ts
  - crates/agmux-daemon/Cargo.toml
  - crates/agmux-cli/src/companion_gui/ui/scripts/live-test-server.mjs
  - Cargo.lock
  - tests/reports/qa17_native_terminal_report.md
  - tests/reports/qa17_batch2_acceptance_report.md
  - docs/assets/screenshots/qa17_native_terminal_verification.png
  - docs/assets/screenshots/qa17_native_chat_sync_verification.png
  - docs/assets/screenshots/qa17_official_native_terminal_verification.png
  - docs/assets/screenshots/embedded_companion_verification.png
  - docs/assets/screenshots/gemini_provider_verification.png
  - docs/assets/screenshots/qa17_auth_verification.png
  - docs/assets/screenshots/qa17_b2_activity_verification.png
  - docs/assets/screenshots/qa17_b2_inactive_verification.png
  - docs/assets/screenshots/qa17_b2_keyboard_verification.png
  - docs/assets/screenshots/qa17_b2_live_changes_verification.png
  - docs/assets/screenshots/qa17_b2_projects_verification.png
  - docs/assets/screenshots/qa17_b2_proposed_verification.png
  - docs/assets/screenshots/qa17_chat_verification.png
  - docs/assets/screenshots/qa17_nongit_verification.png
  - docs/assets/screenshots/qa17_setup_verification.png
  - docs/assets/screenshots/qa17_terminal_verification.png
  - docs/specs/TRACEABILITY_MATRIX.md
priority: high
type: bug
ordinal: 43000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
User correction approved 2026-11-18, issue #17 / QA-09. Deliver proper conversational Chat and actual native Gemini CLI/TUI synchronized through one persisted provider UUID and exclusive writer handoff. Keep running turns, native drafts and approvals with their current owner. Requirements 113 and 114 in docs/specs/13_session_workflow_spec.md. Blocked by: None. No independent context or terminal-styled Chat.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Proper Chat and real native CLI/TUI synchronize the same provider UUID/context with exclusive writer ownership, guarded turns/approvals and no fresh-session fallback.
- [x] #2 Requirements, LLD, public-seam regressions and actual user-view screenshots are linked.
- [x] #3 Native protocol/PTY and official CLI startup evidence are distinguished from signed-in Windows model/user acceptance.
<!-- AC:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
Implemented and public-seam regressions passed. See tests/reports/qa17_batch2_acceptance_report.md, docs/specs/13_session_workflow_spec.md and the linked screenshot. External ACP fixture results are distinct from signed-in Gemini/native user acceptance, retained under TASK-50 and issue #17 / draft PR #18. Rust 126, workflow browser 13, official CLI/application 6, UI state/design/motion 11 passed.

User correction 2026-11-18: preserve proper Chat and actual native Gemini CLI/TUI. Reopen shared-session slice; replace terminal-styled Chat with single-writer persisted-session handoff (ACP loadSession / native --resume), live native transcript synchronization and turn/approval handoff guards. No second independent conversation.

Native correction public-seam verification passed: 18 full workflow cases, six final focused handoff/PID/tool-activity cases, seven official CLI/application cases (actual Gemini 0.68 native authentication TUI), 126 Rust tests and 11 state/design/motion cases. Updated 113/114 contracts are verified; signed-in Windows model/DPI acceptance remains under TASK-50. See tests/reports/qa17_native_terminal_report.md.

Native TUI follow-up: real cursor/device reports were being classified as user input, incorrectly preventing an idle handoff. Reproduced through actual PTY → xterm → WebSocket → native input with a provider CSI 6n query. Add a public round-trip regression and exclude only recognized automatic reports from the draft guard; raw bytes still pass unchanged.

Follow-up native protocol regression fixed: a real provider CSI 6n request receives a real xterm reply before idle is asserted. The earlier false Working state reproduced; all six final handoff/PID/tool/approval/exit/rapid-switch cases pass with the fix. Terminal replies pass unchanged, while real input invalidates readiness; ownership readiness is rechecked atomically before reaping.
<!-- SECTION:NOTES:END -->
