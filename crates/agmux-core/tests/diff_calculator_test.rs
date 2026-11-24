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

#[test]
fn untracked_and_unborn_head_changes_are_visible() {
    let dir = std::env::temp_dir().join(format!("agmux-new-diff-{}", std::process::id()));
    std::fs::create_dir_all(&dir).unwrap();
    git(&dir, &["init", "-b", "main"]);
    std::fs::write(dir.join("new.txt"), "new content\n").unwrap();
    std::fs::write(dir.join(".gitignore"), "*.bin\n").unwrap();
    std::fs::write(dir.join("ignored.bin"), "not reviewed").unwrap();
    std::fs::write(dir.join("staged.txt"), "indexed text\n").unwrap();
    git(&dir, &["add", "staged.txt"]);
    std::fs::write(dir.join("staged.txt"), "working text\n").unwrap();
    std::fs::write(dir.join("large.txt"), vec![b'x'; 1_000_001]).unwrap();
    std::fs::write(dir.join("binary.dat"), [0, 1, 2]).unwrap();
    let files = calculate_diff(&dir, DiffScope::FullUncommitted, None, None).unwrap();
    let staged = files
        .iter()
        .find(|f| f.new_path.as_deref() == Some("staged.txt"))
        .unwrap();
    assert_eq!(staged.hunks[0].lines[0].content, "working text");
    for name in ["large.txt", "binary.dat"] {
        assert!(
            files
                .iter()
                .find(|f| f.new_path.as_deref() == Some(name))
                .unwrap()
                .hunks
                .is_empty()
        );
    }
    assert!(
        !files
            .iter()
            .any(|f| f.new_path.as_deref() == Some("ignored.bin"))
    );
    let file = files
        .iter()
        .find(|f| f.new_path.as_deref() == Some("new.txt"))
        .unwrap();
    assert_eq!(file.hunks[0].lines[0].content, "new content");
    assert_eq!(file.old_path, None);
    std::fs::remove_dir_all(dir).unwrap();
}
