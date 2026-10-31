# Software Verification Test Report: MCP and deferred teardown

## Execution summary

- Date: 2026-10-30
- Baseline: `17308ca` plus uncommitted TASK-29 changes
- Verification: `cargo test --test e2e_mcp_and_teardown_test` — PASSED (1 test)
- Crate coverage: `agent_context_test`, `mcp_registration_test`, `teardown_storage_test`, and `teardown_test` — PASSED as part of the workspace suite.
- Live terminal evidence: [task-29_verification.png](../../docs/assets/screenshots/task-29_verification.png)

## Requirement verification matrix

| Requirement | Verification test | Result | Evidence |
| --- | --- | --- | --- |
| REQ-ORCH-AGENT-CONTEXT-006 | TEST-VER-ORCH-006 | Passed | CLI overview and task/lock references with usage and JSON examples, without a daemon dependency. |
| REQ-ORCH-MCP-SERVER-007 | TEST-VER-ORCH-007 | Passed | Real stdio server and daemon: initialization, tool discovery, all 16 tool routes, task and lock persistence, invalid input, missing session, notifications, and opt-in server mode. |
| REQ-ORCH-SELF-TEARDOWN-008 | TEST-VER-ORCH-008 | Passed | Schedule, inspect pending action, cancel, archive on stop hook, kill on stop and process exit, distinguish ARCHIVED from DEAD, retain snapshot, reject dead session; TUI crate test checks both badges and selected-session cancellation. |

The three verification IDs are exercised by `mcp_and_deferred_teardown` in `tests/e2e_mcp_and_teardown_test.rs`. The test starts the real daemon, sends CLI and MCP requests through authenticated framed IPC, and reads persisted final state for confirmation. The terminal screenshot is a live run against an isolated daemon.

Agent launch registration currently supports Claude and Codex; other MCP clients can connect using `agmux mcp` and configure that command themselves. Windows-specific child termination is implemented through portable-pty but was not exercised in this Linux run.

## Quality gates

| Command | Result |
| --- | --- |
| `cargo check` / `cargo check --workspace` | Passed |
| `cargo test` | Passed: 55 tests, 0 failed, 0 ignored |
| `cargo test --workspace` | Passed: 82 tests, 0 failed, 0 ignored |
| `cargo test --test e2e_mcp_and_teardown_test` | Passed: 1 test, 0 failed |
| `cargo clippy -- -D warnings` / `cargo clippy --workspace -- -D warnings` | Passed |
| `cargo fmt --check` | Passed |
| `node .soul/tools/v-cycle-checker.mjs` | Passed: 60 requirements, 0 schema violations |

The workspace and root suites passed before a subsequent path normalization change to opt-in agent registration; the registration crate test, workspace check, workspace Clippy and formatting were rerun after that change.

## Orchestrator verification

The orchestrator independently reviewed the implementation and live terminal screenshot, then reran `cargo check --workspace`, `cargo test --workspace` (82 passed, zero failures), `cargo clippy --workspace -- -D warnings`, `cargo fmt --check`, `git diff --check`, and the V-cycle checker successfully after the final registration change. The checker reports 55/60 requirements verified (91.7%) with zero schema violations. No commit was made.
