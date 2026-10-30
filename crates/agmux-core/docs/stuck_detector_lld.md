# Low-Level Design: Tool-Specific Stuck Prompt Heuristic (`stuck_detector`)

* **LLD ID:** `LLD-PLUGIN-STUCK-HEURISTIC-002`
* **Upstream Requirement:** `REQ-PLUGIN-STUCK-HEURISTIC-002`
* **Module:** `crates/agmux-core::stuck_detector`
* **Verification Test:** `TEST-VER-PLUGIN-002`

## 1. Design Overview
`stuck_detector` evaluates whether unsubmitted text sits after an agent's prompt prefix on the bottom visible screen rows for `>= stuck_timeout_ms` (default `10_000` ms) with `>= stuck_timeout_ms` of PTY output silence and no active child processes, transitioning the session state to `STUCK`.
