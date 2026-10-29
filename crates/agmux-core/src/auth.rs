//! Authentication token generation, storage, and validation.

use std::fs::{self, OpenOptions};
use std::io::{Read, Write};
use std::path::{Path, PathBuf};

/// Error types related to authentication token handling.
#[derive(Debug, thiserror::Error)]
pub enum AuthError {
    /// An underlying filesystem or standard I/O error occurred.
    #[error("Filesystem I/O error: {0}")]
    Io(#[from] std::io::Error),

    /// Token file exists but contains invalid ASCII / UTF-8 data.
    #[error("Invalid token format in {0}: must be non-empty valid UTF-8 string")]
    InvalidToken(PathBuf),

    /// Unable to determine user's home directory.
    #[error("Could not determine user home directory")]
    HomeNotFound,
}

/// Retrieve the default authentication token file path.
///
/// Priority:
/// 1. `AGMUX_AUTH_TOKEN_PATH` environment variable if set.
/// 2. `APPDATA/agmux/auth_token` on Windows or `~/.agmux/auth_token` on Unix.
/// 3. Fallback to `/tmp/.agmux_auth_token` or temp directory.
pub fn default_token_path() -> PathBuf {
    if let Ok(path) = std::env::var("AGMUX_AUTH_TOKEN_PATH") {
        if !path.is_empty() {
            return PathBuf::from(path);
        }
    }

    #[cfg(windows)]
    {
        if let Ok(appdata) = std::env::var("APPDATA") {
            if !appdata.is_empty() {
                return PathBuf::from(appdata).join("agmux").join("auth_token");
            }
        }
        if let Ok(userprofile) = std::env::var("USERPROFILE") {
            if !userprofile.is_empty() {
                return PathBuf::from(userprofile).join(".agmux").join("auth_token");
            }
        }
    }

    #[cfg(not(windows))]
    if let Ok(home) = std::env::var("HOME") {
        if !home.is_empty() {
            return PathBuf::from(home).join(".agmux").join("auth_token");
        }
    }

    std::env::temp_dir().join(".agmux_auth_token")
}

/// Generate 32 cryptographically secure random bytes from `/dev/urandom` on POSIX systems.
#[cfg(unix)]
pub fn generate_random_32_bytes() -> Result<[u8; 32], std::io::Error> {
    let mut buf = [0u8; 32];
    let mut file = fs::File::open("/dev/urandom")?;
    file.read_exact(&mut buf)?;
    Ok(buf)
}

/// Generate 32 pseudorandom entropy bytes on non-POSIX systems.
#[cfg(not(unix))]
pub fn generate_random_32_bytes() -> Result<[u8; 32], std::io::Error> {
    let mut buf = [0u8; 32];
    let t = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let pid = std::process::id();
    let mut state = (t ^ ((pid as u128) << 32)) as u64;
    for byte in buf.iter_mut() {
        state ^= state << 13;
        state ^= state >> 7;
        state ^= state << 17;
        *byte = (state & 0xFF) as u8;
    }
    Ok(buf)
}

/// Load an existing authentication token, or securely generate and write a new 32-byte secret.
///
/// The token is stored as a 64-character lowercase hex string.
/// On Unix, directory permissions are set to `0700` and file permissions to `0600`.
pub fn get_or_create_token(path: &Path) -> Result<String, AuthError> {
    if path.exists() {
        return read_token(path);
    }

    // Ensure parent directory exists with 0700 permissions on Unix
    if let Some(parent) = path.parent() {
        if !parent.exists() {
            fs::create_dir_all(parent)?;
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                let perms = fs::Permissions::from_mode(0o700);
                let _ = fs::set_permissions(parent, perms);
            }
        }
    }

    let bytes = generate_random_32_bytes()?;
    let hex_token = bytes.iter().map(|b| format!("{b:02x}")).collect::<String>();

    // Write file with 0600 permissions
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);

    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }

    let mut file = match options.open(path) {
        Ok(file) => file,
        Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => return read_token(path),
        Err(e) => return Err(e.into()),
    };
    file.write_all(hex_token.as_bytes())?;
    file.write_all(b"\n")?;
    file.flush()?;

    Ok(hex_token)
}

fn read_token(path: &Path) -> Result<String, AuthError> {
    let content = fs::read_to_string(path)?;
    let token = content.trim();
    if token.len() != 64 || !token.bytes().all(|b| b.is_ascii_hexdigit()) {
        return Err(AuthError::InvalidToken(path.to_owned()));
    }
    Ok(token.to_owned())
}

/// Validate that a candidate token matches the expected token in constant time.
pub fn validate_token(expected: &str, candidate: &str) -> bool {
    let a_bytes = expected.trim().as_bytes();
    let b_bytes = candidate.trim().as_bytes();

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
    fn test_token_creation_and_reload() {
        let temp_dir = std::env::temp_dir().join(format!("agmux_test_auth_{}", std::process::id()));
        let _ = fs::remove_dir_all(&temp_dir);
        let token_path = temp_dir.join("auth_token");

        let token1 = get_or_create_token(&token_path).expect("create token");
        assert_eq!(token1.len(), 64);

        let token2 = get_or_create_token(&token_path).expect("reload token");
        assert_eq!(token1, token2);
        assert!(validate_token(&token1, &token2));
        assert!(!validate_token(&token1, "invalid_token"));

        let _ = fs::remove_dir_all(&temp_dir);
    }
}
