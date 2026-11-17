//! Cross-platform single-instance lockfile enforcement (`agmuxd.pid`).

use std::fs::{self, File, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};

/// Holds an OS-level exclusive lock on `agmuxd.pid` for the lifetime of the daemon.
pub struct InstanceLock {
    _file: File,
    path: PathBuf,
}

impl InstanceLock {
    /// Derive the lockfile path corresponding to `socket_path` (or `AGMUX_LOCK_PATH`).
    pub fn lock_path_for_socket(socket_path: &Path) -> PathBuf {
        if let Some(p) = std::env::var_os("AGMUX_LOCK_PATH") {
            return PathBuf::from(p);
        }
        if is_named_pipe_path(socket_path) {
            let database = agmux_core::storage_engine::default_database_path();
            let root = database
                .parent()
                .filter(|p| !p.as_os_str().is_empty())
                .unwrap_or(Path::new("."));
            return named_pipe_lock_path(socket_path, root);
        }
        let stem = socket_path
            .file_stem()
            .and_then(|s| s.to_str())
            .unwrap_or("agmuxd");
        if stem == "daemon" || stem == "agmux" || stem == "agmuxd" {
            socket_path.with_file_name("agmuxd.pid")
        } else {
            socket_path.with_extension("pid")
        }
    }

    /// Acquire an exclusive non-blocking lock on `lock_path`.
    /// Holds the open `File` handle until `InstanceLock` is dropped.
    pub fn acquire(lock_path: &Path) -> anyhow::Result<Self> {
        if let Some(parent) = lock_path.parent().filter(|p| !p.as_os_str().is_empty()) {
            fs::create_dir_all(parent)?;
        }

        #[cfg(unix)]
        let mut file = {
            use std::os::unix::io::AsRawFd;
            let f = OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(false)
                .open(lock_path)?;
            let rc = unsafe { libc::flock(f.as_raw_fd(), libc::LOCK_EX | libc::LOCK_NB) };
            if rc != 0 {
                anyhow::bail!(
                    "Another agmuxd instance already holds the lock at {}",
                    lock_path.display()
                );
            }
            f
        };

        #[cfg(windows)]
        let mut file = {
            use std::os::windows::fs::OpenOptionsExt;
            OpenOptions::new()
                .read(true)
                .write(true)
                .create(true)
                .truncate(false)
                .share_mode(0)
                .open(lock_path)
                .map_err(|error| {
                    if matches!(error.raw_os_error(), Some(32 | 33)) {
                        anyhow::anyhow!(
                            "Another agmuxd instance already holds the lock at {}",
                            lock_path.display()
                        )
                    } else {
                        anyhow::Error::new(error).context(format!(
                            "Unable to open daemon lock file at {}",
                            lock_path.display()
                        ))
                    }
                })?
        };

        let _ = file.set_len(0);
        let _ = writeln!(file, "{}", std::process::id());
        let _ = file.flush();

        Ok(Self {
            _file: file,
            path: lock_path.to_path_buf(),
        })
    }

    /// Path to the locked PID file.
    pub fn path(&self) -> &Path {
        &self.path
    }
}

/// Named pipes are IPC addresses, never filesystem paths for lock or database files.
pub fn is_named_pipe_path(path: &Path) -> bool {
    path.to_string_lossy()
        .to_ascii_lowercase()
        .starts_with(r"\\.\pipe\")
}

fn named_pipe_lock_path(pipe: &Path, data_directory: &Path) -> PathBuf {
    // Stable FNV-1a identity keeps arbitrary pipe names out of filesystem paths.
    // It identifies the endpoint; it is not used for authentication.
    let hash = pipe
        .to_string_lossy()
        .to_ascii_lowercase()
        .bytes()
        .fold(0xcbf29ce484222325u64, |hash, byte| {
            (hash ^ u64::from(byte)).wrapping_mul(0x100000001b3)
        });
    data_directory
        .join("run")
        .join(format!("agmuxd-{hash:016x}.pid"))
}

#[cfg(test)]
mod startup_path_tests {
    use super::*;
    #[test]
    fn named_pipe_locks_are_regular_files_outside_the_pipe_namespace() {
        let root =
            std::env::temp_dir().join(format!("agmux-pipe-lock-test-{}", std::process::id()));
        let pipe = Path::new(r"\\.\pipe\agmuxd-test-user");
        assert!(is_named_pipe_path(pipe));
        let path = named_pipe_lock_path(pipe, &root);
        assert!(path.starts_with(&root));
        assert!(!is_named_pipe_path(&path));
        assert_eq!(
            path,
            named_pipe_lock_path(Path::new(r"\\.\PIPE\AGMUXD-TEST-USER"), &root)
        );
        assert_ne!(
            path,
            named_pipe_lock_path(Path::new(r"\\.\pipe\another-daemon"), &root)
        );
        let guard = InstanceLock::acquire(&path)
            .expect("acquire a real filesystem lock for a pipe endpoint");
        assert!(path.is_file());
        drop(guard);
        std::fs::remove_dir_all(root).unwrap();
    }
}
