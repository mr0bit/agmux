# Low-Level Design: Environment Diagnostics & Self-Repair (`doctor`)

* **LLD ID:** `LLD-REMOTE-DOCTOR-REPAIR-004`
* **Upstream Requirement:** `REQ-REMOTE-DOCTOR-REPAIR-004`
* **Module:** `crates/agmux-cli::doctor`
* **Verification Test:** `TEST-VER-REMOTE-004`

## 1. Design Overview
`doctor` inspects local PTY availability, `daemon.token` (`0600` on Unix) and `state.db` permissions, discovered agent CLIs, and active remote tunnels (`agmux doctor`), and repairs broken file permissions and stale lockfiles when invoked via `agmux daemon repair`.
