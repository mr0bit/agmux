---
id: TASK-9
title: Two-Tier AgentPlugin Architecture & STUCK Detection
status: To Do
assignee: []
created_date: '2026-10-28T22:39:00Z'
labels:
  - plugin
  - adapter
  - stuck
milestone: m-0
dependencies:
  - TASK-8
references:
  - docs/specs/05_cli_adapter_plugin_system_spec.md
priority: high
type: feature
ordinal: 9000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Implement zero-allocation trait AgentPlugin in Rust for 14 first-class agents with tool-specific STUCK heuristics (>10s held input) and declarative agents.toml.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 trait AgentPlugin definition in agmux-core
- [ ] #2 Built-in adapters for 14 standard CLI agents
- [ ] #3 Tool-specific prompt boundary and check_stuck heuristic
- [ ] #4 Declarative agents.toml custom regex parser
<!-- AC:END -->
