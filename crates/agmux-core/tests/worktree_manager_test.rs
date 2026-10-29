use agmux_core::worktree_manager::{provision_worktree, remove_worktree};
use std::{
    path::Path,
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

fn git(root: &Path, args: &[&str]) {
    let output = Command::new("git")
        .current_dir(root)
        .args(args)
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn provision_and_remove_worktree() {
    let root = std::env::temp_dir().join(format!(
        "agmux-core-wt-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    git(&root, &["init"]);
    git(&root, &["config", "user.email", "test@example.com"]);
    git(&root, &["config", "user.name", "Test"]);
    std::fs::write(root.join("tracked"), "committed").unwrap();
    git(&root, &["add", "tracked"]);
    git(&root, &["commit", "-m", "initial"]);
    std::fs::write(root.join(".worktreeinclude"), ".env\n*.local\ntracked\n").unwrap();
    std::fs::write(root.join(".env"), "secret").unwrap();
    std::fs::write(root.join("config.local"), "config").unwrap();
    std::fs::write(root.join("tracked"), "modified").unwrap();
    let info = provision_worktree(&root, "feature").unwrap();
    assert_eq!(
        std::fs::read_to_string(info.path.join(".env")).unwrap(),
        "secret"
    );
    assert_eq!(
        std::fs::read_to_string(info.path.join("config.local")).unwrap(),
        "config"
    );
    assert_eq!(
        std::fs::read_to_string(info.path.join("tracked")).unwrap(),
        "committed"
    );
    assert!((8000..9000).contains(&info.port));
    remove_worktree(&root, &info.path).unwrap();
    let again = provision_worktree(&root, "feature").unwrap();
    assert_eq!(info.port, again.port);
    remove_worktree(&root, &again.path).unwrap();
    assert!(provision_worktree(&root, "../escape").is_err());
    std::fs::remove_dir_all(root).unwrap();
}
