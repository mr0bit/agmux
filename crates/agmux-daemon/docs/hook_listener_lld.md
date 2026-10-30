# LLD-STATE-TIER1-HOOKS-002: Tier 1 Explicit IPC Hook Listener

* **Unique ID:** `LLD-STATE-TIER1-HOOKS-002`
* **Upstream Requirement:** `REQ-STATE-TIER1-HOOKS-002`
* **Downstream Verification Test:** `TEST-VER-STATE-002` (`tests/e2e_multi_tier_classifier_test.rs`)
* **SW Component:** `crates/agmux-daemon::hook_listener`

## 1. Overview
`HookListener` processes Tier 1 explicit lifecycle hooks (`agmux internal-hook --session <id> --event {start|stop|wait|error}`) with zero latency (< 10ms), overriding heuristic classifiers and broadcasting state transitions immediately.
