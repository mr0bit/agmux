//! Zstandard level 3 terminal screen snapshot compression and restoration.

use std::io;
use std::num::NonZeroU16;

/// Compress raw terminal ANSI data using Zstandard level 3.
pub fn compress_snapshot(raw_data: &[u8]) -> Result<Vec<u8>, io::Error> {
    zstd::encode_all(raw_data, 3)
}

/// Decompress Zstandard snapshot back into raw ANSI data.
pub fn decompress_snapshot(compressed_data: &[u8]) -> Result<Vec<u8>, io::Error> {
    zstd::decode_all(compressed_data)
}

/// In-memory terminal VT100 screen buffer tracker.
pub struct ScreenBuffer {
    parser: vt100::Parser,
}

impl ScreenBuffer {
    /// Create a new virtual terminal parser with specified rows and cols.
    pub fn new(rows: u16, cols: u16) -> Self {
        let rows = NonZeroU16::new(rows.max(1)).expect("non-zero rows");
        let cols = NonZeroU16::new(cols.max(1)).expect("non-zero cols");
        Self {
            parser: vt100::Parser::new(rows, cols, 10_000),
        }
    }

    /// Process incoming raw PTY byte chunk.
    pub fn process(&mut self, bytes: &[u8]) {
        self.parser.process(bytes);
    }

    /// Capture full formatted ANSI contents.
    pub fn capture_formatted(&self) -> Vec<u8> {
        self.parser.screen().contents_formatted().into_bytes()
    }

    /// Capture plain text contents.
    pub fn capture_plain(&self) -> String {
        self.parser.screen().contents()
    }
}

impl Default for ScreenBuffer {
    fn default() -> Self {
        Self::new(24, 80)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_compress_decompress_roundtrip() {
        let input = b"Hello world! This is a test string to be compressed with zstd.";
        let compressed = compress_snapshot(input).expect("compression failed");
        let decompressed = decompress_snapshot(&compressed).expect("decompression failed");
        assert_eq!(decompressed, input);
    }

    #[test]
    fn test_screen_buffer_capture() {
        let mut screen = ScreenBuffer::new(24, 80);
        screen.process(b"echo \x1b[32mSUCCESS\x1b[0m\r\n");
        let plain = screen.capture_plain();
        assert!(plain.contains("echo SUCCESS"));
        let formatted = screen.capture_formatted();
        let formatted_str = String::from_utf8_lossy(&formatted);
        assert!(formatted_str.contains("SUCCESS"));
    }
}
