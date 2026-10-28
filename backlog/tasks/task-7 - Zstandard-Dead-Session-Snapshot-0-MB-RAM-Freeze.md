---
id: TASK-7
title: Zstandard Dead-Session Snapshot & 0 MB RAM Freeze
status: To Do
assignee: []
created_date: '2026-10-28T22:38:00Z'
labels:
  - zstd
  - compression
  - snapshot
milestone: m-0
dependencies:
  - TASK-4
  - TASK-5
references:
  - docs/specs/06_storage_persistence_fts_spec.md
priority: high
type: feature
ordinal: 7000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Implement session freeze (x -> DEAD) compressing 10,000-line ANSI scrollback with zstd level 3 into SQLite blob, dropping parser and PTY to 0 MB RAM.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 zstd level 3 compression of ANSI buffer under 60 KB
- [ ] #2 Full drop of vt100::Parser and PTY master on kill
- [ ] #3 Decompression stream for read-only viewport inspection
<!-- AC:END -->
