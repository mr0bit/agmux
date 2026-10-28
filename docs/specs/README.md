# Agmux Software Requirements Specifications (SWRS)

This directory contains the formal, traceable Software Requirements Specifications for **Agmux** in accordance with the project V-Cycle systems engineering model.

## Traceability Structure & Attributes

Every requirement in these specifications is atomic (implementable within a single software component/module) and possesses the following mandatory attributes:

| Attribute | Description | Permitted Values |
| :--- | :--- | :--- |
| **Unique ID** | Meaningful identifier prefixed by requirement domain | e.g., `REQ-DAEMON-SPAWN-001` |
| **Text** | Formal behavioral requirement written in user-story style | "The system shall..." / "The user shall..." |
| **Criticality** | Engineering and delivery priority | `High`, `Medium`, `Low` |
| **SW Component** | Target crate / module responsible for this requirement | e.g., `crates/agmux-daemon::supervisor` |
| **Implementation Status** | Current codebase implementation state | `Not implemented`, `Implemented`, `Verified` |
| **Verification Criteria** | Holistic, testable end-to-end / user-level acceptance test | Detailed test scenario |
| **Downstream Req** | Linkage to Low-Level Design (LLD) requirement in module docs | e.g., `LLD-DAEMON-SPAWN-001` |
| **Downstream Test** | Linkage to integration/verification test in `tests/` | e.g., `TEST-VER-DAEMON-001` |
| **Test Status** | Current execution status of the downstream test | `Passed`, `Failed` |

## Specification Suite Index

1. [`01_core_daemon_lifecycle_spec.md`](./01_core_daemon_lifecycle_spec.md): Background daemon auto-spawn, process group supervision (`setpgid`), graceful shutdown, signal propagation (SIGHUP/SIGTERM), and crash recovery.
2. [`02_ipc_transport_protocol_spec.md`](./02_ipc_transport_protocol_spec.md): Framed IPC transport (`agmux-proto`), cookie token authentication, strict single-writer SQLite IPC routing, subscription management, and zero-flicker screen synchronization frames.
3. [`03_pty_terminal_emulation_spec.md`](./03_pty_terminal_emulation_spec.md): Cross-platform PTY management (`portable-pty`), Win32 ConPTY & WSL2 bridging, in-memory VT100/ANSI screen grid, resize reflow, and plain shell sub-tabs.
4. [`04_state_machine_and_classifier_spec.md`](./04_state_machine_and_classifier_spec.md): Unified 7-state classification engine (`WORKING`, `WAITING`, `STUCK`, `FINISHED`, `IDLE`, `ERRORED`, `DEAD`), SIMD tail matching, settle debounce windows, and child process inspection.
5. [`05_cli_adapter_plugin_system_spec.md`](./05_cli_adapter_plugin_system_spec.md): Two-tier CLI adapter architecture (`AgentPlugin` Rust trait for 14 first-class agents + declarative `agents.toml`), tool-specific STUCK heuristics, and session UUID capture.
6. [`06_storage_persistence_fts_spec.md`](./06_storage_persistence_fts_spec.md): SQLite WAL single-writer database, embedded atomic schema migrations (`PRAGMA user_version`), rolling trigram FTS5 indexing with configurable retention, and zstd ANSI snapshots.
7. [`07_tui_viewport_command_palette_spec.md`](./07_tui_viewport_command_palette_spec.md): Ratatui TUI client, sidebar attention filtering, VS Code-style Command Palette (`Ctrl+Shift+P` / `F1` / `:`), Quick Prompt bar (`Space`), split comparison viewport, and cold-start onboarding wizard.
8. [`08_git_worktree_and_review_spec.md`](./08_git_worktree_and_review_spec.md): Automated Git worktree provisioning, `.worktreeinclude` copying, worktree removal safety gate (dirty status checks and `git stash` backup), interactive code review mode (`Ctrl+R`), and dual comment resolution.
9. [`09_multi_agent_orchestration_bus_spec.md`](./09_multi_agent_orchestration_bus_spec.md): Programmatic CLI/MCP orchestration, inter-agent message queuing with `WAITING` approval guards, automatic dead-session lock eviction, hierarchical file reservations, and event journal streaming (`agmux watch`).
10. [`10_remote_federation_spec.md`](./10_remote_federation_spec.md): SSH & Tailscale remote daemon federation (`C`), token authentication, remote host subtree mounting, GitHub release asset binary bootstrap with SHA-256 verification, and self-healing diagnostics.
