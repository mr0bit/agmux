# Software Requirements Specification: PTY Management & Terminal Emulation

## 1. Overview
This specification details the pseudo-terminal allocation (`portable-pty`), Win32 ConPTY & WSL2 bridging, VT100 ANSI parser state engine, terminal window resize reflow, and plain-shell sub-tab execution.

---

## 2. Requirements

### REQ-PTY-CROSSPLAT-001
* **Unique ID:** `REQ-PTY-CROSSPLAT-001`
* **Text:** As a user spawning an agent session on Linux, macOS, or Windows, the system shall allocate native pseudo-terminals using `portable-pty` (POSIX openpty on Unix, CreatePseudoConsole ConPTY on Windows) without requiring tmux or psmux binaries.
* **Criticality:** High
* **SW Component:** `crates/agmux-core::pty_manager`
* **Implementation Status:** Verified
* **Verification Criteria:** Spawn CLI sessions across Linux, macOS, and native Windows 11; verify interactive shell execution, raw terminal mode negotiation, and cursor addressing without external dependencies.
* **Downstream Req:** `LLD-PTY-CROSSPLAT-001`
* **Downstream Test:** `TEST-VER-PTY-001`
* **Test Status:** Passed

---

### REQ-PTY-WSL2-BRIDGE-002
* **Unique ID:** `REQ-PTY-WSL2-BRIDGE-002`
* **Text:** As a Windows user running a session configured for WSL2, the system shall spawn the requested agent command via `wsl.exe -d {distro} -- {command}`, correctly mapping Windows terminal resize events and standard I/O streams into the Linux environment.
* **Criticality:** Medium
* **SW Component:** `crates/agmux-core::wsl_bridge`
* **Implementation Status:** Verified
* **Verification Criteria:** Launch a Linux agent inside Ubuntu WSL2 from Windows native `agmuxd`; verify bidirectional text input, ANSI color sequences, and exit code capture.
* **Downstream Req:** `LLD-PTY-WSL2-BRIDGE-002`
* **Downstream Test:** `TEST-VER-PTY-002`
* **Test Status:** Passed

---

### REQ-PTY-VT100-GRID-003
* **Unique ID:** `REQ-PTY-VT100-GRID-003`
* **Text:** As an active session receives ANSI/VT100 escape sequences from its PTY master, `agmuxd` shall parse raw bytes in 8 KB chunks into an in-memory `vt100::Parser` maintaining a configurable 10,000-line scrollback ring buffer.
* **Criticality:** High
* **SW Component:** `crates/agmux-core::vt_parser`
* **Implementation Status:** Verified
* **Verification Criteria:** Stream a high-speed ANSI stress log (100,000 lines); verify that the ring buffer retains the latest 10,000 lines with accurate cell color/attribute integrity and consumes under 4 MB RAM per active session.
* **Downstream Req:** `LLD-PTY-VT100-GRID-003`
* **Downstream Test:** `TEST-VER-PTY-003`
* **Test Status:** Passed

---

### REQ-PTY-RESIZE-REFLOW-004
* **Unique ID:** `REQ-PTY-RESIZE-REFLOW-004`
* **Text:** As a user resizes the terminal emulator or toggles Split Comparison View (`v`), the system shall handle `IpcRequest::Resize { session_id, cols, rows }` by simultaneously resizing the underlying OS pseudo-terminal (`TIOCSWINSZ`) and reflowing the in-memory VT100 grid.
* **Criticality:** High
* **SW Component:** `crates/agmux-core::pty_resizer`
* **Implementation Status:** Verified
* **Verification Criteria:** Resize terminal from 80x24 to 160x50 while running a responsive TUI agent (e.g. Claude Code); verify that SIGWINCH is delivered to the agent process and the screen content re-renders without line truncation.
* **Downstream Req:** `LLD-PTY-RESIZE-REFLOW-004`
* **Downstream Test:** `TEST-VER-PTY-004`
* **Test Status:** Passed

---

### REQ-PTY-SHELL-SUBTAB-005
* **Unique ID:** `REQ-PTY-SHELL-SUBTAB-005`
* **Text:** As a user working inside a session, the system shall allow opening plain interactive shell sub-tabs (bash, zsh, fish, PowerShell `❯`) that inherit the parent session's working directory or Git worktree and injected environment variables (`PORT`, `AGMUX_SESSION_ID`).
* **Criticality:** Medium
* **SW Component:** `crates/agmux-core::subtab_spawner`
* **Implementation Status:** Verified
* **Verification Criteria:** Spawn a sub-tab inside a worktree session; verify that `$PWD` matches the worktree path and `$PORT` matches the worktree's allocated port slot.
* **Downstream Req:** `LLD-PTY-SHELL-SUBTAB-005`
* **Downstream Test:** `TEST-VER-PTY-005`
* **Test Status:** Passed
