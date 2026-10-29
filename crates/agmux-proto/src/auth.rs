//! Protocol-level authentication constants and checks.

/// The expected length of raw authentication tokens in bytes.
pub const TOKEN_BYTE_LEN: usize = 32;

/// Constant-time comparison of two ASCII or hex token strings to prevent timing attacks.
pub fn constant_time_eq(a: &str, b: &str) -> bool {
    let a_bytes = a.as_bytes();
    let b_bytes = b.as_bytes();
    if a_bytes.len() != b_bytes.len() {
        return false;
    }
    let mut diff = 0u8;
    for (x, y) in a_bytes.iter().zip(b_bytes.iter()) {
        diff |= x ^ y;
    }
    diff == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_constant_time_eq() {
        assert!(constant_time_eq("token-123", "token-123"));
        assert!(!constant_time_eq("token-123", "token-124"));
        assert!(!constant_time_eq("token-123", "token-12"));
    }
}
