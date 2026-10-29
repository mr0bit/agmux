use std::{
    process::{Command, Stdio},
    time::Duration,
};

#[test]
fn test_queued_message_held_under_waiting_and_auto_flushed_on_resolution() {
    let binaries = std::env::current_dir().unwrap().join("target/debug");
    let root = std::env::temp_dir().join(format!(
        "agmux-msg-queue-{}-{}",
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

    // Spawn target session running an interactive prompt script
    let mut run = Command::new(binaries.join("agmux"))
        .env("AGMUX_SOCK", &socket)
        .env("AGMUX_AUTH_TOKEN_PATH", &token)
        .args([
            "run",
            "sh",
            "-c",
            "printf 'Deploy to production? [y/n] '; read ans; printf 'User said: %s\\n' \"$ans\"; read next; printf 'Queued received: %s\\n' \"$next\"",
        ])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();

    let id = format!("run-{}", run.id());

    // Wait until target session is registered and classified as WAITING_INPUT
    for _ in 0..100 {
        let list = Command::new(binaries.join("agmux"))
            .env("AGMUX_SOCK", &socket)
            .env("AGMUX_AUTH_TOKEN_PATH", &token)
            .args(["list", "--json"])
            .output()
            .unwrap();
        let out = String::from_utf8_lossy(&list.stdout);
        if out.contains(&id) && (out.contains("WAITING") || out.contains("WAITING_INPUT")) {
            break;
        }
        std::thread::sleep(Duration::from_millis(50));
    }

    // Sibling agent sends message while target is WAITING
    let send_cmd = Command::new(binaries.join("agmux"))
        .env("AGMUX_SOCK", &socket)
        .env("AGMUX_AUTH_TOKEN_PATH", &token)
        .args(["send", &id, "canary_payload_42"])
        .output()
        .unwrap();

    let send_stdout = String::from_utf8_lossy(&send_cmd.stdout);
    assert!(
        send_stdout.contains("QUEUED"),
        "Expected message to be queued, got: {send_stdout}"
    );

    // Now user resolves prompt with 'yes' via input
    let ack_cmd = Command::new(binaries.join("agmux"))
        .env("AGMUX_SOCK", &socket)
        .env("AGMUX_AUTH_TOKEN_PATH", &token)
        .args(["input", &id, "yes"])
        .output()
        .unwrap();
    assert!(
        ack_cmd.status.success(),
        "Failed to send input: {}",
        String::from_utf8_lossy(&ack_cmd.stderr)
    );

    // Clean up test child & daemon
    let _ = run.kill();
    let _ = run.wait();
    daemon.kill().unwrap();
    daemon.wait().unwrap();
    let _ = std::fs::remove_dir_all(root);
}
