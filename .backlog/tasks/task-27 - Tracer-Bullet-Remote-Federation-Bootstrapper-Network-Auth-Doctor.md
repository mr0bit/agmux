---
id: TASK-27
title: 'Tracer Bullet: Remote SSH/Tailscale Federation, Pinned Binary Bootstrapper, Network Auth & Doctor Repair'
status: Done
assignee:
  - '@codex'
  - '@agy'
created_date: '2026-10-30T00:00:00Z'
labels:
  - tracer-bullet
  - federation
  - ssh
  - bootstrap
  - doctor
milestone: m-3
dependencies:
  - TASK-26
references:
  - docs/specs/10_remote_federation_spec.md
  - REQ-REMOTE-FEDERATION-SSH-001
  - REQ-REMOTE-BOOTSTRAP-PINNED-002
  - REQ-REMOTE-AUTH-TOKEN-003
  - REQ-REMOTE-DOCTOR-REPAIR-004
priority: high
type: feature
ordinal: 27000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Implements `REQ-REMOTE-FEDERATION-SSH-001`, `REQ-REMOTE-BOOTSTRAP-PINNED-002`, `REQ-REMOTE-AUTH-TOKEN-003`, and `REQ-REMOTE-DOCTOR-REPAIR-004`: bidirectional SSH stdio proxy and Tailscale federation mounting remote host sessions (`agmux connect`), pinned GitHub release binary bootstrapper with SHA-256 checksum validation, cookie token handshake verification on remote network transports, and `agmux doctor` / `agmux daemon repair` diagnostics and auto-remediation.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 SSH/Tailscale federation mounts remote host sessions and forwards I/O frames
- [x] #2 Remote bootstrapper resolves target triple, verifies SHA-256 checksum, and installs binary
- [x] #3 Network auth validates daemon.token cookie on remote/stdio connections and rejects invalid tokens
- [x] #4 CLI `agmux doctor` and `agmux daemon repair` diagnose and repair token/db permissions and environment
<!-- AC:END -->
