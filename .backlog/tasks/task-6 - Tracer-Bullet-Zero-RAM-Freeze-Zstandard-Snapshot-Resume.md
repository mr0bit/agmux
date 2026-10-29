---
id: TASK-6
title: 'Tracer Bullet: Zero-RAM Freeze & Zstandard Snapshot Resume'
status: Done
assignee: []
created_date: '2026-10-28T23:40:00Z'
updated_date: '2026-10-29T00:31:00Z'
labels:
  - tracer-bullet
  - zstd
  - freeze
  - performance
milestone: m-0
dependencies:
  - TASK-5
references:
  - docs/specs/01_core_daemon_lifecycle_spec.md
  - docs/specs/06_storage_persistence_fts_spec.md
priority: high
type: feature
ordinal: 6000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
When child process exits or is suspended, daemon serializes VT100 screen grid, compresses snapshot with Zstandard (level 3) into SQLite, closes OS PTY handles, freeing memory to 0 MB RAM. When resumed, snapshot is decompressed and restored to viewport.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Inactive/dead session snapshot compressed via Zstd into SQLite
- [x] #2 Dead session holds exactly 0 MB resident RAM (PTY descriptor dropped)
- [x] #3 Auto-resume or inspection seamlessly decompresses and renders frozen grid
- [x] #4 Memory leak verification test confirms 0 MB heap retention for frozen sessions
<!-- AC:END -->
