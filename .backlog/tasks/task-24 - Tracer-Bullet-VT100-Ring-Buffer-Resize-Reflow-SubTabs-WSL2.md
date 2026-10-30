---
id: TASK-24
title: 'Tracer Bullet: VT100 10k Ring Buffer, Terminal Resize Reflow, Sub-Tabs & WSL2 Bridge'
status: Done
assignee:
  - '@codex'
  - '@agy'
created_date: '2026-10-30T00:00:00Z'
labels:
  - tracer-bullet
  - pty
  - vt100
  - resize
  - subtabs
  - wsl2
milestone: m-2
dependencies:
  - TASK-23
references:
  - docs/specs/03_pty_terminal_emulation_spec.md
  - REQ-PTY-WSL2-BRIDGE-002
  - REQ-PTY-VT100-GRID-003
  - REQ-PTY-RESIZE-REFLOW-004
  - REQ-PTY-SHELL-SUBTAB-005
priority: high
type: feature
ordinal: 24000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Implements `REQ-PTY-WSL2-BRIDGE-002`, `REQ-PTY-VT100-GRID-003`, `REQ-PTY-RESIZE-REFLOW-004`, and `REQ-PTY-SHELL-SUBTAB-005`: WSL2 bridge command builder (`wsl.exe -d <distro> -- <cmd>`), 8 KB chunked VT100 parser with a 10,000-line scrollback ring buffer, terminal resize IPC and grid reflow (`ResizeSessionRequest`), and plain interactive shell sub-tabs inheriting parent working directory, worktree, `$PORT`, and `$AGMUX_SESSION_ID`.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 WSL2 bridge constructs command invocation and maps resize/IO events cleanly
- [x] #2 VT100 parser processes 8 KB chunks maintaining a 10,000-line scrollback ring buffer
- [x] #3 Resize handler updates both OS PTY dimensions and VT100 grid reflow
- [x] #4 Shell sub-tab spawner inherits worktree CWD, $PORT, and $AGMUX_SESSION_ID
<!-- AC:END -->
