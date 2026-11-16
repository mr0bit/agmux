# Live companion integration design

The `companion_gui` boundary owns the browser bridge and frontend. Running
`agmux gui` after building the UI serves the same React Native Web design as the
prototype, connected to the actual Rust daemon over its public authenticated IPC.
The daemon remains authoritative for process execution, session metadata, file
panels, terminal state, Git data and inter-agent delivery.

## Boundaries and runtime

- `companion_gui.rs`: existing CLI entry, snapshot/deep-link helpers and legacy
  basic page. Selects the enhanced server when built assets exist.
- `companion_gui/live.rs`: Axum HTTP/SSE/WebSocket server. Binds `127.0.0.1` only;
  serves `/assets` from the built UI directory and injects a per-process browser
  credential into the root HTML. It does not expose arbitrary IPC envelopes.
- `ui/src/backend.ts`: typed browser client; browser secret stays in memory.
- `ui/src/LiveApp.tsx`: actual workspace, prompt, file, diff and process controls.
- `ui/src/LiveTerminal.tsx`: xterm viewport sync, byte streaming, resize, reconnect
  and cleanup. It uses separate connections per selected terminal.
- `ui/src/App.tsx`: isolated design preview, without live provider calls.
  `ConnectBackend.tsx` guides users into the locally served application.

The entrypoint chooses live mode only when the bridge bootstrap is present.
Demo storage and scripted replies are not used in live mode. Node.js is required
for building, not for serving the built app. `AGMUX_UI_DIST` overrides asset lookup;
`AGMUX_GUI_PORT` selects an optional loopback port.

## Public browser operations

| Route | Rust public seam and behavior |
| --- | --- |
| `GET /api/info` | Installed built-in CLI discovery, shell availability, machine/version and explicit capability flags. Filesystem discovery runs off the async reactor. |
| `GET /api/snapshot?session_id=…` | `WorkspaceRequest::List`, `CompanionRequest::List`, `ReadSessionRequest`. Explicit selection prevents another browser's selection from replacing this browser's screen. |
| `GET /api/events?token=…&session_id=…` | Initial snapshot, journal replay using `WatchEventsRequest`/Last-Event-ID, and viewport subscription. Dirty updates coalesce on a 20 ms tick; unchanged state is not repeatedly polled. Named `offline` events surface disconnects. |
| `GET /api/terminal?token=…&session_id=…` | WebSocket over authenticated `SubscribeViewport`, `ScreenSyncFrame`, `TerminalOutputFrame`, `TerminalInputFrame`, resize and exit messages. Binary frames preserve bytes; JSON handles resize/sync/exit metadata. |
| `POST /api/spawn` | Built-in adapter or explicit custom executable, argument array, canonical existing CWD, detached PTY and random session ID. Persists title and selects the new session. |
| `POST /api/select` | `CompanionRequest::Select`. Live UI commits selection after success. |
| `POST /api/reply` | Explicit user message plus newline through `CompanionRequest::Reply`, with a 64 KiB limit. Direct PTY input, not an approval API or an inferred assistant turn. |
| `POST /api/send` | `SendMessageRequest`; returns backend immediate/queued status. Retains daemon WAITING safety behavior. |
| `POST /api/interrupt` | Ctrl C as raw terminal input to the specified session. |
| `POST /api/freeze` | Existing freeze/snapshot behavior; stops the process and retains archived screen output. It is not native CLI resume/handoff. |
| `POST /api/workspace` | Existing workspace metadata mutation, including title/favorite. |
| `POST /api/files` | Bounded directory listing under canonical session CWD. Rejects parent escape and symlinks outside the workspace. Excludes large generated directories. |
| `POST /api/panels/open` | Existing panel registration and canonical path boundary; returns the exact opened panel ID. |
| `POST /api/file/read` | Existing registered file read plus a SHA-256 version fingerprint. |
| `POST /api/file/write` | Validates optional loaded version before existing registered file write. Rejects observed external changes with HTTP 409. The preflight is not atomic against arbitrary external filesystem writers. |
| `POST /api/diff` | `GetDiffRequest` with explicit existing scope, returning actual files/hunks/lines. |

## Security and resource behavior

The bridge checks exact Host, browser API token, and the exact loopback Origin
for mutations and WebSocket upgrades. SSE accepts the browser token in its query
because EventSource cannot set the custom header. Neither daemon credentials nor
provider credentials are sent to the browser. There is no wildcard CORS or remote
listener. API bodies, WebSocket frames, file reads/writes, directory listings,
terminal dimensions and live connection counts are bounded. IPC operations have
timeouts; slow raw WebSocket writes time out. The local UI rejects framing and
uses no-referrer/no-store responses. Only the built asset directory is served.

The SSE stream watches native journal and viewport activity. It does not perform
full snapshots continuously while idle. Disconnecting a browser drops its IPC
subscriptions; backend sessions continue. Replay restores journal state, while
terminal reconnect starts with a fresh screen sync to avoid duplicated bytes.

Files are registered and revalidated by the daemon before read/write. Browser
unsaved edits survive an error and block session/file changes until saved or
discarded. Directory listings also respect workspace bounds for archived local
sessions. Remote-mounted filesystem access is not provided by this local bridge.

## Capability limits

The first functional release runs arbitrary installed CLIs in their actual PTYs.
Chat is the real terminal transcript plus explicit prompt input. Rich structured
chat/tool events, native approval buttons, attachment translation, model/mode
changes and cross-provider conversation handoff are not fabricated. They need
provider-specific contracts. Users handle their own CLI login/settings and answer
permission prompts in Terminal. The browser does not auto-acknowledge WAITING.

The hosted private Site is still a design sandbox and connection guide. It cannot
reach a user's localhost or host the Rust daemon. Authenticated remote pairing /
relay access and Tauri packaging are separate capabilities.

## Verification

The live Playwright fixture starts real `agmuxd` and `agmux gui` processes, an
isolated SQLite/socket/token setup and a temporary Git repository. It verifies
real prompt and WebSocket input/output, two-session isolation, browser reload,
actual disk writes and Git diffs, stale-write rejection, authentication/origins,
workspace/symlink confinement, inter-agent delivery, PTY resize and end/snapshot
behavior. It captures an actual user-view screenshot in
`docs/assets/screenshots/live_companion_verification.png`.

Existing Rust HTTP/IPC companion regression tests exercise the enhanced server
when assets are present. The full Rust suite also revealed a packetization-
dependent PTY assertion: it counted frames containing a marker rather than
occurrences in the stream. The corrected assertion buffers raw bytes and still
requires exactly two marker occurrences, regardless of split/coalesced frames.

Provider account authentication and model responses are not part of the shell
fixture; they must be validated with the user's installed/authenticated CLIs.
