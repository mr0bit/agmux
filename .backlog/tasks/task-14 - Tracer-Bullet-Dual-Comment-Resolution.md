---
id: TASK-14
title: 'Tracer Bullet: Dual Comment Resolution (CLI & Automated Hunk Tracking)'
status: Done
assignee:
  - '@codex'
  - '@agy'
created_date: '2026-10-29T01:10:00Z'
labels:
  - tracer-bullet
  - review
  - resolution
  - comments
  - git
milestone: m-1
dependencies:
  - TASK-13
references:
  - docs/specs/08_git_worktree_and_review_spec.md
priority: medium
type: feature
ordinal: 14000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Implements `REQ-GIT-DUAL-RESOLUTION-005` dual comment resolution for agent code review: (1) explicit command `agmux review resolve <id>`, and (2) automated diff hunk tracking that marks comments resolved if their target file line was modified in a subsequent commit or turn.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Explicit CLI `agmux review resolve <id>` marks comment status as resolved in SQLite
- [x] #2 Automated comment tracker marks comments resolved if matching line or file was edited during the turn
- [x] #3 IPC messages for comment resolution integrated and handled by daemon
- [x] #4 Integration test asserts explicit resolution and automated tracking transitions
<!-- AC:END -->
