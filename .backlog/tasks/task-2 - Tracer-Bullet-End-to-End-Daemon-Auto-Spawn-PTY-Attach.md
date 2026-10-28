---
id: TASK-2
title: 'Tracer Bullet: End-to-End Daemon Auto-Spawn & PTY Attach'
status: To Do
assignee: []
created_date: '2026-10-28T23:40:00Z'
labels:
  - tracer-bullet
  - pty
  - daemon
  - ipc
milestone: m-0
dependencies:
  - TASK-1
references:
  - docs/specs/01_core_daemon_lifecycle_spec.md
  - docs/specs/02_ipc_transport_protocol_spec.md
  - docs/specs/03_pty_terminal_emulation_spec.md
priority: high
type: feature
ordinal: 2000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Cut complete vertical path from CLI 'agmux run <command>' -> auto-spawning background daemon if not running -> framed IPC connection with token auth -> cross-platform PTY spawn (ConPTY on Windows / forkpty on Unix) -> streaming child output back to terminal.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 CLI auto-spawns daemon and negotiates framed IPC handshake with auth token
- [ ] #2 Daemon allocates PTY and executes child process
- [ ] #3 Child process stdout/stderr streams cleanly back to CLI terminal
- [ ] #4 Verification integration test passes in tests/e2e_spawn_test.rs
<!-- AC:END -->
