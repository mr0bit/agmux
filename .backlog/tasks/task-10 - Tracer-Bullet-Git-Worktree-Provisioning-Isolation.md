---
id: TASK-10
title: 'Tracer Bullet: Git Worktree Provisioning & .worktreeinclude Isolation'
status: Done
assignee: []
created_date: '2026-10-29T00:48:00Z'
updated_date: '2026-10-29T00:52:00Z'
labels:
  - tracer-bullet
  - git
  - worktree
  - isolation
milestone: m-1
dependencies:
  - TASK-9
references:
  - docs/specs/08_git_worktree_and_review_spec.md
priority: high
type: feature
ordinal: 10000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Creates isolated Git worktrees under `.agmux-worktrees/{branch-slug}` for agent sessions, copying untracked files matching `.worktreeinclude` (e.g. `.env`, local configs) and injecting deterministic $PORT offset into session environment.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 Worktree manager creates isolated worktree directory and checks out target branch
- [x] #2 Untracked files matching .worktreeinclude are copied automatically to the new worktree
- [x] #3 Session environment includes assigned PORT and target CWD pointing to the worktree
- [x] #4 Verification test creates worktree session, validates file inclusion and PTY spawn in worktree
<!-- AC:END -->
