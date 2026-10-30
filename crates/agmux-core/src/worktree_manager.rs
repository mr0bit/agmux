//! Git worktree provisioning and local file inclusion.
use std::path::{Component, Path, PathBuf};
use std::process::Command;

/// An isolated checkout and its assigned port.
#[derive(Debug, Clone)]
pub struct WorktreeInfo {
    /// Checkout location.
    pub path: PathBuf,
    /// Checked out branch.
    pub branch: String,
    /// Stable port derived from the branch name.
    pub port: u16,
}

/// Worktree provisioning failure.
#[derive(Debug, thiserror::Error)]
pub enum WorktreeError {
    /// Filesystem or process launch failure.
    #[error("worktree I/O error: {0}")]
    Io(#[from] std::io::Error),
    /// Git rejected the request.
    #[error("git failed: {0}")]
    Git(String),
    /// The checkout contains uncommitted changes.
    #[error("dirty working tree: {0}")]
    DirtyWorkingTree(PathBuf),
    /// A branch or include path was unsafe.
    #[error("invalid worktree path or pattern: {0}")]
    InvalidPath(String),
}

fn git(root: &Path, args: &[&std::ffi::OsStr]) -> Result<(), WorktreeError> {
    let output = Command::new("git").current_dir(root).args(args).output()?;
    if output.status.success() {
        Ok(())
    } else {
        Err(WorktreeError::Git(
            String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        ))
    }
}

/// Compute a deterministic port (`8000..=8999`) from a branch or session identifier using FNV-1a.
pub fn port_for_branch(branch: &str) -> u16 {
    let hash = branch.bytes().fold(2166136261u32, |h, b| {
        (h ^ u32::from(b)).wrapping_mul(16777619)
    });
    8000 + (hash % 1000) as u16
}

/// Create a worktree, copying matching root-level files.
pub fn provision_worktree(repo_root: &Path, branch: &str) -> Result<WorktreeInfo, WorktreeError> {
    if branch.is_empty()
        || branch.starts_with('-')
        || branch
            .split('/')
            .any(|s| s.is_empty() || s == "." || s == "..")
        || branch.contains('\\')
    {
        return Err(WorktreeError::InvalidPath(branch.into()));
    }
    let repo_root = std::fs::canonicalize(repo_root)?;
    let path = repo_root.join(".agmux-worktrees").join(branch);
    std::fs::create_dir_all(
        path.parent()
            .ok_or_else(|| WorktreeError::InvalidPath(branch.into()))?,
    )?;
    git(
        &repo_root,
        &[
            "worktree".as_ref(),
            "add".as_ref(),
            "-B".as_ref(),
            branch.as_ref(),
            path.as_os_str(),
        ],
    )?;
    let result = copy_includes(&repo_root, &path);
    if let Err(error) = result {
        let _ = git(
            &repo_root,
            &[
                "worktree".as_ref(),
                "remove".as_ref(),
                "--force".as_ref(),
                path.as_os_str(),
            ],
        );
        return Err(error);
    }
    Ok(WorktreeInfo {
        path,
        branch: branch.into(),
        port: port_for_branch(branch),
    })
}

fn copy_includes(root: &Path, dest: &Path) -> Result<(), WorktreeError> {
    let contents = match std::fs::read_to_string(root.join(".worktreeinclude")) {
        Ok(s) => s,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e.into()),
    };
    for pattern in contents
        .lines()
        .map(str::trim)
        .filter(|s| !s.is_empty() && !s.starts_with('#'))
    {
        let p = Path::new(pattern);
        if p.components().any(|c| !matches!(c, Component::Normal(_))) || p.components().count() != 1
        {
            return Err(WorktreeError::InvalidPath(pattern.into()));
        }
        for entry in std::fs::read_dir(root)? {
            let entry = entry?;
            let name = entry.file_name();
            if entry.file_type()?.is_file()
                && matches_pattern(pattern.as_bytes(), name.to_string_lossy().as_bytes())
            {
                let target = dest.join(&name);
                // Never replace a file already present in the checked-out tree.
                if !target.exists() {
                    std::fs::copy(entry.path(), target)?;
                }
            }
        }
    }
    Ok(())
}

fn matches_pattern(pattern: &[u8], name: &[u8]) -> bool {
    match pattern.split_first() {
        None => name.is_empty(),
        Some((&b'*', rest)) => (0..=name.len()).any(|i| matches_pattern(rest, &name[i..])),
        Some((&b'?', rest)) => !name.is_empty() && matches_pattern(rest, &name[1..]),
        Some((&c, rest)) => name.first() == Some(&c) && matches_pattern(rest, &name[1..]),
    }
}

/// Remove a Git worktree from disk and Git tracking.
pub fn remove_worktree(repo_root: &Path, worktree_path: &Path) -> Result<(), WorktreeError> {
    git(
        repo_root,
        &[
            "worktree".as_ref(),
            "remove".as_ref(),
            "--force".as_ref(),
            worktree_path.as_os_str(),
        ],
    )
}

/// Check for tracked or untracked changes, or unpushed commits in a checkout.
pub fn is_worktree_dirty(worktree_path: &Path) -> Result<bool, WorktreeError> {
    let output = Command::new("git")
        .current_dir(worktree_path)
        .args(["status", "--porcelain"])
        .output()?;
    if !output.status.success() {
        return Err(WorktreeError::Git(
            String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        ));
    }
    if !output.stdout.is_empty() {
        return Ok(true);
    }
    // Also check for unpushed commits if an upstream branch is tracked
    if let Ok(unpushed) = Command::new("git")
        .current_dir(worktree_path)
        .args(["log", "@{u}..HEAD", "--oneline"])
        .output()
    {
        if unpushed.status.success() && !unpushed.stdout.is_empty() {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Stash tracked and untracked changes before removing a checkout.
pub fn create_backup_stash(worktree_path: &Path) -> Result<String, WorktreeError> {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_nanos();
    let name = format!("agmux-auto-stash-{timestamp}");
    git(
        worktree_path,
        &[
            "stash".as_ref(),
            "push".as_ref(),
            "-u".as_ref(),
            "-m".as_ref(),
            name.as_ref(),
        ],
    )?;
    Ok(name)
}

/// Remove a checkout only after preserving any uncommitted changes.
pub fn safe_remove_worktree(
    repo_root: &Path,
    worktree_path: &Path,
    force: bool,
) -> Result<Option<String>, WorktreeError> {
    let stash = if is_worktree_dirty(worktree_path)? {
        if !force {
            return Err(WorktreeError::DirtyWorkingTree(worktree_path.to_path_buf()));
        }
        Some(create_backup_stash(worktree_path)?)
    } else {
        None
    };
    remove_worktree(repo_root, worktree_path)?;
    Ok(stash)
}
