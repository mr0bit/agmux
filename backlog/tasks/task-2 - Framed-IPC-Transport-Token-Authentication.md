---
id: TASK-2
title: Framed IPC Transport & Token Authentication
status: To Do
assignee: []
created_date: '2026-10-28T22:38:00Z'
labels:
  - ipc
  - security
  - proto
milestone: m-0
dependencies:
  - TASK-1
references:
  - docs/specs/02_ipc_transport_protocol_spec.md
priority: high
type: feature
ordinal: 2000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Implement length-delimited Bincode 2.0 frames over Unix Domain Sockets and Windows Named Pipes with 256-bit cookie token authentication (daemon.token).
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 Bincode 2.0 framed serialization and deserialization
- [ ] #2 daemon.token generation with 0600 permissions
- [ ] #3 Handshake validation rejecting invalid tokens
<!-- AC:END -->
