---
id: TASK-1
title: 'Foundation: Workspace Scaffolding & Cargo Crates Setup'
status: Done
assignee: []
created_date: '2026-10-28T23:40:00Z'
updated_date: '2026-10-28T23:59:00Z'
labels:
  - workspace
  - setup
milestone: m-0
dependencies: []
references:
  - docs/archi/static-architecture.md
  - docs/archi/module-list.md
  - docs/specs/01_core_daemon_lifecycle_spec.md
priority: high
type: chore
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Initialize root Cargo workspace with the 5 crates (agmux-proto, agmux-core, agmux-daemon, agmux-tui, agmux-cli), common error boundaries, and establish compiler CI gate.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Root Cargo.toml declares workspace members and Rust 2024 edition
- [x] #2 cargo check compiles all 5 skeleton crates cleanly with zero warnings
- [x] #3 docs/archi/module-list.md matches initial crate root structures
<!-- AC:END -->
