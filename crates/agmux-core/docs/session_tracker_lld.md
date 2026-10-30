# Low-Level Design: Native Session UUID Capture & Persistence (`session_tracker`)

* **LLD ID:** `LLD-PLUGIN-UUID-CAPTURE-005`
* **Upstream Requirement:** `REQ-PLUGIN-UUID-CAPTURE-005`
* **Module:** `crates/agmux-core::session_tracker`
* **Verification Test:** `TEST-VER-PLUGIN-005`

## 1. Design Overview
`session_tracker` captures native CLI conversation UUIDs via either pre-minted UUID flag injection (`--session-id <uuid>`) or filesystem/stdout store inspection (`session_store`) and persists the UUID in SQLite (`sessions.cli_session_uuid`).
