---
id: TASK-26
title: 'Tracer Bullet: Two-Tier Agent Plugin System (14 Built-in Adapters, Declarative TOML, Stuck Heuristics & UUID Capture)'
status: Done
assignee:
  - '@codex'
  - '@agy'
created_date: '2026-10-30T00:00:00Z'
labels:
  - tracer-bullet
  - plugins
  - adapters
  - stuck
  - uuid
milestone: m-2
dependencies:
  - TASK-25
references:
  - docs/specs/05_cli_adapter_plugin_system_spec.md
  - REQ-PLUGIN-TRAIT-001
  - REQ-PLUGIN-STUCK-HEURISTIC-002
  - REQ-PLUGIN-BUILTIN-ADAPTERS-003
  - REQ-PLUGIN-DECLARATIVE-TOML-004
  - REQ-PLUGIN-UUID-CAPTURE-005
  - REQ-PLUGIN-MISSING-DETECTOR-006
priority: high
type: feature
ordinal: 26000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Implements `REQ-PLUGIN-TRAIT-001`, `REQ-PLUGIN-STUCK-HEURISTIC-002`, `REQ-PLUGIN-BUILTIN-ADAPTERS-003`, `REQ-PLUGIN-DECLARATIVE-TOML-004`, `REQ-PLUGIN-UUID-CAPTURE-005`, and `REQ-PLUGIN-MISSING-DETECTOR-006`: zero-overhead `AgentPlugin: Send + Sync` trait, tool-specific `check_stuck` prompt heuristics (>10s), compiled-in adapters for all 14 first-class CLI tools (claude, agy, gemini, codex, opencode, aider, cursor-agent, grok, q, pi, omp, cmd, hermes, muse), declarative `agents.toml` parser, automatic UUID capture (`session_id_flag` and store watchers), and missing CLI binary detector with install instructions.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 AgentPlugin trait supports spawn args, resume args, stuck check, and ANSI state parsing
- [x] #2 Built-in adapters for all 14 first-class CLI tools with Windows cmd collision guard
- [x] #3 Declarative agents.toml loader registers custom user agent profiles dynamically
- [x] #4 Session tracker captures conversation UUIDs via flag injection and store watchers
- [x] #5 Missing CLI detector returns official installation banner and link when binary is absent
<!-- AC:END -->
