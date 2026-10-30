# LLD-IPC-SYNC-FRAME-004: Atomic ScreenSyncFrame Subscription Manager

* **Unique ID:** `LLD-IPC-SYNC-FRAME-004`
* **Upstream Requirement:** `REQ-IPC-SYNC-FRAME-004`
* **Downstream Verification Test:** `TEST-VER-IPC-004` (`tests/e2e_ipc_sync_broadcast_test.rs`)
* **SW Component:** `crates/agmux-daemon::subscription_manager`

## 1. Overview
`SubscriptionManager` coordinates client viewport subscriptions (`IpcMessage::SubscribeViewport { session_id }`) and constructs an atomic `ScreenSyncFrame` from the session's `ScreenBuffer` at the exact output boundary before streaming live `TerminalOutputFrame` deltas.

## 2. Atomic Snapshot Construction
When a client subscribes to `session_id`:
1. `SubscriptionManager::subscribe_session` locks the session's `ScreenBuffer` while simultaneously subscribing to `output_tx`.
2. Extracts `rows`, `cols`, `cursor_row`, `cursor_col`, `scrollback_offset`, `formatted_screen` (ANSI escape sequence restoration), and plain-text `lines`.
3. Immediately emits `IpcMessage::ScreenSyncFrame` followed by `IpcMessage::TerminalOutputFrame` deltas, guaranteeing zero-flicker reattachment.
