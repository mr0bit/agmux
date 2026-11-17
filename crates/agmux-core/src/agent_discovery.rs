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
        let found = std::env::split_paths(&path)
            .find_map(|dir| find_executable(&dir, binary, cfg!(windows)));
        DiscoveredAgent {
            name: name.into(),
            binary_name: binary.into(),
            available: found.is_some(),
            path: found,
        }
    })
    .collect()
}

/// Discover all 14 built-in first-class CLI tools across custom search directories,
/// applying OS-aware binary name resolution (`cmd` -> `command-code` on Windows)
/// and rejecting Windows `System32\cmd.exe` collisions.
pub fn discover_all_builtin_agents_in_paths(
    search_dirs: &[PathBuf],
    is_windows: bool,
) -> Vec<DiscoveredAgent> {
    crate::builtin_plugins::all_builtin_plugins()
        .into_iter()
        .map(|plugin| {
            let binary = plugin.binary_name_for_os(is_windows);
            let found = search_dirs.iter().find_map(|dir| {
                find_executable(dir, binary, is_windows).filter(|p| {
                    !crate::builtin_plugins::is_windows_system_cmd_collision(plugin.id(), p)
                })
            });
            DiscoveredAgent {
                name: plugin.display_name().to_string(),
                binary_name: binary.to_string(),
                available: found.is_some(),
                path: found,
            }
        })
        .collect()
}

fn find_executable(dir: &Path, binary: &str, is_windows: bool) -> Option<PathBuf> {
    let candidates: Vec<PathBuf> = if is_windows {
        // npm also writes an extensionless POSIX shim on Windows; never execute it.
        ["exe", "com", "cmd", "bat"]
            .iter()
            .map(|ext| dir.join(format!("{binary}.{ext}")))
            .collect()
    } else {
        vec![dir.join(binary)]
    };
    candidates.into_iter().find(|path| {
        let Ok(metadata) = path.metadata() else {
            return false;
        };
        if !metadata.is_file() {
            return false;
        }
        #[cfg(unix)]
        if !is_windows {
            use std::os::unix::fs::PermissionsExt;
            return metadata.permissions().mode() & 0o111 != 0;
        }
        true
    })
}

/// Actionable failures while resolving a Windows Gemini installation.
#[derive(Debug, thiserror::Error)]
pub enum GeminiLaunchError {
    #[error("Gemini CLI not found. Install @google/gemini-cli and restart Agmux.")]
    MissingCli,
    #[error("Gemini npm package missing beside its launcher. Reinstall @google/gemini-cli.")]
    MissingPackage,
    #[error("Gemini package.json is invalid")]
    InvalidMetadata,
    #[error("Gemini CLI executable entry point is missing")]
    MissingEntry,
    #[error("Gemini entry point escapes its package")]
    EntryOutsidePackage,
    #[error("Node.js is required for Gemini CLI. Install Node.js 20 or newer and restart Agmux.")]
    MissingNode,
}

/// A Gemini launch resolved without invoking a Windows command shell.
pub struct GeminiWindowsLaunch {
    pub command: PathBuf,
    pub args: Vec<String>,
}

/// Resolve npm's official Gemini entry point to Node, preserving argument boundaries.
/// Other agents are untouched. Native Gemini executables are launched directly.
pub fn windows_gemini_launch(
    command: &str,
    search_dirs: &[PathBuf],
) -> Result<Option<GeminiWindowsLaunch>, GeminiLaunchError> {
    let path = Path::new(command);
    let name = path
        .file_name()
        .unwrap_or_default()
        .to_string_lossy()
        .to_ascii_lowercase();
    if !matches!(
        name.as_str(),
        "gemini" | "gemini.cmd" | "gemini.bat" | "gemini.exe"
    ) {
        return Ok(None);
    }
    let resolved =
        if path.is_absolute() || path.parent().is_some_and(|p| !p.as_os_str().is_empty()) {
            path.is_file().then(|| path.to_path_buf())
        } else {
            search_dirs
                .iter()
                .find_map(|dir| find_executable(dir, "gemini", true))
        }
        .ok_or(GeminiLaunchError::MissingCli)?;
    if resolved
        .extension()
        .is_some_and(|ext| ext.eq_ignore_ascii_case("exe"))
    {
        return Ok(Some(GeminiWindowsLaunch {
            command: resolved,
            args: vec![],
        }));
    }
    let directory = resolved.parent().ok_or(GeminiLaunchError::MissingPackage)?;
    let package = directory.join("node_modules/@google/gemini-cli");
    let metadata: serde_json::Value = serde_json::from_slice(
        &std::fs::read(package.join("package.json"))
            .map_err(|_| GeminiLaunchError::MissingPackage)?,
    )
    .map_err(|_| GeminiLaunchError::InvalidMetadata)?;
    let entry = metadata["bin"]["gemini"]
        .as_str()
        .or_else(|| metadata["bin"].as_str())
        .ok_or(GeminiLaunchError::MissingEntry)?;
    let entry = package
        .join(entry)
        .canonicalize()
        .map_err(|_| GeminiLaunchError::MissingEntry)?;
    let package = package
        .canonicalize()
        .map_err(|_| GeminiLaunchError::MissingPackage)?;
    if !entry.starts_with(&package) || !entry.is_file() {
        return Err(GeminiLaunchError::EntryOutsidePackage);
    }
    let node = directory.join("node.exe");
    let node = if node.is_file() {
        Some(node)
    } else {
        search_dirs
            .iter()
            .find_map(|dir| find_executable(dir, "node", true))
    }
    .ok_or(GeminiLaunchError::MissingNode)?;
    Ok(Some(GeminiWindowsLaunch {
        command: node,
        args: vec![entry.to_string_lossy().into_owned()],
    }))
}
