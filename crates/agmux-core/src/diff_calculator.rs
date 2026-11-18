//! Structured Git review diffs.
pub use agmux_proto::{DiffHunk, DiffLine, DiffLineType, DiffScope, FileDiff};
use std::{io::Read, path::Path, process::Command};

#[derive(Debug, thiserror::Error)]
pub enum DiffError {
    #[error("Git I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("This project is not a Git repository")]
    NotRepository,
    #[error("Git failed: {0}")]
    Git(String),
    #[error("Session has no initial Git commit")]
    MissingBaseline,
    #[error("Invalid Git diff: {0}")]
    InvalidDiff(String),
}

/// Compare tracked changes against the requested base. Target defaults to `main`.
pub fn calculate_diff(
    cwd: &Path,
    scope: DiffScope,
    baseline: Option<&str>,
    target: Option<&str>,
) -> Result<Vec<FileDiff>, DiffError> {
    // Probe separately: `git diff` outside a repository prints pages of usage.
    let probe = Command::new("git")
        .env("LC_ALL", "C")
        .current_dir(cwd)
        .args(["rev-parse", "--is-inside-work-tree"])
        .output()?;
    if !probe.status.success() {
        let detail = String::from_utf8_lossy(&probe.stderr);
        if detail.contains("not a git repository") {
            return Err(DiffError::NotRepository);
        }
        return Err(DiffError::Git(detail.trim().to_owned()));
    }
    if probe.stdout != b"true\n" && probe.stdout != b"true\r\n" {
        return Err(DiffError::NotRepository);
    }
    let has_head = Command::new("git")
        .current_dir(cwd)
        .args(["rev-parse", "--verify", "HEAD"])
        .output()?
        .status
        .success();
    let include_untracked = matches!(scope, DiffScope::FullUncommitted | DiffScope::SessionDiff);
    let mut command = Command::new("git");
    command.current_dir(cwd).args([
        "-c",
        "core.quotePath=false",
        "diff",
        "--no-ext-diff",
        "--no-color",
        "--no-renames",
        "--no-prefix",
    ]);
    match scope {
        DiffScope::SessionDiff => {
            command.arg(baseline.ok_or(DiffError::MissingBaseline)?);
        }
        DiffScope::FullUncommitted => {
            if has_head {
                command.arg("HEAD");
            } else {
                // Compare the final working tree against Git's empty tree, so
                // staged additions also appear before the first commit. Ask
                // Git for the hash to respect SHA-256 repositories, without
                // writing an object or changing the index.
                let empty = Command::new("git")
                    .current_dir(cwd)
                    .stdin(std::process::Stdio::null())
                    .args(["hash-object", "-t", "tree", "--stdin"])
                    .output()?;
                if !empty.status.success() {
                    return Err(DiffError::Git(
                        String::from_utf8_lossy(&empty.stderr).trim().into(),
                    ));
                }
                command.arg(String::from_utf8_lossy(&empty.stdout).trim());
            }
        }
        DiffScope::VersusTarget => {
            let target = target.unwrap_or("main");
            if target.starts_with('-') {
                return Err(DiffError::Git("invalid target revision".into()));
            }
            command.arg(target);
        }
        DiffScope::LastCommit => {
            command.args(["HEAD~1", "HEAD"]);
        }
        DiffScope::Staged => {
            command.arg("--cached");
            if has_head {
                command.arg("HEAD");
            }
        }
    }
    let output = command.arg("--").output()?;
    if !output.status.success() {
        return Err(DiffError::Git(
            String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        ));
    }
    let mut files = parse(&String::from_utf8_lossy(&output.stdout))?;
    if include_untracked {
        let untracked = Command::new("git")
            .current_dir(cwd)
            .args(["ls-files", "--others", "--exclude-standard", "-z"])
            .output()?;
        if !untracked.status.success() {
            return Err(DiffError::Git(
                String::from_utf8_lossy(&untracked.stderr).trim().into(),
            ));
        }
        for bytes in untracked
            .stdout
            .split(|b| *b == 0)
            .filter(|b| !b.is_empty())
        {
            let name = String::from_utf8_lossy(bytes).into_owned();
            let path = cwd.join(&name);
            let metadata = match std::fs::symlink_metadata(&path) {
                Ok(m) => m,
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => continue,
                Err(e) => return Err(e.into()),
            };
            let text = if metadata.file_type().is_symlink() {
                std::fs::read_link(&path)
                    .map(|p| p.to_string_lossy().into_owned())
                    .ok()
            } else if metadata.is_file() && metadata.len() <= 1_000_000 {
                let mut text = String::new();
                std::fs::File::open(&path)
                    .and_then(|file| file.take(1_000_001).read_to_string(&mut text))
                    .ok()
                    .and_then(|_| (text.len() <= 1_000_000 && !text.contains('\0')).then_some(text))
            } else {
                None
            };
            let mut file = FileDiff {
                old_path: None,
                new_path: Some(name),
                hunks: Vec::new(),
            };
            if let Some(text) = text {
                let mut lines: Vec<_> = text
                    .lines()
                    .enumerate()
                    .map(|(i, line)| DiffLine {
                        kind: DiffLineType::Added,
                        content: line.into(),
                        old_line: None,
                        new_line: Some(i as u64 + 1),
                    })
                    .collect();
                let count = lines.len() as u64;
                if !text.is_empty() && !text.ends_with('\n') {
                    lines.push(DiffLine {
                        kind: DiffLineType::NoNewline,
                        content: " No newline at end of file".into(),
                        old_line: None,
                        new_line: None,
                    });
                }
                if count > 0 {
                    file.hunks.push(DiffHunk {
                        old_start: 0,
                        old_count: 0,
                        new_start: 1,
                        new_count: count,
                        header: format!("@@ -0,0 +1,{count} @@"),
                        lines,
                    });
                }
            }
            files.push(file);
        }
    }
    Ok(files)
}

