# Low-Level Design: SSH & Tailscale Remote Federation (`ssh_federation`)

* **LLD ID:** `LLD-REMOTE-FEDERATION-SSH-001`
* **Upstream Requirement:** `REQ-REMOTE-FEDERATION-SSH-001`
* **Module:** `crates/agmux-daemon::ssh_federation`
* **Verification Test:** `TEST-VER-REMOTE-001`

## 1. Design Overview
`SshFederationManager` establishes a bidirectional framed IPC bridge to a remote `agmuxd` instance (over an SSH stdio tunnel or forwarded local socket), authenticates via `HandshakeRequest`, mounts the remote host's sessions under `[<host>] <session_id>`, and forwards `TerminalInputFrame` / `TerminalOutputFrame` messages across the bridge with < 50ms latency.
