//! Discovery of supported agent executables on PATH.

use std::path::{Path, PathBuf};

/// An agent and the result of looking for its executable.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiscoveredAgent {
    pub name: String,
    pub binary_name: String,
    pub path: Option<PathBuf>,
    pub available: bool,
}

/// Find supported agent executables, preserving a stable display order.
pub fn discover_installed_agents() -> Vec<DiscoveredAgent> {
    let path = std::env::var_os("PATH").unwrap_or_default();
    [
        ("Codex", "codex"),
        ("Antigravity", "agy"),
        ("Claude", "claude"),
        ("Aider", "aider"),
        ("Cursor", "cursor-agent"),
        ("Gemini", "gemini"),
    ]
    .into_iter()
    .map(|(name, binary)| {
        let found = std::env::split_paths(&path).find_map(|dir| find_executable(&dir, binary));
        DiscoveredAgent {
            name: name.into(),
            binary_name: binary.into(),
            available: found.is_some(),
            path: found,
        }
    })
    .collect()
}

fn find_executable(dir: &Path, binary: &str) -> Option<PathBuf> {
    #[cfg(windows)]
    let candidates: Vec<PathBuf> = std::env::var_os("PATHEXT")
        .unwrap_or_else(|| ".COM;.EXE;.BAT;.CMD".into())
        .to_string_lossy()
        .split(';')
        .map(|ext| dir.join(format!("{binary}{ext}")))
        .collect();
    #[cfg(not(windows))]
    let candidates = [dir.join(binary)];
    candidates.into_iter().find(|path| {
        let Ok(metadata) = path.metadata() else {
            return false;
        };
        if !metadata.is_file() {
            return false;
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            metadata.permissions().mode() & 0o111 != 0
        }
        #[cfg(not(unix))]
        {
            true
        }
    })
}
