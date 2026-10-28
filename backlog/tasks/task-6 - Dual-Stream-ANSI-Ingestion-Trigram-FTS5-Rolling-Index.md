---
id: TASK-6
title: Dual-Stream ANSI Ingestion & Trigram FTS5 Rolling Index
status: To Do
assignee: []
created_date: '2026-10-28T22:38:00Z'
labels:
  - search
  - fts5
  - persistence
milestone: m-0
dependencies:
  - TASK-5
references:
  - docs/specs/06_storage_persistence_fts_spec.md
priority: high
type: feature
ordinal: 6000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Implement 250ms batch ANSI stripping and SQLite FTS5 trigram full-text search with rolling retention pruning (default 50,000 lines/session).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 250ms batch line ingestion into SQLite WAL
- [ ] #2 FTS5 trigram tokenized search under 5ms
- [ ] #3 Rolling FIFO retention pruning of older lines
<!-- AC:END -->
