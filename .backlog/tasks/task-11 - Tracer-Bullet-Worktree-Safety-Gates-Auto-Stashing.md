---
id: TASK-11
title: 'Tracer Bullet: Worktree Safety Gates & Auto-Stashing'
status: Done
assignee:
  - '@codex'
  - '@agy'
created_date: '2026-10-29T00:48:00Z'
labels:
  - tracer-bullet
  - git
  - safety
  - stash
milestone: m-1
dependencies:
  - TASK-10
references:
  - docs/specs/08_git_worktree_and_review_spec.md
priority: high
type: feature
ordinal: 11000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Prevents accidental loss of uncommitted agent code during session removal. Inspects worktree git status; if dirty or unpushed, blocks teardown unless --force is specified, creating an automatic backup git stash before pruning worktree.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Teardown inspects worktree status and rejects removal of dirty working tree without force
- [x] #2 Forced removal creates timestamped automatic git stash before pruning
- [x] #3 Pruned worktree cleans up OS directories and unregisters from git worktree list
- [x] #4 Integration test asserts dirty rejection, force stash creation, and clean removal
<!-- AC:END -->
