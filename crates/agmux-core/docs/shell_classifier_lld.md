# LLD-STATE-SHELL-TRACKING-006: Dynamic Shell Sub-Tab Activity Classifier

* **Unique ID:** `LLD-STATE-SHELL-TRACKING-006`
* **Upstream Requirement:** `REQ-STATE-SHELL-TRACKING-006`
* **Downstream Verification Test:** `TEST-VER-STATE-006` (`tests/e2e_multi_tier_classifier_test.rs`)
* **SW Component:** `crates/agmux-core::shell_classifier`

## 1. Overview
`ShellClassifier` monitors plain interactive shell sub-tabs (`❯`) via `ChildProcessMonitor`. When the shell spawns a foreground command (e.g. `cargo build` or `sleep`), the sub-tab state transitions to `RUNNING`/`WORKING`; when all child processes exit and control returns to the shell prompt, the sub-tab transitions back to `IDLE`.
