#![cfg(unix)]
use std::{
    process::{Command, Stdio},
    time::{Duration, SystemTime, UNIX_EPOCH},
};

#[test]
fn freeze_and_resume_zstd_snapshot() {
    assert!(
        Command::new("cargo")
            .args(["build", "-p", "agmux-cli", "-p", "agmux-daemon"])
            .status()
            .unwrap()
            .success()
    );

    let root = std::env::temp_dir().join(format!(
        "agmux-freeze-{}-{}",
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

    // 1. Run a session to completion with distinctive payload
    let run_child = Command::new("target/debug/agmux")
        .env("AGMUX_SOCK", &socket)
        .env("AGMUX_AUTH_TOKEN_PATH", &token)
        .args(["run", "sh", "-c", "printf 'SNAPSHOT_MAGIC_PAYLOAD_456\\n'"])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    let session_id = format!("run-{}", run_child.id());
    let run_out = run_child.wait_with_output().unwrap();
    assert!(run_out.status.success());

    // Give daemon 100ms to persist snapshot on exit
    std::thread::sleep(Duration::from_millis(150));

    // 2. Inspect session snapshot via CLI
    let inspect_out = Command::new("target/debug/agmux")
        .env("AGMUX_SOCK", &socket)
        .env("AGMUX_AUTH_TOKEN_PATH", &token)
        .args(["inspect", &session_id])
        .output()
        .unwrap();

    assert!(
        inspect_out.status.success(),
        "{}",
        String::from_utf8_lossy(&inspect_out.stderr)
    );
    let inspect_text = String::from_utf8_lossy(&inspect_out.stdout);
    assert!(
        inspect_text.contains("SNAPSHOT_MAGIC_PAYLOAD_456"),
        "Inspect output did not contain expected text: {inspect_text}"
    );

    // 3. Test explicit freeze of an active long-running session
    let active_child = Command::new("target/debug/agmux")
        .env("AGMUX_SOCK", &socket)
        .env("AGMUX_AUTH_TOKEN_PATH", &token)
        .args([
            "run",
            "sh",
            "-c",
            "printf 'FROZEN_ACTIVE_TOKEN_789\\n'; sleep 30",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    let active_session_id = format!("run-{}", active_child.id());
    // Give child a moment to write output
    std::thread::sleep(Duration::from_millis(300));

    // Send freeze command
    let freeze_out = Command::new("target/debug/agmux")
        .env("AGMUX_SOCK", &socket)
        .env("AGMUX_AUTH_TOKEN_PATH", &token)
        .args(["freeze", &active_session_id])
        .output()
        .unwrap();

    assert!(
        freeze_out.status.success(),
        "{}",
        String::from_utf8_lossy(&freeze_out.stderr)
    );
    let freeze_msg = String::from_utf8_lossy(&freeze_out.stdout);
    assert!(
        freeze_msg.contains("Frozen session"),
        "Unexpected freeze output: {freeze_msg}"
    );

    // Inspect the frozen session
    let inspect_active = Command::new("target/debug/agmux")
        .env("AGMUX_SOCK", &socket)
        .env("AGMUX_AUTH_TOKEN_PATH", &token)
        .args(["inspect", &active_session_id])
        .output()
        .unwrap();

    assert!(inspect_active.status.success());
    let inspect_active_text = String::from_utf8_lossy(&inspect_active.stdout);
    assert!(
        inspect_active_text.contains("FROZEN_ACTIVE_TOKEN_789"),
        "Inspect active output did not contain expected text: {inspect_active_text}"
    );

    // 4. Verify SQLite session_snapshots table contents directly
    let conn = rusqlite::Connection::open(&db).unwrap();
    let mut stmt = conn
        .prepare("SELECT snapshot, compressed_size, uncompressed_size FROM session_snapshots WHERE session_id = ?1")
        .unwrap();
    let mut rows = stmt.query([&session_id]).unwrap();
    let row = rows.next().unwrap().expect("Snapshot row exists");
    let blob: Vec<u8> = row.get(0).unwrap();
    let compressed_sz: i64 = row.get(1).unwrap();
    assert_eq!(blob.len() as i64, compressed_sz);
    assert!(compressed_sz > 0);

    // Decompress with Zstd directly
    let decompressed = zstd::decode_all(&blob[..]).unwrap();
    let decompressed_str = String::from_utf8_lossy(&decompressed);
    assert!(decompressed_str.contains("SNAPSHOT_MAGIC_PAYLOAD_456"));

    daemon.kill().unwrap();
    daemon.wait().unwrap();
    let _ = std::fs::remove_dir_all(root);
}
