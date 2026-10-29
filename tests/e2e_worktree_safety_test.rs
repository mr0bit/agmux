#![cfg(unix)]
use std::{
    path::Path,
    process::{Command, Stdio},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

fn git(root: &Path, args: &[&str]) {
    assert!(
        Command::new("git")
            .current_dir(root)
            .args(args)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .status()
            .unwrap()
            .success()
    );
}

#[test]
fn protects_dirty_worktree_and_stashes_on_forced_removal() {
    assert!(
        Command::new("cargo")
            .args(["build", "-p", "agmux-cli", "-p", "agmux-daemon"])
            .status()
            .unwrap()
            .success()
    );
    let binaries = std::env::current_dir().unwrap().join("target/debug");
    let root = std::env::temp_dir().join(format!(
        "agmux-worktree-{}-{}",
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
    std::fs::write(root.join("tracked"), "initial").unwrap();
    git(&root, &["add", "tracked"]);
    git(&root, &["commit", "-m", "initial"]);
    let socket = root.join("daemon.sock");
    let token = root.join("token");
    let mut daemon = Command::new(binaries.join("agmuxd"))
        .env("AGMUX_SOCK", &socket)
        .env("AGMUX_AUTH_TOKEN_PATH", &token)
        .env("AGMUX_DB_PATH", root.join("sessions.db"))
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    for _ in 0..100 {
        if socket.exists() {
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    let mut run = Command::new(binaries.join("agmux"))
        .current_dir(&root)
        .env("AGMUX_SOCK", &socket)
        .env("AGMUX_AUTH_TOKEN_PATH", &token)
        .args(["run", "--worktree", "test-branch", "sh", "-c", "sleep 30"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let id = format!("run-{}", run.id());
    let path = root.join(".agmux-worktrees/test-branch");
    for _ in 0..100 {
        if path.join("tracked").exists() {
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    // Wait until session metadata has been persisted.
    for _ in 0..100 {
        let list = Command::new(binaries.join("agmux"))
            .env("AGMUX_SOCK", &socket)
            .env("AGMUX_AUTH_TOKEN_PATH", &token)
            .args(["list", "--json"])
            .output()
            .unwrap();
        if String::from_utf8_lossy(&list.stdout).contains(&id) {
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    std::fs::write(path.join("tracked"), "changed").unwrap();
    std::fs::write(path.join("untracked"), "new file").unwrap();
    let remove = |force: bool| {
        let mut cmd = Command::new(binaries.join("agmux"));
        cmd.env("AGMUX_SOCK", &socket)
            .env("AGMUX_AUTH_TOKEN_PATH", &token)
            .args(["remove", &id]);
        if force {
            cmd.arg("--force");
        }
        cmd.output().unwrap()
    };
    let denied = remove(false);
    assert!(!denied.status.success());
    assert!(String::from_utf8_lossy(&denied.stderr).contains("dirty working tree"));
    assert_eq!(
        std::fs::read_to_string(path.join("tracked")).unwrap(),
        "changed"
    );
    let removed = remove(true);
    assert!(
        removed.status.success(),
        "{}",
        String::from_utf8_lossy(&removed.stderr)
    );
    let stash_name = String::from_utf8_lossy(&removed.stdout)
        .trim()
        .strip_prefix("Backup stash: ")
        .unwrap()
        .to_owned();
    let stash = Command::new("git")
        .current_dir(&root)
        .args(["stash", "list"])
        .output()
        .unwrap();
    assert!(String::from_utf8_lossy(&stash.stdout).contains(&stash_name));
    assert!(!path.exists());
    let _ = run.kill();
    let _ = run.wait();
    daemon.kill().unwrap();
    daemon.wait().unwrap();
    std::fs::remove_dir_all(root).unwrap();
}
