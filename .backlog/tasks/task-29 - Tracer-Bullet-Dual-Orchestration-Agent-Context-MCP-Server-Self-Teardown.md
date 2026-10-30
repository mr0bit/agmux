---
id: TASK-29
title: >-
  Tracer Bullet: Dual Orchestration Interface (Agent-Context CLI Docs, Opt-In
  MCP Server & Self-Teardown)
status: Done
assignee:
  - '@codex'
created_date: '2026-10-30T20:30:00Z'
updated_date: '2026-10-30T19:01:00Z'
labels:
  - tracer-bullet
  - orchestration
  - mcp
  - cli
milestone: m-2
dependencies:
  - TASK-28
references:
  - docs/specs/09_multi_agent_orchestration_bus_spec.md
priority: high
type: feature
ordinal: 29000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Completes the Phase 2 Dual Orchestration Interface (`FR-P2-01`, `FR-P2-02`, `FR-P2-04`) across `agmux-cli`, `agmux-proto`, and `agmux-daemon`:
1. `REQ-ORCH-AGENT-CONTEXT-006` (`crates/agmux-cli::agent_context`): Progressive CLI documentation via `agmux agent-context` (compact ~200-token command overview) and `agmux agent-context --command <name>` (detailed per-command reference with JSON/flag examples).
2. `REQ-ORCH-MCP-SERVER-007` (`crates/agmux-cli::mcp_server`): Built-in JSON-RPC 2.0 stdio Model Context Protocol server (`agmux mcp` and `--enable-mcp` flag) exposing orchestration tools (`list_sessions`, `spawn_session`, `send_message`, `read_session`, `task_list`, `task_create`, `task_claim`, `task_finish`, `report_submit`, `lock_acquire`, `lock_release`, `reserve_files`, `release_files`, `resolve_review_comment`, `archive_self`, `kill_self`) backed exclusively by framed IPC to `agmuxd`.
3. `REQ-ORCH-SELF-TEARDOWN-008` (`crates/agmux-daemon::teardown_scheduler`): Self-teardown scheduling (`agmux archive-self <session_id>`, `agmux kill-self <session_id>`, `agmux cancel-teardown <session_id>`) marking sessions with `↓` (archive on finish) or `■` (kill/freeze on finish) and automatically executing teardown when the current turn transitions to `FINISHED` or `IDLE`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 `agmux agent-context` prints a compact (<250 tokens) overview of all subcommands and `agmux agent-context --command <name>` prints detailed usage & JSON examples
- [x] #2 `agmux mcp` handles JSON-RPC 2.0 `initialize`, `tools/list`, and `tools/call` over stdio, routing tool invocations to `agmuxd` via IPC
- [x] #3 `agmux archive-self`, `agmux kill-self`, and `agmux cancel-teardown` schedule/cancel automatic teardown (`↓` / `■` markers) executed when the session turn finishes
- [x] #4 Level 3 LLDs (`agent_context_lld.md`, `mcp_server_lld.md`, `teardown_scheduler_lld.md`) and Level 5 SVT (`tests/e2e_mcp_and_teardown_test.rs`) pass
<!-- AC:END -->
