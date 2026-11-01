# Software Requirements Specification: Rich Notifications & Phase 3 Desktop/Web Companion Bridge

## 1. Overview
This specification details the rich notification and custom shell hook dispatcher (`FR-P0-08`) alongside the Phase 3 Desktop/Web Companion GUI, embedded panel launchers (`panels open <url|file>`), and `agmux://` deep-link handler (`FR-P3-01`–`FR-P3-03`).

---

## 2. Requirements

### REQ-GUI-NOTIFICATION-HOOKS-001
* **Unique ID:** `REQ-GUI-NOTIFICATION-HOOKS-001`
* **Text:** As a user running background agent sessions, when a session transitions to `WAITING`, `FINISHED`, or `ERRORED`, `agmuxd` shall emit a structured notification containing a 120-character quote of the latest output and invoke configured custom shell notification hooks with `AGMUX_NOTIFY_KIND`, `AGMUX_SESSION_ID`, `AGMUX_SESSION_NAME`, `AGMUX_TOOL`, `AGMUX_CWD`, `AGMUX_BRANCH`, `AGMUX_TITLE`, and `AGMUX_BODY` environment variables under a 15-second execution timeout.
* **Criticality:** Medium
* **SW Component:** `crates/agmux-daemon::notification_dispatcher`
* **Implementation Status:** Verified
* **Verification Criteria:** Configure a shell notification hook script; trigger a session transition to `WAITING` and `FINISHED`; verify the hook executes with all 8 `AGMUX_*` environment variables populated and a 120-char body excerpt, and enforces the 15s timeout.
* **Downstream Req:** `LLD-GUI-NOTIFICATION-HOOKS-001`
* **Downstream Test:** `TEST-VER-GUI-001`
* **Test Status:** Passed

---

### REQ-GUI-COMPANION-PANELS-002
* **Unique ID:** `REQ-GUI-COMPANION-PANELS-002`
* **Text:** As a user interacting with Agmux across desktop panels and companion devices, the system shall support `agmux panels open <url|file>` (docking/registering file editor and web preview panels), `agmux://` URI deep-link resolution, and `agmux gui` (the Phase 3 companion GUI bridge serving the live Attention Counter badge, session viewports, docked panels, and 1-tap inline reply endpoint).
* **Criticality:** Medium
* **SW Component:** `crates/agmux-cli::companion_gui`
* **Implementation Status:** Verified
* **Verification Criteria:** Run `agmux panels open src/main.rs` and `agmux panels open http://localhost:3000`; resolve an `agmux://session/<id>` deep-link; query the companion GUI snapshot (`agmux gui --snapshot --json`) and verify Attention Counter telemetry, docked panels, and inline reply dispatch.
* **Downstream Req:** `LLD-GUI-COMPANION-PANELS-002`
* **Downstream Test:** `TEST-VER-GUI-002`
* **Test Status:** Passed

---

### REQ-GUI-SELF-UPDATER-003
* **Unique ID:** `REQ-GUI-SELF-UPDATER-003`
* **Text:** As a user maintaining an Agmux installation, the system shall provide `agmux update` and `agmux --update` to check the official GitHub Releases API for new versions, verify the target-triple binary archive against published SHA-256 checksum manifests, and perform atomic in-place binary replacement of `agmux` and `agmuxd` with rollback on failure.
* **Criticality:** High
* **SW Component:** `crates/agmux-cli::self_updater`
* **Implementation Status:** Verified
* **Verification Criteria:** Execute `agmux update --check` against release metadata; download release asset; verify SHA-256 checksum matching and mismatch rejection; replace executable atomically and verify newly installed binary version.
* **Downstream Req:** `LLD-GUI-SELF-UPDATER-003`
* **Downstream Test:** `TEST-VER-GUI-003`
* **Test Status:** Passed

---

### REQ-GUI-MOBILE-PUSH-004
* **Unique ID:** `REQ-GUI-MOBILE-PUSH-004`
* **Text:** As a developer running long-running agent workflows away from the terminal, when a session transitions to `WAITING` or `ERRORED`, `agmuxd` shall dispatch rich push notifications via configured mobile push relays (including `ntfy.sh` topics and generic webhooks) with session title, 120-character output excerpt, and actionable inline reply buttons routed back to the companion endpoint.
* **Criticality:** Medium
* **SW Component:** `crates/agmux-daemon::notification_dispatcher`
* **Implementation Status:** Verified
* **Verification Criteria:** Configure push relay URL and ntfy topic in `config.toml`; trigger `WAITING` state; verify HTTP POST is delivered with action buttons, topic header, and payload containing title and body quote.
* **Downstream Req:** `LLD-GUI-MOBILE-PUSH-004`
* **Downstream Test:** `TEST-VER-GUI-004`
* **Test Status:** Passed

---

### REQ-GUI-DESKTOP-APP-005
* **Unique ID:** `REQ-GUI-DESKTOP-APP-005`
* **Text:** As a desktop user, the companion GUI bridge (`agmux gui`) shall provide real-time Server-Sent Events (SSE) streaming of session viewports (<50ms latency), OS desktop protocol registration for `agmux://` URIs, and side-by-side split panels hosting syntax-highlighted code editing (Monaco/CodeMirror) and live web development previews.
* **Criticality:** High
* **SW Component:** `crates/agmux-cli::companion_gui`
* **Implementation Status:** Not implemented
* **Verification Criteria:** Launch `agmux gui`; connect to `/api/events` SSE stream and observe immediate viewport frame pushes on terminal output; verify `agmux panels open <file>` loads the split code editor; verify `agmux open agmux://session/<id>` activates the target session.
* **Downstream Req:** `LLD-GUI-DESKTOP-APP-005`
* **Downstream Test:** `TEST-VER-GUI-005`
* **Test Status:** Failed
