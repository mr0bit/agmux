---
id: TASK-1
title: Workspace Scaffolding & Cargo Crates Setup
status: To Do
assignee: []
created_date: '2026-10-28T22:38:00Z'
labels:
  - workspace
  - setup
milestone: m-0
dependencies: []
references:
  - docs/archi/static-architecture.md
  - docs/archi/module-list.md
priority: high
type: chore
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Initialize root Cargo.toml workspace and the 5 member crates: agmux-proto, agmux-core, agmux-daemon, agmux-tui, and agmux-cli.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Root Cargo.toml declares workspace members
- [ ] #2 cargo check compiles all 5 skeleton crates cleanly
<!-- AC:END -->
