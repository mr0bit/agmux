---
type: C4 Component
title: Recovery manager
status: stable
groma:
  id: recovery-manager
  parent: agmux-daemon
  code:
    - scanner: rust
      file: crates/agmux-daemon/src/recovery_manager.rs
      symbol: recover_orphaned_sessions
---
