---
id: TASK-50
title: 'QA17 B8: Windows batch 2 acceptance and evidence'
status: In Progress
assignee: []
created_date: '2026-11-18T01:54:00Z'
updated_date: '2026-11-22T22:11:00Z'
labels: []
dependencies:
  - TASK-45
  - TASK-49
references:
  - https://github.com/mr0bit/agmux/issues/17
  - docs/specs/13_session_workflow_spec.md
  - crates/agmux-core/src/pty_manager.rs
  - .cargo/config.toml
documentation:
  - tests/reports/qa17_batch2_acceptance_report.md
  - tests/reports/qa17_native_terminal_report.md
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
  - crates/agmux-core/src/pty_manager.rs
  - crates/agmux-core/docs/pty_manager_lld.md
  - tests/reports/qa17_native_terminal_report.md
  - .cargo/config.toml
  - crates/agmux-cli/docs/companion_gui_lld.md
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

REQ-IPC-CONNECT-CONTENTION-125 verified: both native Windows public named-pipe recovery/deadline tests passed in CI run 99 (Windows job 134556326194), runtime source 352ec6b03c8f84f450554fbe1087ddb312d796e5. Full package/browser results and exact-source artifact remain in PR #18 / issue #17. Signed-in user acceptance remains open; AC1 not closed.

User native-Terminal correction: old terminal-style build is superseded. TASK-43 now implements real native PTY + ACP load/resume synchronization, with 18 workflow cases and 7 official CLI/application checks locally. New Windows native hook/PTY package qualification remains visible in PR #18 and issue #17; user account/DPI acceptance stays open.

Native correction CI run 105: quality and Linux 18/18 passed; Windows 13/18 passed. Five native handoff/exit checks expose ConPTY exit notification waiting for output EOF while the viewport still owns the master. Fix process-exit publication independently of ConPTY output EOF; retain exact-session and no-second-writer guards. No failed Windows archive delivered.

QA-18: user Windows screenshot reports VCRUNTIME140.dll missing before agmux can start. Previous hosted CI had the redistributable installed, masking the loader dependency. Fix MSVC CRT linkage for both shipped executables and gate packaging on their PE import tables; retain standalone application and native browser checks. User clean-machine acceptance remains open.

REQ-PKG-WINDOWS-CRT-126: .cargo/config.toml sets +crt-static for x86_64-pc-windows-msvc; CI TEST-VER-PKG-126 inspects both real release PE dependency tables, rejecting external Visual C++ runtime DLLs before package upload. Windows native verification pending.

QA-18 / REQ-PKG-WINDOWS-CRT-126 verified in CI109, runtime 599e04750d6209971bebbe10463602351a3b1ac2. Both PE import audits, standalone Windows startup, native pipe/PTY regressions, all 18 Windows and 18 Linux browser cases plus 126 Rust tests passed. Downloaded archive checksums and only Windows system DLL imports independently verified. User clean-machine and signed-in/DPI acceptance remain open; AC1 unchanged.
<!-- SECTION:NOTES:END -->
