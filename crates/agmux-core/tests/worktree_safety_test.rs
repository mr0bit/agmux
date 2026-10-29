use agmux_core::worktree_manager::{WorktreeError, provision_worktree, safe_remove_worktree};
use std::process::Command;

#[test]
fn dirty_checkout_requires_force_and_is_stashed() {
    let root = std::env::temp_dir().join(format!(
        "agmux-safety-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let git = |args: &[&str]| {
        let out = Command::new("git")
            .current_dir(&root)
            .args(args)
            .output()
            .unwrap();
        assert!(
            out.status.success(),
            "{}",
            String::from_utf8_lossy(&out.stderr)
        );
        out
    };
    git(&["init"]);
    git(&["config", "user.email", "test@example.com"]);
    git(&["config", "user.name", "Test"]);
    std::fs::write(root.join("tracked"), "original").unwrap();
    git(&["add", "tracked"]);
    git(&["commit", "-m", "initial"]);
    let worktree = provision_worktree(&root, "test-branch").unwrap().path;
    std::fs::write(worktree.join("tracked"), "modified").unwrap();
    std::fs::write(worktree.join("untracked"), "new").unwrap();
    assert!(matches!(
        safe_remove_worktree(&root, &worktree, false),
        Err(WorktreeError::DirtyWorkingTree(_))
    ));
    assert_eq!(
        std::fs::read_to_string(worktree.join("tracked")).unwrap(),
        "modified"
    );
    let stash = safe_remove_worktree(&root, &worktree, true)
        .unwrap()
        .unwrap();
    assert!(String::from_utf8_lossy(&git(&["stash", "list"]).stdout).contains(&stash));
    assert!(!worktree.exists());
    std::fs::remove_dir_all(root).unwrap();
}
