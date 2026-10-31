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
