# Software Requirements Specification: IPC Transport & Protocol

## 1. Overview
This specification defines the framed inter-process communication protocol (`agmux-proto`), token authentication, single-writer IPC router, and viewport synchronization frame mechanisms between client interfaces and `agmuxd`.

---

## 2. Requirements

### REQ-IPC-FRAMING-001
* **Unique ID:** `REQ-IPC-FRAMING-001`
* **Text:** As a client communicating with `agmuxd`, the system shall transmit length-delimited binary frames serialized with Bincode 2.0 over native local transports (Unix Domain Sockets on Linux/macOS/WSL2 at `/run/user/$UID/agmuxd.sock`, Named Pipes on Windows at `\\.\pipe\agmuxd-%USERNAME%`).
* **Criticality:** High
* **SW Component:** `crates/agmux-proto::transport`
* **Implementation Status:** Verified
* **Verification Criteria:** Benchmark and verify bidirectional packet transmission across 10,000 requests; ensure 100% frame integrity, zero message truncation, and serialization overhead under 50 microseconds per frame.
* **Downstream Req:** `LLD-IPC-FRAMING-001`
* **Downstream Test:** `TEST-VER-IPC-001`
* **Test Status:** Passed

---

### REQ-IPC-TOKEN-AUTH-002
* **Unique ID:** `REQ-IPC-TOKEN-AUTH-002`
* **Text:** As a security-conscious multi-user system operator, `agmuxd` shall generate a cryptographically secure 256-bit token stored in user-restricted storage (`~/.local/share/agmux/daemon.token` with mode `0600`) and require every connecting client to complete a handshake validation before processing any command frames.
* **Criticality:** High
* **SW Component:** `crates/agmux-proto::auth`
* **Implementation Status:** Verified
* **Verification Criteria:** Connect to the IPC socket with an invalid or absent authentication token. Verify that `agmuxd` immediately closes the connection without processing any input and logs an unauthorized connection attempt.
* **Downstream Req:** `LLD-IPC-TOKEN-AUTH-002`
* **Downstream Test:** `TEST-VER-IPC-002`
* **Test Status:** Passed

---

### REQ-IPC-SINGLE-WRITER-003
* **Unique ID:** `REQ-IPC-SINGLE-WRITER-003`
* **Text:** As a CLI subcommand invocation (`agmux task`, `agmux lock`, `agmux report`, `agmux reserve`), the system shall execute state operations exclusively by sending typed IPC requests to `agmuxd` over the socket, never opening or writing to `state.db` directly.
* **Criticality:** High
* **SW Component:** `crates/agmux-cli::ipc_client`
* **Implementation Status:** Verified
* **Verification Criteria:** Execute 50 concurrent CLI commands while `agmuxd` is actively flushing FTS batches. Verify zero `SQLITE_BUSY` database lock errors and complete synchronization of event journals across all invocations.
* **Downstream Req:** `LLD-IPC-SINGLE-WRITER-003`
* **Downstream Test:** `TEST-VER-IPC-003`
* **Test Status:** Passed

---

### REQ-IPC-SYNC-FRAME-004
* **Unique ID:** `REQ-IPC-SYNC-FRAME-004`
* **Text:** As a TUI or GUI frontend subscribing to a session viewport (`SubscribeViewport`), `agmuxd` shall immediately return an atomic `ScreenSyncFrame` containing the entire active VT100 character and attribute matrix, cursor position, and scrollback offset, guaranteeing zero-flicker reattachment.
* **Criticality:** High
* **SW Component:** `crates/agmux-daemon::subscription_manager`
* **Implementation Status:** Verified
* **Verification Criteria:** Detach the TUI while an agent is generating continuous fast output; re-attach after 10 seconds. Verify that the TUI renders the exact latest screen grid instantly within a single render cycle (<16ms) without missing or garbled characters.
* **Downstream Req:** `LLD-IPC-SYNC-FRAME-004`
* **Downstream Test:** `TEST-VER-IPC-004`
* **Test Status:** Passed

---

### REQ-IPC-EVENT-DISPATCH-005
* **Unique ID:** `REQ-IPC-EVENT-DISPATCH-005`
* **Text:** As an attached frontend client, the system shall receive asynchronous `NotificationEvent` and `SessionListUpdate` push messages over the IPC stream whenever session state transitions occur, without requiring client polling loops.
* **Criticality:** Medium
* **SW Component:** `crates/agmux-daemon::event_broadcaster`
* **Implementation Status:** Verified
* **Verification Criteria:** Trigger a session state transition in the daemon via background timer; verify that subscribed clients receive the notification event within 1ms and update their visual state badges without issuing pull requests.
* **Downstream Req:** `LLD-IPC-EVENT-DISPATCH-005`
* **Downstream Test:** `TEST-VER-IPC-005`
* **Test Status:** Passed
