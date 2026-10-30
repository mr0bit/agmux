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
                .map_err(|_| {
                    anyhow::anyhow!(
                        "Another agmuxd instance already holds the lock at {}",
                        lock_path.display()
                    )
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
