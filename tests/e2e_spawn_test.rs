#![cfg(unix)]
use std::{
    process::{Command, Stdio},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

#[test]
fn auto_spawn_and_run_command() {
    let status = Command::new("cargo")
        .args(["build", "-p", "agmux-cli", "-p", "agmux-daemon"])
        .status()
        .expect("build binaries");
    assert!(status.success());
    let root = std::env::temp_dir().join(format!(
        "agmux-e2e-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let socket = root.join("daemon.sock");
    let token = root.join("auth_token");
    // Start the daemon explicitly so the test owns and reaps it.
    let mut daemon = Command::new("target/debug/agmuxd")
        .env("AGMUX_SOCK", &socket)
        .env("AGMUX_AUTH_TOKEN_PATH", &token)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    for _ in 0..60 {
        if socket.exists() {
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    let output = Command::new("target/debug/agmux")
        .env("AGMUX_SOCK", &socket)
        .env("AGMUX_AUTH_TOKEN_PATH", &token)
        .args(["run", "echo", "HELLO_AGMUX_PTY"])
        .stdin(Stdio::null())
        .output()
        .unwrap();
    daemon.kill().unwrap();
    daemon.wait().unwrap();
    std::fs::remove_dir_all(root).unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("HELLO_AGMUX_PTY"));
}

#[test]
fn pure_auto_spawn_and_run() {
    let status = Command::new("cargo")
        .args(["build", "-p", "agmux-cli", "-p", "agmux-daemon"])
        .status()
        .expect("build binaries");
    assert!(status.success());
    let root = std::env::temp_dir().join(format!(
        "agmux-autospawn-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let socket = root.join("autospawn_daemon.sock");
    let token = root.join("auth_token");

    // Do NOT start daemon manually. Let agmux auto-spawn it.
    let output = Command::new("target/debug/agmux")
        .env("AGMUX_SOCK", &socket)
        .env("AGMUX_AUTH_TOKEN_PATH", &token)
        .args(["run", "echo", "HELLO_AUTO_SPAWNED_PTY"])
        .stdin(Stdio::null())
        .output()
        .unwrap();

    let _ = std::fs::remove_dir_all(root);
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("HELLO_AUTO_SPAWNED_PTY"));
}
