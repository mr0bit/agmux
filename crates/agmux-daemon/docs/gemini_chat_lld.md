# Gemini managed conversation design

## Ownership — LLD-UX-SHARED-CONTEXT-113 / LLD-UX-CHAT-MESSAGES-105
A managed GUI Gemini session owns one lazy ACP process and one provider session
ID. Chat and Agent Terminal are two presentations of that same conversation.
Terminal is explicitly a provider transcript/prompt interface, not an OS shell
or a second native Gemini TUI. Switching views never starts a provider or forks
context. Legacy native/custom/CLI sessions retain their raw PTY interface.
The API's explicit --version diagnostic path remains native for launch checks.

Managed creation records an IDLE logical session (PID zero until its provider
starts), its project, initial Git baseline and conversation record. The actual
ACP PID is recorded after spawn. Existing PTY teardown only kills actual PTYs;
managed end uses the conversation lifecycle, never PID-zero process signalling.
Companion Reply and SendMessage entry points route to the same managed actor;
a busy actor rejects a concurrent new message explicitly rather than silently
injecting text into another process. The browser uses authenticated IPC only.

## Turn state — LLD-UX-TURN-LIFECYCLE-106 / LLD-UX-CHAT-ISOLATION-111
Initialize v1, session/new, session/prompt and session/update use JSON-RPC stdio.
State is idle/working/permission/cancelling/cancelled/failed/ended. Request IDs
are deduplicated per session under the state lock. Native CLI process lifetime
does not imply an active turn. Initialization/session creation have 45-second
deadlines; turns have a 300-second watchdog; an unacknowledged Stop reaps the
provider after five seconds. Explicit options are returned for permissions;
the adapter never auto-approves or fabricates an assistant response.

Frame/text limits are 2 MiB, active loaded conversations 32 and pending approvals
32. Transports are reaped on session end and graceful daemon shutdown, using
process groups on Unix and process-tree termination on Windows. Official npm
launcher resolution and ordinary Windows project paths are retained.
GEMINI_CLI_NO_RELAUNCH keeps the supervised process directly responsible for ACP.

## Shared data — LLD-UX-SHARED-STATE-114 / LLD-UX-RETAINED-HISTORY-124
Both views read daemon-owned messages, provider identity, tool events, pending
approvals and state. Their input draft is controlled by per-session UI state;
presentation changes keep the same component/operation/request-ID ownership.
Lightweight snapshot metadata contains states, actual activity timestamps and
managed IDs. Ended records are loaded read-only without consuming live slots.

SQLite managed_conversations retains transcript/model/request identifiers and
activity on accepted input, completed/failed turn and end. No provider credentials
are stored in the browser or transcript. Ending stops transport and retains
history. Existing startup recovery makes orphaned sessions inactive; opening
that history never automatically resends a prompt or starts a new provider.
This is retained history, not automatic active-provider recovery after a daemon
restart. Old volatile histories cannot be reconstructed from a terminal screenshot.

## Edit review — LLD-UX-PROPOSED-DIFF-116 / LLD-UX-EDIT-APPROVAL-117
Tool updates retain actual content blocks and tool IDs. Approval requests retain
the provider's complete toolCall, including diff path/oldText/newText when
provided. Pending permissions are queued and separately exposed in snapshots.
Only advertised option IDs for the currently pending request are accepted.
Review shows a truthful unavailable-preview message when no diff is supplied.
A preview never writes files. The real provider writes after approval; applied
review is recalculated independently from Git/filesystem state.

## Models — LLD-UX-PROVIDER-MODELS-108
Inventory/default comes from an actual transient ACP session/new, without a
model prompt. Successful project-scoped probes cache for 60 seconds, max 32
entries and two concurrent probes. Unsupported/auth/deadline failures preserve
an honest CLI-default fallback. Shared sessions accept one explicit model flag;
unsupported native CLI arguments fail clearly rather than being ignored.
Other native diagnostics remain available through explicit custom executables.

## Verification
Public browser/HTTP/IPC/daemon tests use an external provider boundary fixture
with process/session IDs and ordered request history. They prove shared routing,
concurrency, request dedupe, approval/rejection and retention without claiming
fixture text is a real model response. Native Windows packaging runs the same
regressions. Authenticated Windows context and actual display scaling remain
separate user acceptance evidence.
