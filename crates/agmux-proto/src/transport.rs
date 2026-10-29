//! Length-delimited framed transport codec and streaming utilities.

use bytes::{Buf, BufMut, BytesMut};
use tokio_util::codec::{Decoder, Encoder, Framed};

use crate::messages::IpcMessage;

/// Maximum allowable frame size (16 MB) to prevent denial-of-service allocations.
pub const MAX_FRAME_SIZE: usize = 16 * 1024 * 1024;

/// Protocol and transport errors.
#[derive(Debug, thiserror::Error)]
pub enum ProtoError {
    /// Standard I/O failure.
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// JSON serialization or deserialization error.
    #[error("JSON serialization error: {0}")]
    Serialization(#[from] serde_json::Error),

    /// Frame size exceeds the allowable limit.
    #[error("Frame size {0} bytes exceeds maximum allowed {MAX_FRAME_SIZE} bytes")]
    FrameTooLarge(usize),
}

/// A 4-byte big-endian length-delimited codec for framed [`IpcMessage`] transmission.
#[derive(Debug, Default, Clone, Copy)]
pub struct IpcCodec;

impl IpcCodec {
    /// Create a new `IpcCodec`.
    pub fn new() -> Self {
        Self
    }
}

impl Encoder<IpcMessage> for IpcCodec {
    type Error = ProtoError;

    fn encode(&mut self, item: IpcMessage, dst: &mut BytesMut) -> Result<(), Self::Error> {
        let payload = serde_json::to_vec(&item)?;
        if payload.len() > MAX_FRAME_SIZE {
            return Err(ProtoError::FrameTooLarge(payload.len()));
        }

        let length =
            u32::try_from(payload.len()).map_err(|_| ProtoError::FrameTooLarge(payload.len()))?;

        dst.reserve(4 + payload.len());
        dst.put_u32(length);
        dst.put_slice(&payload);
        Ok(())
    }
}

impl Decoder for IpcCodec {
    type Item = IpcMessage;
    type Error = ProtoError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        if src.len() < 4 {
            return Ok(None);
        }

        // Read 4-byte length prefix without advancing read pointer yet
        let length = u32::from_be_bytes([src[0], src[1], src[2], src[3]]) as usize;
        if length > MAX_FRAME_SIZE {
            return Err(ProtoError::FrameTooLarge(length));
        }

        let total_frame_len = 4 + length;
        if src.len() < total_frame_len {
            // Wait for full frame to arrive
            src.reserve(total_frame_len - src.len());
            return Ok(None);
        }

        // Advance past length header
        src.advance(4);
        let payload = src.split_to(length);

        let message: IpcMessage = serde_json::from_slice(&payload)?;
        Ok(Some(message))
    }
}

/// Wrap an asynchronous stream in framed IPC communication.
pub type FramedStream<T> = Framed<T, IpcCodec>;

/// Construct a framed stream around any [`tokio::io::AsyncRead`] + [`tokio::io::AsyncWrite`] stream.
pub fn new_framed<T>(stream: T) -> FramedStream<T>
where
    T: tokio::io::AsyncRead + tokio::io::AsyncWrite,
{
    Framed::new(stream, IpcCodec::new())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::messages::{HandshakeRequest, TerminalOutputFrame};

    #[test]
    fn test_codec_roundtrip() {
        let mut codec = IpcCodec::new();
        let mut buffer = BytesMut::new();

        let original_msg = IpcMessage::HandshakeRequest(HandshakeRequest {
            client_version: "0.1.0".to_string(),
            token: "secret-token-1234567890".to_string(),
        });

        codec.encode(original_msg.clone(), &mut buffer).unwrap();
        assert!(buffer.len() > 4);

        let decoded = codec.decode(&mut buffer).unwrap().expect("message");
        assert_eq!(decoded, original_msg);
        assert_eq!(buffer.len(), 0);
    }

    #[test]
    fn test_codec_partial_frame() {
        let mut codec = IpcCodec::new();
        let mut buffer = BytesMut::new();

        let msg = IpcMessage::TerminalOutputFrame(TerminalOutputFrame {
            session_id: "s1".to_string(),
            data: b"hello world\n".to_vec(),
        });

        codec.encode(msg.clone(), &mut buffer).unwrap();
        let full_data = buffer.split();

        // Feed only 2 bytes
        buffer.extend_from_slice(&full_data[..2]);
        assert_eq!(codec.decode(&mut buffer).unwrap(), None);

        // Feed rest
        buffer.extend_from_slice(&full_data[2..]);
        let decoded = codec.decode(&mut buffer).unwrap().expect("decoded");
        assert_eq!(decoded, msg);
    }
}
