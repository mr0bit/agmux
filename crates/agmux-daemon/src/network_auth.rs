//! Cookie token authentication validator for incoming IPC and remote network tunnels (`REQ-REMOTE-AUTH-TOKEN-003`).

use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};

use agmux_proto::{HandshakeRequest, HandshakeResponse};

/// Validates 256-bit cookie tokens in constant time and records rejected authentication attempts.
#[derive(Clone)]
pub struct NetworkAuthValidator {
    expected_token: Arc<String>,
    rejected_attempts: Arc<AtomicU64>,
}

impl NetworkAuthValidator {
    /// Create a new `NetworkAuthValidator` with the daemon's expected cookie token.
    pub fn new(expected_token: impl Into<String>) -> Self {
        Self {
            expected_token: Arc::new(expected_token.into()),
            rejected_attempts: Arc::new(AtomicU64::new(0)),
        }
    }

    /// Verify a `HandshakeRequest` in constant time and produce the corresponding `HandshakeResponse`.
    pub fn verify_handshake(&self, req: &HandshakeRequest) -> HandshakeResponse {
        let valid = !self.expected_token.is_empty()
            && agmux_core::validate_token(&self.expected_token, &req.token);
        if valid {
            HandshakeResponse {
                success: true,
                daemon_version: env!("CARGO_PKG_VERSION").to_string(),
                error: None,
            }
        } else {
            self.rejected_attempts.fetch_add(1, Ordering::SeqCst);
            tracing::warn!(
                "Unauthorized network/IPC connection rejected (client_version={})",
                req.client_version
            );
            HandshakeResponse {
                success: false,
                daemon_version: env!("CARGO_PKG_VERSION").to_string(),
                error: Some("Unauthorized: invalid authentication token".to_string()),
            }
        }
    }

    /// Number of unauthorized connection attempts rejected since startup.
    pub fn rejected_count(&self) -> u64 {
        self.rejected_attempts.load(Ordering::SeqCst)
    }
}
