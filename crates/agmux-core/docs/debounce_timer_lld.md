# LLD-STATE-DEBOUNCE-TIMERS-005: State Transition Settle & Debounce Timers

* **Unique ID:** `LLD-STATE-DEBOUNCE-TIMERS-005`
* **Upstream Requirement:** `REQ-STATE-DEBOUNCE-TIMERS-005`
* **Downstream Verification Test:** `TEST-VER-STATE-005` (`tests/e2e_multi_tier_classifier_test.rs`)
* **SW Component:** `crates/agmux-core::debounce_timer`

## 1. Overview
`DebounceTimer` enforces two configurable settle windows before committing heuristic state transitions:
- `idle_after_ms`: Silence duration required before transitioning an active session without child processes from `RUNNING`/`WORKING` to `IDLE`.
- `blocked_settle_ms`: Silence duration required after a prompt pattern is detected on the bottom rows before firing `WAITING_INPUT` notifications, preventing false positives while text is still streaming.
