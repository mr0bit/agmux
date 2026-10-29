#![cfg(unix)]
use std::{
    process::{Command, Stdio},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

#[test]
fn lists_running_session_and_removes_it_after_exit() {
    assert!(
        Command::new("cargo")
            .args(["build", "-p", "agmux-cli", "-p", "agmux-daemon"])
            .status()
            .unwrap()
            .success()
    );
    let root = std::env::temp_dir().join(format!(
        "agmux-list-{}-{}",
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
        .args([
            "--socket",
            socket.to_str().unwrap(),
            "--token-path",
            token.to_str().unwrap(),
        ])
        .env("AGMUX_DB_PATH", &db)
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    for _ in 0..100 {
        if socket.exists() {
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    let cli = || {
        let mut c = Command::new("target/debug/agmux");
        c.args([
            "--socket",
            socket.to_str().unwrap(),
            "--token-path",
            token.to_str().unwrap(),
        ]);
        c
    };
    let mut run = cli()
        .args(["run", "sleep", "2"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();
    let mut found = None;
    for _ in 0..60 {
        let output = cli().args(["list", "--json"]).output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let sessions: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        if let Some(session) = sessions.as_array().unwrap().first() {
            found = Some(session.clone());
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    let session = found.expect("session appeared");
    assert_eq!(session["command"], "sleep 2");
    let pid = session["pid"].as_u64().unwrap();
    assert!(pid > 0);
    #[cfg(target_os = "linux")]
    assert!(std::path::Path::new(&format!("/proc/{pid}")).exists());
    let table = cli().arg("list").output().unwrap();
    let table = String::from_utf8(table.stdout).unwrap();
    assert!(
        table.contains("SESSION ID")
            && table.contains("sleep 2")
            && table.contains(&pid.to_string())
    );
    assert!(run.wait().unwrap().success());
    for _ in 0..60 {
        let output = cli().args(["list", "--json"]).output().unwrap();
        let sessions: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        if sessions.as_array().unwrap().is_empty() {
            break;
        }
        std::thread::sleep(Duration::from_millis(20));
    }
    let output = cli().args(["list", "--json"]).output().unwrap();
    assert_eq!(
        serde_json::from_slice::<serde_json::Value>(&output.stdout)
            .unwrap()
            .as_array()
            .unwrap()
            .len(),
        0
    );
    daemon.kill().unwrap();
    daemon.wait().unwrap();
    std::fs::remove_dir_all(root).unwrap();
}
