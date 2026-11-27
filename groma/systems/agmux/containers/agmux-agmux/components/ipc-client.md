---
type: C4 Component
title: Ipc client
status: stable
groma:
  id: ipc-client
  parent: agmux-agmux
  code:
    - scanner: rust
      file: crates/agmux-cli/src/ipc_client.rs
---

## Relationships

| Source | Target | Description | Technology |
| --- | --- | --- | --- |
| [crates/agmux-cli/src/ipc_client.rs](../../../../../../crates/agmux-cli/src/ipc_client.rs) | [crates/agmux-daemon/src/main.rs](../../../../../../crates/agmux-daemon/src/main.rs) | Connects to daemon | Authenticated framed IPC |
