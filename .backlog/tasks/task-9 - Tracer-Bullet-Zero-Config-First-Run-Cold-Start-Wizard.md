---
id: TASK-9
title: 'Tracer Bullet: Zero-Config First-Run Cold-Start Wizard'
status: Done
assignee: []
created_date: '2026-10-28T23:40:00Z'
updated_date: '2026-10-29T00:47:00Z'
labels:
  - tracer-bullet
  - wizard
  - onboarding
  - cli
milestone: m-0
dependencies:
  - TASK-8
references:
  - docs/specs/05_cli_adapter_plugin_system_spec.md
  - docs/specs/06_storage_persistence_fts_spec.md
priority: high
type: feature
ordinal: 9000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
If launched without existing configuration, agmux opens an interactive first-run onboarding wizard. Discovers installed AI CLIs (Codex, Antigravity, Claude, Aider), generates agmux.toml, verifies delegate fleet config, and spawns default workspace session.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 CLI/TUI detects absence of configuration and triggers first-run wizard
- [x] #2 Discovers installed AI coding agent CLIs in PATH
- [x] #3 Generates valid agmux.toml with default terminal keybindings
- [x] #4 Seamlessly transitions into default shell session without restart
<!-- AC:END -->
