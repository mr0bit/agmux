# LLD-PTY-VT100-GRID-003: VT100 Parser with 10,000-Line Ring Buffer & 8 KB Chunking

* **Unique ID:** `LLD-PTY-VT100-GRID-003`
* **Upstream Requirement:** `REQ-PTY-VT100-GRID-003`
* **Downstream Verification Test:** `TEST-VER-PTY-003` (`tests/e2e_pty_emulation_test.rs`)
* **SW Component:** `crates/agmux-core::vt_parser`

## 1. Overview
`VtParser` wraps `vt100::Parser::new(rows, cols, 0)` (zero full-cell history overhead) alongside a bounded `VecDeque<String>` scrollback ring buffer (`DEFAULT_SCROLLBACK_LINES = 10_000`), ingesting raw PTY output in 8 KB (`CHUNK_SIZE = 8192`) slices to guarantee < 4 MB memory usage per active session even under 100,000-line output floods.
