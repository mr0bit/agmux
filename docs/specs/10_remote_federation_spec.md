# Software Requirements Specification: Remote Federation & Security

## 1. Overview
This specification details the SSH and Tailscale remote daemon federation (`C`), token authentication across tunnels, remote host session tree mounting, binary bootstrapping with SHA-256 verification, and self-healing diagnostics.

---

## 2. Requirements

### REQ-REMOTE-FEDERATION-SSH-001
* **Unique ID:** `REQ-REMOTE-FEDERATION-SSH-001`
* **Text:** As a user connecting to a remote development host over SSH (`~/.ssh/config` aware) or Tailscale (`C`), the system shall establish a bidirectional IPC bridge over the SSH stdio tunnel, mounting the remote host's groups and sessions as a foldable subtree within the local `agmux-tui`.
* **Criticality:** High
* **SW Component:** `crates/agmux-daemon::ssh_federation`
* **Implementation Status:** Not implemented
* **Verification Criteria:** Connect to a remote host via `agmux connect ssh://user@box`; verify that the remote host's sessions appear in the sidebar under a remote root node and terminal input/output flows seamlessly with under 50ms latency.
* **Downstream Req:** `LLD-REMOTE-FEDERATION-SSH-001`
* **Downstream Test:** `TEST-VER-REMOTE-001`
* **Test Status:** Failed

---

### REQ-REMOTE-BOOTSTRAP-PINNED-002
* **Unique ID:** `REQ-REMOTE-BOOTSTRAP-PINNED-002`
* **Text:** As a user connecting to a remote host that lacks `agmuxd`, the local client shall query the remote architecture and automatically download the matching target-triple release binary from official GitHub release assets, verify its cryptographic SHA-256 checksum, and deploy it to `~/.local/bin/agmuxd` before initiating the session.
* **Criticality:** High
* **SW Component:** `crates/agmux-cli::remote_bootstrapper`
* **Implementation Status:** Not implemented
* **Verification Criteria:** Initiate connection to a bare remote Linux aarch64 server with no `agmuxd` installed; verify that the client downloads the correct aarch64 release binary, validates SHA-256, copies to `~/.local/bin/agmuxd`, and connects cleanly.
* **Downstream Req:** `LLD-REMOTE-BOOTSTRAP-PINNED-002`
* **Downstream Test:** `TEST-VER-REMOTE-002`
* **Test Status:** Failed

---

### REQ-REMOTE-AUTH-TOKEN-003
* **Unique ID:** `REQ-REMOTE-AUTH-TOKEN-003`
* **Text:** As an operator running `agmuxd` across shared hosts or network interfaces, all incoming network connections over TCP or stdio proxies shall require authentication against a local cryptographically generated cookie token (`daemon.token`), rejecting any unauthenticated frames.
* **Criticality:** High
* **SW Component:** `crates/agmux-daemon::network_auth`
* **Implementation Status:** Not implemented
* **Verification Criteria:** Attempt to connect an unauthenticated TCP/Tailscale client to a listening remote `agmuxd`; verify immediate disconnect with an authentication failure logged.
* **Downstream Req:** `LLD-REMOTE-AUTH-TOKEN-003`
* **Downstream Test:** `TEST-VER-REMOTE-003`
* **Test Status:** Failed

---

### REQ-REMOTE-DOCTOR-REPAIR-004
* **Unique ID:** `REQ-REMOTE-DOCTOR-REPAIR-004`
* **Text:** As a user troubleshooting environmental or connectivity issues, invoking `agmux doctor` shall verify local PTY availability, permissions of `state.db` and `daemon.token`, detected CLI paths, and active remote tunnels, providing automated repair suggestions via `agmux daemon repair`.
* **Criticality:** Medium
* **SW Component:** `crates/agmux-cli::doctor`
* **Implementation Status:** Not implemented
* **Verification Criteria:** Run `agmux doctor` in a healthy environment and in a broken environment (corrupted token permissions); verify that `doctor` flags the permission failure and `agmux daemon repair` restores correct `0600` permissions.
* **Downstream Req:** `LLD-REMOTE-DOCTOR-REPAIR-004`
* **Downstream Test:** `TEST-VER-REMOTE-004`
* **Test Status:** Failed
