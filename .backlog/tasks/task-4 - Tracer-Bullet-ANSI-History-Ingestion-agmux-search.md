---
id: TASK-4
title: 'Tracer Bullet: ANSI History Ingestion & agmux search'
status: To Do
assignee: []
created_date: '2026-10-28T23:40:00Z'
labels:
  - tracer-bullet
  - search
  - fts5
  - ansi
milestone: m-0
dependencies:
  - TASK-3
references:
  - docs/specs/03_pty_terminal_emulation_spec.md
  - docs/specs/06_storage_persistence_fts_spec.md
priority: high
type: feature
ordinal: 4000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Daemon splits PTY byte stream into raw ANSI buffer and stripped plaintext buffer, indexing stripped text into SQLite FTS5 with trigram tokenization. User executes 'agmux search <query>' to query past session output across all active/historical sessions.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Dual ring-buffer ingests PTY output without blocking terminal throughput
- [ ] #2 SQLite FTS5 table indexes terminal text with trigram tokenizer
- [ ] #3 CLI agmux search returns matching lines with timestamps and session context
- [ ] #4 Automated benchmark verifies zero-allocation hot loop under 50 MB/s burst
<!-- AC:END -->
