---
id: TASK-12
title: 'Tracer Bullet: 5-Scope Review Diff Engine & Hunk Navigation'
status: To Do
assignee: []
created_date: '2026-10-29T00:48:00Z'
labels:
  - tracer-bullet
  - diff
  - review
  - git
milestone: m-1
dependencies:
  - TASK-10
references:
  - docs/specs/08_git_worktree_and_review_spec.md
priority: high
type: feature
ordinal: 12000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Computes diff hunks across 5 review scopes (Session Diff since start SHA, Full Uncommitted, Versus Target Branch, Last Commit, Staged). Enables unified and side-by-side hunk navigation for agent review mode.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Diff calculator computes file diffs and hunks across specified scope and base commit
- [ ] #2 CLI and IPC support querying diff for an active session
- [ ] #3 Correctly parses modified, added, and deleted files with line number mappings
- [ ] #4 Integration test verifies hunk computation against multi-commit repository fixture
<!-- AC:END -->
