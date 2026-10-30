# LLD-PTY-SHELL-SUBTAB-005: Interactive Shell Sub-Tab Spawner

* **Unique ID:** `LLD-PTY-SHELL-SUBTAB-005`
* **Upstream Requirement:** `REQ-PTY-SHELL-SUBTAB-005`
* **Downstream Verification Test:** `TEST-VER-PTY-005` (`tests/e2e_pty_emulation_test.rs`)
* **SW Component:** `crates/agmux-core::subtab_spawner`

## 1. Overview
`SubtabSpawner` spawns plain interactive shell sub-tabs (`SpawnSubtabRequest`) attached to a parent agent session, inheriting the parent session's isolated Git worktree directory (`cwd`) and deterministic `PORT`, `AGMUX_SESSION_ID`, and `AGMUX_PARENT_SESSION_ID` environment variables.
