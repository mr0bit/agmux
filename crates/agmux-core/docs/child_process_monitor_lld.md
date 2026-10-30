# LLD-STATE-TIER2-CHILDPID-003: Tier 2 Child Process Tree Monitor

* **Unique ID:** `LLD-STATE-TIER2-CHILDPID-003`
* **Upstream Requirement:** `REQ-STATE-TIER2-CHILDPID-003`
* **Downstream Verification Test:** `TEST-VER-STATE-003` (`tests/e2e_multi_tier_classifier_test.rs`)
* **SW Component:** `crates/agmux-core::child_process_monitor`

## 1. Overview
`ChildProcessMonitor` inspects an agent session's OS child process tree (`/proc/<pid>/task/*/children` and `/proc/<pid>/stat` on Linux, filtering out Zombie `'Z'` and Dead `'X'` processes; `pgrep -P` on macOS; `tasklist` on Windows). While live child processes exist, session state remains locked in `RUNNING`/`WORKING` even during prolonged output silence.
