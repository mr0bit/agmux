# Issue #17 — real native Terminal and synchronized Chat

Date: 2026-11-18 (Africa/Cairo). Tracker: https://github.com/mr0bit/agmux/issues/17.
Fix PR: https://github.com/mr0bit/agmux/pull/18.
Runtime source is the native-terminal correction commit recorded by CI in that PR.
Earlier terminal-styled Chat builds do not qualify this correction. v0.1.0 is unchanged.

## Corrected requirement and delivered behavior

The user clarified that Chat must stay conversational and Terminal must be the
provider's actual interactive CLI/TUI. Requirements REQ-UX-SHARED-CONTEXT-113 and
REQ-UX-SHARED-STATE-114 were reopened and rewritten; TASK-43 tracks the correction.

Chat renders structured provider messages. Terminal mounts LiveTerminal/xterm
with actual native PTY output, keyboard input, resize and process exit events.
The rejected agent-terminal Chat styling is removed.

Gemini 0.68.0 supports ACP session/load and native --resume UUID, but does not
attach its native TUI to an ACP stdio process. Agmux therefore retains one
canonical provider UUID and gives one interface writer ownership at a time.
At an idle boundary it reaps the old writer before starting the other interface
against that exact saved conversation. It never uses latest-session selection
or a new conversation as a fallback after a failed load.

The installed Gemini source/docs were checked for session persistence, load,
resume, lifecycle hooks, JSON/JSONL record updates and Windows PowerShell hook
execution. These are supported provider seams; the installed CLI is not patched.

## Synchronization and ownership

- Native message previews and tool activity read the provider's real JSON/JSONL transcript,
  validate sessionId, and replace repeated message IDs. Terminal banners,
  controls and ANSI screens never become assistant replies.
- Queued view switches wait for running turns, approvals and native input drafts.
  ACP approvals stay in Chat; native CLI approvals stay in Terminal. No tool
  approval is guessed, auto-selected or replayed during a handoff.
- Chat drafts remain intact. Native input editing stays inside the actual CLI.
  Chat submit is guarded while the requested interface is synchronizing.
- Per-session view queues preserve the user's latest tab choice without blocking
  an unrelated agent session behind another session's slow provider startup.
- A fresh Terminal-first conversation uses --session-id. An empty native session
  retains its owner until its first turn creates resumable content; Chat can
  submit that first turn through the same native owner, keeping the UUID.
- Native /quit reports the actual exit code. Chat can then load the same saved
  conversation. Explicit End retains its synchronized transcript read-only.
- Unsupported/failed load preserves history and identity and reports an error;
  no session/new replacement is attempted.

Native readiness uses process-scoped SessionStart/BeforeAgent/AfterAgent hooks.
Their tiny private event includes identity, phase, transcript path and time;
it contains no provider credentials or extra model context. The helper is a
hidden local agmuxd entry point and never starts another daemon or opens IPC.

The private system-defaults overlay preserves JSONC defaults and concatenates
existing hooks. User/project settings and enforced system policy are unchanged.
Hook execution uses Bash on Unix and PowerShell on Windows, matching Gemini.
The native regression provider executes these configured shell commands and
checks that existing defaults/hooks survived. Disabled or unavailable hooks do
not imply idle: finish/exit the native CLI to release control rather than
interrupting a turn or assuming a prompt is ready.

## Verification and evidence boundaries

| Check | Result / evidence |
| --- | --- |
| Workflow browser → real packaged app/daemon → external provider | 18 cases passed, including same UUID/context in both directions, native approval guard, failed load, Terminal-first, native exit, rapid switching and existing workflows |
| Provider process ownership | Final six focused checks passed, including distinct ACP/native PIDs, retired writers gone, native tool activity, failed load, exit, Terminal-first and rapid switching |
| Official Gemini CLI 0.68.0/application | 7 passed, including the actual native TUI authentication screen in a PTY and isolated ACP missing-auth behavior |
| Rust workspace | 126 passed |
| UI state/design/motion | 11 passed |
| Production UI/embedded binaries, Cargo check, formatting, all-target Clippy | Passed locally; existing unused futures-util manifest warning remains |
| Groma and strict V-cycle schema | Refreshed source scan and generated matrix; requirements and Backlog links kept current |
| Corrected native Windows packaged workflow | CI result and exact-source executable artifact/checksum are recorded in PR #18 / issue #17 |
| Signed-in Gemini Chat ↔ native CLI context on user Windows PC | Pending user acceptance; fixture memory is not a Gemini model response |
| Actual Windows DPI 100/125/150% | Pending user acceptance |

