# LLD-PTY-RESIZE-REFLOW-004: Dynamic Terminal Resize & Grid Reflow

* **Unique ID:** `LLD-PTY-RESIZE-REFLOW-004`
* **Upstream Requirement:** `REQ-PTY-RESIZE-REFLOW-004`
* **Downstream Verification Test:** `TEST-VER-PTY-004` (`tests/e2e_pty_emulation_test.rs`)
* **SW Component:** `crates/agmux-core::pty_resizer`

## 1. Overview
`PtyResizer` coordinates terminal resize events (`ResizeSessionRequest { session_id, cols, rows }`) across both the OS pseudo-terminal master (`portable_pty::MasterPty::resize` emitting `SIGWINCH` on Unix / `ResizePseudoConsole` on Windows) and the in-memory `VtParser` / `ScreenBuffer` grid.
