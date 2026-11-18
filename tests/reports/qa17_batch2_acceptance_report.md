# Issue #17 batch 2 — shared session workflow

Date: 2026-11-18 UTC. Tracker: https://github.com/mr0bit/agmux/issues/17.
Draft fix PR: https://github.com/mr0bit/agmux/pull/18.
Tested runtime source: `352ec6b03c8f84f450554fbe1087ddb312d796e5`, branch `fix/qa17-vcycle`.
[CI run 99](https://github.com/mr0bit/agmux/actions/runs/44230823784) records native verification; later report-only commits do not change the executable source.
This is a PR acceptance build; v0.1.0 has not been changed.

## Architecture and scope

The approved B1–B8 plan supersedes batch 1's independent Chat/Terminal contexts.
Managed Gemini sessions now own one ACP process and conversation. Chat and the
terminal-style view show the same ordered messages, draft, running turn and
permissions. The terminal-style view is an agent conversation presentation;
it is not Gemini's separate native TUI or an operating-system shell. Custom
executables and other providers retain native PTY capabilities.

Legacy Reply and delegated SendMessage requests enter that same actor; busy
managed conversations reject concurrent turns instead of creating another
provider context. No unused native Gemini PTY is started. Credential handling
remains in the installed Gemini CLI. The capability handshake rejects older
daemons before sending the expanded workflow protocol.

Conversation snapshots are retained in SQLite at accepted inputs and completed
turns, and when ended. Ended/archived records are read-only, including after a
daemon restart. Restart does not auto-replay prompts or transparently start a
new conversation. Archive/restore changes visibility without deleting history.

## Traceability and slice results

All requirements are in `docs/specs/13_session_workflow_spec.md`; each requirement
has a corresponding `LLD-UX-*` contract in its owning module's LLD and a
`TEST-VER-UX-<number>` verification case. Tasks were created using Backlog CLI
only after user approval; dependency edges remain explicit.

| Slice / Backlog | Report | Requirement numbers | Public seam and user-visible result | Evidence |
| --- | --- | --- | --- | --- |
| B1 / TASK-43 | QA-09 | 113, 114 | Browser → authenticated bridge → real daemon → external ACP process; one PID/session and ordered fact/recall/history in both directions, shared draft; legacy prompt routes use the same context | `qa17_b2_shared_verification.png` |
| B2 / TASK-44 | QA-10 | 115 | Real Git workspace updates without Refresh, including new/untracked files; unborn HEAD supported; binary/large text previews bounded | `qa17_b2_live_changes_verification.png` |
| B3 / TASK-45 | QA-11 | 116, 117 | Provider-supplied old/new text shown before writes; Reject leaves disk unchanged; Allow produces actual on-disk Applied diff; both columns fit the review panel | `qa17_b2_proposed_verification.png` |
| B4 / TASK-46 | QA-12 | 118, 124 | End → Inactive → Archive/restore; actual daemon restart retains transcript; ended Send rejects rather than creating a process | `qa17_b2_inactive_verification.png` |
| B5 / TASK-47 | QA-13, QA-14 | 119, 120 | Recent real activity and canonical project groups with same-basename disambiguation; persisted preference; 34px one-line buttons, virtual scrolling and long-label/keyboard checks | `qa17_b2_activity_verification.png`, `qa17_b2_projects_verification.png` |
| B6 / TASK-48 | QA-15 | 121 | No Shell choice/default/fallback; unavailable-agent case provides setup; native backend capability retained | Agent selection browser regression |
| B7 / TASK-49 | QA-16 | 122 | Command palette, quick session/path search, shortcut help, Escape/focus, IME guard and shared Terminal shortcut | `qa17_b2_keyboard_verification.png` |
| B8 / TASK-50 | All | 123 | Packaged Windows/Linux regression, exact-source archive and explicit automated/account/user evidence boundaries | Native CI and acceptance matrix below |

The LLD owners are `crates/agmux-daemon/docs/gemini_chat_lld.md`,
`crates/agmux-cli/docs/companion_gui_lld.md`, and the core
`diff_calculator_lld.md` / `storage_engine_lld.md` documents.

## Verification record

| Check | Result / boundary |
| --- | --- |
| Rust workspace | 126 passed; includes real Git/unborn/untracked and retained SQLite reopen tests |
| Production UI + embedded binaries | Build passed; UI assets built before compiling the executables |
| UI state/design/motion | 11 passed |
| Browser + real daemon + external ACP provider | 13 passed on the final embedded executables, including restart, approval/rejection and compact-list stress |
| Official Gemini CLI 0.68.0 | 6 passed; official version launch, isolated missing-auth and real native PTY/files/security regressions. No account-backed model turn claimed |
| Formatting and all-target Clippy | Passed; Cargo reports the existing unused `futures-util` manifest dependency warning |
| Groma readiness and scan | Passed; new CommandPalette owner and callback evidence recorded. Initial scanner map remains uncurated; documented callback/duplicate compilation-context gaps remain |
| Strict V-cycle schema and Backlog linkage | 88/88 requirement records structurally valid; final verification matrix generated alongside the results |
| Native Windows pipe contention | 2 native public-pipe recovery/deadline regressions passed in CI run 99; TEST-VER-IPC-125 verified |
| Native Windows packaged suite | See CI run 99 / PR #18 for final package results and archive checksum. TEST-VER-UX-123 remains pending user qualification |
| Signed-in Gemini shared context/edit approval | **Not executed here**; user-machine acceptance remains open |
| Native Windows DPI 100/125/150% | **Not executed here**; inherited batch-1 acceptance remains open |

The ACP fixture is an external executable used only by the test harness.
Production code contains no fixture responses or browser test write shortcuts.
Screenshots capture the actual packaged app and Rust daemon; deterministic
fixture text demonstrates rendering/protocol behavior, not Gemini's model output.
The daemon restart trigger exists only in the test launcher filesystem, outside
the production application and HTTP interface.

## User acceptance steps

1. Extract the exact acceptance archive and keep `agmux.exe` and `agmuxd.exe`
   together. Save current work and stop any old daemon before launching the new
   pair. Sign into the installed Gemini CLI on the Windows machine.
2. Start a Gemini project session. Tell it a unique fact in Chat. Switch to
   Terminal, ask it to recall the fact and continue the task; switch back and
   check the same complete history. Switch while working and while awaiting
   permission; ensure there is one running turn and shared approval state.
3. Ask for a tracked-file edit and a new file in a real Git repository. Confirm
   automatic Applied updates. For provider-supplied proposed edits, inspect both
   Before and After before allowing. Reject one and verify disk is unchanged;
   allow another and verify its real Applied diff.
4. End the session. Confirm Inactive and read-only history, restart the daemon,
   archive it, reload and restore it. History must remain readable without
   starting Gemini again. Ending an active conversation ends its live provider.
5. Use Activity/Projects with two same-named project folders, compact rows,
   long names and a large list. Try Ctrl+Shift+P, Ctrl+P, Ctrl+B,
   Ctrl+backquote and Ctrl+K then Ctrl+S; Escape must restore focus and preserve
   the prompt draft. Confirm Shell is absent in new-session setup.
6. Record Windows build, Gemini version/auth mode, archive checksum, display
   scaling, observed result and screenshots against QA-09–QA-16 in issue #17.
   Keep user acceptance boxes open until the signed-in workflow is confirmed.

## Evidence and remaining input

Screenshots are committed under `docs/assets/screenshots/` and linked from the
PR and single tracker. The two new handwritten photos are visible in the chat
but their original image files are unavailable in this workspace; the report
transcription is tracked in issue #17. The original five uploaded batch-1 photos
remain linked there. No replacement image is represented as a user original.

## Prevention and V-cycle delta

The new specification adds explicit shared-context/ownership, view-state,
live untracked diffs, proposed-versus-applied approval, retained history,
lifecycle, navigation/compact sizing, agent-only setup, keyboard focus and
packaged evidence contracts. CI's Windows and Linux release-binary jobs run
both QA suites and upload real screenshots, preventing a mere successful
launch or terminal screenshot from being treated as conversation acceptance.

Expanded baseline: 70/88 verified, 75/88 implemented before batch 2 and the native IPC finding.
The 13 batch-2/qualification contracts bring implementation to 88/88. Twelve are verified (eleven workflow contracts plus native IPC recovery), giving 82/88 (93.2%).
Native/account/user acceptance remains explicit; requirement 123 remains Implemented/Failed (pending). Five earlier native/account/DPI contracts also remain open.
See `docs/specs/TRACEABILITY_MATRIX.md` for the generated exact totals.

## Native Windows qualification finding — TEST-VER-IPC-125

Run 97 passed all seven new workflow cases on Windows but failed an earlier
non-Git check because the shared IPC client surfaced Windows ERROR_PIPE_BUSY
(231) during concurrent connections. This was a transport race, not a Git error.
Requirement REQ-IPC-CONNECT-CONTENTION-125 now specifies bounded connection
recovery before any request is transmitted. Native public named-pipe tests are
added to the Windows packaging job. The corrected source and final CI evidence
are recorded in PR #18 / issue #17; no failed Windows archive was delivered.
Both native Windows public-pipe recovery/deadline regressions passed in run 99, verifying requirement 125. The full release-binary UI suite and archive results are recorded in that run and PR #18. The expanded scope is 88 requirements. Account/user acceptance remains open.
