# Implementation Brief: Task 34 - Mobile Push Relays (ntfy.sh & Webhooks with Actionable Inline Replies)

## 1. Context & Objective
You are the delegated implementer for **agmux** working on **TASK-34** (`REQ-GUI-MOBILE-PUSH-004`).
Agmux is an ultra-fast, lightweight cross-platform agentic terminal multiplexer written in modern Rust (2024 Edition, MSRV 1.85+).
Your goal is to implement mobile push notification relays in `crates/agmux-daemon` supporting `ntfy.sh` and generic webhooks, so developers away from their terminal receive push notifications when agent sessions transition to `WAITING` or `ERRORED`, complete with 120-character excerpts and 1-tap actionable inline reply buttons.

## 2. Scope & Target Files
### New Files:
- `tests/e2e_mobile_push_relay_test.rs`: Level 5 SVT verification test suite.

### Modified Files:
- `crates/agmux-daemon/Cargo.toml`: You are authorized to add `reqwest` (with `rustls-tls` / `blocking` or async) or use `tokio::net::TcpStream` for HTTP.
- `Cargo.lock`: Updated cargo lockfile.
- `crates/agmux-daemon/src/notification_dispatcher.rs`: Add `PushRelayConfig` and `dispatch_push_relay` handling `ntfy` and `webhook` formats.
- `crates/agmux-daemon/src/config_watcher.rs`: Parse push relay configuration from `config.toml` (`[notifications.push]`, `provider = "ntfy"|"webhook"`, `url`, `topic`, `token`, `reply_base_url`).
- `crates/agmux-daemon/src/event_broadcaster.rs`: Invoke push relay dispatch on `WAITING` and `ERRORED` states.
- `docs/specs/11_desktop_gui_and_notifications_spec.md`: Update `REQ-GUI-MOBILE-PUSH-004` to `Implementation Status: Verified` and `Test Status: Passed`.
- `.backlog/tasks/task-34 - Tracer-Bullet-Mobile-Push-Relays-Ntfy-Actionable-Replies.md`: Mark acceptance criteria `[x]` and `status: Done`.

Do NOT perform speculative refactors outside of this scope.

## 3. Detailed Specifications (`REQ-GUI-MOBILE-PUSH-004`)
1. **Push Relay Configuration in `config.toml`:**
   - Support parsing:
     ```toml
     [notifications.push]
     provider = "ntfy" # or "webhook"
     url = "https://ntfy.sh"
     topic = "my-agmux-alerts"
     token = "optional-bearer-token"
     reply_base_url = "http://127.0.0.1:4040" # Optional companion bridge base URL
     ```
   - Also support flat keys for backward compatibility: `push_provider`, `push_url`, `push_topic`, `push_token`, `push_reply_url`.
2. **Payload Formatting:**
   - **ntfy Provider (`provider = "ntfy"`):**
     - Target endpoint: `{url}/{topic}` (e.g. `https://ntfy.sh/my-topic`).
     - HTTP Headers:
       - `Title`: notification title (e.g. `Session session-123 -> WAITING`)
       - `Priority`: `high` (for ERRORED), `default` or `high` (for WAITING)
       - `Tags`: `warning` for WAITING, `rotating_light,skull` for ERRORED
       - `Actions`: If `reply_base_url` is configured, attach actionable reply buttons:
         - `http, Reply Yes, {reply_base_url}/api/reply, method=POST, headers.Content-Type=application/json, body={"session_id":"{session_id}","message":"y\n"}`
         - `http, Reply No, {reply_base_url}/api/reply, method=POST, headers.Content-Type=application/json, body={"session_id":"{session_id}","message":"n\n"}`
         - `view, Open agmux, agmux://session/{session_id}`
       - `Authorization`: `Bearer {token}` if token is set.
     - HTTP Body: 120-character output quote (`notification.body`).
   - **webhook Provider (`provider = "webhook"`):**
     - Target endpoint: `{url}`.
     - HTTP Headers: `Content-Type: application/json`, and `Authorization: Bearer {token}` if token is set.
     - JSON Body:
       ```json
       {
         "kind": "WAITING",
         "session_id": "...",
         "session_name": "...",
         "tool": "...",
         "cwd": "...",
         "branch": "...",
         "title": "...",
         "body": "...",
         "actions": [
           { "label": "Reply Yes", "url": "...", "method": "POST", "body": "..." }
         ]
       }
       ```
3. **Bounded Asynchronous Execution:**
   - Push relay requests must run asynchronously in the background using the semaphore bounded slots from `NotificationDispatcher` (or a dedicated bounded queue) with a 10-second timeout so the daemon event loop is never blocked by slow network requests.
4. **E2E Integration Test (`tests/e2e_mobile_push_relay_test.rs`):**
   - Spins up a local `tokio::net::TcpListener` mock HTTP server.
   - Configures `config.toml` pointing to the mock server.
   - Tests:
     1. `test_ntfy_push_relay_dispatch_on_waiting`: Transitions session to `WAITING_INPUT`, verifies mock server receives POST to `/{topic}` with `Title`, `Priority`, `Actions`, and body quote.
     2. `test_webhook_push_relay_dispatch_on_errored`: Transitions session to `FAILED`, verifies mock server receives webhook JSON with correct fields.
     3. `test_push_relay_action_reply_integration`: Verifies that sending the action reply body to the companion `/api/reply` endpoint delivers input to the target session.

## 4. Verification Gates
Before reporting completion, run and ensure 100% clean passes:
- `cargo check --workspace`
- `cargo test --test e2e_mobile_push_relay_test`
- `cargo clippy --workspace -- -D warnings`
- `cargo fmt --check`
- `node .soul/tools/v-cycle-checker.mjs` (must pass with 0 errors)

## 5. Constraints
- Do NOT commit changes (`git commit`). The orchestrator will review the diff and land commits.
- Follow the Karpathy guidelines: keep code clean, simple, surgical, with robust error handling.
- Preserve all existing functionality and documentation.
