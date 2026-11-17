# Companion bridge

`LLD-GUI-COMPANION-PANELS-002` implements `REQ-GUI-COMPANION-PANELS-002`. The daemon owns persistent panel registrations, notification history and the selected session in SQLite. Authenticated framed IPC registers and queries panels and carries file reads, writes, and replies. File registrations must resolve to existing regular files under the session working directory, and every subsequent access validates the registration and canonical path again. Only HTTP and HTTPS web targets are accepted.

`agmux panels open <target> --session <id>` registers a panel. `agmux open agmux://session/<id>` and `agmux://session/<id>/panel/<numeric-id>` validate and select a registered target. `agmux gui --snapshot --json` prints state; `agmux gui` binds an ephemeral IPv4 loopback port and prints its URL. Its browser dashboard polls the daemon, displays the attention count and viewport, edits registered files, embeds sandboxed web previews, and submits replies to the target PTY through IPC.

The bridge validates its Host header and requires a per-process secret for API access; mutations also require the exact loopback Origin. The secret is embedded in the local HTML and is never the daemon authentication token. The HTTP listener is not a remote service. Tests exercise actual HTTP requests, invalid origins, unregistered files, deep links, reply delivery and restart persistence.

## Issue #17 UI acceptance design

### LLD-UX-BOUNDED-ERROR-103
Global errors summarize the first line (at most 220 characters); full diagnostics
are expandable with a bounded scroll area. Modal errors follow the same rule.
Git errors stay in the Git review area, including a friendly non-Git empty state,
so they do not replace the conversation. `TEST-VER-UX-103` injects a long external
diagnostic through the real UI boundary and measures its height.

### LLD-UX-TERMINAL-CELLS-104 (renderer)
Xterm uses measured monospace cells with lineHeight 1, FitAddon and ResizeObserver.
Font readiness/loading and window resize trigger a refit; PTY dimensions follow
the measured grid over the existing WebSocket. Parser state survives resize.
Browser fixtures check borders/input and resizing. Native Windows fonts and
100/125/150 percent display scaling are a separate acceptance matrix.

### LLD-UX-CHAT-MESSAGES-105 (view)
`GeminiChat` renders actual ACP user/assistant/tool records, permission controls,
errors, state and Stop. It never consumes screen snapshots. Other providers keep
their explicit native transcript view until a structured adapter exists.
History remains daemon-owned across UI reload; rendering uses React text nodes.

### LLD-UX-ADVANCED-SETUP-107
Accessible native details/summary hides JSON arguments until Advanced settings is
expanded. Session name, project, provider and model remain in the primary flow.

### LLD-UX-MODEL-DEFAULTS-109
Debounced inventory probes are scoped to provider and cwd; cleanup guards discard
stale responses when either changes. Switching resets the selected model. The
empty selection preserves CLI settings; explicit advertised selections append
one --model argument. Conflicting Model/advanced flags fail validation. Probe
failures do not block starting with the configured default.

### LLD-UX-WINDOWS-ACCEPTANCE-110 / LLD-UX-STARTUP-DIAGNOSTICS-112
CI builds packaged binaries and runs the browser regression suite with an
external ACP fixture on Linux and Windows, in addition to existing native
startup/default pipe, Gemini version and invalid-database diagnostics checks.
Authenticated two-turn Gemini and native DPI results remain mandatory release
acceptance evidence: exact source/archive checksum, steps, observations and
screenshots belong in tests/reports/qa17_acceptance_report.md. An executable
version probe alone must never be reported as conversation acceptance.

### Upgrade compatibility (LLD-UX-STARTUP-DIAGNOSTICS-112)
Before sending new Companion variants or opening the browser, the GUI sends the
existing List operation and checks its content capability marker. Older daemons
respond safely without the marker; the GUI explains how to stop the old daemon
and relaunch matching executables. It never silently terminates active sessions.
`TEST-VER-UX-112` has a real IPC stand-in for the prior backend contract.
