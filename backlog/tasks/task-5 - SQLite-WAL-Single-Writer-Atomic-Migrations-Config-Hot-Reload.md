---
id: TASK-5
title: SQLite WAL Single-Writer, Atomic Migrations & Config Hot-Reload
status: To Do
assignee: []
created_date: '2026-10-28T22:38:00Z'
labels:
  - sqlite
  - storage
  - migrations
milestone: m-0
dependencies:
  - TASK-1
references:
  - docs/specs/06_storage_persistence_fts_spec.md
priority: high
type: feature
ordinal: 5000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Implement single-writer SQLite WAL storage engine with embedded atomic migrations (PRAGMA user_version) and notify crate config.toml watcher.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Single-writer SQLite WAL database setup
- [ ] #2 PRAGMA user_version atomic schema migrations
- [ ] #3 config.toml hot-reload without daemon restart
<!-- AC:END -->
