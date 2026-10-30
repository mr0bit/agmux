# LLD-IPC-EVENT-DISPATCH-005: Asynchronous Push Event Broadcaster

* **Unique ID:** `LLD-IPC-EVENT-DISPATCH-005`
* **Upstream Requirement:** `REQ-IPC-EVENT-DISPATCH-005`
* **Downstream Verification Test:** `TEST-VER-IPC-005` (`tests/e2e_ipc_sync_broadcast_test.rs`)
* **SW Component:** `crates/agmux-daemon::event_broadcaster`

## 1. Overview
`EventBroadcaster` dispatches asynchronous push notifications (`IpcMessage::NotificationEvent` and `IpcMessage::SessionListUpdate`) to connected IPC clients whenever session lifecycle or state transitions occur, eliminating client-side polling loops.

## 2. Broadcast Channels & Triggers
- On session spawn, state transition (`RUNNING` ➔ `WAITING_INPUT` / `STUCK` / `COMPLETED` / `FAILED` / `IDLE`), freeze, or removal:
  - Queries the updated active session list from `StorageEngine` and broadcasts `IpcMessage::SessionListUpdate`.
  - Emits `IpcMessage::NotificationEvent` describing the transition (`session_id`, `kind`, `title`, `body`, `timestamp`).
