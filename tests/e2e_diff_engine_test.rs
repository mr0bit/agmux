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
fn diff_over_ipc() {
    assert!(
        Command::new("cargo")
            .args(["build", "-p", "agmux-cli", "-p", "agmux-daemon"])
            .status()
            .unwrap()
            .success()
    );
    let bin = std::env::current_dir().unwrap().join("target/debug");
    let root = std::env::temp_dir().join(format!(
        "agmux-diff-e2e-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    git(&root, &["init"]);
    git(&root, &["config", "user.name", "Test"]);
    git(&root, &["config", "user.email", "test@example.com"]);
    std::fs::write(root.join("file"), "before\n").unwrap();
    git(&root, &["add", "file"]);
    git(&root, &["commit", "-m", "initial"]);
    let socket = root.join("socket");
    let token = root.join("token");
    let mut daemon = Command::new(bin.join("agmuxd"))
        .env("AGMUX_SOCK", &socket)
        .env("AGMUX_AUTH_TOKEN_PATH", &token)
        .env("AGMUX_DB_PATH", root.join("db"))
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
    let mut session = Command::new(bin.join("agmux"))
        .current_dir(&root)
        .env("AGMUX_SOCK", &socket)
        .env("AGMUX_AUTH_TOKEN_PATH", &token)
        .args(["run", "sh", "-c", "sleep 2"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let id = format!("run-{}", session.id());
    std::thread::sleep(Duration::from_millis(500));
    std::fs::write(root.join("file"), "after\n").unwrap();
    let output = Command::new(bin.join("agmux"))
        .current_dir(&root)
        .env("AGMUX_SOCK", &socket)
        .env("AGMUX_AUTH_TOKEN_PATH", &token)
        .args(["diff", &id, "--json"])
        .output()
        .unwrap();
    let _ = session.kill();
    let _ = session.wait();
    let _ = daemon.kill();
    let _ = daemon.wait();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let files: Vec<agmux_proto::FileDiff> = serde_json::from_slice(&output.stdout).unwrap();
    assert!(
        files[0].hunks[0]
            .lines
            .iter()
            .any(|l| l.content == "after" && l.new_line == Some(1))
    );
    std::fs::remove_dir_all(root).unwrap();
}
