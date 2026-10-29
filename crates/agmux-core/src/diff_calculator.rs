//! Structured Git review diffs.
pub use agmux_proto::{DiffHunk, DiffLine, DiffLineType, DiffScope, FileDiff};
use std::{path::Path, process::Command};

#[derive(Debug, thiserror::Error)]
pub enum DiffError {
    #[error("Git I/O error: {0}")]
    Io(#[from] std::io::Error),
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
            command.arg("HEAD");
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
            command.args(["--cached", "HEAD"]);
        }
    }
    let output = command.arg("--").output()?;
    if !output.status.success() {
        return Err(DiffError::Git(
            String::from_utf8_lossy(&output.stderr).trim().to_owned(),
        ));
    }
    parse(&String::from_utf8_lossy(&output.stdout))
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
