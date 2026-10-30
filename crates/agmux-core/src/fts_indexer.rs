//! Bounded raw and clean terminal history with streaming ANSI stripping.
use std::collections::VecDeque;

/// Maintains the most recent raw and clean bytes and decodes terminal lines.
pub struct TerminalHistory {
    raw: VecDeque<u8>,
    clean: VecDeque<u8>,
    capacity: usize,
    state: u8,
    line: Vec<u8>,
}
impl TerminalHistory {
    /// Construct a history with bounded byte streams.
    pub fn new(capacity: usize) -> Self {
        Self {
            raw: VecDeque::new(),
            clean: VecDeque::new(),
            capacity,
            state: 0,
            line: Vec::new(),
        }
    }
    fn push_ring(ring: &mut VecDeque<u8>, capacity: usize, byte: u8) {
        if capacity == 0 {
            return;
        }
        if ring.len() == capacity {
            ring.pop_front();
        }
        ring.push_back(byte);
    }
    /// Append raw bytes and return complete stripped lines. Escape sequences may span chunks.
    pub fn ingest(&mut self, data: &[u8]) -> Vec<String> {
        let mut lines = Vec::new();
        for &b in data {
            Self::push_ring(&mut self.raw, self.capacity, b);
            match self.state {
                0 if b == 0x1b => self.state = 1,
                0 if b == b'\n' => {
                    lines.push(String::from_utf8_lossy(&self.line).into_owned());
                    self.line.clear();
                    Self::push_ring(&mut self.clean, self.capacity, b);
                }
                0 if b == b'\r' => {}
                0 if b >= 0x20 || b == b'\t' => {
                    Self::push_ring(&mut self.clean, self.capacity, b);
                    self.line.push(b);
                }
                0 => {}
                1 if b == b'[' => self.state = 2,
                1 if b == b']' => self.state = 3,
                1 => self.state = 0,
                2 if (0x40..=0x7e).contains(&b) => self.state = 0,
                3 if b == 7 => self.state = 0,
                3 if b == 0x1b => self.state = 4,
                4 if b == b'\\' => self.state = 0,
                4 => self.state = 3,
                _ => {}
            }
        }
        lines
    }
    /// Flush the final unterminated line.
    pub fn finish(&mut self) -> Option<String> {
        if self.line.is_empty() {
            None
        } else {
            Some(String::from_utf8_lossy(&std::mem::take(&mut self.line)).into_owned())
        }
    }
    /// Current unterminated line, if any, for live search indexing.
    pub fn pending_line(&self) -> Option<String> {
        (!self.line.is_empty()).then(|| String::from_utf8_lossy(&self.line).into_owned())
    }
    /// Snapshot the raw byte stream.
    pub fn raw(&self) -> Vec<u8> {
        self.raw.iter().copied().collect()
    }
    /// Snapshot the stripped byte stream.
    pub fn clean(&self) -> Vec<u8> {
        self.clean.iter().copied().collect()
    }
}

#[cfg(test)]
mod tests {
    use super::TerminalHistory;
    #[test]
    fn strips_split_sequences_and_preserves_raw() {
        let mut history = TerminalHistory::new(64);
        assert!(history.ingest(b"\x1b[3").is_empty());
        assert_eq!(history.ingest(b"1mhello\x1b[0m\r\n"), ["hello"]);
        assert_eq!(history.raw(), b"\x1b[31mhello\x1b[0m\r\n");
        assert_eq!(history.clean(), b"hello\n");
    }
}