The external provider fixture stands in only at the third-party boundary. It
persists real test-provider records, runs actual shell hooks, uses actual PTYs,
and transports actual terminal bytes through authenticated HTTP/IPC/WebSocket
paths. Production code contains no test-provider messages or scripted model
responses. The official CLI test isolates credentials and does not claim a
signed-in model conversation or edit approval result.

## User point-of-view screenshots

- Official Gemini 0.68.0 native authentication TUI:
  `docs/assets/screenshots/qa17_official_native_terminal_verification.png`.
- Actual native PTY with shared context, external provider fixture:
  `docs/assets/screenshots/qa17_native_terminal_verification.png`.
- Proper Chat after importing the native conversation, external provider fixture:
  `docs/assets/screenshots/qa17_native_chat_sync_verification.png`.

## User Windows acceptance

Use the new native-correction build linked from PR #18, keeping agmux.exe and
agmuxd.exe together. Save work and stop an older daemon first; capability
negotiation rejects an older backend instead of using the rejected presentation.
Sign into the installed Gemini CLI using the user's normal local account.

Tell Chat a unique fact, switch to Terminal, confirm that Gemini's own TUI and
keyboard controls are present, recall/add a fact there, then switch to Chat and
verify recall of both facts under the same provider session. Repeat with an
in-flight turn, pending approval, unsent Chat draft, native input draft and
rapid tab switching. Finish native approvals in Terminal and Chat approvals in
Chat. Check native exit/resume and End → Inactive/Archive retention.

Record the archive checksum, Windows/Gemini version, display scaling, actual
result and screenshots in issue #17. TASK-50 and authenticated/user acceptance
remain open; the PR stays draft until that acceptance is recorded.

## Native terminal protocol regression

A real CSI 6n query/response reproduced an incorrect permanent Working state:
automatic xterm replies were being treated as user draft edits. The readiness
guard now distinguishes known cursor/device/mode/size/color/focus reports while
forwarding every byte unchanged. Actual keys, edits, pastes and prompts still
invalidate readiness until the provider's next lifecycle boundary. The public
native-context regression waits for an actual xterm reply before asserting idle
and proceeding through the same saved provider session.


## Windows qualification follow-up: ConPTY exit versus output EOF
Run 105 passed quality and all 18 Linux packaged browser checks. Windows passed
13/18 but exposed five native exit/handoff failures: the child was reaped, while
its exit event waited indefinitely for ConPTY output EOF with the viewport's
master still alive. Windows now bounds the final output drain at 250 ms before
publishing the actual reaped-child exit. This preserves the single-writer guard;
it does not bypass a live process. A native public-PTY regression holds the
viewport/master open during both natural and forced exit. The final corrected
Windows run/artifact remains recorded in PR #18 / issue #17; failed run 105 did
not publish a Windows executable artifact. Signed-in/DPI acceptance stays open.


## QA-18 — missing Windows Visual C++ runtime (22 November 2026)
The user's system-loader dialog reports `VCRUNTIME140.dll` missing. Inspection
of the exact previous Windows test ZIP confirms both `agmux.exe` and `agmuxd.exe`
import it. Hosted Windows CI had the runtime installed, so successful startup
there did not establish portable loading on a clean user machine.

REQ-PKG-WINDOWS-CRT-126 / TASK-50 now enable static CRT linkage and gate packaging
on inspection of both actual PE dependency tables, followed by the existing
standalone and packaged browser checks. Native Windows verification is pending
until the corrected CI run completes; exact source/artifact/checksum will be
recorded in issue #17 and PR #18. A user clean-machine retest remains separate.
The official Microsoft x64 redistributable can unblock the old build; users
should not download individual runtime DLLs from third-party sites.
