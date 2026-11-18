# Issue #17 — implementation and acceptance record

> Historical batch-1 record. The approved batch-2 shared-context design supersedes
> the independent Terminal/context behavior described below. See
> [the current batch-2 report](qa17_batch2_acceptance_report.md) for runtime behavior
> and current verification evidence.

Date: 2026-11-18 (Africa/Cairo). Source: this PR, based on main
`93f6790cd22e122c245146f309dec40ee548afff`. Local results refer to the PR working
tree before commit; CI records its exact head SHA. No new release has been made.
Tracker: https://github.com/mr0bit/agmux/issues/17.

Overall result: **automated Linux checks passed; native/user product acceptance pending**.

## Results and evidence boundaries

**Product acceptance remains open.** Linux browser/daemon protocol regressions
are independent of authenticated Gemini acceptance on the user's Windows PC.
The external provider fixture deliberately produces deterministic protocol
responses. Its screenshots prove UI rendering, not Gemini model responses.

| Check | Execution / result | Evidence |
| --- | --- | --- |
| Rust workspace | 124 passed | `cargo test --workspace` |
| TypeScript and embedded UI | Passed | `npm run build`, production assets embedded in Rust binaries |
| UI state/design/motion | 11 passed | `npm run test:state` |
| Browser → daemon → external ACP provider | 6 passed, including the real 45-second silent-provider deadline | `npx playwright test -c playwright.qa17.config.ts` |
| Official Gemini 0.68.0 | 6 passed: existing app regressions, official version launch and ACP missing-auth error; isolated without account credentials | `AGMUX_GEMINI_TEST=1 npm run test:live` |
| Formatting / Clippy | Passed, final source including compatibility check | `cargo fmt --all -- --check`; `cargo clippy --workspace --all-targets -- -D warnings` |
| Architecture and strict schema | Schema, readiness and scan passed | `npm run arch:check`; `npm run arch:scan`; checker `--strict` |
| Native Windows packaged regression | Pending GitHub Actions | CI's Windows build runs browser suite against the packaged release binaries |
| Authenticated native Windows / DPI | **Not executed** | Requires signed-in user's machine, exact archive and native display settings |

## V-cycle traceability

All requirement IDs below have the prefix `REQ-UX-`; LLD IDs have matching
`LLD-UX-` suffixes. Verification IDs are `TEST-VER-UX-101` through `112`.

| Report | Backlog | Requirement suffixes | Design | Verification seam | Acceptance state |
| --- | --- | --- | --- | --- | --- |
| QA-07 | TASK-36 | PROMPT-SUBMIT-101 | `crates/agmux-core/docs/pty_manager_lld.md` | Browser reply → authenticated IPC → native PTY; two messages and multiline framing | Linux regression passed; native Windows pending |
| QA-03 | TASK-37 | NONGIT-DIFF-102; BOUNDED-ERROR-103 | `crates/agmux-core/docs/diff_calculator_lld.md`; CLI companion LLD | Real Git/non-Git folder, chat usability, long UI error | Linux SVT passed; Windows/user retest pending |
| QA-04 | TASK-38 | TERMINAL-CELLS-104 | Core snapshot LLD; CLI companion LLD | Public screen-buffer cursor/mode test + browser terminal border/resize | Linux regression passed; native DPI acceptance pending |
| QA-05/06 | TASK-39 | CHAT-MESSAGES-105; TURN-LIFECYCLE-106; CHAT-ISOLATION-111 | `crates/agmux-daemon/docs/gemini_chat_lld.md`; CLI companion LLD | Real browser/daemon + external ACP streaming, permission, cancel, failure, dedupe and independent histories | Protocol SVT passed; authenticated Gemini two-turn acceptance pending |
| QA-02 | TASK-40 | ADVANCED-SETUP-107 | `crates/agmux-cli/docs/companion_gui_lld.md` | Keyboard disclosure, basic launch, deliberate advanced arguments | Linux SVT passed; Windows/user retest pending |
| QA-01 | TASK-41 | PROVIDER-MODELS-108; MODEL-DEFAULTS-109 | Gemini and companion LLDs | Provider-advertised inventory/default, stale-provider reset, conflicting flags, unavailable/unsupported/deadline fallback | Protocol/default SVT passed; authenticated Windows inventory pending |
| QA-08/all | TASK-42 | WINDOWS-ACCEPTANCE-110; STARTUP-DIAGNOSTICS-112 | CLI companion and existing daemon spawner LLD | Native startup CI, old-backend IPC compatibility, packaged Windows browser regression + manual account/DPI matrix | Acceptance pending; prior release startup fix remains released |

See the generated [requirements matrix](../../docs/specs/TRACEABILITY_MATRIX.md)
and [atomic specification](../../docs/specs/12_companion_acceptance_spec.md).

## Missing requirements and prevention added