fn range(s: &str) -> Result<(u64, u64), DiffError> {
    let mut parts = s[1..].split(',');
    let start = parts
        .next()
        .and_then(|v| v.parse().ok())
        .ok_or_else(|| DiffError::InvalidDiff(s.into()))?;
    let count = parts
        .next()
        .map_or(Ok(1), str::parse)
        .map_err(|_| DiffError::InvalidDiff(s.into()))?;
    Ok((start, count))
}
fn parse(text: &str) -> Result<Vec<FileDiff>, DiffError> {
    let mut files: Vec<FileDiff> = Vec::new();
    let (mut old, mut new) = (0, 0);
    for line in text.lines() {
        if line.starts_with("diff --git ") {
            files.push(FileDiff {
                old_path: None,
                new_path: None,
                hunks: Vec::new(),
            });
        } else if let Some(file) = files.last_mut() {
            if file.hunks.is_empty() && line.starts_with("--- ") {
                let path = line[4..].strip_suffix('\t').unwrap_or(&line[4..]);
                file.old_path = (path != "/dev/null").then(|| path.to_owned());
            } else if file.hunks.is_empty() && line.starts_with("+++ ") {
                let path = line[4..].strip_suffix('\t').unwrap_or(&line[4..]);
                file.new_path = (path != "/dev/null").then(|| path.to_owned());
            } else if line.starts_with("@@ ") {
                let mut parts = line.split_whitespace();
                parts.next();
                let (os, oc) = range(
                    parts
                        .next()
                        .ok_or_else(|| DiffError::InvalidDiff(line.into()))?,
                )?;
                let (ns, nc) = range(
                    parts
                        .next()
                        .ok_or_else(|| DiffError::InvalidDiff(line.into()))?,
                )?;
                old = os;
                new = ns;
                file.hunks.push(DiffHunk {
                    old_start: os,
                    old_count: oc,
                    new_start: ns,
                    new_count: nc,
                    header: line.into(),
                    lines: Vec::new(),
                });
            } else if let Some(hunk) = file.hunks.last_mut() {
                let (kind, o, n) = match line.as_bytes().first() {
                    Some(b' ') => {
                        let result = (DiffLineType::Context, Some(old), Some(new));
                        old += 1;
                        new += 1;
                        result
                    }
                    Some(b'-') => {
                        let result = (DiffLineType::Removed, Some(old), None);
                        old += 1;
                        result
                    }
                    Some(b'+') => {
                        let result = (DiffLineType::Added, None, Some(new));
                        new += 1;
                        result
                    }
                    Some(b'\\') => (DiffLineType::NoNewline, None, None),
                    _ => continue,
                };
                hunk.lines.push(DiffLine {
                    kind,
                    content: line[1..].to_owned(),
                    old_line: o,
                    new_line: n,
                });
            }
        }
    }
    Ok(files)
}
