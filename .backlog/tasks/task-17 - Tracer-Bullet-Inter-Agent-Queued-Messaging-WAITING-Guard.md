---
id: TASK-17
title: 'Tracer Bullet: Inter-Agent Queued Messaging & WAITING Safety Guard'
status: Done
assignee:
  - '@codex'
  - '@agy'
created_date: '2026-10-29T01:25:00Z'
labels:
  - tracer-bullet
  - orchestration
  - messaging
  - queue
milestone: m-2
dependencies:
  - TASK-5
references:
  - docs/specs/09_multi_agent_orchestration_bus_spec.md
priority: high
type: feature
ordinal: 17000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Implements `REQ-ORCH-QUEUED-MSG-GUARD-001`: as asynchronous messages are dispatched between agents (`agmux send <target_id> <message>`), holds the message in queue if target session is in `WAITING` state on a permission prompt. Automatically delivers queued messages to PTY stdin once user resolves prompt (`IDLE`/`FINISHED`).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 CLI command `agmux send <target_id> <message>` sends inter-agent message over IPC
- [x] #2 Messages targeted at `WAITING` sessions are queued without writing to PTY stdin
- [x] #3 Target session state transition to `IDLE`/`FINISHED` flushes queued messages in FIFO order to PTY
- [x] #4 Integration test asserts message queuing under WAITING and delivery on prompt resolution
<!-- AC:END -->
