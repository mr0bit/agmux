//! VT100 terminal screen grid parser with a bounded 10,000-line scrollback ring buffer and 8 KB chunking.

use std::collections::VecDeque;
use std::num::NonZeroU16;

/// Default maximum scrollback lines retained in the ring buffer per session.
pub const DEFAULT_SCROLLBACK_LINES: usize = 10_000;

/// Fixed 8 KB chunk size used when feeding raw PTY byte streams into the VT100 parser.
pub const CHUNK_SIZE: usize = 8192;

/// In-memory VT100 screen grid paired with a bounded `VecDeque<String>` scrollback ring buffer.
pub struct VtParser {
    parser: vt100::Parser,
    rows: u16,
    cols: u16,
    max_scrollback: usize,
    scrollback: VecDeque<String>,
    partial_line: Vec<u8>,
}

impl VtParser {
    /// Create a new `VtParser` with the specified dimensions and default 10,000-line scrollback cap.
    pub fn new(rows: u16, cols: u16) -> Self {
        Self::with_scrollback(rows, cols, DEFAULT_SCROLLBACK_LINES)
    }

    /// Create a new `VtParser` with custom scrollback line capacity.
    pub fn with_scrollback(rows: u16, cols: u16, max_scrollback: usize) -> Self {
        let r = NonZeroU16::new(rows.max(1)).expect("non-zero rows");
        let c = NonZeroU16::new(cols.max(1)).expect("non-zero cols");
        Self {
            parser: vt100::Parser::new(r, c, 0),
            rows: r.get(),
            cols: c.get(),
            max_scrollback: max_scrollback.max(1),
            scrollback: VecDeque::with_capacity(max_scrollback.min(1024)),
            partial_line: Vec::new(),
        }
    }

    /// Feed raw PTY output bytes in 8 KB (`CHUNK_SIZE = 8192`) slices, updating both the VT100 grid
    /// and the bounded scrollback ring buffer.
    pub fn feed_bytes(&mut self, data: &[u8]) {
        for chunk in data.chunks(CHUNK_SIZE) {
            self.parser.process(chunk);
            for &b in chunk {
                if b == b'\n' {
                    if self.partial_line.last() == Some(&b'\r') {
                        self.partial_line.pop();
                    }
                    let line = String::from_utf8_lossy(&self.partial_line).into_owned();
                    self.partial_line.clear();
                    if self.scrollback.len() >= self.max_scrollback {
                        self.scrollback.pop_front();
                    }
                    self.scrollback.push_back(line);
                } else {
                    self.partial_line.push(b);
                }
            }
        }
    }

    /// Resize the VT100 grid to `(rows, cols)` and reflow visible contents.
    pub fn resize(&mut self, rows: u16, cols: u16) {
        let r = NonZeroU16::new(rows.max(1)).expect("non-zero rows");
        let c = NonZeroU16::new(cols.max(1)).expect("non-zero cols");
        let formatted = self.parser.screen().contents_formatted();
        self.rows = r.get();
        self.cols = c.get();
        self.parser = vt100::Parser::new(r, c, 0);
        self.parser.process(formatted.as_bytes());
    }

    /// Current `(rows, cols)` dimensions.
    pub fn size(&self) -> (u16, u16) {
        (self.rows, self.cols)
    }

    /// Current `(cursor_row, cursor_col)` position.
    pub fn cursor_position(&self) -> (u16, u16) {
        self.parser.screen().cursor_position()
    }

    /// Visible screen rows as plain text lines.
    pub fn visible_lines(&self) -> Vec<String> {
        let screen = self.parser.screen();
        let mut lines: Vec<String> = screen
            .rows(0, self.cols)
            .map(|r| r.trim_end().to_string())
            .collect();
        while lines.last().is_some_and(|l| l.is_empty()) {
            lines.pop();
        }
        lines
    }

    /// Bounded scrollback history ring buffer.
    pub fn scrollback_lines(&self) -> &VecDeque<String> {
        &self.scrollback
    }

    /// Formatted ANSI byte sequence representing the active screen grid.
    pub fn capture_formatted(&self) -> Vec<u8> {
        self.parser.screen().contents_formatted().into_bytes()
    }
}
