//! Verification Test: `TEST-VER-ORCH-005`
//! Upstream Requirement: `REQ-ORCH-KANBAN-REPORT-005`
//! Downstream LLD: `LLD-ORCH-KANBAN-REPORT-005`
//!
//! Verifies that Agent 1 claims task `T-101`, posts a structured report with PR URL
//! and head commit SHA, marks task finished, and that the task status and report
//! are queryable via `agmux task list --json` and `agmux report view T-101`.

#![cfg(unix)]
use std::{
    process::{Command, Stdio},
    time::Duration,
};

#[test]
fn test_kanban_task_lifecycle_and_worker_report() {
    let binaries = std::env::current_dir().unwrap().join("target/debug");
    let root = std::env::temp_dir().join(format!(
        "agmux-kanban-{}-{}",
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

    // 1. Create task T-101 explicitly (or let claim auto-create, test both!)
    let create_out = Command::new(binaries.join("agmux"))
        .env("AGMUX_SOCK", &socket)
        .env("AGMUX_AUTH_TOKEN_PATH", &token)
        .args([
            "task",
            "create",
            "T-101",
            "Implement Kanban & Reporting",
            "--description",
            "Add SQLite Kanban board and worker reports",
        ])
        .output()
        .unwrap();
    assert!(
        create_out.status.success(),
        "task create failed: {}",
        String::from_utf8_lossy(&create_out.stderr)
    );

    // 2. Agent 1 claims task T-101
    let claim_out = Command::new(binaries.join("agmux"))
        .env("AGMUX_SOCK", &socket)
        .env("AGMUX_AUTH_TOKEN_PATH", &token)
        .args(["task", "claim", "T-101", "--session", "agent-1"])
        .output()
        .unwrap();
    assert!(
        claim_out.status.success(),
        "task claim failed: {}",
        String::from_utf8_lossy(&claim_out.stderr)
    );

    // Verify Agent 2 cannot claim T-101 while in_progress by Agent 1
    let conflict_claim = Command::new(binaries.join("agmux"))
        .env("AGMUX_SOCK", &socket)
        .env("AGMUX_AUTH_TOKEN_PATH", &token)
        .args(["task", "claim", "T-101", "--session", "agent-2"])
        .output()
        .unwrap();
    assert!(
        !conflict_claim.status.success(),
        "Competing claim by agent-2 should be rejected"
    );

    // Exercise `agmux task release` and re-claim
    let release_out = Command::new(binaries.join("agmux"))
        .env("AGMUX_SOCK", &socket)
        .env("AGMUX_AUTH_TOKEN_PATH", &token)
        .args(["task", "release", "T-101", "--session", "agent-1"])
        .output()
        .unwrap();
    assert!(
        release_out.status.success(),
        "task release failed: {}",
        String::from_utf8_lossy(&release_out.stderr)
    );
    let reclaim_out = Command::new(binaries.join("agmux"))
        .env("AGMUX_SOCK", &socket)
        .env("AGMUX_AUTH_TOKEN_PATH", &token)
        .args(["task", "claim", "T-101", "--session", "agent-1"])
        .output()
        .unwrap();
    assert!(reclaim_out.status.success());

    // 3. Agent 1 posts a structured report with PR URL and head commit SHA
    let pr_url = "https://github.com/org/agmux/pull/101";
    let commit_sha = "deadbeefcafe1234567890abcdef1234567890ab";
    let report_post = Command::new(binaries.join("agmux"))
        .env("AGMUX_SOCK", &socket)
        .env("AGMUX_AUTH_TOKEN_PATH", &token)
        .args([
            "report",
            "post",
            "T-101",
            "--session",
            "agent-1",
            "--summary",
            "Implemented Kanban task board and worker reporting over IPC.",
            "--pr-url",
            pr_url,
            "--commit-sha",
            commit_sha,
        ])
        .output()
        .unwrap();
    assert!(
        report_post.status.success(),
        "report post failed: {}",
        String::from_utf8_lossy(&report_post.stderr)
    );

    // 4. Agent 1 marks task T-101 finished
    let finish_out = Command::new(binaries.join("agmux"))
        .env("AGMUX_SOCK", &socket)
        .env("AGMUX_AUTH_TOKEN_PATH", &token)
        .args(["task", "finish", "T-101", "--session", "agent-1"])
        .output()
        .unwrap();
    assert!(
        finish_out.status.success(),
        "task finish failed: {}",
        String::from_utf8_lossy(&finish_out.stderr)
    );

    // 5. Verify task status via `agmux task list --json`
    let list_out = Command::new(binaries.join("agmux"))
        .env("AGMUX_SOCK", &socket)
        .env("AGMUX_AUTH_TOKEN_PATH", &token)
        .args(["task", "list", "--json"])
        .output()
        .unwrap();
    assert!(
        list_out.status.success(),
        "task list --json failed: {}",
        String::from_utf8_lossy(&list_out.stderr)
    );
    let tasks_json: serde_json::Value =
        serde_json::from_slice(&list_out.stdout).expect("valid JSON from task list --json");
    let tasks = tasks_json.as_array().expect("tasks array");
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0]["id"], "T-101");
    assert_eq!(tasks[0]["status"], "done");
    assert_eq!(tasks[0]["claimed_by"], "agent-1");

    // 6. Verify report is queryable via `agmux report view T-101` (human-readable and `--json`)
    let view_text_out = Command::new(binaries.join("agmux"))
        .env("AGMUX_SOCK", &socket)
        .env("AGMUX_AUTH_TOKEN_PATH", &token)
        .args(["report", "view", "T-101"])
        .output()
        .unwrap();
    assert!(
        view_text_out.status.success(),
        "report view T-101 failed: {}",
        String::from_utf8_lossy(&view_text_out.stderr)
    );
    let view_text = String::from_utf8_lossy(&view_text_out.stdout);
    assert!(view_text.contains("T-101"));
    assert!(view_text.contains(pr_url));
    assert!(view_text.contains(commit_sha));

    let view_json_out = Command::new(binaries.join("agmux"))
        .env("AGMUX_SOCK", &socket)
        .env("AGMUX_AUTH_TOKEN_PATH", &token)
        .args(["report", "view", "T-101", "--json"])
        .output()
        .unwrap();
    assert!(view_json_out.status.success());
    let reports_json: serde_json::Value =
        serde_json::from_slice(&view_json_out.stdout).expect("valid JSON from report view --json");
    let reports = reports_json.as_array().expect("reports array");
    assert_eq!(reports.len(), 1);
    assert_eq!(reports[0]["task_id"], "T-101");
    assert_eq!(reports[0]["session_id"], "agent-1");
    assert_eq!(reports[0]["pr_url"], pr_url);
    assert_eq!(reports[0]["commit_sha"], commit_sha);

    let _ = daemon.kill();
    let _ = daemon.wait();
    let _ = std::fs::remove_dir_all(&root);
}
