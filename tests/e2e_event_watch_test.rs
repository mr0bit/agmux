#![cfg(unix)]
use std::{
    process::{Command, Stdio},
    time::Duration,
};

#[test]
fn test_event_watch_reconnection_replay_with_cursor() {
    let binaries = std::env::current_dir().unwrap().join("target/debug");
    let root = std::env::temp_dir().join(format!(
        "agmux-watch-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    std::fs::create_dir_all(&root).unwrap();

    let socket = root.join("daemon.sock");
    let token = root.join("token");
    let db_path = root.join("sessions.db");

    let mut daemon = Command::new(binaries.join("agmuxd"))
        .env("AGMUX_SOCK", &socket)
        .env("AGMUX_AUTH_TOKEN_PATH", &token)
        .env("AGMUX_DB_PATH", &db_path)
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

    // 1. Initial watch connection with named cursor "orch_cursor" (initializes cursor position)
    let init_watch = Command::new(binaries.join("agmux"))
        .env("AGMUX_SOCK", &socket)
        .env("AGMUX_AUTH_TOKEN_PATH", &token)
        .args(["watch", "--as", "orch_cursor", "--json"])
        .output()
        .unwrap();
    if !init_watch.status.success() {
        eprintln!(
            "init_watch stderr: {}",
            String::from_utf8_lossy(&init_watch.stderr)
        );
    }
    assert!(init_watch.status.success());

    // 2. Client is now disconnected while away.
    // Trigger 5 session lifecycle state transitions by spawning 5 quick tasks.
    for i in 1..=5 {
        let child = Command::new(binaries.join("agmux"))
            .env("AGMUX_SOCK", &socket)
            .env("AGMUX_AUTH_TOKEN_PATH", &token)
            .args(["run", "echo", &format!("task-event-{i}")])
            .output()
            .unwrap();
        assert!(child.status.success());
    }

    // Give daemon a brief moment to finish persisting transitions
    std::thread::sleep(Duration::from_millis(200));

    // 3. Reconnect `agmux watch --as orch_cursor --json`
    let reconnect_watch = Command::new(binaries.join("agmux"))
        .env("AGMUX_SOCK", &socket)
        .env("AGMUX_AUTH_TOKEN_PATH", &token)
        .args(["watch", "--as", "orch_cursor", "--json"])
        .output()
        .unwrap();
    assert!(
        reconnect_watch.status.success(),
        "Watch reconnect failed: {}",
        String::from_utf8_lossy(&reconnect_watch.stderr)
    );

    let stdout_str = String::from_utf8_lossy(&reconnect_watch.stdout);
    let lines: Vec<&str> = stdout_str
        .lines()
        .filter(|l| !l.trim().is_empty())
        .collect();
    assert!(
        lines.len() >= 5,
        "Expected at least 5 missed events replayed upon reconnection, got {}: {}",
        lines.len(),
        stdout_str
    );

    let mut last_seq = 0u64;
    for line in lines {
        let frame: serde_json::Value = serde_json::from_str(line).expect("valid json frame");
        assert_eq!(
            frame["changed_while_away"], true,
            "Replayed frame must have changed_while_away: true"
        );
        let ev = &frame["event"];
        let seq = ev["seq"].as_u64().expect("seq is u64");
        assert!(
            seq > last_seq,
            "Sequences must be monotonically strictly increasing: {seq} > {last_seq}"
        );
        last_seq = seq;
    }

    // 4. Immediate second reconnect with same cursor: all events were already replayed & cursor updated
    let empty_watch = Command::new(binaries.join("agmux"))
        .env("AGMUX_SOCK", &socket)
        .env("AGMUX_AUTH_TOKEN_PATH", &token)
        .args(["watch", "--as", "orch_cursor", "--json"])
        .output()
        .unwrap();
    assert!(empty_watch.status.success());
    let empty_stdout = String::from_utf8_lossy(&empty_watch.stdout);
    let empty_lines: Vec<&str> = empty_stdout
        .lines()
        .filter(|l| !l.trim().is_empty())
        .collect();
    assert_eq!(
        empty_lines.len(),
        0,
        "No missed events should remain after cursor acknowledgment"
    );

    let _ = daemon.kill();
    let _ = daemon.wait();
    let _ = std::fs::remove_dir_all(&root);
}
