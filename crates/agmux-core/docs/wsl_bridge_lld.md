# LLD-PTY-WSL2-BRIDGE-002: Windows Subsystem for Linux (WSL2) PTY Bridge

* **Unique ID:** `LLD-PTY-WSL2-BRIDGE-002`
* **Upstream Requirement:** `REQ-PTY-WSL2-BRIDGE-002`
* **Downstream Verification Test:** `TEST-VER-PTY-002` (`tests/e2e_pty_emulation_test.rs`)
* **SW Component:** `crates/agmux-core::wsl_bridge`

## 1. Overview
`WslBridge` translates Windows drive paths (`C:\Users\dev\repo`) into WSL2 mount paths (`/mnt/c/Users/dev/repo`) and constructs `wsl.exe -d <distro> --cd <wsl_path> -- <command> <args...>` invocations so Linux agent CLIs execute seamlessly inside a native ConPTY/PTY session.
