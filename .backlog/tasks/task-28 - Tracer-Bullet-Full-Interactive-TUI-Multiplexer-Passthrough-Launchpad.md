---
id: TASK-28
title: 'Tracer Bullet: Full Interactive TUI Multiplexer (PTY Keyboard Passthrough, Sub-Tabs, Agent Launchpad & Kanban Overlay)'
status: Done
assignee:
  - '@codex'
  - '@agy'
created_date: '2026-10-30T00:00:00Z'
labels:
  - tracer-bullet
  - tui
  - multiplexer
  - interactive
  - launchpad
milestone: m-3
dependencies:
  - TASK-27
references:
  - docs/specs/07_tui_viewport_command_palette_spec.md
  - docs/specs/03_pty_terminal_emulation_spec.md
  - docs/specs/05_cli_adapter_plugin_system_spec.md
  - docs/specs/09_multi_agent_orchestration_bus_spec.md
priority: high
type: feature
ordinal: 28000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Upgrades `agmux-tui` into a complete interactive terminal multiplexer: bidirectional Interactive PTY Passthrough Mode (`i` / `Enter` to type directly into the active shell or agent with `Esc` returning to Normal navigation mode), platform-aware default shell detection (`powershell.exe` / `cmd.exe` on Windows, `$SHELL` on Unix), rich empty-state Welcome & Agent Launchpad dashboard showing all 14 supported agents with missing-CLI install guidance, horizontal sub-tabs bar (`t` to open shell sub-tab), and live Kanban / status bar telemetry.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Pressing `i` or `Enter` enters Interactive PTY Passthrough Mode forwarding keystrokes to the active session; `Esc` exits back to Normal Mode
- [x] #2 Platform-aware default shell spawns `powershell.exe` / `cmd.exe` on Windows and `$SHELL` / `bash` on Unix
- [x] #3 Empty-state viewport renders an informative Agent Launchpad & Quick-Start guide rather than a blank box
- [x] #4 Horizontal sub-tab indicator and Kanban board modal (`K`) are integrated into the TUI
<!-- AC:END -->
