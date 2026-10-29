#![cfg(unix)]
use std::{
    process::{Command, Stdio},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

#[test]
fn searches_ansi_terminal_output() {
    assert!(
        Command::new("cargo")
            .args(["build", "-p", "agmux-cli", "-p", "agmux-daemon"])
            .status()
            .unwrap()
            .success()
    );
    let root = std::env::temp_dir().join(format!(
        "agmux-search-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();
    let socket = root.join("daemon.sock");
    let token = root.join("token");
    let db = root.join("state.db");
    let mut daemon = Command::new("target/debug/agmuxd")
        .env("AGMUX_SOCK", &socket)
        .env("AGMUX_AUTH_TOKEN_PATH", &token)
        .env("AGMUX_DB_PATH", &db)
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
    let run_child = Command::new("target/debug/agmux")
        .env("AGMUX_SOCK", &socket)
        .env("AGMUX_AUTH_TOKEN_PATH", &token)
        .args([
            "run",
            "printf",
            "\u{1b}[31mCRITICAL_AGMUX_TOKEN_XYZ123\u{1b}[0m\n",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();
    let expected_session = format!("run-{}", run_child.id());
    let run = run_child.wait_with_output().unwrap();
    assert!(
        run.status.success(),
        "{}",
        String::from_utf8_lossy(&run.stderr)
    );
    let search = Command::new("target/debug/agmux")
        .env("AGMUX_SOCK", &socket)
        .env("AGMUX_AUTH_TOKEN_PATH", &token)
        .args(["search", "TOKEN_XYZ", "--json"])
        .output()
        .unwrap();
    daemon.kill().unwrap();
    daemon.wait().unwrap();
    std::fs::remove_dir_all(root).unwrap();
    assert!(
        search.status.success(),
        "{}",
        String::from_utf8_lossy(&search.stderr)
    );
    let matches: Vec<agmux_proto::SearchMatch> = serde_json::from_slice(&search.stdout).unwrap();
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].line_no, 1);
    assert_eq!(matches[0].session_id, expected_session);
    assert_eq!(matches[0].snippet, "CRITICAL_AGMUX_TOKEN_XYZ123");
}
