# Implementation Brief: Task 35 - Desktop Companion GUI, Real-Time SSE Viewport & Split Monaco Panels

## 1. Context & Objective
You are the delegated implementer for **agmux** working on **TASK-35** (`REQ-GUI-DESKTOP-APP-005`).
Agmux is an ultra-fast, lightweight cross-platform agentic terminal multiplexer written in modern Rust (2024 Edition, MSRV 1.85+).
Your goal is to enhance the Desktop Companion GUI in `crates/agmux-cli/src/companion_gui.rs` by adding:
1. Low-latency Server-Sent Events (`/api/events`) real-time streaming (<50ms).
2. A side-by-side split panel layout hosting a syntax-highlighted code editor (Monaco/CodeMirror/highlighted editor) alongside the live terminal viewport and web dev previews.
3. OS desktop `.desktop` protocol handler registration for `agmux://` deep links (`agmux register-scheme` or automatic desktop entry creation).

## 2. Scope & Target Files
### New Files:
- `tests/e2e_desktop_companion_gui_test.rs`: Level 5 SVT verification test suite.

### Modified Files:
- `crates/agmux-cli/src/companion_gui.rs`: Implement `/api/events` SSE endpoint, enhanced split-panel HTML/JS dashboard with syntax editor, and `register_desktop_protocol`.
- `crates/agmux-cli/src/main.rs`: Expose `register-scheme` subcommand and integrate desktop protocol handler.
- `docs/specs/11_desktop_gui_and_notifications_spec.md`: Update `REQ-GUI-DESKTOP-APP-005` to `Implementation Status: Verified` and `Test Status: Passed`.
- `.backlog/tasks/task-35 - Tracer-Bullet-Desktop-Companion-GUI-SSE-Split-Monaco-Panels.md`: Mark acceptance criteria `[x]` and `status: Done`.

Do NOT perform speculative refactors outside of this scope.

## 3. Detailed Specifications (`REQ-GUI-DESKTOP-APP-005`)
1. **Server-Sent Events (`GET /api/events`):**
   - In `crates/agmux-cli/src/companion_gui.rs`, handle `GET /api/events`:
     - Validate `X-Agmux-Token` authentication (or `?token=` query param).
     - Response headers: `HTTP/1.1 200 OK\r\nContent-Type: text/event-stream\r\nCache-Control: no-cache\r\nConnection: keep-alive\r\n\r\n`.
     - Immediately emit initial state: `event: snapshot\ndata: { ... }\n\n`.
     - Whenever session outputs, state changes, or periodic ticks occur, push `event: update\ndata: { ... }\n\n` or `event: terminal_output\ndata: { "session_id": "...", "data": "..." }\n\n`.
   - In the frontend JavaScript (`PAGE`), replace polling `setInterval(refresh, 2000)` with an `EventSource('/api/events')` listener with automatic fallback polling if EventSource disconnects.
2. **Side-by-Side Split View & Syntax Editor Panels:**
   - In `PAGE`, layout is structured as a two-column or split pane:
     - Left column: Session switcher + Live Terminal Viewport (`<pre id="screen">` or xterm container) + Inline Reply bar.
     - Right column: Split Docked Panels:
       - Web panel: `<iframe>` preview of web apps (`http://localhost:...`).
       - File panel: Syntax-highlighted code editor with line numbers, code styling, file path title, and Save button (`Ctrl+S` / Save button triggering `/api/file/write`).
3. **OS Desktop Protocol Registration (`agmux://`):**
   - Provide `pub fn register_desktop_protocol(app_path: Option<&Path>) -> Result<PathBuf>` in `companion_gui.rs`.
   - On Linux/Unix: writes `~/.local/share/applications/agmux-scheme-handler.desktop` with:
     ```ini
     [Desktop Entry]
     Type=Application
     Name=Agmux
     Exec={current_exe} open %u
     StartupNotify=false
     Terminal=false
     MimeType=x-scheme-handler/agmux;
     ```
   - Expose via CLI subcommand: `agmux register-scheme` (e.g. `Commands::RegisterScheme`).
4. **E2E Integration Test (`tests/e2e_desktop_companion_gui_test.rs`):**
   - Spins up test daemon and starts `companion_gui::serve`.
   - Tests:
     1. `test_companion_gui_sse_event_stream`: Connects to `http://127.0.0.1:<port>/api/events`, sends input to session, asserts SSE stream emits real-time event updates within 100ms.
     2. `test_companion_gui_split_panels_read_write`: Opens file panel with `panels open`, reads via `/api/file/read`, edits and writes via `/api/file/write`, asserts file on disk updated.
     3. `test_companion_gui_deep_link_resolution`: Tests `open_link` with `agmux://session/<id>` and `agmux://session/<id>/panel/<panel_id>` successfully selecting target session and panel.
     4. `test_desktop_protocol_registration`: Calls `register_desktop_protocol` and verifies generated `.desktop` file contains `x-scheme-handler/agmux;` and valid executable path.

## 4. Verification Gates
Before reporting completion, run and ensure 100% clean passes:
- `cargo check --workspace`
- `cargo test --test e2e_desktop_companion_gui_test`
- `cargo clippy --workspace -- -D warnings`
- `cargo fmt --check`
- `node .soul/tools/v-cycle-checker.mjs` (must pass with 0 errors and 100% completion)

## 5. Constraints
- Do NOT commit changes (`git commit`). The orchestrator will review the diff and land commits.
- Follow the Karpathy guidelines: keep code clean, simple, surgical, with robust error handling.
- Preserve all existing functionality and documentation.
