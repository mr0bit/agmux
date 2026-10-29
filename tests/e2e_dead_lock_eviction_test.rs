use std::{
    process::{Command, Stdio},
    time::Duration,
};

#[test]
fn test_dead_lock_auto_eviction_wakes_waiting_agent() {
    let binaries = std::env::current_dir().unwrap().join("target/debug");
    let root = std::env::temp_dir().join(format!(
        "agmux-dead-lock-{}-{}",
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

    // 1. Spawn Agent 1 session running a long-running process
    let mut agent1_proc = Command::new(binaries.join("agmux"))
        .env("AGMUX_SOCK", &socket)
        .env("AGMUX_AUTH_TOKEN_PATH", &token)
        .args(["run", "sh", "-c", "sleep 60"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();

    // Discover Agent 1 session id and PID from agmux list --json
    let mut agent1_id = String::new();
    let mut agent1_pid = 0u32;
    for _ in 0..100 {
        let list_out = Command::new(binaries.join("agmux"))
            .env("AGMUX_SOCK", &socket)
            .env("AGMUX_AUTH_TOKEN_PATH", &token)
            .args(["list", "--json"])
            .output()
            .unwrap();
        if list_out.status.success() {
            let json_str = String::from_utf8_lossy(&list_out.stdout);
            if let Ok(sessions) = serde_json::from_str::<Vec<serde_json::Value>>(&json_str) {
                if let Some(s) = sessions.iter().find(|s| s["state"] == "RUNNING") {
                    agent1_id = s["session_id"].as_str().unwrap().to_string();
                    agent1_pid = s["pid"].as_u64().unwrap() as u32;
                    break;
                }
            }
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    assert!(
        !agent1_id.is_empty(),
        "Agent 1 session must be registered and running"
    );
    assert!(agent1_pid > 0, "Agent 1 PID must be known");

    // 2. Agent 1 acquires named lock "db_migrate"
    let acq1 = Command::new(binaries.join("agmux"))
        .env("AGMUX_SOCK", &socket)
        .env("AGMUX_AUTH_TOKEN_PATH", &token)
        .args(["lock", "acquire", "db_migrate", "--session", &agent1_id])
        .output()
        .unwrap();
    assert!(acq1.status.success(), "Agent 1 should acquire lock");
    let acq1_str = String::from_utf8_lossy(&acq1.stdout);
    assert!(
        acq1_str.contains("acquired by session"),
        "stdout should confirm acquisition: {acq1_str}"
    );

    // Verify lock list shows Agent 1 holding db_migrate
    let list_locks_out = Command::new(binaries.join("agmux"))
        .env("AGMUX_SOCK", &socket)
        .env("AGMUX_AUTH_TOKEN_PATH", &token)
        .args(["lock", "list", "--json"])
        .output()
        .unwrap();
    assert!(list_locks_out.status.success());
    let list_json = String::from_utf8_lossy(&list_locks_out.stdout);
    assert!(list_json.contains("db_migrate"));
    assert!(list_json.contains(&agent1_id));

    // 3. Agent 2 calls `agmux lock acquire db_migrate --wait --session agent-2-worker`
    // This should block until the lock becomes available.
    let mut agent2_waiter = Command::new(binaries.join("agmux"))
        .env("AGMUX_SOCK", &socket)
        .env("AGMUX_AUTH_TOKEN_PATH", &token)
        .args([
            "lock",
            "acquire",
            "db_migrate",
            "--wait",
            "--session",
            "agent-2-worker",
        ])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .unwrap();

    // Verify Agent 2 is waiting (still running after 200ms)
    std::thread::sleep(Duration::from_millis(200));
    match agent2_waiter.try_wait().unwrap() {
        Some(status) => {
            panic!("Agent 2 should be blocked waiting for lock, but exited with {status:?}")
        }
        None => {
            // As expected, still waiting!
        }
    }

    // 4. Kill Agent 1 with SIGKILL (kill -9)
    #[cfg(unix)]
    let _ = Command::new("kill")
        .args(["-9", &agent1_pid.to_string()])
        .status();

    // 5. Verify that agmuxd evicts Agent 1's lock and Agent 2 immediately acquires it
    let agent2_output = agent2_waiter.wait_with_output().unwrap();
    assert!(
        agent2_output.status.success(),
        "Agent 2 must acquire lock after Agent 1 died. Stderr: {}",
        String::from_utf8_lossy(&agent2_output.stderr)
    );
    let agent2_str = String::from_utf8_lossy(&agent2_output.stdout);
    assert!(
        agent2_str.contains("Lock 'db_migrate' acquired by session 'agent-2-worker'"),
        "Agent 2 output should confirm acquisition: {agent2_str}"
    );

    // Verify lock list now shows agent-2-worker as holder
    let list_after = Command::new(binaries.join("agmux"))
        .env("AGMUX_SOCK", &socket)
        .env("AGMUX_AUTH_TOKEN_PATH", &token)
        .args(["lock", "list", "--json"])
        .output()
        .unwrap();
    let after_json = String::from_utf8_lossy(&list_after.stdout);
    assert!(after_json.contains("agent-2-worker"));

    // 6. Agent 2 releases the lock
    let rel_out = Command::new(binaries.join("agmux"))
        .env("AGMUX_SOCK", &socket)
        .env("AGMUX_AUTH_TOKEN_PATH", &token)
        .args([
            "lock",
            "release",
            "db_migrate",
            "--session",
            "agent-2-worker",
        ])
        .output()
        .unwrap();
    assert!(rel_out.status.success());

    // Verify no active locks remain
    let list_empty = Command::new(binaries.join("agmux"))
        .env("AGMUX_SOCK", &socket)
        .env("AGMUX_AUTH_TOKEN_PATH", &token)
        .args(["lock", "list"])
        .output()
        .unwrap();
    let empty_str = String::from_utf8_lossy(&list_empty.stdout);
    assert!(empty_str.contains("No active locks"));

    let _ = agent1_proc.kill();
    let _ = daemon.kill();
    let _ = std::fs::remove_dir_all(&root);
}
