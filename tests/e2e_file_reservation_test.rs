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
fn test_cross_worktree_file_reservation_prefix_exclusion() {
    let binaries = std::env::current_dir().unwrap().join("target/debug");
    let root = std::env::temp_dir().join(format!(
        "agmux-file-res-{}-{}",
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

    let core_src = root.join("crates/core/src");
    std::fs::create_dir_all(&core_src).unwrap();
    std::fs::write(core_src.join("state.rs"), "// state\n").unwrap();
    git(&root, &["add", "."]);
    git(&root, &["commit", "-m", "initial structure"]);

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

    // 1. Spawn Agent A in worktree 1
    let mut agent_a_proc = Command::new(binaries.join("agmux"))
        .current_dir(&root)
        .env("AGMUX_SOCK", &socket)
        .env("AGMUX_AUTH_TOKEN_PATH", &token)
        .args(["run", "--worktree", "feature-a", "sh", "-c", "sleep 60"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();

    // 2. Spawn Agent B in worktree 2
    let mut agent_b_proc = Command::new(binaries.join("agmux"))
        .current_dir(&root)
        .env("AGMUX_SOCK", &socket)
        .env("AGMUX_AUTH_TOKEN_PATH", &token)
        .args(["run", "--worktree", "feature-b", "sh", "-c", "sleep 60"])
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .spawn()
        .unwrap();

    // Discover worktree session details from agmux list
    let mut agent_a_id = String::new();
    let mut agent_b_id = String::new();
    let mut agent_a_cwd = String::new();
    let mut agent_b_cwd = String::new();

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
                for s in &sessions {
                    let cwd = s["cwd"].as_str().unwrap_or_default();
                    if cwd.contains("feature-a") {
                        agent_a_id = s["session_id"].as_str().unwrap().to_string();
                        agent_a_cwd = cwd.to_string();
                    } else if cwd.contains("feature-b") {
                        agent_b_id = s["session_id"].as_str().unwrap().to_string();
                        agent_b_cwd = cwd.to_string();
                    }
                }
                if !agent_a_id.is_empty() && !agent_b_id.is_empty() {
                    break;
                }
            }
        }
        std::thread::sleep(Duration::from_millis(50));
    }
    assert!(
        !agent_a_id.is_empty(),
        "Agent A must be running in worktree 1"
    );
    assert!(
        !agent_b_id.is_empty(),
        "Agent B must be running in worktree 2"
    );

    // 3. Agent A in worktree 1 reserves "crates/core/src/"
    let res_a = Command::new(binaries.join("agmux"))
        .current_dir(Path::new(&agent_a_cwd))
        .env("AGMUX_SOCK", &socket)
        .env("AGMUX_AUTH_TOKEN_PATH", &token)
        .args(["reserve", "crates/core/src/", "--session", &agent_a_id])
        .output()
        .unwrap();
    assert!(
        res_a.status.success(),
        "Agent A should successfully reserve crates/core/src/. Stderr: {}",
        String::from_utf8_lossy(&res_a.stderr)
    );

    // 4. Agent B in worktree 2 attempts to reserve "crates/core/src/state.rs"
    // Must be rejected with an active reservation conflict error!
    let res_b = Command::new(binaries.join("agmux"))
        .current_dir(Path::new(&agent_b_cwd))
        .env("AGMUX_SOCK", &socket)
        .env("AGMUX_AUTH_TOKEN_PATH", &token)
        .args([
            "reserve",
            "crates/core/src/state.rs",
            "--session",
            &agent_b_id,
        ])
        .output()
        .unwrap();
    assert!(
        !res_b.status.success(),
        "Agent B's overlapping reservation must fail"
    );
    let res_b_err = String::from_utf8_lossy(&res_b.stderr);
    assert!(
        res_b_err.contains("Active reservation conflict") && res_b_err.contains("crates/core/src"),
        "Error must detail active conflict: {res_b_err}"
    );

    // 5. Inspect active reservations via agmux reservations
    let list_res = Command::new(binaries.join("agmux"))
        .env("AGMUX_SOCK", &socket)
        .env("AGMUX_AUTH_TOKEN_PATH", &token)
        .args(["reservations", "--json"])
        .output()
        .unwrap();
    assert!(list_res.status.success());
    let list_json = String::from_utf8_lossy(&list_res.stdout);
    assert!(list_json.contains("crates/core/src"));
    assert!(list_json.contains(&agent_a_id));

    // 6. Agent A releases "crates/core/src/"
    let rel_a = Command::new(binaries.join("agmux"))
        .current_dir(Path::new(&agent_a_cwd))
        .env("AGMUX_SOCK", &socket)
        .env("AGMUX_AUTH_TOKEN_PATH", &token)
        .args(["release", "crates/core/src/", "--session", &agent_a_id])
        .output()
        .unwrap();
    assert!(rel_a.status.success());

    // 7. Agent B can now reserve "crates/core/src/state.rs"
    let res_b_retry = Command::new(binaries.join("agmux"))
        .current_dir(Path::new(&agent_b_cwd))
        .env("AGMUX_SOCK", &socket)
        .env("AGMUX_AUTH_TOKEN_PATH", &token)
        .args([
            "reserve",
            "crates/core/src/state.rs",
            "--session",
            &agent_b_id,
        ])
        .output()
        .unwrap();
    assert!(
        res_b_retry.status.success(),
        "Agent B should succeed after Agent A released lock"
    );

    let _ = agent_a_proc.kill();
    let _ = agent_b_proc.kill();
    let _ = daemon.kill();
    let _ = std::fs::remove_dir_all(&root);
}
