use agmux_core::diff_calculator::{DiffLineType, DiffScope, calculate_diff};
use std::{
    path::Path,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};
fn git(p: &Path, args: &[&str]) -> String {
    let o = Command::new("git")
        .current_dir(p)
        .args(args)
        .output()
        .unwrap();
    assert!(o.status.success(), "{}", String::from_utf8_lossy(&o.stderr));
    String::from_utf8(o.stdout).unwrap().trim().into()
}
#[test]
fn five_scopes_and_line_positions() {
    let dir = std::env::temp_dir().join(format!(
        "agmux-diff-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&dir).unwrap();
    git(&dir, &["init", "-b", "main"]);
    git(&dir, &["config", "user.name", "Test"]);
    git(&dir, &["config", "user.email", "test@example.com"]);
    std::fs::write(dir.join("file"), "original\n").unwrap();
    git(&dir, &["add", "."]);
    git(&dir, &["commit", "-m", "base"]);
    let base = git(&dir, &["rev-parse", "HEAD"]);
    git(&dir, &["checkout", "-b", "feature"]);
    std::fs::write(dir.join("file"), "committed\n").unwrap();
    git(&dir, &["commit", "-am", "change"]);
    std::fs::write(dir.join("file"), "staged\n").unwrap();
    git(&dir, &["add", "file"]);
    std::fs::write(dir.join("file"), "unstaged\n").unwrap();
    for (scope, expected) in [
        (DiffScope::SessionDiff, "unstaged"),
        (DiffScope::FullUncommitted, "unstaged"),
        (DiffScope::VersusTarget, "unstaged"),
        (DiffScope::LastCommit, "committed"),
        (DiffScope::Staged, "staged"),
    ] {
        let files = calculate_diff(&dir, scope, Some(&base), Some("main")).unwrap();
        assert_eq!(files.len(), 1);
        assert_eq!(files[0].new_path.as_deref(), Some("file"));
        assert!(
            files[0].hunks[0]
                .lines
                .iter()
                .any(|l| l.kind == DiffLineType::Added
                    && l.content == expected
                    && l.new_line == Some(1))
        );
        assert!(
            files[0].hunks[0]
                .lines
                .iter()
                .any(|l| l.kind == DiffLineType::Removed && l.old_line == Some(1))
        );
    }
    std::fs::remove_dir_all(dir).unwrap();
}

#[test]
fn non_git_directory_is_an_explicit_state_without_initializing_it() {
    use agmux_core::diff_calculator::DiffError;
    let dir = std::env::temp_dir().join(format!("agmux-nongit-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    let result = calculate_diff(&dir, DiffScope::FullUncommitted, None, None);
    assert!(matches!(result, Err(DiffError::NotRepository)));
    assert!(!dir.join(".git").exists());
    std::fs::remove_dir_all(dir).unwrap();
}
