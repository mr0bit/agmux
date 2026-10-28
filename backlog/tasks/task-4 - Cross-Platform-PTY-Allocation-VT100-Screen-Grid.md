---
id: TASK-4
title: Cross-Platform PTY Allocation & VT100 Screen Grid
status: To Do
assignee: []
created_date: '2026-10-28T22:38:00Z'
labels:
  - pty
  - vt100
  - terminal
milestone: m-0
dependencies:
  - TASK-1
references:
  - docs/specs/03_pty_terminal_emulation_spec.md
priority: high
type: feature
ordinal: 4000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Implement cross-platform pseudo-terminal allocation via portable-pty, WSL2 bridging, in-memory 10,000-line VT100 parser, and TIOCSWINSZ window resizing.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 POSIX openpty and Win32 ConPTY support
- [ ] #2 In-memory 10,000-line scrollback buffer
- [ ] #3 Dynamic resize reflow with TIOCSWINSZ
<!-- AC:END -->