The earlier 63 verified requirements covered infrastructure, snapshots and CLI
launch, but did not establish a true two-turn conversation, native Enter,
non-Git usability, provider model inventory or UI rendering at Windows scaling.
Twelve explicit acceptance contracts now cover those gaps. Strict schema checking
is enforced in CI. Both packaged build jobs run the new browser/daemon regression
suite and upload screenshots/test results. Model version checks remain useful
startup probes and are explicitly insufficient for conversation acceptance.

Gemini conversations use official ACP; Terminal retains an independent native
CLI context in the same project. The UI explains that separation. History
survives UI reload while the daemon runs; daemon restart does not recover the
in-memory ACP context. Other providers retain their labeled native transcript
view. No provider credentials enter the browser.

## Native Windows acceptance procedure — TEST-VER-UX-110/112

Do not mark these complete using fixture responses or a version-only launch.
Do not describe this PR as a verified functional release until this record is
filled with native evidence. Per `.soul/requirements/state-transitions.md`, keep
the PR draft while in-scope requirements remain Implemented/Failed.

1. Record the tested workflow URL, source SHA, ZIP filename/SHA256, Windows and
   browser versions, browser zoom, display scaling, and Gemini version.
2. Extract both executables together. Stop an older daemon deliberately after
   saving its sessions; `agmux gui` must open without lock-path workarounds.
   Also try the new GUI against an old daemon: it must give a readable restart
   instruction before sending unsupported requests; it must not kill sessions.
3. Choose a plain folder. Start Gemini with basic fields/default model; Chat,
   Terminal and Files remain usable. Git shows a bounded non-repository state
   and does not create `.git`. Repeat in a repository and check actual diffs.
4. With Gemini signed in, discover its inventory/default. Send a greeting and a
   follow-up that depends on the first message. Capture actual assistant replies,
   ordered messages and return to idle. Inspect the native Terminal independently.
5. Send multiline text using Shift+Enter, then a second different prompt. Verify
   complete content, one submission each, no concatenation, no duplicate turns.
   In Terminal, verify native prompts/Enter as well as Chat's structured submission.
6. Trigger a tool permission. Reject and separately allow the requested operation;
   verify the provider observes each choice. Test Stop, login failure and provider
   exit. Switch sessions/reload: histories remain isolated and state catches up.
7. Show a deterministic border/cursor screen and the real Gemini TUI at
   100/125/150 percent **native Windows display scaling**, after browser/panel
   resize and Chat/Terminal switches. Attach full-window screenshots.
8. Force an invalid database location in an isolated test configuration; capture
   the understandable cause and retained diagnostic-log location. Restore normal
   settings and verify a fresh GUI launch.

| Required native record | Value |
| --- | --- |
| Workflow / source SHA | Pending |
| Windows archive SHA256 | Pending |
| Windows/browser/Gemini versions | Pending |
| Authenticated two-turn messages | Pending |
| Native scaling 100 / 125 / 150 percent | Pending / Pending / Pending |
| Plain-folder + actual repository | Pending |
| Startup + controlled failure | Pending |
| User confirmation / follow-up PR | Pending |

## Visual proofs

- [Conversation UI](../../docs/assets/screenshots/qa17_chat_verification.png): actual
  packaged app + Rust daemon, **external ACP fixture responses**.
- [Non-Git project](../../docs/assets/screenshots/qa17_nongit_verification.png): real
  plain folder, no Git initialization or global usage dump.
- [Setup/default fallback](../../docs/assets/screenshots/qa17_setup_verification.png):
  accessible basic setup, configured-default fallback.
- [Terminal](../../docs/assets/screenshots/qa17_terminal_verification.png): native
  PTY border/input fixture; Linux, not Windows DPI sign-off.

## Traceability contribution

| Scope | Before explicit QA contracts | Expanded baseline | After this implementation |
| --- | --- | --- | --- |
| Existing requirements | 63/63 verified | 63/63 | 63/63 unchanged |
| New QA acceptance contracts | Absent | 0/12 verified | 7/12 verified; 5 pending native/authenticated acceptance |
| Total verification | 63/63 (100%, smaller scope) | 63/75 (84%) | 70/75 (93.3%); +9.3 points against expanded scope |
| Implementation | 63/63 | 63/75 | 75/75 implemented |

Newly verified contracts: REQ-UX-NONGIT-DIFF-102, REQ-UX-BOUNDED-ERROR-103,
REQ-UX-TURN-LIFECYCLE-106, REQ-UX-ADVANCED-SETUP-107,
REQ-UX-PROVIDER-MODELS-108, REQ-UX-MODEL-DEFAULTS-109,
REQ-UX-CHAT-ISOLATION-111. These results use the platform/protocol evidence above;
they do not mark the issue's final Windows/user acceptance checkboxes complete.

- [Official CLI missing-auth UI](../../docs/assets/screenshots/qa17_auth_verification.png): real Gemini 0.68.0, isolated local configuration, no model response or account acceptance claimed.
