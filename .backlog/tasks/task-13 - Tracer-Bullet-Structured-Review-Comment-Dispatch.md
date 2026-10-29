---
id: TASK-13
title: 'Tracer Bullet: Structured Review Comment Round Dispatch'
status: Done
assignee:
  - '@codex'
  - '@agy'
created_date: '2026-10-29T00:48:00Z'
labels:
  - tracer-bullet
  - review
  - comments
  - pty
milestone: m-1
dependencies:
  - TASK-12
references:
  - docs/specs/08_git_worktree_and_review_spec.md
priority: high
type: feature
ordinal: 13000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Stores draft review comments on specific file lines. When user triggers submission ('C'), flattens all pending draft comments into a numbered structured prompt round with stable IDs (e.g. [ID: rev_01]) and dispatches payload directly into the agent's PTY.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Stores review comments with file path, line number, and content in SQLite
- [x] #2 Flattens draft comments into formatted prompt payload with stable comment IDs
- [x] #3 Dispatches review round into agent PTY and transitions session to WORKING state
- [x] #4 Integration test asserts comment staging, flattening, and PTY delivery
<!-- AC:END -->
